pub enum Command {
    Run {
        file: Option<String>,
        disassemble: bool,
    },
    Repl,
    Version,
    Fmt {
        file: String,
    },
    ProjectNew {
        name: String,
    },
    ProjectInit,
    ProjectBuild,
    ProjectRun {
        args: Vec<String>,
    },
    Test {
        filter: Option<String>,
    },
    Help,
}

pub fn parse_args() -> Command {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        return Command::Help;
    }

    match args[1].as_str() {
        "run" => {
            if args.len() >= 3 && !args[2].starts_with('-') {
                let file = args[2].clone();
                let disassemble = args.iter().any(|a| a == "--disassemble");
                Command::Run { file: Some(file), disassemble }
            } else {
                let extra: Vec<String> = args[2..].iter().filter(|a| *a != "--disassemble").cloned().collect();
                if is_in_project() {
                    Command::ProjectRun { args: extra }
                } else {
                    eprintln!("Usage: zamin run <file> [--disassemble]");
                    std::process::exit(1);
                }
            }
        }
        "repl" => Command::Repl,
        "version" | "--version" | "-v" => Command::Version,
        "fmt" => {
            if args.len() < 3 {
                eprintln!("Usage: zamin fmt <file>");
                std::process::exit(1);
            }
            Command::Fmt {
                file: args[2].clone(),
            }
        }
        "new" => {
            if args.len() < 3 {
                eprintln!("Usage: zamin new <project_name>");
                std::process::exit(1);
            }
            Command::ProjectNew { name: args[2].clone() }
        }
        "init" => Command::ProjectInit,
        "build" => Command::ProjectBuild,
        "test" => {
            let filter = if args.len() > 2 {
                Some(args[2].clone())
            } else {
                None
            };
            Command::Test { filter }
        }
        _ => Command::Help,
    }
}

fn is_in_project() -> bool {
    std::env::current_dir()
        .ok()
        .and_then(|d| {
            let mut dir = Some(d.as_path());
            while let Some(p) = dir {
                if p.join("zamin.json").exists() {
                    return Some(true);
                }
                dir = p.parent();
            }
            None
        })
        .unwrap_or(false)
}
