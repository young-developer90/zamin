use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;

use crate::compiler::Compiler;
use crate::gc::*;
use crate::parser::Parser;
use crate::vm::Vm;

thread_local! {
    static OUTPUT: RefCell<String> = const { RefCell::new(String::new()) };
}

fn write_output(s: &str) {
    OUTPUT.with(|out| out.borrow_mut().push_str(s));
}

fn setup_wasm_env(vm: &mut Vm) {
    // Build stdlib modules from the shared implementation
    let mut modules = crate::stdlib::build_stdlib(&mut vm.heap);

    // Patch the io module to capture output instead of printing to stdout
    for (mod_name, items) in &mut modules {
        if mod_name != "io" { continue; }
        for (key, val) in items.iter_mut() {
            match key.as_str() {
                "print" => {
                    *val = Value::NativeFunc(NativeFunc {
                        name: "<io.print>".to_string(),
                        func: Rc::new(|args, ctx| {
                            for arg in args {
                                write_output(&arg.to_string(ctx.heap));
                            }
                            Ok(Value::Nil)
                        }),
                    });
                }
                "println" => {
                    *val = Value::NativeFunc(NativeFunc {
                        name: "<io.println>".to_string(),
                        func: Rc::new(|args, ctx| {
                            for arg in args {
                                write_output(&arg.to_string(ctx.heap));
                            }
                            write_output("\n");
                            Ok(Value::Nil)
                        }),
                    });
                }
                _ => {}
            }
        }
    }

    // Register top-level print
    vm.globals.insert("print".to_string(), Value::NativeFunc(NativeFunc {
        name: "<print>".to_string(),
        func: Rc::new(|args, ctx| {
            for arg in args {
                write_output(&arg.to_string(ctx.heap));
            }
            write_output("\n");
            Ok(Value::Nil)
        }),
    }));

    // Register comet (HTML templating)
    for (name, val) in crate::comet_mod::build_comet() {
        vm.globals.insert(name, val);
    }

    // Register all modules as globals
    vm.globals.insert("main".to_string(), Value::Nil);
    for (mod_name, items) in &modules {
        let mut dict_items = Vec::new();
        for (key, val) in items {
            dict_items.push((make_string(&mut vm.heap, &key), val.clone()));
        }
        let module_val = Value::Dict(vm.heap.alloc(GcObj::Dict(dict_items)));
        vm.globals.insert(mod_name.clone(), module_val);
    }
}

#[wasm_bindgen]
pub fn run_zamin(source: &str) -> String {
    OUTPUT.with(|out| *out.borrow_mut() = String::new());

    let mut parser = Parser::new(source);
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => return format!("Parse error: {}", e),
    };

    let mut compiler = Compiler::new();
    if let Err(e) = compiler.compile(&program) {
        return format!("Compile error: {}", e);
    }

    let chunks = compiler.chunks;
    let mut vm = Vm::new(chunks);
    setup_wasm_env(&mut vm);

    match vm.run() {
        Ok(_) => {
            OUTPUT.with(|out| {
                let output = out.borrow().clone();
                if output.is_empty() { "ok".to_string() } else { output }
            })
        }
        Err(e) => {
            let output = OUTPUT.with(|out| out.borrow().clone());
            if output.is_empty() {
                format!("Runtime error: {}", e)
            } else {
                format!("{}\nRuntime error: {}", output.trim_end(), e)
            }
        }
    }
}
