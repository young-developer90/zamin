use std::process::Command;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Zamin RS - Quick runner for Zamin files");
        eprintln!("Usage: zamin-rs <file> [args...]");
        std::process::exit(1);
    }

    let file = &args[1];
    let rest: Vec<&str> = args[2..].iter().map(|s| s.as_str()).collect();

    // Find the zamin binary (same directory as zamin-rs)
    let exe = std::env::current_exe().unwrap_or_default();
    let dir = exe.parent().unwrap_or(std::path::Path::new("."));
    let zamin_exe = if cfg!(windows) {
        dir.join("zamin.exe")
    } else {
        dir.join("zamin")
    };

    let status = Command::new(&zamin_exe)
        .arg("run")
        .arg(file)
        .args(&rest)
        .status()
        .unwrap_or_else(|e| {
            // Fallback: try zamin in PATH
            Command::new("zamin")
                .arg("run")
                .arg(file)
                .args(&rest)
                .status()
                .unwrap_or_else(|_| {
                    eprintln!("error: cannot find zamin binary ({})", e);
                    std::process::exit(1);
                })
        });

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
}
