use std::rc::Rc;
use std::process::Command;
use crate::gc::*;

pub fn build_subprocess() -> Vec<(String, Value)> {
    let mut funcs = Vec::new();

    funcs.push(("run".to_string(), Value::NativeFunc(NativeFunc {
        name: "<subprocess.run>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.is_empty() { return Err("subprocess.run requires a command".to_string()); }
            let cmd_str = args[0].to_string(ctx.heap);
            let mut parts: Vec<&str> = cmd_str.split_whitespace().collect();
            if parts.is_empty() { return Err("subprocess.run: empty command".to_string()); }
            let program = parts.remove(0);
            let output = Command::new(program)
                .args(&parts)
                .output()
                .map_err(|e| format!("subprocess.run: {}", e))?;
            Ok(make_result_dict(ctx.heap, &output))
        }),
    })));

    funcs.push(("run_shell".to_string(), Value::NativeFunc(NativeFunc {
        name: "<subprocess.run_shell>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.is_empty() { return Err("subprocess.run_shell requires a command".to_string()); }
            let cmd_str = args[0].to_string(ctx.heap);
            let shell = if cfg!(windows) { "cmd" } else { "sh" };
            let shell_arg = if cfg!(windows) { "/C" } else { "-c" };
            let output = Command::new(shell)
                .arg(shell_arg)
                .arg(&cmd_str)
                .output()
                .map_err(|e| format!("subprocess.run_shell: {}", e))?;
            Ok(make_result_dict(ctx.heap, &output))
        }),
    })));

    funcs.push(("run_output".to_string(), Value::NativeFunc(NativeFunc {
        name: "<subprocess.run_output>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.is_empty() { return Err("subprocess.run_output requires a command".to_string()); }
            let cmd_str = args[0].to_string(ctx.heap);
            let mut parts: Vec<&str> = cmd_str.split_whitespace().collect();
            if parts.is_empty() { return Err("subprocess.run_output: empty command".to_string()); }
            let program = parts.remove(0);
            let output = Command::new(program)
                .args(&parts)
                .output()
                .map_err(|e| format!("subprocess.run_output: {}", e))?;
            Ok(make_string(ctx.heap, &String::from_utf8_lossy(&output.stdout)))
        }),
    })));

    funcs.push(("run_shell_output".to_string(), Value::NativeFunc(NativeFunc {
        name: "<subprocess.run_shell_output>".to_string(),
        func: Rc::new(|args, ctx| {
            if args.is_empty() { return Err("subprocess.run_shell_output requires a command".to_string()); }
            let cmd_str = args[0].to_string(ctx.heap);
            let shell = if cfg!(windows) { "cmd" } else { "sh" };
            let shell_arg = if cfg!(windows) { "/C" } else { "-c" };
            let output = Command::new(shell)
                .arg(shell_arg)
                .arg(&cmd_str)
                .output()
                .map_err(|e| format!("subprocess.run_shell_output: {}", e))?;
            Ok(make_string(ctx.heap, &String::from_utf8_lossy(&output.stdout)))
        }),
    })));

    funcs
}

fn make_result_dict(heap: &mut GcHeap, output: &std::process::Output) -> Value {
    let mut entries = Vec::new();
    entries.push((make_string(heap, "returncode"), Value::Int(output.status.code().unwrap_or(-1) as i64)));
    entries.push((make_string(heap, "stdout"), make_string(heap, &String::from_utf8_lossy(&output.stdout))));
    entries.push((make_string(heap, "stderr"), make_string(heap, &String::from_utf8_lossy(&output.stderr))));
    make_dict(heap, entries)
}
