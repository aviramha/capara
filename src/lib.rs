use pyo3::ffi::{
    PyContextVar_Get, PyEval_SetProfile, PyFrameObject, PyUnicode_AsUTF8, Py_tracefunc,
    CO_ASYNC_GENERATOR, CO_COROUTINE, CO_ITERABLE_COROUTINE,
};
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use pyo3::{AsPyPointer, PyAny, Python};
use std::cell::Cell;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::ffi::CStr;
use std::os::raw::c_int;
use std::time::Instant;

/// Enum of possible Python's Trace/Profiling events
#[allow(dead_code)]
#[non_exhaustive]
#[repr(i32)]
enum TraceEvent {
    Call,
    Exception,
    Line,
    Return,
    CCall,
    CException,
    CReturn,
    Opcode,
}

const LARGER_THAN_ANY_TRACE_EVENT: i32 = TraceEvent::Opcode as i32 + 1;

impl TryFrom<c_int> for TraceEvent {
    type Error = &'static str;
    /// Cast i32 event (raw from Python) to Rust enum.
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if value >= LARGER_THAN_ANY_TRACE_EVENT || value < 0 {
            return Err("Not valid enum value");
        }
        Ok(unsafe { std::mem::transmute(value) })
    }
}

/// Struct representing frame data
/// Identifier is frame pointer casted to usize.
struct FrameData {
    func_name: String,
    file_name: String,
    identifier: usize,
    is_yielded_coroutine: bool,
}

impl TryFrom<*mut PyFrameObject> for FrameData {
    type Error = &'static str;
    fn try_from(frame: *mut PyFrameObject) -> Result<Self, Self::Error> {
        if frame.is_null() {
            return Err("frame is null");
        }

        let dframe = unsafe { *frame };

        if dframe.f_code.is_null() {
            return Err("f_code is null");
        }

        let code = unsafe { *dframe.f_code };
        let is_yielded_coroutine = {
            (code.co_flags & (CO_COROUTINE | CO_ITERABLE_COROUTINE | CO_ASYNC_GENERATOR) > 0)
                && (!dframe.f_stacktop.is_null())
        };

        let file_name = pyo3_to_string(code.co_filename);
        let func_name = pyo3_to_string(code.co_name);
        Ok(FrameData {
            func_name,
            file_name,
            identifier: frame as usize,
            is_yielded_coroutine,
        })
    }
}

fn pyo3_to_string(obj: *mut pyo3::ffi::PyObject) -> String {
    unsafe {
        match obj.is_null() {
            true => "<null>".to_string(),
            false => CStr::from_ptr(PyUnicode_AsUTF8(obj))
                .to_string_lossy()
                .into_owned(),
        }
    }
}

/// end can be None due to lack of Return callback
#[cfg(Py_3_8)]
struct ProfilerEntry {
    file_name: String,
    func_name: String,
    start: Instant,
    end: Option<Instant>,
    index: i32,
}

/// Profiler context to be used as a value for ContextVar.
#[pyclass]
struct ProfilerContext {
    entries: Cell<HashMap<usize, ProfilerEntry>>,
    count: i32,
}

/// Format an entry into file_name, func_name and duration.
fn format_entry(entry: &ProfilerEntry) -> (String, String, Option<u128>, i32) {
    let duration = entry
        .end
        .and_then(|v| Some(v.duration_since(entry.start).as_nanos()));
    (
        entry.file_name.clone(),
        entry.func_name.clone(),
        duration,
        entry.index,
    )
}

#[pymethods]
impl ProfilerContext {
    #[new]
    fn new() -> Self {
        ProfilerContext {
            entries: Cell::new(HashMap::new()),
            count: 0,
        }
    }
    #[getter]
    fn entries(&mut self) -> PyResult<Vec<(String, String, Option<u128>, i32)>> {
        Ok(self
            .entries
            .get_mut()
            .iter()
            .map(|(_, entry)| format_entry(entry))
            .collect())
    }
}

fn get_context(py: Python, obj: *mut pyo3::ffi::PyObject) -> Option<Py<ProfilerContext>> {
    let mut context_obj: *mut pyo3::ffi::PyObject = std::ptr::null_mut();

    unsafe {
        match PyContextVar_Get(obj, std::ptr::null_mut(), &mut context_obj) {
            0 => (),
            _ => return None,
        };
    }
    let context = unsafe { Py::from_owned_ptr_or_opt(py, context_obj)? };
    if context.is_none(py) {
        None
    } else {
        Some(context)
    }
}

/// Our profiler callback
extern "C" fn callback(
    obj: *mut pyo3::ffi::PyObject,
    frame: *mut PyFrameObject,
    what: c_int,
    _arg: *mut pyo3::ffi::PyObject,
) -> c_int {
    let event: TraceEvent = match what.try_into() {
        Ok(event) => match event {
            TraceEvent::Call | TraceEvent::Return => event,
            _ => return 0,
        },
        _ => return 0,
    };

    let frame_data = match FrameData::try_from(frame) {
        Ok(v) => v,
        Err(err) => {
            println!("{}", err);
            return 0;
        }
    };

    let gil = Python::acquire_gil();
    let py = gil.python();

    let context = match get_context(py, obj) {
        Some(v) => v,
        _ => return 0,
    };
    let mut context = context.as_ref(py).borrow_mut();

    match event {
        TraceEvent::Call => {
            // Frame already exists in hashmap, means that we're in a yielded function.

            if context
                .entries
                .get_mut()
                .contains_key(&frame_data.identifier)
            {
                return 0;
            }

            let start = Instant::now();
            let entry = ProfilerEntry {
                func_name: frame_data.func_name,
                file_name: frame_data.file_name,
                start,
                end: None,
                index: context.count,
            };

            context
                .entries
                .get_mut()
                .insert(frame_data.identifier, entry);

            context.count += 1;
        }
        TraceEvent::Return => {
            if frame_data.is_yielded_coroutine {
                return 0;
            }

            if let Some(entry) = context.entries.get_mut().get_mut(&frame_data.identifier) {
                entry.end = Some(Instant::now());
            };
        }
        _ => unreachable!(),
    }
    0
}

#[pyfunction]
fn start(context_var: &PyAny) -> PyResult<()> {
    let cb: Py_tracefunc = callback;
    unsafe {
        PyEval_SetProfile(cb, context_var.as_ptr());
    }
    Ok(())
}

#[pymodule]
fn capara(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(start, m)?)?;
    m.add_class::<ProfilerContext>()?;
    Ok(())
}
