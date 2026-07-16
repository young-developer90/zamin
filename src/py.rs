use std::rc::Rc;
use std::sync::Mutex;
use std::sync::OnceLock;

use pyo3::conversion::IntoPyObject;
use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::types::PyTuple;
use pyo3::types::PyDict;
use pyo3::types::PyList;

use crate::gc::*;

type PyObj = Py<PyAny>;

static PY_HANDLES: OnceLock<Mutex<Vec<PyObj>>> = OnceLock::new();

fn get_handles() -> &'static Mutex<Vec<PyObj>> {
    PY_HANDLES.get_or_init(|| Mutex::new(Vec::new()))
}

fn store_py_obj(obj: Bound<'_, PyAny>) -> usize {
    let raw = obj.as_ptr() as usize;
    get_handles().lock().unwrap().push(obj.unbind());
    raw
}

fn value_to_py(val: &Value, py: Python<'_>, heap: &GcHeap) -> PyObj {
    let result: PyObj = match val {
        Value::Int(n) => n.into_pyobject(py).unwrap().unbind().into(),
        Value::UInt(n) => (*n as i64).into_pyobject(py).unwrap().unbind().into(),
        Value::Float(n) => n.into_pyobject(py).unwrap().unbind().into(),
        Value::Bool(b) => (*b as i64).into_pyobject(py).unwrap().unbind().into(),
        Value::Nil => py.None(),
        Value::String(r) => {
            if let GcObj::String(s) = heap.get(*r) {
                s.into_pyobject(py).unwrap().unbind().into()
            } else {
                py.None()
            }
        }
        Value::List(r) => {
            if let GcObj::List(items) = heap.get(*r) {
                let py_items: Vec<PyObj> = items.iter().map(|v| value_to_py(v, py, heap)).collect();
                PyList::new(py, py_items).unwrap().into_any().unbind()
            } else {
                py.None()
            }
        }
        Value::Tuple(r) => {
            if let GcObj::Tuple(items) = heap.get(*r) {
                let py_vec: Vec<PyObj> = items.iter().map(|v| value_to_py(v, py, heap)).collect();
                PyTuple::new(py, py_vec).unwrap().into_any().unbind()
            } else {
                py.None()
            }
        }
        Value::Dict(r) => {
            if let GcObj::Dict(entries) = heap.get(*r) {
                let d = PyDict::new(py);
                for (k, v) in entries {
                    d.set_item(value_to_py(k, py, heap), value_to_py(v, py, heap)).ok();
                }
                d.into_any().unbind()
            } else {
                py.None()
            }
        }
        _ => py.None(),
    };
    result
}

fn py_value_to_zamin(obj: &Bound<'_, PyAny>, heap: &mut GcHeap, py: Python<'_>) -> Result<Value, String> {
    if let Ok(s) = obj.extract::<String>() {
        return Ok(make_string(heap, &s));
    }
    if let Ok(i) = obj.extract::<i64>() {
        return Ok(Value::Int(i));
    }
    if let Ok(f) = obj.extract::<f64>() {
        return Ok(Value::Float(f));
    }
    if let Ok(b) = obj.extract::<bool>() {
        return Ok(Value::Bool(b));
    }
    if obj.is_none() {
        return Ok(Value::Nil);
    }
    if let Ok(items) = obj.cast::<PyList>() {
        let mut zamin_items = Vec::new();
        for item in items.iter() {
            zamin_items.push(py_value_to_zamin(&item, heap, py)?);
        }
        return Ok(make_list(heap, zamin_items));
    }
    Ok(wrap_py_callable(obj, heap, py))
}

fn wrap_py_callable(obj: &Bound<'_, PyAny>, heap: &mut GcHeap, _py: Python<'_>) -> Value {
    let obj_id = store_py_obj(obj.clone());

    let mut entries: Vec<(Value, Value)> = Vec::new();
    entries.push((make_string(heap, "__pyobj__"), Value::Int(obj_id as i64)));
    entries.push((make_string(heap, "__call__"), Value::NativeFunc(NativeFunc {
        name: "<py.call>".to_string(),
        func: Rc::new(move |args, ctx| call_py_function(obj_id, args, ctx)),
    })));

    Value::Dict(heap.alloc(GcObj::Dict(entries)))
}

fn call_py_function(obj_id: usize, args: &[Value], ctx: &mut VmContext) -> Result<Value, String> {
    let raw_ptr: *mut ffi::PyObject = {
        let handles = get_handles().lock().map_err(|e| format!("py lock: {}", e))?;
        let idx = handles.iter().position(|o| o.as_ptr() as usize == obj_id)
            .ok_or("Python object not found")?;
        handles.get(idx).unwrap().as_ptr()
    };

    Python::attach(|py| {
        let bound: Borrowed<'_, '_, PyAny> = unsafe { Borrowed::from_ptr(py, raw_ptr) };
        let py_args: Vec<PyObj> = args.iter().map(|a| value_to_py(a, py, ctx.heap)).collect();
        let py_tuple = PyTuple::new(py, py_args)
            .map_err(|e| format!("Python tuple error: {}", e))?;
        let result = bound.call(py_tuple, None)
            .map_err(|e| format!("Python error: {}", e))?;
        py_value_to_zamin(&result, ctx.heap, py)
    })
}

fn py_import(module_name: &str, heap: &mut GcHeap) -> Result<Value, String> {
    Python::attach(|py| {
        let module = PyModule::import(py, module_name)
            .map_err(|e| format!("Python import error: {}", e))?;
        Ok(wrap_py_callable(&module, heap, py))
    })
}

pub fn py_get_attr(obj_id: i64, name: &str, heap: &mut GcHeap) -> Result<Value, String> {
    let raw_ptr: *mut ffi::PyObject = {
        let handles = get_handles().lock().map_err(|e| format!("py lock: {}", e))?;
        let idx = handles.iter().position(|o| o.as_ptr() as usize == obj_id as usize)
            .ok_or("Python object not found")?;
        handles.get(idx).unwrap().as_ptr()
    };

    Python::attach(|py| {
        let bound: Borrowed<'_, '_, PyAny> = unsafe { Borrowed::from_ptr(py, raw_ptr) };
        let attr = bound.getattr(name)
            .map_err(|e| format!("Python error accessing '{}': {}", name, e))?;
        py_value_to_zamin(&attr, heap, py)
    })
}

pub fn build_py() -> Vec<(String, Value)> {
    let mut items = Vec::new();
    items.push(("import".to_string(), Value::NativeFunc(NativeFunc {
        name: "<py.import>".to_string(),
        func: Rc::new(move |args, ctx| {
            let name = args.first().ok_or("py.import requires a module name")?.to_string(ctx.heap);
            py_import(&name, ctx.heap)
        }),
    })));
    items.push(("version".to_string(), Value::NativeFunc(NativeFunc {
        name: "<py.version>".to_string(),
        func: Rc::new(|_, _| {
            Python::attach(|_| {
                let v = Python::version_str();
                let mut h = GcHeap::new();
                Ok(make_string(&mut h, v))
            })
        }),
    })));
    items
}
