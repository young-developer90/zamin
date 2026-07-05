use std::rc::Rc;
use std::collections::HashMap;
use crate::gc::*;

pub fn build_collections() -> Vec<(String, Value)> {
    let mut funcs = Vec::new();

    funcs.push(("Counter".to_string(), Value::NativeFunc(NativeFunc {
        name: "<collections.Counter>".to_string(),
        func: Rc::new(|args, ctx| {
            let list = args.first().ok_or("collections.Counter requires a list")?;
            let items: Vec<Value> = match list {
                Value::List(r) => match ctx.heap.get(*r) { GcObj::List(items) => items.clone(), _ => return Err("expected list".to_string()) },
                Value::String(r) => {
                    let s = match ctx.heap.get(*r) { GcObj::String(s) => s.clone(), _ => return Err("expected list or string".to_string()) };
                    s.chars().map(|c| make_string(ctx.heap, &c.to_string())).collect()
                }
                _ => return Err("collections.Counter requires a list or string".to_string()),
            };
            let mut buckets: HashMap<u64, Vec<(Value, i64)>> = HashMap::new();
            for item in &items {
                let h = item.hash(ctx.heap);
                let bucket = buckets.entry(h).or_default();
                let mut found = false;
                for (val, count) in bucket.iter_mut() {
                    if val.eq(item, ctx.heap) { *count += 1; found = true; break; }
                }
                if !found { bucket.push((item.clone(), 1)); }
            }
            let mut dict_entries = Vec::new();
            for (_, bucket) in buckets {
                for (k, v) in bucket { dict_entries.push((k, Value::Int(v))); }
            }
            Ok(make_dict(ctx.heap, dict_entries))
        }),
    })));

    funcs.push(("deque".to_string(), Value::NativeFunc(NativeFunc {
        name: "<collections.deque>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.is_empty() { return Ok(make_list(ctx.heap, Vec::new())); }
            match &args[0] {
                Value::List(r) => {
                    let items = match ctx.heap.get(*r) { GcObj::List(items) => items.clone(), _ => return Err("expected list".to_string()) };
                    Ok(make_list(ctx.heap, items))
                }
                _ => Err("collections.deque requires a list or nothing".to_string()),
            }
        }),
    })));

    funcs.push(("push_left".to_string(), Value::NativeFunc(NativeFunc {
        name: "<collections.push_left>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("push_left requires a list and a value".to_string()); }
            match &args[0] {
                Value::List(r) => match ctx.heap.get_mut(*r) {
                    GcObj::List(ref mut items) => { items.insert(0, args[1].clone()); Ok(Value::Nil) }
                    _ => Err("not a list".to_string()),
                },
                _ => Err("push_left requires a list".to_string()),
            }
        }),
    })));

    funcs.push(("pop_left".to_string(), Value::NativeFunc(NativeFunc {
        name: "<collections.pop_left>".to_string(),
        func: Rc::new(|args, ctx| {
            let list_val = args.first().ok_or("pop_left requires a list")?;
            match list_val {
                Value::List(r) => match ctx.heap.get_mut(*r) {
                    GcObj::List(ref mut items) => {
                        if items.is_empty() { return Err("pop_left from empty list".to_string()); }
                        Ok(items.remove(0))
                    }
                    _ => Err("not a list".to_string()),
                },
                _ => Err("pop_left requires a list".to_string()),
            }
        }),
    })));

    funcs.push(("push_right".to_string(), Value::NativeFunc(NativeFunc {
        name: "<collections.push_right>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("push_right requires a list and a value".to_string()); }
            match &args[0] {
                Value::List(r) => match ctx.heap.get_mut(*r) {
                    GcObj::List(ref mut items) => { items.push(args[1].clone()); Ok(Value::Nil) }
                    _ => Err("not a list".to_string()),
                },
                _ => Err("push_right requires a list".to_string()),
            }
        }),
    })));

    funcs.push(("pop_right".to_string(), Value::NativeFunc(NativeFunc {
        name: "<collections.pop_right>".to_string(),
        func: Rc::new(|args, ctx| {
            let list_val = args.first().ok_or("pop_right requires a list")?;
            match list_val {
                Value::List(r) => match ctx.heap.get_mut(*r) {
                    GcObj::List(ref mut items) => items.pop().ok_or("pop_right from empty list".to_string()),
                    _ => Err("not a list".to_string()),
                },
                _ => Err("pop_right requires a list".to_string()),
            }
        }),
    })));

    funcs.push(("group_by".to_string(), Value::NativeFunc(NativeFunc {
        name: "<collections.group_by>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("group_by requires a list and a key function".to_string()); }
            let items = match &args[0] {
                Value::List(r) => match ctx.heap.get(*r) { GcObj::List(items) => items.clone(), _ => return Err("expected list".to_string()) },
                _ => return Err("expected list".to_string()),
            };
            let key_fn = &args[1];
            let mut buckets: HashMap<u64, Vec<(Value, Vec<Value>)>> = HashMap::new();
            for item in &items {
                let key = call_fn_with_arg(key_fn, item, ctx)?;
                let h = key.hash(ctx.heap);
                let bucket = buckets.entry(h).or_default();
                let mut found = false;
                for (k, group) in bucket.iter_mut() {
                    if k.eq(&key, ctx.heap) { group.push(item.clone()); found = true; break; }
                }
                if !found { bucket.push((key, vec![item.clone()])); }
            }
            let mut dict_entries = Vec::new();
            for (_, bucket) in buckets {
                for (k, group) in bucket { dict_entries.push((k, make_list(ctx.heap, group))); }
            }
            Ok(make_dict(ctx.heap, dict_entries))
        }),
    })));

    funcs.push(("flatten".to_string(), Value::NativeFunc(NativeFunc {
        name: "<collections.flatten>".to_string(),
        func: Rc::new(|args, ctx| {
            let list_val = args.first().ok_or("flatten requires a list")?;
            let items = match list_val {
                Value::List(r) => flatten_list(*r, ctx.heap),
                _ => return Err("expected list".to_string()),
            };
            Ok(make_list(ctx.heap, items))
        }),
    })));

    funcs
}

fn flatten_list(r: ObjRef, heap: &mut GcHeap) -> Vec<Value> {
    let items = match heap.get(r) { GcObj::List(items) => items.clone(), _ => return Vec::new() };
    let mut result = Vec::with_capacity(items.len());
    for item in items {
        match item {
            Value::List(r2) => result.extend(flatten_list(r2, heap)),
            other => result.push(other),
        }
    }
    result
}

fn call_fn_with_arg(fn_val: &Value, arg: &Value, ctx: &mut VmContext) -> Result<Value, String> {
    match fn_val {
        Value::NativeFunc(f) => (f.func)(&[arg.clone()], ctx),
        _ => Err("expected a function".to_string()),
    }
}
