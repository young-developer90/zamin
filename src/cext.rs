use std::ffi::CString;
use std::path::Path;
use std::rc::Rc;

use crate::gc::*;

#[repr(C)]
#[derive(Clone)]
pub struct ZaminValue {
    tag: i32,
    data: ZaminValueData,
}

#[repr(C)]
#[derive(Clone, Copy)]
union ZaminValueData {
    as_int: i64,
    as_float: f64,
    as_bool: u8,
    as_str: StrData,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StrData {
    ptr: *const u8,
    len: usize,
}

fn zamin_int(n: i64) -> ZaminValue {
    ZaminValue { tag: 1, data: ZaminValueData { as_int: n } }
}

fn zamin_float(f: f64) -> ZaminValue {
    ZaminValue { tag: 2, data: ZaminValueData { as_float: f } }
}

fn zamin_bool(b: bool) -> ZaminValue {
    ZaminValue { tag: 3, data: ZaminValueData { as_bool: b as u8 } }
}

fn zamin_nil() -> ZaminValue {
    ZaminValue { tag: 0, data: ZaminValueData { as_int: 0 } }
}

fn value_to_zamin(v: &Value, heap: &GcHeap) -> (ZaminValue, Option<Vec<u8>>) {
    match v {
        Value::Nil => (zamin_nil(), None),
        Value::Int(n) => (zamin_int(*n), None),
        Value::Float(f) => (zamin_float(*f), None),
        Value::Bool(b) => (zamin_bool(*b), None),
        Value::String(_) => {
            let s = v.to_string(heap);
            let owned = s.as_bytes().to_vec();
            let len = owned.len();
            let ptr = owned.as_ptr();
            (ZaminValue { tag: 4, data: ZaminValueData { as_str: StrData { ptr, len } } }, Some(owned))
        }
        _ => (zamin_nil(), None),
    }
}

unsafe fn zamin_to_value(lv: &ZaminValue, heap: &mut GcHeap) -> Value {
    match lv.tag {
        0 => Value::Nil,
        1 => Value::Int(lv.data.as_int),
        2 => Value::Float(lv.data.as_float),
        3 => Value::Bool(lv.data.as_bool != 0),
        4 => {
            let bytes = std::slice::from_raw_parts(lv.data.as_str.ptr, lv.data.as_str.len);
            let s = std::str::from_utf8_unchecked(bytes);
            make_string(heap, s)
        }
        _ => Value::Nil,
    }
}

type InitFunc = unsafe extern "C" fn(count: *mut i32, funcs: *mut *mut LibFunc) -> i32;

#[repr(C)]
struct LibFunc {
    name: *const u8,
    func: Option<unsafe extern "C" fn(i32, *const ZaminValue) -> ZaminValue>,
}

#[cfg(unix)]
extern "C" {
    fn dlopen(filename: *const u8, flag: i32) -> *mut std::ffi::c_void;
    fn dlsym(handle: *mut std::ffi::c_void, symbol: *const u8) -> *mut std::ffi::c_void;
    fn dlclose(handle: *mut std::ffi::c_void) -> i32;
}

#[cfg(unix)]
const RTLD_NOW: i32 = 2;

#[cfg(windows)]
extern "system" {
    fn LoadLibraryA(lpFileName: *const u8) -> isize;
    fn GetProcAddress(hModule: isize, lpProcName: *const u8) -> isize;
    fn FreeLibrary(hModule: isize) -> i32;
    fn GetLastError() -> u32;
    fn FormatMessageA(
        dwFlags: u32, lpSource: isize, dwMessageId: u32,
        dwLanguageId: u32, lpBuffer: *mut u8, nSize: u32, Arguments: isize,
    ) -> u32;
}

struct LibHandle {
    #[cfg(unix)]
    ptr: *mut std::ffi::c_void,
    #[cfg(windows)]
    ptr: isize,
}

unsafe impl Send for LibHandle {}
unsafe impl Sync for LibHandle {}

fn open_lib(path: &Path) -> Result<LibHandle, String> {
    let cpath = CString::new(path.to_str().ok_or("invalid path")?).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        let handle = unsafe { dlopen(cpath.as_ptr(), RTLD_NOW) };
        if handle.is_null() {
            return Err(format!("dlopen failed: {}", path.display()));
        }
        Ok(LibHandle { ptr: handle })
    }
    #[cfg(windows)]
    {
        let handle = unsafe { LoadLibraryA(cpath.as_ptr() as *const u8) };
        if handle == 0 {
            let err = last_error_str();
            return Err(format!("LoadLibraryA failed: {} ({})", path.display(), err));
        }
        Ok(LibHandle { ptr: handle })
    }
}

fn find_sym(handle: &LibHandle, name: &str) -> Result<*mut std::ffi::c_void, String> {
    let cname = CString::new(name).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        let ptr = unsafe { dlsym(handle.ptr, cname.as_ptr()) };
        if ptr.is_null() {
            return Err(format!("symbol '{}' not found", name));
        }
        Ok(ptr)
    }
    #[cfg(windows)]
    {
        let ptr = unsafe { GetProcAddress(handle.ptr, cname.as_ptr() as *const u8) };
        if ptr == 0 {
            return Err(format!("symbol '{}' not found", name));
        }
        Ok(ptr as *mut _)
    }
}

fn close_lib(handle: LibHandle) {
    #[cfg(unix)]
    unsafe { dlclose(handle.ptr); }
    #[cfg(windows)]
    unsafe { FreeLibrary(handle.ptr); }
}

#[cfg(windows)]
fn last_error_str() -> String {
    unsafe {
        let err = GetLastError();
        let mut buf = [0u8; 512];
        let len = FormatMessageA(0x00001000, 0, err, 0, buf.as_mut_ptr(), buf.len() as u32, 0);
        if len > 0 {
            String::from_utf8_lossy(&buf[..len as usize]).trim().to_string()
        } else {
            format!("error code {}", err)
        }
    }
}

static LOADED_LIBS: std::sync::Mutex<Vec<LibHandle>> = std::sync::Mutex::new(Vec::new());

pub fn load_extension(path: &Path, _heap: &mut GcHeap) -> Result<Vec<(String, Value)>, String> {
    let lib = open_lib(path)?;

    let init_ptr = find_sym(&lib, "zamin_module_init")?;
    let init: InitFunc = unsafe { std::mem::transmute(init_ptr) };

    let mut count: i32 = 0;
    let mut funcs: *mut LibFunc = std::ptr::null_mut();
    let result = unsafe { init(&mut count, &mut funcs) };
    if result != 0 {
        close_lib(lib);
        return Err(format!("C extension init failed with code {}", result));
    }

    let mut registered = Vec::new();
    if count > 0 && !funcs.is_null() {
        for i in 0..count as isize {
            let entry = unsafe { &*funcs.offset(i) };
            if entry.name.is_null() {
                continue;
            }
            let name = unsafe { std::ffi::CStr::from_ptr(entry.name).to_string_lossy().into_owned() };
            if let Some(c_func) = entry.func {
                let native_name = format!("<ext.{}>", name);
                let func = Rc::new(move |args: &[Value], ctx: &mut VmContext| {
                    let mut c_args: Vec<ZaminValue> = Vec::with_capacity(args.len());
                    let mut guards: Vec<Vec<u8>> = Vec::new();
                    for arg in args {
                        let (lv, owned) = value_to_zamin(arg, ctx.heap);
                        if let Some(owned) = owned {
                            guards.push(owned);
                        }
                        c_args.push(lv);
                    }
                    let result = unsafe { (c_func)(c_args.len() as i32, c_args.as_ptr()) };
                    Ok(unsafe { zamin_to_value(&result, ctx.heap) })
                });
                registered.push((name, Value::NativeFunc(NativeFunc { name: native_name, func })));
            }
        }
    }

    if let Ok(mut libs) = LOADED_LIBS.lock() {
        libs.push(lib);
    }

    Ok(registered)
}
