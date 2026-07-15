use std::collections::HashMap;
use std::ffi::{CStr, CString};

use crate::bytecode::*;
use crate::gc::Value;

type JitFn = unsafe extern "C" fn(*mut f64, i64, *mut f64, i64) -> i64;

const HOT_THRESHOLD: usize = 3;

struct CompiledFn {
    func_ptr: JitFn,
    #[allow(dead_code)]
    so_handle: *mut std::ffi::c_void,
    #[allow(dead_code)]
    _source: String,
}

pub struct JitCache {
    call_counts: Vec<usize>,
    compiled: HashMap<usize, CompiledFn>,
    chunks: *const Vec<crate::bytecode::Chunk>,
}

unsafe impl Send for JitCache {}
unsafe impl Sync for JitCache {}

impl JitCache {
    pub fn new(chunks: &Vec<crate::bytecode::Chunk>) -> Self {
        JitCache {
            call_counts: vec![0; chunks.len()],
            compiled: HashMap::new(),
            chunks: chunks as *const Vec<crate::bytecode::Chunk>,
        }
    }

    pub fn record_call(&mut self, chunk_idx: usize) {
        if chunk_idx >= self.call_counts.len() {
            return;
        }
        if self.compiled.contains_key(&chunk_idx) {
            return;
        }
        if self.call_counts[chunk_idx] == usize::MAX {
            return;
        }
        self.call_counts[chunk_idx] += 1;
        if self.call_counts[chunk_idx] >= HOT_THRESHOLD {
            self.call_counts[chunk_idx] = usize::MAX;
            match self.try_compile(chunk_idx) {
                Ok(()) => {
                    self.call_counts[chunk_idx] = 0;
                    eprintln!("JIT compiled chunk {}", chunk_idx);
                }
                Err(e) => eprintln!("JIT compile failed for chunk {}: {}", chunk_idx, e),
            }
        }
    }

    pub fn execute(&self, chunk_idx: usize, args: &[f64], nlocals: usize) -> Option<Vec<f64>> {
        let compiled = self.compiled.get(&chunk_idx)?;
        let mut stack = vec![0.0f64; 256];
        let mut locals = vec![0.0f64; nlocals.max(1)];

        for (i, arg) in args.iter().enumerate() {
            if i < locals.len() {
                locals[i] = *arg;
            }
        }

        let result_sp = unsafe {
            (compiled.func_ptr)(stack.as_mut_ptr(), 0, locals.as_mut_ptr(), nlocals as i64)
        };

        if result_sp < 0 || result_sp as usize >= stack.len() {
            return None;
        }
        if result_sp == 0 {
            return Some(vec![]);
        }
        Some(stack[..result_sp as usize].to_vec())
    }

    pub fn can_execute(&self, chunk_idx: usize) -> bool {
        self.compiled.contains_key(&chunk_idx)
    }

    fn try_compile(&mut self, chunk_idx: usize) -> Result<(), String> {
        let chunks = unsafe { &*self.chunks };
        let chunk = chunks.get(chunk_idx).ok_or("invalid chunk index")?;
        let c_code = translate_chunk(chunk, chunk_idx)?;
        let (func_ptr, so_handle) = compile_c(&c_code, chunk_idx)?;
        self.compiled.insert(
            chunk_idx,
            CompiledFn {
                func_ptr,
                so_handle,
                _source: c_code,
            },
        );
        Ok(())
    }
}

fn constant_to_f64(val: &Value) -> Option<f64> {
    match val {
        Value::Int(n) => Some(*n as f64),
        Value::UInt(n) => Some(*n as f64),
        Value::Float(n) => Some(*n),
        Value::Bool(true) => Some(1.0),
        Value::Bool(false) => Some(0.0),
        Value::Nil => Some(0.0),
        _ => None,
    }
}

fn is_jittable(chunk: &Chunk) -> bool {
    let code = &chunk.code;
    let mut i = 0;
    while i < code.len() {
        let op = OpCode::from_u8(code[i]).unwrap_or(OpCode::Halt);
        match op {
            OpCode::Halt | OpCode::Pop | OpCode::Dup
            | OpCode::Nil | OpCode::True | OpCode::False
            | OpCode::Return
            | OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div
            | OpCode::Mod | OpCode::Pow | OpCode::IntDiv
            | OpCode::Neg | OpCode::Not
            | OpCode::Eq | OpCode::Ne | OpCode::Lt | OpCode::Gt
            | OpCode::Le | OpCode::Ge
            | OpCode::And | OpCode::Or => {
                i += 1;
            }
            OpCode::LoadConst | OpCode::LoadLocal | OpCode::StoreLocal
            | OpCode::Jump | OpCode::JumpIfTrue | OpCode::JumpIfFalse
            | OpCode::JumpIfNil => {
                if op == OpCode::LoadConst {
                    let idx = u16::from_le_bytes([code[i + 1], code[i + 2]]) as usize;
                    if constant_to_f64(&chunk.constants[idx]).is_none() {
                        return false;
                    }
                }
                i += 3;
            }
            _ => return false,
        }
    }
    true
}

fn translate_chunk(chunk: &Chunk, chunk_idx: usize) -> Result<String, String> {
    if !is_jittable(chunk) {
        return Err("chunk is not jittable".to_string());
    }

    let code = &chunk.code;
    let mut c_lines: Vec<String> = Vec::new();
    let name = chunk.name.as_deref().unwrap_or("anonymous");
    let indent = "    ";

    // Track the virtual stack of C expressions
    let mut virt: Vec<String> = Vec::new();

    // Declare C local variables for each bytecode local
    let nlocals = chunk.locals.max(1);
    let mut local_vars: Vec<String> = (0..nlocals).map(|i| format!("l{}", i)).collect();

    let mut header = Vec::new();
    header.push(format!("/* JIT compiled: chunk {} ({}) */", chunk_idx, name));
    header.push("#include <math.h>".to_string());
    header.push("#include <stdint.h>".to_string());
    header.push("static inline int truthy(double x) {".to_string());
    header.push("    return x != 0.0 && x == x;".to_string());
    header.push("}".to_string());
    header.push(format!(
        "int64_t jit_chunk_{}(double *stack, int64_t sp, double *locals, int64_t nlocals) {{",
        chunk_idx
    ));

    // Copy locals into C local variables
    for (i, v) in local_vars.iter().enumerate() {
        header.push(format!("{}double {} = locals[{}];", indent, v, i));
    }

    c_lines = header;

    // Helper to emit a statement and manage virt
    macro_rules! emit {
        ($fmt:expr $(, $arg:expr)*) => {
            c_lines.push(format!(concat!("{}", $fmt), indent $(, $arg)*));
        };
    }
    macro_rules! push {
        ($expr:expr) => { virt.push(format!("{}", $expr)); };
    }
    macro_rules! pop {
        () => { virt.pop().unwrap_or_else(|| "0.0".to_string()) };
    }

    let mut i = 0;
    let mut label_counter = 0;
    let mut jump_targets: HashMap<usize, String> = HashMap::new();

    // First pass: collect jump targets
    let mut ti = 0;
    while ti < code.len() {
        let op = OpCode::from_u8(code[ti]).unwrap_or(OpCode::Halt);
        match op {
            OpCode::Jump | OpCode::JumpIfTrue | OpCode::JumpIfFalse | OpCode::JumpIfNil => {
                let target = u16::from_le_bytes([code[ti + 1], code[ti + 2]]) as usize;
                if !jump_targets.contains_key(&target) {
                    jump_targets.insert(target, format!("l{}", label_counter));
                    label_counter += 1;
                }
                ti += 3;
            }
            OpCode::LoadConst | OpCode::LoadLocal | OpCode::StoreLocal => ti += 3,
            _ => ti += 1,
        }
    }

    // Second pass: emit C with stack tracking
    while i < code.len() {
        // At jump targets, flush virtual stack to real stack
        if jump_targets.contains_key(&i) {
            // Save virt stack to real stack
            for (vi, expr) in virt.iter().enumerate() {
                emit!("stack[{}] = {};", vi, expr);
            }
            if !virt.is_empty() {
                emit!("sp = {};", virt.len());
            }
            emit!("{}:", jump_targets.get(&i).unwrap());
        }

        let op = OpCode::from_u8(code[i]).unwrap_or(OpCode::Halt);

        match op {
            OpCode::Halt => {
                emit!("return sp;");
                i += 1;
            }
            OpCode::Pop => {
                virt.pop();
                i += 1;
            }
            OpCode::Dup => {
                let v = virt.last().cloned().unwrap_or_else(|| "0.0".to_string());
                virt.push(v);
                i += 1;
            }
            OpCode::Nil | OpCode::True | OpCode::False => {
                let v = match op { OpCode::Nil => "0.0", OpCode::True => "1.0", _ => "0.0" };
                push!(v);
                i += 1;
            }
            OpCode::LoadConst => {
                let idx = u16::from_le_bytes([code[i + 1], code[i + 2]]) as usize;
                let val = constant_to_f64(&chunk.constants[idx]).unwrap();
                let repr = if val.fract() == 0.0 && val.is_finite() {
                    format!("{}.0", val as i64)
                } else if val.is_infinite() && val.is_sign_positive() {
                    "INFINITY".to_string()
                } else if val.is_infinite() && val.is_sign_negative() {
                    "-INFINITY".to_string()
                } else {
                    format!("{}", val)
                };
                push!(repr);
                i += 3;
            }
            OpCode::LoadLocal => {
                let idx = u16::from_le_bytes([code[i + 1], code[i + 2]]) as usize;
                let v = if idx < local_vars.len() { local_vars[idx].clone() } else { format!("locals[{}]", idx) };
                push!(v);
                i += 3;
            }
            OpCode::StoreLocal => {
                let idx = u16::from_le_bytes([code[i + 1], code[i + 2]]) as usize;
                let v = pop!();
                if idx < local_vars.len() {
                    emit!("{} = {};", local_vars[idx], v);
                } else {
                    emit!("locals[{}] = {};", idx, v);
                }
                i += 3;
            }
            OpCode::Add => {
                let b = pop!();
                let a = pop!();
                push!(format!("({} + {})", a, b));
                i += 1;
            }
            OpCode::Sub => {
                let b = pop!();
                let a = pop!();
                push!(format!("({} - {})", a, b));
                i += 1;
            }
            OpCode::Mul => {
                let b = pop!();
                let a = pop!();
                push!(format!("({} * {})", a, b));
                i += 1;
            }
            OpCode::Div => {
                let b = pop!();
                let a = pop!();
                push!(format!("({} / {})", a, b));
                i += 1;
            }
            OpCode::Mod => {
                let b = pop!();
                let a = pop!();
                push!(format!("fmod({}, {})", a, b));
                i += 1;
            }
            OpCode::Pow => {
                let b = pop!();
                let a = pop!();
                push!(format!("pow({}, {})", a, b));
                i += 1;
            }
            OpCode::IntDiv => {
                let b = pop!();
                let a = pop!();
                push!(format!("(double)((int64_t)({}) / (int64_t)({}))", a, b));
                i += 1;
            }
            OpCode::Neg => {
                let a = pop!();
                push!(format!("(-{})", a));
                i += 1;
            }
            OpCode::Not => {
                let a = pop!();
                push!(format!("(truthy({}) ? 0.0 : 1.0)", a));
                i += 1;
            }
            OpCode::Eq => {
                let b = pop!();
                let a = pop!();
                push!(format!("(double)(({}) == ({}))", a, b));
                i += 1;
            }
            OpCode::Ne => {
                let b = pop!();
                let a = pop!();
                push!(format!("(double)(({}) != ({}))", a, b));
                i += 1;
            }
            OpCode::Lt => {
                let b = pop!();
                let a = pop!();
                push!(format!("(double)(({}) < ({}))", a, b));
                i += 1;
            }
            OpCode::Gt => {
                let b = pop!();
                let a = pop!();
                push!(format!("(double)(({}) > ({}))", a, b));
                i += 1;
            }
            OpCode::Le => {
                let b = pop!();
                let a = pop!();
                push!(format!("(double)(({}) <= ({}))", a, b));
                i += 1;
            }
            OpCode::Ge => {
                let b = pop!();
                let a = pop!();
                push!(format!("(double)(({}) >= ({}))", a, b));
                i += 1;
            }
            OpCode::And => {
                let b = pop!();
                let a = pop!();
                let tmp = format!("_t{}", virt.len());
                emit!("double {} = truthy({}) ? ({}) : ({});", tmp, a, b, a);
                push!(tmp);
                i += 1;
            }
            OpCode::Or => {
                let b = pop!();
                let a = pop!();
                let tmp = format!("_t{}", virt.len());
                emit!("double {} = truthy({}) ? ({}) : ({});", tmp, a, a, b);
                push!(tmp);
                i += 1;
            }
            OpCode::Return => {
                let v = pop!();
                emit!("stack[0] = {};", v);
                emit!("return 1;");
                // Skip dead code after Return until end or Halt
                i += 1;
                while i < code.len() {
                    let next = OpCode::from_u8(code[i]).unwrap_or(OpCode::Halt);
                    if next == OpCode::Halt { i += 1; break; }
                    let skip = match next {
                        OpCode::LoadConst | OpCode::LoadLocal | OpCode::StoreLocal
                        | OpCode::Jump | OpCode::JumpIfTrue | OpCode::JumpIfFalse
                        | OpCode::JumpIfNil => 3,
                        _ => 1,
                    };
                    i += skip;
                }
            }
            OpCode::Jump => {
                let target = u16::from_le_bytes([code[i + 1], code[i + 2]]) as usize;
                // Flush virt stack before jump
                for (vi, expr) in virt.iter().enumerate() {
                    emit!("stack[{}] = {};", vi, expr);
                }
                if !virt.is_empty() {
                    emit!("sp = {};", virt.len());
                }
                let label = jump_targets.get(&target).unwrap();
                emit!("goto {};", label);
                i += 3;
            }
            OpCode::JumpIfTrue => {
                let target = u16::from_le_bytes([code[i + 1], code[i + 2]]) as usize;
                let v = pop!();
                // Flush remaining virt stack  
                for (vi, expr) in virt.iter().enumerate() {
                    emit!("stack[{}] = {};", vi, expr);
                }
                if !virt.is_empty() {
                    emit!("sp = {};", virt.len());
                }
                let label = jump_targets.get(&target).unwrap();
                emit!("if (truthy({})) goto {};", v, label);
                i += 3;
            }
            OpCode::JumpIfFalse => {
                let target = u16::from_le_bytes([code[i + 1], code[i + 2]]) as usize;
                let v = pop!();
                for (vi, expr) in virt.iter().enumerate() {
                    emit!("stack[{}] = {};", vi, expr);
                }
                if !virt.is_empty() {
                    emit!("sp = {};", virt.len());
                }
                let label = jump_targets.get(&target).unwrap();
                emit!("if (!truthy({})) goto {};", v, label);
                i += 3;
            }
            OpCode::JumpIfNil => {
                let target = u16::from_le_bytes([code[i + 1], code[i + 2]]) as usize;
                let v = pop!();
                for (vi, expr) in virt.iter().enumerate() {
                    emit!("stack[{}] = {};", vi, expr);
                }
                if !virt.is_empty() {
                    emit!("sp = {};", virt.len());
                }
                let label = jump_targets.get(&target).unwrap();
                emit!("if ({} == 0.0) goto {};", v, label);
                i += 3;
            }
            _ => unreachable!(), // is_jittable prevents this
        }
    }

    // Final flush and return
    for (vi, expr) in virt.iter().enumerate() {
        emit!("stack[{}] = {};", vi, expr);
    }
    if !virt.is_empty() {
        emit!("sp = {};", virt.len());
    }
    emit!("return sp;");
    c_lines.push("}".to_string());
    Ok(c_lines.join("\n"))
}

fn compile_c(source: &str, chunk_idx: usize) -> Result<(JitFn, *mut std::ffi::c_void), String> {
    let tmp = std::env::temp_dir();
    let lib_name = format!("lion_jit_{}", chunk_idx);
    let lib_ext = if cfg!(target_os = "windows") { "dll" } else { "so" };
    let lib_path = tmp.join(format!("{}.{}", lib_name, lib_ext));
    let c_path = tmp.join(format!("lion_jit_{}.c", chunk_idx));

    std::fs::write(&c_path, source).map_err(|e| format!("write C source: {}", e))?;

    if cfg!(target_os = "windows") {
        compile_c_windows(&c_path, &lib_path)?;
    } else {
        compile_c_unix(&c_path, &lib_path)?;
    }

    let lib_path_c = CString::new(lib_path.to_str().unwrap()).map_err(|e| e.to_string())?;

    let handle = if cfg!(target_os = "windows") {
        unsafe { load_library_windows(lib_path_c.as_ptr()) }
    } else {
        unsafe { dlopen(lib_path_c.as_ptr(), 2) }
    };

    if handle.is_null() {
        return Err(format!("failed to load dynamic library: {}", dlerror_str()));
    }

    let fn_name = CString::new(format!("jit_chunk_{}", chunk_idx)).map_err(|e| e.to_string())?;

    let ptr = if cfg!(target_os = "windows") {
        unsafe { get_proc_address_windows(handle, fn_name.as_ptr()) }
    } else {
        unsafe { dlsym(handle, fn_name.as_ptr()) }
    };

    if ptr.is_null() {
        return Err(format!("symbol lookup failed: {}", dlerror_str()));
    }

    let func_ptr: JitFn = unsafe { std::mem::transmute(ptr) };
    Ok((func_ptr, handle))
}

#[cfg(not(target_os = "windows"))]
fn compile_c_unix(c_path: &std::path::Path, lib_path: &std::path::Path) -> Result<(), String> {
    let output = std::process::Command::new("gcc")
        .args(&[
            "-O3", "-shared", "-fPIC", "-lm",
            "-o", lib_path.to_str().unwrap(),
            c_path.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| format!("run gcc: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gcc failed: {}", stderr));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn compile_c_windows(c_path: &std::path::Path, lib_path: &std::path::Path) -> Result<(), String> {
    // Try MSVC (cl.exe) first, then MinGW (gcc)
    if let Ok(output) = std::process::Command::new("cl.exe")
        .args(&[
            "/O2", "/LD",
            "/Fe", lib_path.to_str().unwrap(),
            c_path.to_str().unwrap(),
        ])
        .output()
    {
        if output.status.success() {
            return Ok(());
        }
    }

    if let Ok(output) = std::process::Command::new("gcc")
        .args(&[
            "-O3", "-shared",
            "-o", lib_path.to_str().unwrap(),
            c_path.to_str().unwrap(),
        ])
        .output()
    {
        if output.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gcc failed: {}", stderr));
    }

    if let Ok(output) = std::process::Command::new("clang")
        .args(&[
            "-O3", "-shared",
            "-o", lib_path.to_str().unwrap(),
            c_path.to_str().unwrap(),
        ])
        .output()
    {
        if output.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("clang failed: {}", stderr));
    }

    Err("no suitable C compiler found (tried cl.exe, gcc, clang)".to_string())
}

fn dlerror_str() -> String {
    #[cfg(target_os = "windows")]
    unsafe {
        let err = GetLastError();
        if err == 0 { return "unknown".to_string(); }
        let mut buf: [u16; 256] = [0; 256];
        let len = FormatMessageW(
            0x00000100 | 0x00001000, None, err, 0,
            buf.as_mut_ptr() as *mut std::ffi::c_void, buf.len() as u32,
            std::ptr::null_mut(),
        );
        if len > 0 {
            String::from_utf16_lossy(&buf[..len as usize])
        } else {
            format!("error code {}", err)
        }
    }

    #[cfg(not(target_os = "windows"))]
    unsafe {
        let p = dlerror();
        if p.is_null() { "unknown".to_string() } else { CStr::from_ptr(p).to_string_lossy().into_owned() }
    }
}

// Platform-specific dynamic loading
#[cfg(not(target_os = "windows"))]
extern "C" {
    fn dlopen(path: *const std::ffi::c_char, flags: i32) -> *mut std::ffi::c_void;
    fn dlsym(handle: *mut std::ffi::c_void, sym: *const std::ffi::c_char) -> *mut std::ffi::c_void;
    fn dlerror() -> *const std::ffi::c_char;
}

#[cfg(target_os = "windows")]
extern "system" {
    fn LoadLibraryA(lpLibFileName: *const u8) -> *mut std::ffi::c_void;
    fn GetProcAddress(hModule: *mut std::ffi::c_void, lpProcName: *const u8) -> *mut std::ffi::c_void;
    fn GetLastError() -> u32;
    fn FormatMessageW(
        dwFlags: u32,
        lpSource: *const std::ffi::c_void,
        dwMessageId: u32,
        dwLanguageId: u32,
        lpBuffer: *mut std::ffi::c_void,
        nSize: u32,
        Arguments: *mut std::ffi::c_void,
    ) -> u32;
}

#[cfg(target_os = "windows")]
unsafe fn load_library_windows(path: *const u8) -> *mut std::ffi::c_void {
    LoadLibraryA(path)
}

#[cfg(target_os = "windows")]
unsafe fn get_proc_address_windows(handle: *mut std::ffi::c_void, name: *const u8) -> *mut std::ffi::c_void {
    GetProcAddress(handle, name)
}
