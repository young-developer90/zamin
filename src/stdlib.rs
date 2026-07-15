use std::rc::Rc;

use crate::gc::*;
use crate::html_mod;
use crate::csv_mod;
use crate::stats_mod;
use crate::url_mod;
use crate::json_mod;
use crate::string_mod;
use crate::re_mod;
use crate::datetime_mod;
use crate::logging_mod;
use crate::subprocess_mod;
use crate::path_mod;
use crate::hashlib_mod;
use crate::collections_mod;
use crate::itertools_mod;
use crate::test_mod;

pub fn build_stdlib(heap: &mut GcHeap) -> Vec<(String, Vec<(String, Value)>)> {
    let mut modules = Vec::new();

    // io module
    let mut io = Vec::new();
    io.push((
        "print".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<io.print>".to_string(),
            func: Rc::new(|args, ctx| {
                for arg in args {
                    print!("{}", arg.to_string(ctx.heap));
                }
                Ok(Value::Nil)
            }),
        }),
    ));
    io.push((
        "println".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<io.println>".to_string(),
            func: Rc::new(|args, ctx| {
                for arg in args {
                    print!("{}", arg.to_string(ctx.heap));
                }
                println!();
                Ok(Value::Nil)
            }),
        }),
    ));
    io.push((
        "input".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<io.input>".to_string(),
            func: Rc::new(|args, ctx| {
                use std::io::Write;
                let prompt = args.first().map(|a| a.to_string(ctx.heap)).unwrap_or_default();
                let mut input = String::new();
                print!("{}", prompt);
                std::io::stdout().flush().ok();
                std::io::stdin().read_line(&mut input).ok();
                Ok(make_string(ctx.heap, input.trim()))
            }),
        }),
    ));
    modules.push(("io".to_string(), io));

    // math module
    let mut math = Vec::new();
    math.push((
        "sqrt".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<math.sqrt>".to_string(),
            func: Rc::new(|args, _ctx| {
                let x = to_f64(&args[0])?;
                Ok(Value::Float(x.sqrt()))
            }),
        }),
    ));
    math.push((
        "pow".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<math.pow>".to_string(),
            func: Rc::new(|args, _| {
                if args.len() < 2 { return Err("pow requires 2 arguments".to_string()); }
                let x = to_f64(&args[0])?;
                let y = to_f64(&args[1])?;
                Ok(Value::Float(x.powf(y)))
            }),
        }),
    ));
    math.push((
        "abs".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<math.abs>".to_string(),
            func: Rc::new(|args, _| {
                let x = to_f64(&args[0])?;
                Ok(Value::Float(x.abs()))
            }),
        }),
    ));
    math.push((
        "sin".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<math.sin>".to_string(),
            func: Rc::new(|args, _| {
                let x = to_f64(&args[0])?;
                Ok(Value::Float(x.sin()))
            }),
        }),
    ));
    math.push((
        "cos".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<math.cos>".to_string(),
            func: Rc::new(|args, _| {
                let x = to_f64(&args[0])?;
                Ok(Value::Float(x.cos()))
            }),
        }),
    ));
    math.push((
        "tan".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<math.tan>".to_string(),
            func: Rc::new(|args, _| {
                let x = to_f64(&args[0])?;
                Ok(Value::Float(x.tan()))
            }),
        }),
    ));
    math.push(("pi".to_string(), Value::Float(std::f64::consts::PI)));
    math.push(("e".to_string(), Value::Float(std::f64::consts::E)));
    modules.push(("math".to_string(), math));

    // time module
    let mut time = Vec::new();
    time.push((
        "now".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<time.now>".to_string(),
            func: Rc::new(|_, _| {
                use std::time::{SystemTime, UNIX_EPOCH};
                let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                Ok(Value::Float(dur.as_secs_f64()))
            }),
        }),
    ));
    time.push((
        "unix".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<time.unix>".to_string(),
            func: Rc::new(|_, _| {
                use std::time::{SystemTime, UNIX_EPOCH};
                let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                Ok(Value::Int(dur.as_secs() as i64))
            }),
        }),
    ));
    time.push((
        "sleep".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<time.sleep>".to_string(),
            func: Rc::new(|args, _ctx| {
                let ms = args.first().ok_or("sleep requires ms argument")?;
                let ms_f = to_f64(ms)?;
                std::thread::sleep(std::time::Duration::from_millis(ms_f as u64));
                Ok(Value::Nil)
            }),
        }),
    ));
    modules.push(("time".to_string(), time));

    // rand module
    let mut rand = Vec::new();
    rand.push((
        "int".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<rand.int>".to_string(),
            func: Rc::new(|args, _ctx| {
                if args.len() < 2 { return Err("rand.int requires min and max".to_string()); }
                let min = to_i64(&args[0])?;
                let max = to_i64(&args[1])?;
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
                let mut rng = seed;
                rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                let range = (max - min + 1).abs();
                let val = min + (rng % range as u32) as i64;
                Ok(Value::Int(val.max(min).min(max)))
            }),
        }),
    ));
    rand.push((
        "float".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<rand.float>".to_string(),
            func: Rc::new(|_, _| {
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
                let mut rng = seed as f64;
                rng = (rng * 1103515245.0 + 12345.0) % 2147483648.0;
                Ok(Value::Float(rng / 2147483648.0))
            }),
        }),
    ));
    rand.push((
        "choice".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<rand.choice>".to_string(),
            func: Rc::new(|args, ctx| {
                let list = args.first().ok_or("rand.choice requires a list")?;
                match list {
                    Value::List(r) => {
                        let items = match ctx.heap.get(*r) { GcObj::List(items) => items, _ => return Err("not a list".to_string()) };
                        if items.is_empty() { return Err("empty list".to_string()); }
                        use std::time::{SystemTime, UNIX_EPOCH};
                        let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
                        let idx = (seed as usize) % items.len();
                        Ok(items[idx].clone())
                    }
                    _ => Err("rand.choice requires a list".to_string()),
                }
            }),
        }),
    ));
    modules.push(("rand".to_string(), rand));

    // fs module
    let mut fs = Vec::new();
    fs.push((
        "read".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<fs.read>".to_string(),
            func: Rc::new(|args, ctx| {
                let path = args.first().ok_or("fs.read requires a path")?;
                let path_str = path.to_string(ctx.heap);
                match std::fs::read_to_string(&path_str) {
                    Ok(content) => Ok(make_string(ctx.heap, &content)),
                    Err(e) => Err(format!("fs.read: {}", e)),
                }
            }),
        }),
    ));
    fs.push((
        "write".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<fs.write>".to_string(),
            func: Rc::new(|args, ctx| {
                if args.len() < 2 { return Err("fs.write requires path and text".to_string()); }
                let path = args[0].to_string(ctx.heap);
                let text = args[1].to_string(ctx.heap);
                match std::fs::write(&path, &text) {
                    Ok(()) => Ok(Value::Bool(true)),
                    Err(e) => Err(format!("fs.write: {}", e)),
                }
            }),
        }),
    ));
    fs.push((
        "exists".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<fs.exists>".to_string(),
            func: Rc::new(|args, ctx| {
                let path = args.first().ok_or("fs.exists requires a path")?;
                let path_str = path.to_string(ctx.heap);
                Ok(Value::Bool(std::path::Path::new(&path_str).exists()))
            }),
        }),
    ));
    fs.push((
        "mkdir".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<fs.mkdir>".to_string(),
            func: Rc::new(|args, ctx| {
                let path = args.first().ok_or("fs.mkdir requires a path")?;
                let path_str = path.to_string(ctx.heap);
                match std::fs::create_dir_all(&path_str) {
                    Ok(()) => Ok(Value::Bool(true)),
                    Err(e) => Err(format!("fs.mkdir: {}", e)),
                }
            }),
        }),
    ));
    modules.push(("fs".to_string(), fs));

    // os module
    let mut os = Vec::new();
    os.push((
        "cwd".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<os.cwd>".to_string(),
            func: Rc::new(|_, ctx| {
                match std::env::current_dir() {
                    Ok(path) => Ok(make_string(ctx.heap, &path.to_string_lossy())),
                    Err(e) => Err(format!("os.cwd: {}", e)),
                }
            }),
        }),
    ));
    os.push((
        "args".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<os.args>".to_string(),
            func: Rc::new(|_, ctx| {
                let args: Vec<Value> = std::env::args()
                    .map(|a| make_string(ctx.heap, &a))
                    .collect();
                Ok(make_list(ctx.heap, args))
            }),
        }),
    ));
    os.push((
        "getenv".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<os.getenv>".to_string(),
            func: Rc::new(|args, ctx| {
                let name = args.first().ok_or("os.getenv requires a name")?;
                let name_str = name.to_string(ctx.heap);
                match std::env::var(&name_str) {
                    Ok(val) => Ok(make_string(ctx.heap, &val)),
                    Err(_) => Ok(Value::Nil),
                }
            }),
        }),
    ));
    os.push((
        "name".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<os.name>".to_string(),
            func: Rc::new(|_, ctx| {
                Ok(make_string(ctx.heap, std::env::consts::OS))
            }),
        }),
    ));
    os.push((
        "system".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<os.system>".to_string(),
            func: Rc::new(|args, ctx| {
                if args.is_empty() { return Err("os.system requires a command".to_string()); }
                let cmd_str = args[0].to_string(ctx.heap);
                let shell = if cfg!(windows) { "cmd" } else { "sh" };
                let shell_arg = if cfg!(windows) { "/C" } else { "-c" };
                match std::process::Command::new(shell).arg(shell_arg).arg(&cmd_str).status() {
                    Ok(status) => Ok(Value::Int(status.code().unwrap_or(-1) as i64)),
                    Err(e) => Err(format!("os.system: {}", e)),
                }
            }),
        }),
    ));
    modules.push(("os".to_string(), os));

    // matrix module
    let mut matrix = Vec::new();
    matrix.push((
        "from_rows".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<matrix.from_rows>".to_string(),
            func: Rc::new(|args, ctx| {
                if args.is_empty() { return Err("from_rows requires a list of rows".to_string()); }
                let rows_list = &args[0];
                let rows_items = match rows_list {
                    Value::List(r) => match ctx.heap.get(*r) { GcObj::List(items) => items.clone(), _ => return Err("expected a list".to_string()) },
                    _ => return Err("expected a list".to_string()),
                };
                if rows_items.is_empty() { return Ok(make_matrix(ctx.heap, 0, 0, vec![])); }
                let nrows = rows_items.len();
                let ncols = match &rows_items[0] {
                    Value::List(r) => match ctx.heap.get(*r) { GcObj::List(items) => items.len(), _ => return Err("expected list of lists".to_string()) },
                    _ => return Err("expected list of lists".to_string()),
                };
                let mut data = Vec::with_capacity(nrows * ncols);
                for row_val in &rows_items {
                    let row_list = match row_val {
                        Value::List(r) => match ctx.heap.get(*r) { GcObj::List(items) => items, _ => return Err("expected list of lists".to_string()) },
                        _ => return Err("expected list of lists".to_string()),
                    };
                    if row_list.len() != ncols { return Err("all rows must have the same length".to_string()); }
                    for item in row_list {
                        data.push(to_f64(item)?);
                    }
                }
                Ok(make_matrix(ctx.heap, nrows, ncols, data))
            }),
        }),
    ));
    matrix.push((
        "zeros".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<matrix.zeros>".to_string(),
            func: Rc::new(|args, ctx| {
                if args.len() < 2 { return Err("zeros requires rows and cols".to_string()); }
                let rows = to_i64(&args[0])? as usize;
                let cols = to_i64(&args[1])? as usize;
                Ok(make_matrix(ctx.heap, rows, cols, vec![0.0; rows * cols]))
            }),
        }),
    ));
    matrix.push((
        "ones".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<matrix.ones>".to_string(),
            func: Rc::new(|args, ctx| {
                if args.len() < 2 { return Err("ones requires rows and cols".to_string()); }
                let rows = to_i64(&args[0])? as usize;
                let cols = to_i64(&args[1])? as usize;
                Ok(make_matrix(ctx.heap, rows, cols, vec![1.0; rows * cols]))
            }),
        }),
    ));
    matrix.push((
        "identity".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<matrix.identity>".to_string(),
            func: Rc::new(|args, ctx| {
                if args.is_empty() { return Err("identity requires n".to_string()); }
                let n = to_i64(&args[0])? as usize;
                let mut data = vec![0.0; n * n];
                for i in 0..n { data[i * n + i] = 1.0; }
                Ok(make_matrix(ctx.heap, n, n, data))
            }),
        }),
    ));
    matrix.push((
        "random".to_string(),
        Value::NativeFunc(NativeFunc {
            name: "<matrix.random>".to_string(),
            func: Rc::new(|args, ctx| {
                if args.len() < 2 { return Err("random requires rows and cols".to_string()); }
                let rows = to_i64(&args[0])? as usize;
                let cols = to_i64(&args[1])? as usize;
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos() as u64;
                let mut rng = seed;
                let mut data = Vec::with_capacity(rows * cols);
                for _ in 0..rows * cols {
                    rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                    data.push((rng % 1000000) as f64 / 1000000.0);
                }
                Ok(make_matrix(ctx.heap, rows, cols, data))
            }),
        }),
    ));
    modules.push(("json".to_string(), json_mod::build_json()));
    modules.push(("string".to_string(), string_mod::build_string()));

    modules.push(("html".to_string(), html_mod::build_html()));
    modules.push(("csv".to_string(), csv_mod::build_csv()));
    modules.push(("stats".to_string(), stats_mod::build_stats()));
    modules.push(("url".to_string(), url_mod::build_url()));
    modules.push(("re".to_string(), re_mod::build_re()));
    modules.push(("datetime".to_string(), datetime_mod::build_datetime()));
    modules.push(("logging".to_string(), logging_mod::build_logging()));
    modules.push(("subprocess".to_string(), subprocess_mod::build_subprocess()));
    modules.push(("path".to_string(), path_mod::build_path()));
    modules.push(("hashlib".to_string(), hashlib_mod::build_hashlib()));
    modules.push(("collections".to_string(), collections_mod::build_collections()));
    modules.push(("itertools".to_string(), itertools_mod::build_itertools()));
    modules.push(("test".to_string(), test_mod::build_test()));

    modules
}

fn to_f64(val: &Value) -> Result<f64, String> {
    match val {
        Value::Int(n) => Ok(*n as f64),
        Value::UInt(n) => Ok(*n as f64),
        Value::Float(n) => Ok(*n),
        _ => Err(format!("cannot convert {} to float", val.type_name())),
    }
}

fn to_i64(val: &Value) -> Result<i64, String> {
    match val {
        Value::Int(n) => Ok(*n),
        Value::UInt(n) => Ok(*n as i64),
        Value::Float(n) => Ok(*n as i64),
        _ => Err(format!("cannot convert {} to int", val.type_name())),
    }
}
