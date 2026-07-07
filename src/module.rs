use std::collections::HashMap;
use std::rc::Rc;

use crate::cext;
use crate::cheetah_mod;
use crate::compiler::Compiler;
use crate::gc::*;
use crate::http_module;
use crate::jaguar_mod;
#[cfg(target_os = "windows")]
use crate::leopard_mod;
#[cfg(all(not(target_os = "windows"), feature = "panther"))]
use crate::panther_mod;
use crate::parser::Parser;
use crate::stdlib;
use crate::vm::Vm;

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
            let mut leopard_items = Vec::new();
            for (key, val) in leopard_mod::build_leopard() {
                leopard_items.push((make_string(heap, &key), val));
            }
            modules.insert("leopard".to_string(), Value::Dict(heap.alloc(GcObj::Dict(leopard_items))));
        }

        #[cfg(all(not(target_os = "windows"), feature = "panther"))]
        {
            let mut panther_items = Vec::new();
            for (key, val) in panther_mod::build_panther() {
                panther_items.push((make_string(heap, &key), val));
            }
            modules.insert("panther".to_string(), Value::Dict(heap.alloc(GcObj::Dict(panther_items))));
        }

        // jaguar and cheetah are registered as top-level globals in add_globals

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

        // Add jaguar() as a top-level function
        for (name, val) in jaguar_mod::build_jaguar() {
            globals.insert(name, val);
        }

        // Add cheetah() as a top-level function
        for (name, val) in cheetah_mod::build_cheetah() {
            globals.insert(name, val);
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

        let modules = Self::build_modules(&mut vm.heap);
        for (mod_name, val) in modules {
            vm.globals.insert(mod_name, val);
        }

        Self::add_globals(&mut vm.globals);
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

        let modules = Self::build_modules(&mut vm.heap);
        for (mod_name, val) in modules {
            vm.globals.insert(mod_name, val);
        }

        Self::add_globals(&mut vm.globals);

        match vm.run() {
            Ok(result) => Ok(result.to_string(&vm.heap)),
            Err(e) => Err(e),
        }
    }
}
