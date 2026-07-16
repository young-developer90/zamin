use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::rc::Rc;

use crate::bytecode::{Chunk, OpCode};
use crate::cext;
use crate::comet_mod;
use crate::compiler::Compiler;
use crate::gc::*;
use crate::http_module;
use crate::nova_mod;
#[cfg(target_os = "windows")]
use crate::sol_mod;
#[cfg(all(not(target_os = "windows"), feature = "luna"))]
use crate::luna_mod;
use crate::parser::Parser;
use crate::stdlib;
use crate::vm::Vm;

fn builtin_names() -> HashSet<String> {
    [
        "print", "main", "nova", "comet", "nova_version", "_nova_row",
    ].iter().map(|s| s.to_string()).collect()
}

fn load_zamin_library(path: &Path) -> Option<(String, Vec<Chunk>, Vec<String>)> {
    let source = std::fs::read_to_string(path).ok()?;
    let stem = path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string())?;

    let mut parser = Parser::new(&source);
    let program = parser.parse().ok()?;

    // Scan for exported names in the AST
    let mut exports = Vec::new();
    for stmt in &program.stmts {
        if let crate::ast::Stmt::FuncDef { name, is_export: true, .. } = stmt {
            exports.push(name.clone());
        }
        if let crate::ast::Stmt::Export { names } = stmt {
            exports.extend(names.clone());
        }
    }

    let mut compiler = Compiler::new();
    compiler.compile(&program).ok()?;

    Some((stem, compiler.chunks, exports))
}

/// Patch OpCode::MakeFunc / MakeClosure chunk indices in library bytecode
/// so they point to the correct chunks after insertion at `offset`.
fn patch_chunk_indices(chunks: &mut [Chunk], offset: u16) {
    for chunk in chunks.iter_mut() {
        let mut i = 0;
        while i < chunk.code.len() {
            let op = OpCode::from_u8(chunk.code[i]);
            match op {
                Some(OpCode::MakeFunc) | Some(OpCode::MakeClosure) => {
                    if i + 3 <= chunk.code.len() {
                        let idx = u16::from_le_bytes([chunk.code[i+1], chunk.code[i+2]]);
                        let patched = idx + offset;
                        chunk.code[i+1..i+3].copy_from_slice(&patched.to_le_bytes());
                    }
                    i += 3;
                }
                Some(OpCode::Call) | Some(_) => {
                    i += 1 + opcode_operand_len(chunk.code[i]);
                }
                None => { i += 1; }
            }
        }
    }
}

fn opcode_operand_len(opcode: u8) -> usize {
    match OpCode::from_u8(opcode) {
        Some(OpCode::LoadConst) | Some(OpCode::LoadGlobal) |
        Some(OpCode::StoreGlobal) | Some(OpCode::LoadLocal) |
        Some(OpCode::StoreLocal) | Some(OpCode::MakeFunc) |
        Some(OpCode::MakeClosure) | Some(OpCode::Jump) |
        Some(OpCode::JumpIfTrue) | Some(OpCode::JumpIfFalse) |
        Some(OpCode::JumpIfNil) | Some(OpCode::Call) |
        Some(OpCode::LoadUpvalue) | Some(OpCode::StoreUpvalue) |
        Some(OpCode::Try) | Some(OpCode::ForPrep) | Some(OpCode::ForIter) |
        Some(OpCode::Inc) | Some(OpCode::Dec) => 2,
        _ => 0,
    }
}

pub struct ModuleLoader;

impl ModuleLoader {
    pub fn new() -> Self {
        ModuleLoader
    }

    pub fn load_stdlib(&mut self) {
        // stdlib is now loaded during VM execution
    }

    fn build_modules(heap: &mut GcHeap) -> HashMap<String, Value> {
        let mut modules = HashMap::new();
        for (name, items) in stdlib::build_stdlib(heap) {
            let mut dict_items = Vec::new();
            for (key, val) in items {
                dict_items.push((make_string(heap, &key), val));
            }
            let module_val = Value::Dict(heap.alloc(GcObj::Dict(dict_items)));
            modules.insert(name, module_val);
        }

        let mut http_items = Vec::new();
        for (key, val) in http_module::build_http() {
            http_items.push((make_string(heap, &key), val));
        }
        modules.insert("http".to_string(), Value::Dict(heap.alloc(GcObj::Dict(http_items))));

        #[cfg(target_os = "windows")]
        {
            let mut sol_items = Vec::new();
            for (key, val) in sol_mod::build_sol() {
                sol_items.push((make_string(heap, &key), val));
            }
            modules.insert("sol".to_string(), Value::Dict(heap.alloc(GcObj::Dict(sol_items))));
        }

        #[cfg(all(not(target_os = "windows"), feature = "luna"))]
        {
            let mut luna_items = Vec::new();
            for (key, val) in luna_mod::build_luna() {
                luna_items.push((make_string(heap, &key), val));
            }
            modules.insert("luna".to_string(), Value::Dict(heap.alloc(GcObj::Dict(luna_items))));
        }

        // nova and comet are registered as top-level globals in add_globals

        #[cfg(cuda_support)]
        {
            if crate::cuda::init().is_ok() {
                let _ = crate::linum::init_kernels();
            }
            let mut linum_items = Vec::new();
            for (key, val) in crate::linum::build_linum() {
                linum_items.push((make_string(heap, &key), val));
            }
            modules.insert("linum".to_string(), Value::Dict(heap.alloc(GcObj::Dict(linum_items))));
        }

        #[cfg(feature = "python")]
        {
            let mut py_items = Vec::new();
            for (key, val) in crate::py::build_py() {
                py_items.push((make_string(heap, &key), val));
            }
            modules.insert("py".to_string(), Value::Dict(heap.alloc(GcObj::Dict(py_items))));
        }

        // Load C extension modules (*.dll / *.so)
        if let Ok(dir) = std::env::current_dir() {
            let ext_dir = dir.join("modules");
            if ext_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&ext_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                        if ext != "dll" && ext != "so" && ext != "dylib" {
                            continue;
                        }
                        let stem = path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string());
                        let mod_name = match stem {
                            Some(n) if !modules.contains_key(&n) => n,
                            _ => continue,
                        };
                        match cext::load_extension(&path, heap) {
                            Ok(funcs) => {
                                let mut items = Vec::new();
                                for (key, val) in funcs {
                                    items.push((make_string(heap, &key), val));
                                }
                                modules.insert(mod_name, Value::Dict(heap.alloc(GcObj::Dict(items))));
                            }
                            Err(e) => {
                                eprintln!("warning: failed to load C extension '{}': {}", mod_name, e);
                            }
                        }
                    }
                }
            }
        }

        modules
    }

    fn add_globals(globals: &mut HashMap<String, Value>) {
        globals.insert("main".to_string(), Value::Nil);
        globals.insert("print".to_string(), Value::NativeFunc(NativeFunc {
            name: "<print>".to_string(),
            func: Rc::new(|args, ctx| {
                for arg in args {
                    print!("{}", arg.to_string(ctx.heap));
                }
                println!();
                Ok(Value::Nil)
            }),
        }));

        // Add nova() as a top-level function
        for (name, val) in nova_mod::build_nova() {
            globals.insert(name, val);
        }

        // Add comet() as a top-level function
        for (name, val) in comet_mod::build_comet() {
            globals.insert(name, val);
        }
    }

    fn load_zamin_libs(vm: &mut Vm, modules: &mut HashMap<String, Value>) {
        let lib_dir = Path::new("/etc/zamin/lib");
        if !lib_dir.is_dir() { return; }

        let builtins = builtin_names();
        let entries: Vec<_> = std::fs::read_dir(lib_dir).ok()
            .into_iter().flat_map(|d| d.flatten()).collect();

        // First pass: compile all libraries and collect their chunks
        struct LibInfo {
            stem: String,
            chunks: Vec<Chunk>,
            #[allow(dead_code)]
            exports: Vec<String>,
        }
        let mut libs: Vec<LibInfo> = Vec::new();
        for entry in &entries {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("zamin") { continue; }
            let stem = path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string());
            let _ = match stem { Some(n) if !modules.contains_key(&n) => n, _ => continue, };
            if let Some((s, chunks, exports)) = crate::module::load_zamin_library(&path) {
                libs.push(LibInfo { stem: s, chunks, exports });
            } else {
                eprintln!("warning: failed to compile library '{}'", path.display());
            }
        }

        // Compute library chunk start index (after user chunks)
        let lib_start = vm.chunks.len();
        // Append library chunks, patching their internal chunk indices
        for lib in &libs {
            let mut patched = lib.chunks.clone();
            patch_chunk_indices(&mut patched, lib_start as u16);
            for chunk in &mut patched {
                for (i, content) in chunk.string_constants.iter().enumerate() {
                    if let Some(s) = content {
                        chunk.constants[i] = Value::String(vm.heap.alloc(GcObj::String(s.clone())));
                    }
                }
                vm.chunks.push(chunk.clone());
            }
        }

        // Run library entry chunks to populate globals
        for (i, lib) in libs.iter().enumerate() {
            let entry_chunk = lib_start + libs[..i].iter().map(|l| l.chunks.len()).sum::<usize>();
            let before: HashSet<String> = vm.globals.keys().cloned().collect();
            if let Err(e) = vm.run_chunk(entry_chunk) {
                eprintln!("warning: failed to execute library '{}': {}", lib.stem, e);
                continue;
            }
            // Build module dict from newly-set globals
            let mut items = Vec::new();
            for (name, val) in &vm.globals {
                if !builtins.contains(name) && !modules.contains_key(name) && !before.contains(name) {
                    items.push((make_string(&mut vm.heap, name), val.clone()));
                }
            }
            if !items.is_empty() {
                modules.insert(lib.stem.clone(), Value::Dict(vm.heap.alloc(GcObj::Dict(items))));
                vm.globals.insert(lib.stem.clone(), modules[&lib.stem].clone());
            }
        }
    }

    pub fn execute_file(&mut self, path: &str) -> Result<(), String> {
        let source = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read file '{}': {}", path, e))?;

        let mut parser = Parser::new(&source);
        let program = parser.parse().map_err(|e| e.to_string())?;

        let mut compiler = Compiler::new();
        compiler.compile(&program)?;

        let chunks = compiler.chunks;
        let mut vm = Vm::new(chunks);

        let mut modules = Self::build_modules(&mut vm.heap);
        Self::add_globals(&mut vm.globals);
        for (mod_name, val) in &modules {
            vm.globals.insert(mod_name.clone(), val.clone());
        }
        Self::load_zamin_libs(&mut vm, &mut modules);

        for (mod_name, val) in modules {
            vm.globals.insert(mod_name, val);
        }

        vm.run()?;
        Ok(())
    }

    pub fn execute_source(&mut self, source: &str) -> Result<String, String> {
        let mut parser = Parser::new(source);
        let program = parser.parse().map_err(|e| e.to_string())?;

        let mut compiler = Compiler::new();
        compiler.compile(&program)?;

        let chunks = compiler.chunks;
        let mut vm = Vm::new(chunks);

        let mut modules = Self::build_modules(&mut vm.heap);
        Self::add_globals(&mut vm.globals);
        for (mod_name, val) in &modules {
            vm.globals.insert(mod_name.clone(), val.clone());
        }
        Self::load_zamin_libs(&mut vm, &mut modules);

        for (mod_name, val) in modules {
            vm.globals.insert(mod_name, val);
        }

        match vm.run() {
            Ok(result) => Ok(result.to_string(&vm.heap)),
            Err(e) => Err(e),
        }
    }
}
