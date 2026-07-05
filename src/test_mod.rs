use std::rc::Rc;
use crate::gc::*;

pub fn build_test() -> Vec<(String, Value)> {
    let mut funcs = Vec::new();

    funcs.push(("assert_eq".to_string(), Value::NativeFunc(NativeFunc {
        name: "<test.assert_eq>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("test.assert_eq requires expected and actual".to_string()); }
            let expected = &args[0];
            let actual = &args[1];
            let msg = if args.len() >= 3 { args[2].to_string(ctx.heap) } else { String::new() };
            if !expected.eq(actual, ctx.heap) {
                let err_msg = if msg.is_empty() {
                    format!("FAIL: assert_eq failed: expected {} but got {}", expected.to_string(ctx.heap), actual.to_string(ctx.heap))
                } else {
                    format!("FAIL: {}: expected {} but got {}", msg, expected.to_string(ctx.heap), actual.to_string(ctx.heap))
                };
                return Err(err_msg);
            }
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("assert_ne".to_string(), Value::NativeFunc(NativeFunc {
        name: "<test.assert_ne>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("test.assert_ne requires expected and actual".to_string()); }
            let expected = &args[0];
            let actual = &args[1];
            let msg = if args.len() >= 3 { args[2].to_string(ctx.heap) } else { String::new() };
            if expected.eq(actual, ctx.heap) {
                let err_msg = if msg.is_empty() {
                    format!("FAIL: assert_ne failed: values are equal: {}", expected.to_string(ctx.heap))
                } else {
                    format!("FAIL: {}: values are equal: {}", msg, expected.to_string(ctx.heap))
                };
                return Err(err_msg);
            }
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("assert_true".to_string(), Value::NativeFunc(NativeFunc {
        name: "<test.assert_true>".to_string(),
        func: Rc::new(|args, ctx| {
            let val = args.first().ok_or("test.assert_true requires a value")?;
            let msg = if args.len() >= 2 { args[1].to_string(ctx.heap) } else { String::new() };
            if !val.is_truthy() {
                let err_msg = if msg.is_empty() {
                    "FAIL: assert_true failed".to_string()
                } else {
                    format!("FAIL: {}", msg)
                };
                return Err(err_msg);
            }
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("assert_false".to_string(), Value::NativeFunc(NativeFunc {
        name: "<test.assert_false>".to_string(),
        func: Rc::new(|args, ctx| {
            let val = args.first().ok_or("test.assert_false requires a value")?;
            let msg = if args.len() >= 2 { args[1].to_string(ctx.heap) } else { String::new() };
            if val.is_truthy() {
                let err_msg = if msg.is_empty() {
                    "FAIL: assert_false failed".to_string()
                } else {
                    format!("FAIL: {}", msg)
                };
                return Err(err_msg);
            }
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("assert_contains".to_string(), Value::NativeFunc(NativeFunc {
        name: "<test.assert_contains>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("test.assert_requires container and item".to_string()); }
            let container = &args[0];
            let item = &args[1];
            let msg = if args.len() >= 3 { args[2].to_string(ctx.heap) } else { String::new() };
            let found = match container {
                Value::List(r) => {
                    if let GcObj::List(items) = ctx.heap.get(*r) {
                        items.iter().any(|v| v.eq(item, ctx.heap))
                    } else { false }
                }
                Value::String(r) => {
                    if let GcObj::String(s) = ctx.heap.get(*r) {
                        s.contains(&item.to_string(ctx.heap))
                    } else { false }
                }
                Value::Dict(r) => {
                    if let GcObj::Dict(entries) = ctx.heap.get(*r) {
                        entries.iter().any(|(k, _)| k.eq(item, ctx.heap))
                    } else { false }
                }
                Value::Set(r) => {
                    if let GcObj::Set(items) = ctx.heap.get(*r) {
                        items.iter().any(|v| v.eq(item, ctx.heap))
                    } else { false }
                }
                _ => false,
            };
            if !found {
                let err_msg = if msg.is_empty() {
                    format!("FAIL: assert_contains failed: container does not contain {}", item.to_string(ctx.heap))
                } else {
                    format!("FAIL: {}: container does not contain {}", msg, item.to_string(ctx.heap))
                };
                return Err(err_msg);
            }
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("assert_raises".to_string(), Value::NativeFunc(NativeFunc {
        name: "<test.assert_raises>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 1 { return Err("test.assert_raises requires a function".to_string()); }
            let fn_val = &args[0];
            let expected_msg = if args.len() >= 2 { Some(args[1].to_string(ctx.heap)) } else { None };
            let call_args: Vec<Value> = if args.len() >= 3 {
                match &args[2] {
                    Value::List(r) => match ctx.heap.get(*r) { GcObj::List(items) => items.clone(), _ => vec![] },
                    _ => vec![],
                }
            } else { vec![] };
            match fn_val {
                Value::NativeFunc(f) => {
                    let result = (f.func)(&call_args, ctx);
                    match result {
                        Ok(_) => Err("FAIL: assert_raises failed: no error was raised".to_string()),
                        Err(e) => {
                            if let Some(ref expected) = expected_msg {
                                if e.contains(expected) {
                                    Ok(Value::Nil)
                                } else {
                                    Err(format!("FAIL: assert_raises failed: expected error containing '{}' but got '{}'", expected, e))
                                }
                            } else {
                                Ok(Value::Nil)
                            }
                        }
                    }
                }
                _ => Err("assert_raises requires a native function".to_string()),
            }
        }),
    })));

    funcs.push(("run_test".to_string(), Value::NativeFunc(NativeFunc {
        name: "<test.run_test>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("test.run_test requires a name and a function".to_string()); }
            let name = args[0].to_string(ctx.heap);
            let fn_val = &args[1];
            match fn_val {
                Value::NativeFunc(f) => {
                    let result = (f.func)(&[], ctx);
                    match result {
                        Ok(_) => {
                            println!("PASS: {}", name);
                            Ok(Value::Nil)
                        }
                        Err(e) => {
                            println!("{}", e);
                            Ok(Value::Nil)
                        }
                    }
                }
                _ => Err("test.run_test requires a function".to_string()),
            }
        }),
    })));

    funcs
}
