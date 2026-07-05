use std::rc::Rc;
use crate::gc::*;

pub fn build_path() -> Vec<(String, Value)> {
    let mut funcs = Vec::new();

    funcs.push(("join".to_string(), Value::NativeFunc(NativeFunc {
        name: "<path.join>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.is_empty() { return Err("path.join requires at least one path component".to_string()); }
            let parts: Vec<String> = args.iter().map(|a| a.to_string(ctx.heap)).collect();
            let joined = parts.join("/").replace("\\", "/");
            Ok(make_string(ctx.heap, &joined))
        }),
    })));

    funcs.push(("basename".to_string(), Value::NativeFunc(NativeFunc {
        name: "<path.basename>".to_string(),
        func: Rc::new(|args, ctx| {
            let path = args.first().ok_or("path.basename requires a path")?.to_string(ctx.heap);
            let p = path.replace("\\", "/");
            let base = p.rsplit('/').next().unwrap_or("");
            Ok(make_string(ctx.heap, base))
        }),
    })));

    funcs.push(("dirname".to_string(), Value::NativeFunc(NativeFunc {
        name: "<path.dirname>".to_string(),
        func: Rc::new(|args, ctx| {
            let path = args.first().ok_or("path.dirname requires a path")?.to_string(ctx.heap);
            let p = path.replace("\\", "/");
            let idx = p.rfind('/').map(|i| i).unwrap_or(0);
            let dir = if idx > 0 { &p[..idx] } else { "." };
            Ok(make_string(ctx.heap, dir))
        }),
    })));

    funcs.push(("ext".to_string(), Value::NativeFunc(NativeFunc {
        name: "<path.ext>".to_string(),
        func: Rc::new(|args, ctx| {
            let path = args.first().ok_or("path.ext requires a path")?.to_string(ctx.heap);
            let p = path.replace("\\", "/");
            let base = p.rsplit('/').next().unwrap_or("");
            let ext = base.rsplit('.').next().filter(|s| *s != base).unwrap_or("");
            Ok(make_string(ctx.heap, ext))
        }),
    })));

    funcs.push(("is_file".to_string(), Value::NativeFunc(NativeFunc {
        name: "<path.is_file>".to_string(),
        func: Rc::new(|args, ctx| {
            let path = args.first().ok_or("path.is_file requires a path")?.to_string(ctx.heap);
            Ok(Value::Bool(std::path::Path::new(&path).is_file()))
        }),
    })));

    funcs.push(("is_dir".to_string(), Value::NativeFunc(NativeFunc {
        name: "<path.is_dir>".to_string(),
        func: Rc::new(|args, ctx| {
            let path = args.first().ok_or("path.is_dir requires a path")?.to_string(ctx.heap);
            Ok(Value::Bool(std::path::Path::new(&path).is_dir()))
        }),
    })));

    funcs.push(("size".to_string(), Value::NativeFunc(NativeFunc {
        name: "<path.size>".to_string(),
        func: Rc::new(|args, ctx| {
            let path = args.first().ok_or("path.size requires a path")?.to_string(ctx.heap);
            match std::fs::metadata(&path) {
                Ok(m) => Ok(Value::Int(m.len() as i64)),
                Err(e) => Err(format!("path.size: {}", e)),
            }
        }),
    })));

    funcs.push(("rename".to_string(), Value::NativeFunc(NativeFunc {
        name: "<path.rename>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("path.rename requires old and new paths".to_string()); }
            let old = args[0].to_string(ctx.heap);
            let new = args[1].to_string(ctx.heap);
            match std::fs::rename(&old, &new) {
                Ok(()) => Ok(Value::Bool(true)),
                Err(e) => Err(format!("path.rename: {}", e)),
            }
        }),
    })));

    funcs.push(("copy".to_string(), Value::NativeFunc(NativeFunc {
        name: "<path.copy>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.len() < 2 { return Err("path.copy requires source and destination".to_string()); }
            let src = args[0].to_string(ctx.heap);
            let dst = args[1].to_string(ctx.heap);
            match std::fs::copy(&src, &dst) {
                Ok(_) => Ok(Value::Bool(true)),
                Err(e) => Err(format!("path.copy: {}", e)),
            }
        }),
    })));

    funcs.push(("remove".to_string(), Value::NativeFunc(NativeFunc {
        name: "<path.remove>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.is_empty() { return Err("path.remove requires a path".to_string()); }
            let path = args[0].to_string(ctx.heap);
            match std::fs::remove_file(&path) {
                Ok(()) => Ok(Value::Bool(true)),
                Err(e) => Err(format!("path.remove: {}", e)),
            }
        }),
    })));

    funcs.push(("remove_dir".to_string(), Value::NativeFunc(NativeFunc {
        name: "<path.remove_dir>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.is_empty() { return Err("path.remove_dir requires a path".to_string()); }
            let path = args[0].to_string(ctx.heap);
            match std::fs::remove_dir_all(&path) {
                Ok(()) => Ok(Value::Bool(true)),
                Err(e) => Err(format!("path.remove_dir: {}", e)),
            }
        }),
    })));

    funcs.push(("list_dir".to_string(), Value::NativeFunc(NativeFunc {
        name: "<path.list_dir>".to_string(),
        func: Rc::new(|args, ctx| {
            let path = args.first().map(|a| a.to_string(ctx.heap)).unwrap_or_else(|| ".".to_string());
            let mut entries = Vec::new();
            match std::fs::read_dir(&path) {
                Ok(dir) => {
                    for entry in dir {
                        match entry {
                            Ok(e) => entries.push(make_string(ctx.heap, &e.file_name().to_string_lossy())),
                            Err(e) => return Err(format!("path.list_dir: {}", e)),
                        }
                    }
                }
                Err(e) => return Err(format!("path.list_dir: {}", e)),
            }
            Ok(make_list(ctx.heap, entries))
        }),
    })));

    funcs.push(("walk".to_string(), Value::NativeFunc(NativeFunc {
        name: "<path.walk>".to_string(),
        func: Rc::new(|args, ctx| {
            let path = args.first().map(|a| a.to_string(ctx.heap)).unwrap_or_else(|| ".".to_string());
            let mut results = Vec::new();
            walk_dir(&path, &path, &mut results, ctx.heap)?;
            Ok(make_list(ctx.heap, results))
        }),
    })));

    funcs.push(("abs".to_string(), Value::NativeFunc(NativeFunc {
        name: "<path.abs>".to_string(),
        func: Rc::new(|args, ctx| {
            let path = args.first().ok_or("path.abs requires a path")?.to_string(ctx.heap);
            match std::path::Path::new(&path).canonicalize() {
                Ok(p) => Ok(make_string(ctx.heap, &p.to_string_lossy())),
                Err(_) => {
                    let cwd = std::env::current_dir().unwrap_or_default();
                    let joined = format!("{}/{}", cwd.to_string_lossy(), path);
                    Ok(make_string(ctx.heap, &joined))
                }
            }
        }),
    })));

    funcs.push(("split".to_string(), Value::NativeFunc(NativeFunc {
        name: "<path.split>".to_string(),
        func: Rc::new(|args, ctx| {
            let path = args.first().ok_or("path.split requires a path")?.to_string(ctx.heap);
            let p = path.replace("\\", "/");
            let idx = p.rfind('/');
            match idx {
                Some(i) => {
                    let dir_s = if i > 0 { p[..i].to_string() } else { "/".to_string() };
                    let base_s = p[i+1..].to_string();
                    let dir = make_string(ctx.heap, &dir_s);
                    let base = make_string(ctx.heap, &base_s);
                    Ok(make_list(ctx.heap, vec![dir, base]))
                }
                None => {
                    let dot = make_string(ctx.heap, ".");
                    let p2 = make_string(ctx.heap, &p);
                    Ok(make_list(ctx.heap, vec![dot, p2]))
                }
            }
        }),
    })));

    funcs
}

fn walk_dir(root: &str, dir: &str, results: &mut Vec<Value>, heap: &mut GcHeap) -> Result<(), String> {
    match std::fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(e) => {
                        let path = e.path();
                        let path_str = path.to_string_lossy().to_string();
                        results.push(make_string(heap, &path_str));
                        if path.is_dir() {
                            walk_dir(root, &path_str, results, heap)?;
                        }
                    }
                    Err(e) => return Err(format!("path.walk: {}", e)),
                }
            }
            Ok(())
        }
        Err(e) => Err(format!("path.walk: {}", e)),
    }
}
