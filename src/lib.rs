use pyo3::ffi::{
    PyContextVar_Get, PyEval_SetProfile, PyFrameObject, PyUnicode_AsUTF8, Py_tracefunc,
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

/// Extracts FrameData from FFI PyFrameObject
/// # Arguments
/// * ``frame`` - FFI Frame object pointer
fn extract_from_frame(frame: *mut PyFrameObject) -> Option<FrameData> {
    if frame.is_null() {
        return None;
    }

    let dframe = unsafe { *frame };

    if dframe.f_code.is_null() {
        return None;
    }

    let code = unsafe { *dframe.f_code };
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
        Some(FrameData {
            func_name,
            file_name,
            identifier: frame as usize,
        })
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

    let gil = Python::acquire_gil();
    let py = gil.python();

    let frame_data = match extract_from_frame(frame) {
        Some(v) => v,
        None => return 0,
    };

    let mut context_value: *mut pyo3::ffi::PyObject = std::ptr::null_mut();

    unsafe {
        match PyContextVar_Get(
            obj,
            std::ptr::null_mut(),
            &mut context_value as *mut *mut pyo3::ffi::PyObject,
        ) {
            0 => (),
            _ => return 0,
        };
    }

    if context_value.is_null() {
        return 0;
    }

    let context_obj =
        match unsafe { Py::<ProfilerContext>::from_borrowed_ptr_or_opt(py, context_value) } {
            Some(v) => v,
            None => return 0,
        };

    let mut context = context_obj.as_ref(py).borrow_mut();

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
            match context.entries.get_mut().get_mut(&frame_data.identifier) {
                Some(entry) => {
                    entry.end = Some(Instant::now());
                }
                None => println!("shouldn't happen"),
            };
        }
        _ => println!("shouldn't happen"),
    }

    unsafe {
        pyo3::ffi::Py_XDECREF(context_value);
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
