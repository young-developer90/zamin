use std::collections::HashMap;
use std::rc::Rc;

use crate::compiler::Compiler;
use crate::gc::*;
use crate::http_module;
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

        modules
    }

    fn add_print(globals: &mut Vec<(String, Value)>) {
        globals.push(("main".to_string(), Value::Nil));
        globals.push(("print".to_string(), Value::NativeFunc(NativeFunc {
            name: "<print>".to_string(),
            func: Rc::new(|args, ctx| {
                for arg in args {
                    print!("{}", arg.to_string(ctx.heap));
                }
                println!();
                Ok(Value::Nil)
            }),
        })));
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
            vm.globals.push((mod_name, val));
        }

        Self::add_print(&mut vm.globals);
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
            vm.globals.push((mod_name, val));
        }

        Self::add_print(&mut vm.globals);

        match vm.run() {
            Ok(result) => Ok(result.to_string(&vm.heap)),
            Err(e) => Err(e),
        }
    }
}
