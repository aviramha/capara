use pyo3::ffi::{
    PyContextVar_Get, PyEval_SetProfile, PyFrameObject, PyUnicode_AsUTF8, Py_tracefunc,
};
use pyo3::prelude::*;
use pyo3::types::*;
use pyo3::wrap_pyfunction;
use pyo3::{AsPyPointer, PyAny, Python};
use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::c_int;
use std::time::{Duration, Instant};

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
    Opcode
}

struct FrameData {
    func_name: String,
    file_name: String,
    identifier: usize
}

#[cfg(Py_3_8)]
struct ProfilerEntry {
    file_name: String,
    func_name: String,
    start: Instant,
    end: Option<Instant>,
}

#[pyclass]
struct ProfilerContext {
    entries: std::cell::Cell<HashMap<usize, ProfilerEntry>>,
}

fn format_entry(entry: &ProfilerEntry) -> (String, String, Option<u128>) {
    let duration = match entry.end {
        Some(v) => Some(v.duration_since(entry.start).as_nanos()),
        None => None
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


fn extract_context(context_var: *mut pyo3::ffi::PyObject) -> Option<ProfilerContext> {
    let gil = Python::acquire_gil();
    let mut context_value: *mut pyo3::ffi::PyObject = std::ptr::null_mut();

    unsafe {
        match PyContextVar_Get(
            context_var,
            std::ptr::null_mut(),
            &mut context_value as *mut *mut pyo3::ffi::PyObject,
        ) {
            0 => (),
            _ => return None
        };
    }

    if context_value.is_null() {
        return None
    }
    let context = match unsafe { Py::<ProfilerContext>::from_borrowed_ptr_or_opt(gil.python(), context_value)} {
        Some(v) => v,
        None => return None
    };

    let cell = context.as_ref(gil.python());
    let mut ctx = cell.borrow_mut();

    unsafe {pyo3::ffi::Py_XDECREF(context_value);}
    ctx
}

fn extract_from_frame(frame: *mut PyFrameObject) -> Option<FrameData> {
    if frame.is_null() {
        return None
    }

    let dframe = *frame;

    if dframe.f_code.is_null() {
        return None
    }

    let code = *dframe.f_code;
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
        Some(FrameData{func_name, file_name, identifier: frame as usize})
    }
}

extern "C" fn callback(
    obj: *mut pyo3::ffi::PyObject,
    frame: *mut PyFrameObject,
    what: c_int,
    arg: *mut pyo3::ffi::PyObject,
) -> c_int {
    let what: TraceEvent = unsafe {std::mem::transmute(what)};
    match what {
        TraceEvent::Call | TraceEvent::Return => (),
        _ => return 0,
    };

    let context = match extract_context(obj) {
        Some(v) => v,
        None => return 0,
    };

    let frame_data = match extract_from_frame(frame) {
        Some(v) => v,
        None => return 0,
    };


    let start = Instant::now();
    let entry = ProfilerEntry {
        func_name: frame_data.func_name,
        file_name: frame_data.file_name,
            start,
            end: None,
        };
        context.entries.get_mut().insert(frame_data.identifier, entry);

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

// #[pyclass]
// struct Profiler {
//     frames: Hash
// }
#[pymodule]
/// A Python module implemented in Rust.
fn capara(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(start, m)?)?;
    m.add_class::<ProfilerContext>()?;
    Ok(())
}
