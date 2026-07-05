use std::rc::Rc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::gc::*;

static LOG_LEVEL: AtomicI32 = AtomicI32::new(0); // 0=DEBUG, 1=INFO, 2=WARN, 3=ERROR

pub fn build_logging() -> Vec<(String, Value)> {
    let mut funcs = Vec::new();

    funcs.push(("debug".to_string(), Value::NativeFunc(NativeFunc {
        name: "<logging.debug>".to_string(),
        func: Rc::new(|args, ctx| {
            if LOG_LEVEL.load(Ordering::Relaxed) <= 0 {
                log_print("DEBUG", args, ctx);
            }
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("info".to_string(), Value::NativeFunc(NativeFunc {
        name: "<logging.info>".to_string(),
        func: Rc::new(|args, ctx| {
            if LOG_LEVEL.load(Ordering::Relaxed) <= 1 {
                log_print("INFO", args, ctx);
            }
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("warn".to_string(), Value::NativeFunc(NativeFunc {
        name: "<logging.warn>".to_string(),
        func: Rc::new(|args, ctx| {
            if LOG_LEVEL.load(Ordering::Relaxed) <= 2 {
                log_print("WARN", args, ctx);
            }
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("error".to_string(), Value::NativeFunc(NativeFunc {
        name: "<logging.error>".to_string(),
        func: Rc::new(|args, ctx| {
            log_print("ERROR", args, ctx);
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("set_level".to_string(), Value::NativeFunc(NativeFunc {
        name: "<logging.set_level>".to_string(),
        func: Rc::new(|args, ctx| {
            let level = args.first().ok_or("logging.set_level requires a level string")?.to_string(ctx.heap);
            match level.to_uppercase().as_str() {
                "DEBUG" => LOG_LEVEL.store(0, Ordering::Relaxed),
                "INFO" => LOG_LEVEL.store(1, Ordering::Relaxed),
                "WARN" | "WARNING" => LOG_LEVEL.store(2, Ordering::Relaxed),
                "ERROR" => LOG_LEVEL.store(3, Ordering::Relaxed),
                _ => return Err(format!("logging.set_level: unknown level '{}'", level)),
            }
            Ok(Value::Nil)
        }),
    })));

    funcs.push(("basic_config".to_string(), Value::NativeFunc(NativeFunc {
        name: "<logging.basic_config>".to_string(),
        func: Rc::new(|args, ctx| {
            if let Some(level_arg) = args.first() {
                let level = level_arg.to_string(ctx.heap);
                match level.to_uppercase().as_str() {
                    "DEBUG" => LOG_LEVEL.store(0, Ordering::Relaxed),
                    "INFO" => LOG_LEVEL.store(1, Ordering::Relaxed),
                    "WARN" | "WARNING" => LOG_LEVEL.store(2, Ordering::Relaxed),
                    "ERROR" => LOG_LEVEL.store(3, Ordering::Relaxed),
                    _ => return Err(format!("logging.basic_config: unknown level '{}'", level)),
                }
            }
            Ok(Value::Nil)
        }),
    })));

    funcs
}

fn log_print(level: &str, args: &[Value], ctx: &mut VmContext) {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let ts = now.as_secs();
    let msg: String = args.iter().map(|a| a.to_string(ctx.heap)).collect::<Vec<_>>().join(" ");
    eprintln!("[{}][{}] {}", ts, level, msg);
}
