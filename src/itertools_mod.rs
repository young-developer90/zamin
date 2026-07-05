use std::rc::Rc;
use crate::gc::*;

pub fn build_itertools() -> Vec<(String, Value)> {
    let mut funcs = Vec::new();

    funcs.push(("chain".to_string(), Value::NativeFunc(NativeFunc {
        name: "<itertools.chain>".to_string(),
        func: Rc::new(|args, ctx| {
            let mut result = Vec::new();
            for arg in args {
                match arg {
                    Value::List(r) => {
                        if let GcObj::List(items) = ctx.heap.get(*r) {
                            result.extend(items.clone());
                        }
                    }
                    other => result.push(other.clone()),
                }
            }
            Ok(make_list(ctx.heap, result))
        }),
    })));

    funcs.push(("zip".to_string(), Value::NativeFunc(NativeFunc {
        name: "<itertools.zip>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 1 { return Err("zip requires at least one list".to_string()); }
            let lists: Vec<Vec<Value>> = args.iter().map(|a| {
                match a {
                    Value::List(r) => match ctx.heap.get(*r) { GcObj::List(items) => items.clone(), _ => vec![] },
                    _ => vec![],
                }
            }).collect();
            if lists.is_empty() { return Ok(make_list(ctx.heap, vec![])); }
            let min_len = lists.iter().map(|l| l.len()).min().unwrap_or(0);
            let mut result = Vec::new();
            for i in 0..min_len {
                let tuple_items: Vec<Value> = lists.iter().map(|l| l[i].clone()).collect();
                result.push(make_list(ctx.heap, tuple_items));
            }
            Ok(make_list(ctx.heap, result))
        }),
    })));

    funcs.push(("enumerate".to_string(), Value::NativeFunc(NativeFunc {
        name: "<itertools.enumerate>".to_string(),
        func: Rc::new(|args, ctx| {
            let list_val = args.first().ok_or("enumerate requires a list")?;
            let items = match list_val {
                Value::List(r) => match ctx.heap.get(*r) { GcObj::List(items) => items.clone(), _ => return Err("expected list".to_string()) },
                _ => return Err("expected list".to_string()),
            };
            let result: Vec<Value> = items.iter().enumerate().map(|(i, v)| {
                make_list(ctx.heap, vec![Value::Int(i as i64), v.clone()])
            }).collect();
            Ok(make_list(ctx.heap, result))
        }),
    })));

    funcs.push(("map".to_string(), Value::NativeFunc(NativeFunc {
        name: "<itertools.map>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("map requires a function and a list".to_string()); }
            let fn_val = &args[0];
            let items = match &args[1] {
                Value::List(r) => match ctx.heap.get(*r) { GcObj::List(items) => items.clone(), _ => return Err("expected list".to_string()) },
                _ => return Err("expected list".to_string()),
            };
            let mut result = Vec::new();
            for item in &items {
                let mapped = call_fn_with_arg(fn_val, item, ctx)?;
                result.push(mapped);
            }
            Ok(make_list(ctx.heap, result))
        }),
    })));

    funcs.push(("filter".to_string(), Value::NativeFunc(NativeFunc {
        name: "<itertools.filter>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("filter requires a function and a list".to_string()); }
            let fn_val = &args[0];
            let items = match &args[1] {
                Value::List(r) => match ctx.heap.get(*r) { GcObj::List(items) => items.clone(), _ => return Err("expected list".to_string()) },
                _ => return Err("expected list".to_string()),
            };
            let mut result = Vec::new();
            for item in &items {
                let keep = call_fn_with_arg(fn_val, item, ctx)?;
                if keep.is_truthy() {
                    result.push(item.clone());
                }
            }
            Ok(make_list(ctx.heap, result))
        }),
    })));

    funcs.push(("reduce".to_string(), Value::NativeFunc(NativeFunc {
        name: "<itertools.reduce>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("reduce requires a function and a list".to_string()); }
            let fn_val = &args[0];
            let items = match &args[1] {
                Value::List(r) => match ctx.heap.get(*r) { GcObj::List(items) => items.clone(), _ => return Err("expected list".to_string()) },
                _ => return Err("expected list".to_string()),
            };
            if items.is_empty() {
                return if args.len() >= 3 { Ok(args[2].clone()) } else { Err("reduce of empty list with no initial value".to_string()) };
            }
            let mut acc = if args.len() >= 3 {
                args[2].clone()
            } else {
                items[0].clone()
            };
            let start = if args.len() >= 3 { 0 } else { 1 };
            for i in start..items.len() {
                acc = call_fn_with_two_args(fn_val, &acc, &items[i], ctx)?;
            }
            Ok(acc)
        }),
    })));

    funcs.push(("take".to_string(), Value::NativeFunc(NativeFunc {
        name: "<itertools.take>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("take requires n and a list".to_string()); }
            let n = to_i64(&args[0])? as usize;
            let items = match &args[1] {
                Value::List(r) => match ctx.heap.get(*r) { GcObj::List(items) => items.clone(), _ => return Err("expected list".to_string()) },
                _ => return Err("expected list".to_string()),
            };
            let end = n.min(items.len());
            Ok(make_list(ctx.heap, items[..end].to_vec()))
        }),
    })));

    funcs.push(("drop".to_string(), Value::NativeFunc(NativeFunc {
        name: "<itertools.drop>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("drop requires n and a list".to_string()); }
            let n = to_i64(&args[0])? as usize;
            let items = match &args[1] {
                Value::List(r) => match ctx.heap.get(*r) { GcObj::List(items) => items.clone(), _ => return Err("expected list".to_string()) },
                _ => return Err("expected list".to_string()),
            };
            if n >= items.len() {
                Ok(make_list(ctx.heap, vec![]))
            } else {
                Ok(make_list(ctx.heap, items[n..].to_vec()))
            }
        }),
    })));

    funcs.push(("slice".to_string(), Value::NativeFunc(NativeFunc {
        name: "<itertools.slice>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 3 { return Err("slice requires a list, start, and end".to_string()); }
            let items = match &args[0] {
                Value::List(r) => match ctx.heap.get(*r) { GcObj::List(items) => items.clone(), _ => return Err("expected list".to_string()) },
                _ => return Err("expected list".to_string()),
            };
            let start = to_i64(&args[1])? as usize;
            let end = to_i64(&args[2])? as usize;
            let step = if args.len() >= 4 { to_i64(&args[3])? as usize } else { 1 };
            let mut result = Vec::new();
            let mut i = start;
            while i < end.min(items.len()) {
                result.push(items[i].clone());
                i += step;
            }
            Ok(make_list(ctx.heap, result))
        }),
    })));

    funcs.push(("unique".to_string(), Value::NativeFunc(NativeFunc {
        name: "<itertools.unique>".to_string(),
        func: Rc::new(|args, ctx| {
            let list_val = args.first().ok_or("unique requires a list")?;
            let items = match list_val {
                Value::List(r) => match ctx.heap.get(*r) { GcObj::List(items) => items.clone(), _ => return Err("expected list".to_string()) },
                _ => return Err("expected list".to_string()),
            };
            let mut result: Vec<Value> = Vec::new();
            for item in &items {
                let mut found = false;
                for existing in &result {
                    if existing.eq(item, ctx.heap) {
                        found = true;
                        break;
                    }
                }
                if !found {
                    result.push(item.clone());
                }
            }
            Ok(make_list(ctx.heap, result))
        }),
    })));

    funcs.push(("reverse".to_string(), Value::NativeFunc(NativeFunc {
        name: "<itertools.reverse>".to_string(),
        func: Rc::new(|args, ctx| {
            let list_val = args.first().ok_or("reverse requires a list")?;
            let items = match list_val {
                Value::List(r) => match ctx.heap.get(*r) { GcObj::List(items) => items.clone(), _ => return Err("expected list".to_string()) },
                _ => return Err("expected list".to_string()),
            };
            let mut result = items;
            result.reverse();
            Ok(make_list(ctx.heap, result))
        }),
    })));

    funcs.push(("count".to_string(), Value::NativeFunc(NativeFunc {
        name: "<itertools.count>".to_string(),
        func: Rc::new(|args, ctx| {
            let start = if args.is_empty() { 0i64 } else { to_i64(&args[0])? };
            let step = if args.len() < 2 { 1i64 } else { to_i64(&args[1])? };
            let limit = 10000; // bounded for safety
            let mut result = Vec::new();
            for i in 0..limit {
                result.push(Value::Int(start + step * i));
            }
            Ok(make_list(ctx.heap, result))
        }),
    })));

    funcs.push(("repeat".to_string(), Value::NativeFunc(NativeFunc {
        name: "<itertools.repeat>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("repeat requires a value and a count".to_string()); }
            let val = args[0].clone();
            let count = to_i64(&args[1])? as usize;
            let result = vec![val; count];
            Ok(make_list(ctx.heap, result))
        }),
    })));

    funcs.push(("product".to_string(), Value::NativeFunc(NativeFunc {
        name: "<itertools.product>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("product requires at least two lists".to_string()); }
            let lists: Vec<Vec<Value>> = args.iter().map(|a| {
                match a {
                    Value::List(r) => match ctx.heap.get(*r) { GcObj::List(items) => items.clone(), _ => vec![] },
                    _ => vec![],
                }
            }).collect();
            if lists.iter().any(|l| l.is_empty()) {
                return Ok(make_list(ctx.heap, vec![]));
            }
            let mut result = Vec::new();
            let mut indices = vec![0usize; lists.len()];
            loop {
                let tuple: Vec<Value> = indices.iter().enumerate().map(|(i, &idx)| lists[i][idx].clone()).collect();
                result.push(make_list(ctx.heap, tuple));
                let mut carry = true;
                for i in (0..lists.len()).rev() {
                    if carry {
                        indices[i] += 1;
                        if indices[i] >= lists[i].len() {
                            indices[i] = 0;
                            carry = true;
                        } else {
                            carry = false;
                        }
                    }
                }
                if carry { break; }
            }
            Ok(make_list(ctx.heap, result))
        }),
    })));

    funcs.push(("any".to_string(), Value::NativeFunc(NativeFunc {
        name: "<itertools.any>".to_string(),
        func: Rc::new(|args, ctx| {
            let list_val = args.first().ok_or("any requires a list")?;
            let items = match list_val {
                Value::List(r) => match ctx.heap.get(*r) { GcObj::List(items) => items.clone(), _ => return Err("expected list".to_string()) },
                _ => return Err("expected list".to_string()),
            };
            if args.len() >= 2 {
                let fn_val = &args[1];
                for item in &items {
                    let val = call_fn_with_arg(fn_val, item, ctx)?;
                    if val.is_truthy() { return Ok(Value::Bool(true)); }
                }
                Ok(Value::Bool(false))
            } else {
                Ok(Value::Bool(items.iter().any(|v| v.is_truthy())))
            }
        }),
    })));

    funcs.push(("all".to_string(), Value::NativeFunc(NativeFunc {
        name: "<itertools.all>".to_string(),
        func: Rc::new(|args, ctx| {
            let list_val = args.first().ok_or("all requires a list")?;
            let items = match list_val {
                Value::List(r) => match ctx.heap.get(*r) { GcObj::List(items) => items.clone(), _ => return Err("expected list".to_string()) },
                _ => return Err("expected list".to_string()),
            };
            if args.len() >= 2 {
                let fn_val = &args[1];
                for item in &items {
                    let val = call_fn_with_arg(fn_val, item, ctx)?;
                    if !val.is_truthy() { return Ok(Value::Bool(false)); }
                }
                Ok(Value::Bool(true))
            } else {
                Ok(Value::Bool(items.iter().all(|v| v.is_truthy())))
            }
        }),
    })));

    funcs.push(("sorted".to_string(), Value::NativeFunc(NativeFunc {
        name: "<itertools.sorted>".to_string(),
        func: Rc::new(|args, ctx| {
            let list_val = args.first().ok_or("sorted requires a list")?;
            let mut items = match list_val {
                Value::List(r) => match ctx.heap.get(*r) { GcObj::List(items) => items.clone(), _ => return Err("expected list".to_string()) },
                _ => return Err("expected list".to_string()),
            };
            if args.len() >= 2 {
                let key_fn = &args[1];
                let mut keyed: Vec<(Value, Value)> = Vec::new();
                for item in &items {
                    let key = call_fn_with_arg(key_fn, item, ctx)?;
                    keyed.push((key, item.clone()));
                }
                // Simple bubble sort (limited but functional)
                let n = keyed.len();
                for i in 0..n {
                    for j in 0..n - 1 - i {
                        if compare_values(&keyed[j].0, &keyed[j + 1].0, ctx.heap) == std::cmp::Ordering::Greater {
                            keyed.swap(j, j + 1);
                        }
                    }
                }
                items = keyed.into_iter().map(|(_, v)| v).collect();
            } else {
                items.sort_by(|a, b| compare_values(a, b, ctx.heap));
            }
            Ok(make_list(ctx.heap, items))
        }),
    })));

    // functools-style functions
    funcs.push(("identity".to_string(), Value::NativeFunc(NativeFunc {
        name: "<itertools.identity>".to_string(),
        func: Rc::new(|args, _ctx| {
            if args.is_empty() { Ok(Value::Nil) } else { Ok(args[0].clone()) }
        }),
    })));

    funcs.push(("compose".to_string(), Value::NativeFunc(NativeFunc {
        name: "<itertools.compose>".to_string(),
        func: Rc::new(|args, _ctx| {
            if args.len() < 2 {
                return Err("compose requires at least two functions".to_string());
            }
            let fns = args.to_vec();
            let composed_name = format!("<composed[{}]>", fns.len());
            Ok(Value::NativeFunc(NativeFunc {
                name: composed_name,
                func: Rc::new(move |inner_args, inner_ctx| {
                    let mut val = if inner_args.is_empty() { Value::Nil } else { inner_args[0].clone() };
                    for (i, fn_val) in fns.iter().enumerate() {
                        if i == fns.len() - 1 {
                            val = call_fn_with_arg(fn_val, &val, inner_ctx)?;
                        } else {
                            // intermediate calls for non-last functions
                            val = call_fn_with_arg(fn_val, &val, inner_ctx)?;
                        }
                    }
                    Ok(val)
                }),
            }))
        }),
    })));

    funcs.push(("constantly".to_string(), Value::NativeFunc(NativeFunc {
        name: "<itertools.constantly>".to_string(),
        func: Rc::new(|args, _ctx| {
            let val = if args.is_empty() { Value::Nil } else { args[0].clone() };
            let name = format!("<constantly>");
            Ok(Value::NativeFunc(NativeFunc {
                name,
                func: Rc::new(move |_, _| Ok(val.clone())),
            }))
        }),
    })));

    funcs
}

fn compare_values(a: &Value, b: &Value, heap: &GcHeap) -> std::cmp::Ordering {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x.cmp(y),
        (Value::Int(x), Value::Float(y)) => (*x as f64).partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
        (Value::Float(x), Value::Int(y)) => x.partial_cmp(&(*y as f64)).unwrap_or(std::cmp::Ordering::Equal),
        (Value::Float(x), Value::Float(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
        (Value::String(x), Value::String(y)) => {
            match (heap.get(*x), heap.get(*y)) {
                (GcObj::String(sx), GcObj::String(sy)) => sx.cmp(sy),
                _ => std::cmp::Ordering::Equal,
            }
        }
        _ => std::cmp::Ordering::Equal,
    }
}

fn call_fn_with_arg(fn_val: &Value, arg: &Value, ctx: &mut VmContext) -> Result<Value, String> {
    match fn_val {
        Value::NativeFunc(f) => {
            (f.func)(&[arg.clone()], ctx)
        }
        // For Lion functions (closures), we'd need full VM integration
        // For now, only support native functions in higher-order contexts
        other => Err(format!("cannot call {} as a function", other.type_name())),
    }
}

fn call_fn_with_two_args(fn_val: &Value, arg1: &Value, arg2: &Value, ctx: &mut VmContext) -> Result<Value, String> {
    match fn_val {
        Value::NativeFunc(f) => {
            (f.func)(&[arg1.clone(), arg2.clone()], ctx)
        }
        other => Err(format!("cannot call {} as a function", other.type_name())),
    }
}
