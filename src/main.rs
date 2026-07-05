mod ast;
mod bytecode;
mod cext;
mod cli;
mod collections_mod;
mod compiler;
mod csv_mod;
mod datetime_mod;
mod gc;
mod hashlib_mod;
mod html_mod;
mod http;
mod http_module;
mod itertools_mod;
mod json_mod;
mod leopard_mod;
mod lexer;
mod logging_mod;
mod module;
mod parser;
mod path_mod;
mod re_mod;
mod repl;
mod stats_mod;
mod stdlib;
mod string_mod;
mod subprocess_mod;
mod test_mod;
mod url_mod;
pub mod vm;

#[cfg(cuda_support)]
mod cuda;
#[cfg(cuda_support)]
mod linum;

#[cfg(feature = "python")]
mod py;

use cli::Command;

fn main() {
    let cmd = cli::parse_args();

    match cmd {
        Command::Run { file, disassemble } => {
            if disassemble {
                match disassemble_file(&file) {
                    Ok(output) => println!("{}", output),
                    Err(e) => eprintln!("Error: {}", e),
                }
            } else {
                let mut loader = module::ModuleLoader::new();
                loader.load_stdlib();
                match loader.execute_file(&file) {
                    Ok(()) => {}
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
        Command::Repl => {
            let mut repl = repl::Repl::new();
            match repl.run() {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Command::Version => {
            println!("Lion v{}", env!("CARGO_PKG_VERSION"));
        }
        Command::Fmt { file } => {
            match fmt_file(&file) {
                Ok(()) => println!("Formatted: {}", file),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Command::Test { path } => {
            let test_path = path.unwrap_or_else(|| ".".to_string());
            match run_tests(&test_path) {
                Ok(results) => {
                    for (name, passed, msg) in &results {
                        if *passed {
                            println!("PASS: {}", name);
                        } else {
                            println!("FAIL: {}: {}", name, msg);
                        }
                    }
                    let passed = results.iter().filter(|r| r.1).count();
                    let total = results.len();
                    println!("\n{}/{} tests passed", passed, total);
                    if passed != total {
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Command::Help => {
            println!("Lion v{}", env!("CARGO_PKG_VERSION"));
            println!("Usage:");
            println!("  lion run <file>           Run a Lion source file");
            println!("  lion run --disassemble    Show bytecode disassembly");
            println!("  lion repl                 Start interactive REPL");
            println!("  lion version              Show version");
            println!("  lion fmt <file>           Format a source file");
            println!("  lion test [path]          Run tests");
        }
    }
}

fn disassemble_file(path: &str) -> Result<String, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read file '{}': {}", path, e))?;

    let mut parser = parser::Parser::new(&source);
    let program = parser.parse().map_err(|e| e.to_string())?;

    let mut compiler = compiler::Compiler::new();
    compiler.compile(&program)?;

    let mut output = String::new();
    for (i, chunk) in compiler.chunks.iter().enumerate() {
        output.push_str(&format!("--- Chunk {} ---\n", i));
        output.push_str(&chunk.disassemble());
        output.push('\n');
    }

    Ok(output)
}

fn fmt_file(path: &str) -> Result<(), String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read file '{}': {}", path, e))?;

    let mut parser = parser::Parser::new(&source);
    let program = parser.parse().map_err(|e| e.to_string())?;

    let mut formatted = String::new();
    for stmt in &program.stmts {
        formatted.push_str(&stmt_to_string(stmt));
        formatted.push('\n');
    }

    std::fs::write(path, &formatted)
        .map_err(|e| format!("cannot write file '{}': {}", path, e))
}

fn run_tests(path: &str) -> Result<Vec<(String, bool, String)>, String> {
    let mut results = Vec::new();

    let skip_tests = ["test_lsp"];

    let test_files = if std::path::Path::new(path).is_dir() {
        let mut files = Vec::new();
        for entry in std::fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let entry_path = entry.path();
            if entry_path.extension().and_then(|s| s.to_str()) == Some("lion") {
                let fname = entry_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if !skip_tests.iter().any(|s| fname.starts_with(s)) {
                    files.push(entry_path.to_string_lossy().to_string());
                }
            }
        }
        files
    } else {
        vec![path.to_string()]
    };

    for file in &test_files {
        let name = std::path::Path::new(&file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&file)
            .to_string();

        // Run as subprocess to capture all output (stdout + stderr)
        let exe_path = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("lion"));
        let output = match std::process::Command::new(&exe_path)
            .arg("run")
            .arg(file)
            .output()
        {
            Ok(out) => out,
            Err(e) => {
                results.push((name, false, format!("cannot run test: {}", e)));
                continue;
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = format!("{}{}", stdout, stderr);
        let exit_ok = output.status.success();

        let has_fail = combined.contains("FAIL:") || (!exit_ok && combined.contains("Error"));
        let passed = !has_fail;
        results.push((name, passed, if passed { String::new() } else { combined }));
    }

    Ok(results)
}

fn stmt_to_string(stmt: &ast::Stmt) -> String {
    match stmt {
        ast::Stmt::Let { name, value, is_const, .. } => {
            let kw = if *is_const { "const" } else { "let" };
            let val_str = expr_to_string(value);
            format!("{} {} = {};", kw, name, val_str)
        }
        ast::Stmt::Return(Some(expr)) => format!("return {};", expr_to_string(expr)),
        ast::Stmt::Return(None) => "return;".to_string(),
        ast::Stmt::Expr(expr) => format!("{};", expr_to_string(expr)),
        ast::Stmt::StructDef { name, methods } => {
            let mut s = format!("struct {} {{\n", name);
            for stmt in methods {
                s.push_str("    ");
                s.push_str(&stmt_to_string(stmt));
                s.push('\n');
            }
            s.push('}');
            s
        }
        ast::Stmt::FuncDef { name, params, body, .. } => {
            let params_str = params.join(", ");
            let mut s = format!("func {}(", name);
            s.push_str(&params_str);
            s.push_str(") {\n");
            for stmt in body {
                s.push_str("    ");
                s.push_str(&stmt_to_string(stmt));
                s.push('\n');
            }
            s.push('}');
            s
        }
        _ => format!("{:?}", stmt),
    }
}

fn expr_to_string(expr: &ast::Expr) -> String {
    match expr {
        ast::Expr::Int(n) => n.to_string(),
        ast::Expr::UInt(n) => n.to_string(),
        ast::Expr::Float(n) => {
            if n.fract() == 0.0 {
                format!("{}.0", n)
            } else {
                n.to_string()
            }
        }
        ast::Expr::String(s) => format!("\"{}\"", s),
        ast::Expr::Bool(b) => b.to_string(),
        ast::Expr::Nil => "nil".to_string(),
        ast::Expr::Identifier(name) => name.clone(),
        ast::Expr::BinaryOp { op, left, right } => {
            let op_str = match op {
                ast::BinaryOpKind::Add => "+",
                ast::BinaryOpKind::Sub => "-",
                ast::BinaryOpKind::Mul => "*",
                ast::BinaryOpKind::Div => "/",
                ast::BinaryOpKind::Mod => "%",
                ast::BinaryOpKind::Pow => "**",
                ast::BinaryOpKind::IntDiv => "//",
                ast::BinaryOpKind::Eq => "==",
                ast::BinaryOpKind::Ne => "!=",
                ast::BinaryOpKind::Lt => "<",
                ast::BinaryOpKind::Gt => ">",
                ast::BinaryOpKind::Le => "<=",
                ast::BinaryOpKind::Ge => ">=",
                ast::BinaryOpKind::And => "and",
                ast::BinaryOpKind::Or => "or",
                ast::BinaryOpKind::Concat => "..",
                ast::BinaryOpKind::In => "in",
            };
            format!("{} {} {}", expr_to_string(left), op_str, expr_to_string(right))
        }
        ast::Expr::UnaryOp { op, operand } => {
            let op_str = match op {
                ast::UnaryOpKind::Neg => "-",
                ast::UnaryOpKind::Not => "not ",
            };
            format!("{}{}", op_str, expr_to_string(operand))
        }
        ast::Expr::Call { callee, args, .. } => {
            let args_str: Vec<String> = args.iter().map(|a| expr_to_string(a)).collect();
            format!("{}({})", expr_to_string(callee), args_str.join(", "))
        }
        ast::Expr::Index { obj, index } => {
            format!("{}[{}]", expr_to_string(obj), expr_to_string(index))
        }
        ast::Expr::Attr { obj, name } => {
            format!("{}.{}", expr_to_string(obj), name)
        }
        ast::Expr::List(items) => {
            let items_str: Vec<String> = items.iter().map(|i| expr_to_string(i)).collect();
            format!("[{}]", items_str.join(", "))
        }
        ast::Expr::Dict(entries) => {
            let entries_str: Vec<String> = entries
                .iter()
                .map(|(k, v)| format!("{}: {}", expr_to_string(k), expr_to_string(v)))
                .collect();
            format!("{{{}}}", entries_str.join(", "))
        }
        ast::Expr::Tuple(items) => {
            let items_str: Vec<String> = items.iter().map(|i| expr_to_string(i)).collect();
            format!("({})", items_str.join(", "))
        }
        ast::Expr::Lambda { params, body } => {
            format!("|{}| {}", params.join(", "), expr_to_string(body))
        }
        ast::Expr::Range { start, end, step } => {
            if *step == 1 {
                format!("{}..{}", expr_to_string(start), expr_to_string(end))
            } else {
                format!("{}..{}..{}", expr_to_string(start), step, expr_to_string(end))
            }
        }
        ast::Expr::Ternary { condition, then_expr, else_expr } => {
            format!("{} ? {} : {}", expr_to_string(condition), expr_to_string(then_expr), expr_to_string(else_expr))
        }
        ast::Expr::NamedArg { name, value } => {
            format!("{} = {}", name, expr_to_string(value))
        }
        _ => format!("{:?}", expr),
    }
}
