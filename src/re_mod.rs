use std::rc::Rc;
use regex::Regex;
use crate::gc::*;

pub fn build_re() -> Vec<(String, Value)> {
    let mut funcs = Vec::new();

    funcs.push(("find".to_string(), Value::NativeFunc(NativeFunc {
        name: "<re.find>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("re.find requires pattern and string".to_string()); }
            let pattern = args[0].to_string(ctx.heap);
            let text = args[1].to_string(ctx.heap);
            let re = Regex::new(&pattern).map_err(|e| format!("re.find: {}", e))?;
            if let Some(m) = re.find(&text) {
                Ok(make_string(ctx.heap, m.as_str()))
            } else { Ok(Value::Nil) }
        }),
    })));

    funcs.push(("is_match".to_string(), Value::NativeFunc(NativeFunc {
        name: "<re.is_match>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("re.is_match requires pattern and string".to_string()); }
            let pattern = args[0].to_string(ctx.heap);
            let text = args[1].to_string(ctx.heap);
            let re = Regex::new(&pattern).map_err(|e| format!("re.is_match: {}", e))?;
            Ok(Value::Bool(re.is_match(&text)))
        }),
    })));

    funcs.push(("split".to_string(), Value::NativeFunc(NativeFunc {
        name: "<re.split>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("re.split requires pattern and string".to_string()); }
            let pattern = args[0].to_string(ctx.heap);
            let text = args[1].to_string(ctx.heap);
            let re = Regex::new(&pattern).map_err(|e| format!("re.split: {}", e))?;
            let parts: Vec<Value> = re.split(&text).map(|s| make_string(ctx.heap, s)).collect();
            Ok(make_list(ctx.heap, parts))
        }),
    })));

    funcs.push(("sub".to_string(), Value::NativeFunc(NativeFunc {
        name: "<re.sub>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 3 { return Err("re.sub requires pattern, replacement, and string".to_string()); }
            let pattern = args[0].to_string(ctx.heap);
            let replacement = args[1].to_string(ctx.heap);
            let text = args[2].to_string(ctx.heap);
            let re = Regex::new(&pattern).map_err(|e| format!("re.sub: {}", e))?;
            let result = re.replace(&text, replacement.as_str());
            Ok(make_string(ctx.heap, &result))
        }),
    })));

    funcs.push(("sub_all".to_string(), Value::NativeFunc(NativeFunc {
        name: "<re.sub_all>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 3 { return Err("re.sub_all requires pattern, replacement, and string".to_string()); }
            let pattern = args[0].to_string(ctx.heap);
            let replacement = args[1].to_string(ctx.heap);
            let text = args[2].to_string(ctx.heap);
            let re = Regex::new(&pattern).map_err(|e| format!("re.sub_all: {}", e))?;
            let result = re.replace_all(&text, replacement.as_str());
            Ok(make_string(ctx.heap, &result))
        }),
    })));

    funcs.push(("find_all".to_string(), Value::NativeFunc(NativeFunc {
        name: "<re.find_all>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("re.find_all requires pattern and string".to_string()); }
            let pattern = args[0].to_string(ctx.heap);
            let text = args[1].to_string(ctx.heap);
            let re = Regex::new(&pattern).map_err(|e| format!("re.find_all: {}", e))?;
            let matches: Vec<Value> = re.find_iter(&text).map(|m| make_string(ctx.heap, m.as_str())).collect();
            Ok(make_list(ctx.heap, matches))
        }),
    })));

    funcs.push(("captures".to_string(), Value::NativeFunc(NativeFunc {
        name: "<re.captures>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("re.captures requires pattern and string".to_string()); }
            let pattern = args[0].to_string(ctx.heap);
            let text = args[1].to_string(ctx.heap);
            let re = Regex::new(&pattern).map_err(|e| format!("re.captures: {}", e))?;
            if let Some(caps) = re.captures(&text) {
                let groups: Vec<Value> = caps.iter()
                    .map(|m| m.map(|s| make_string(ctx.heap, s.as_str())).unwrap_or(Value::Nil))
                    .collect();
                Ok(make_list(ctx.heap, groups))
            } else { Ok(Value::Nil) }
        }),
    })));

    funcs
}
