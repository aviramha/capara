use pyo3::ffi::{
    PyContextVar_Get, PyEval_SetProfile, PyFrameObject, PyUnicode_AsUTF8, Py_tracefunc,
    CO_ASYNC_GENERATOR, CO_COROUTINE, CO_ITERABLE_COROUTINE,
};
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use pyo3::{AsPyPointer, PyAny, Python};
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::ffi::CStr;
use std::os::raw::c_int;
use std::time::Instant;

/// Enum of possible Python's Trace/Profiling events
#[allow(dead_code)]
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
    __NonExhaustive,
}

impl TryFrom<c_int> for TraceEvent {
    type Error = &'static str;
    /// Cast i32 event (raw from Python) to Rust enum.
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if value > 7 || value < 0 {
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
    yielded_coroutine: bool,
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
        let yielded_coroutine = {
            (0 < (code.co_flags & CO_COROUTINE)
                | (code.co_flags & CO_ITERABLE_COROUTINE)
                | (code.co_flags & CO_ASYNC_GENERATOR))
                && (!dframe.f_stacktop.is_null())
        };

        unsafe {
            let file_name = match code.co_filename.is_null() {
                true => "null".to_string(),
                false => CStr::from_ptr(PyUnicode_AsUTF8(code.co_filename))
                    .to_string_lossy()
                    .into_owned(),
            };
            let func_name = match code.co_name.is_null() {
                true => "null".to_string(),
                false => CStr::from_ptr(PyUnicode_AsUTF8(code.co_name))
                    .to_string_lossy()
                    .into_owned(),
            };
            Ok(FrameData {
                func_name,
                file_name,
                identifier: frame as usize,
                yielded_coroutine,
            })
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
}

/// Profiler context to be used as a value for ContextVar.
#[pyclass]
struct ProfilerContext {
    entries: std::cell::Cell<HashMap<usize, ProfilerEntry>>,
}

/// Format an entry into file_name, func_name and duration.
fn format_entry(entry: &ProfilerEntry) -> (String, String, Option<u128>) {
    let duration = match entry.end {
        Some(v) => Some(v.duration_since(entry.start).as_nanos()),
        None => None,
    };
    (entry.file_name.clone(), entry.func_name.clone(), duration)
}

#[pymethods]
impl ProfilerContext {
    #[new]
    fn new() -> Self {
        ProfilerContext {
            entries: std::cell::Cell::new(HashMap::new()),
        }
    }
    #[getter]
    fn entries(&mut self) -> PyResult<Vec<(String, String, Option<u128>)>> {
        let mut result = Vec::new();
        for (_, entry) in self.entries.get_mut().iter() {
            result.push(format_entry(entry));
        }
        Ok(result)
    }
}

fn get_context(py: Python, obj: *mut pyo3::ffi::PyObject) -> Option<Py<ProfilerContext>> {
    let mut context_value: *mut pyo3::ffi::PyObject = std::ptr::null_mut();

    unsafe {
        match PyContextVar_Get(
            obj,
            std::ptr::null_mut(),
            &mut context_value as *mut *mut pyo3::ffi::PyObject,
        ) {
            0 => (),
            _ => return None,
        };
    }

    unsafe { Py::from_owned_ptr_or_opt(py, context_obj) }
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
            let start = Instant::now();
            let entry = ProfilerEntry {
                func_name: frame_data.func_name,
                file_name: frame_data.file_name,
                start,
                end: None,
            };
            context
                .entries
                .get_mut()
                .insert(frame_data.identifier, entry);
        }
        TraceEvent::Return => {
            if frame_data.yielded_coroutine {
                return 0;
            }
            if let Some(entry) = context.entries.get_mut().get_mut(&frame_data.identifier) {
                entry.end = Some(Instant::now());
            };
        }
        _ => println!("shouldn't happen"),
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
/// A Python module implemented in Rust.
fn capara(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(start, m)?)?;
    m.add_class::<ProfilerContext>()?;
    Ok(())
}
