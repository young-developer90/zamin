use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, IsTerminal, Read, Write};
use std::path::PathBuf;

use crate::module::ModuleLoader;

macro_rules! wout {
    ($dst:expr, $($arg:tt)*) => {
        write!($dst, $($arg)*).map_err(|e| e.to_string())?
    };
}

macro_rules! woutln {
    ($dst:expr, $($arg:tt)*) => {
        writeln!($dst, $($arg)*).map_err(|e| e.to_string())?
    };
}

macro_rules! flush {
    ($dst:expr) => {
        $dst.flush().map_err(|e| e.to_string())?
    };
}

fn history_path() -> PathBuf {
    let mut p = std::env::var("HOME").map(PathBuf::from).unwrap_or_default();
    p.push(".lion_history");
    p
}

fn load_history() -> Vec<String> {
    let p = history_path();
    let file = match File::open(&p) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    BufReader::new(file).lines().filter_map(|l| l.ok()).filter(|l| !l.is_empty()).collect()
}

fn save_history(history: &[String]) {
    if let Ok(mut file) = fs::File::create(history_path()) {
        let max = history.len().saturating_sub(1000);
        for line in history.iter().skip(max) {
            let _ = writeln!(file, "{}", line);
        }
    }
}

fn build_completions() -> HashSet<String> {
    let mut s = HashSet::new();
    // Keywords
    for kw in &["if", "else", "elif", "while", "for", "in", "match", "case",
                 "func", "return", "let", "mut", "const", "true", "false",
                 "nil", "import", "export", "as", "struct", "throw", "try",
                 "catch", "or", "and", "not", "break", "continue"] {
        s.insert(kw.to_string());
    }
    // Built-in functions & modules
    for name in &[
        "print", "assert", "type", "len", "str", "int", "float", "bool",
        "range", "input", "tostr",
        "math", "time", "rand", "fs", "os", "json", "csv", "html", "http",
        "url", "stats", "re", "datetime", "logging", "subprocess", "path",
        "hashlib", "string", "collections", "itertools", "test", "panda",
    ] {
        s.insert(name.to_string());
    }
    // linum module functions
    for name in &[
        "linum", "linum.Linear", "linum.ReLU", "linum.Sigmoid", "linum.Tanh",
        "linum.Sequential", "linum.MSELoss", "linum.CrossEntropyLoss",
        "linum.SGD", "linum.Adam", "linum.train", "linum.dense", "linum.relu",
        "linum.sigmoid", "linum.sequential", "linum.get_device",
    ] {
        s.insert(name.to_string());
    }
    // panda module functions
    for name in &[
        "panda.arange", "panda.zeros", "panda.ones", "panda.linspace",
        "panda.sum", "panda.mean", "panda.min", "panda.max", "panda.std",
        "panda.abs", "panda.sin", "panda.cos", "panda.sqrt", "panda.pow",
        "panda.add", "panda.sub", "panda.mul", "panda.dot", "panda.shape",
        "panda.reshape", "panda.eye",
    ] {
        s.insert(name.to_string());
    }
    s
}

pub struct Repl {
    loader: ModuleLoader,
    history: Vec<String>,
    history_idx: usize,
    completions: HashSet<String>,
}

impl Repl {
    pub fn new() -> Self {
        let mut loader = ModuleLoader::new();
        loader.load_stdlib();
        let history = load_history();
        Repl {
            loader,
            history,
            history_idx: 0,
            completions: build_completions(),
        }
    }

    pub fn run(&mut self) -> Result<(), String> {
        println!("\x1b[1mLion REPL v{}\x1b[0m", env!("CARGO_PKG_VERSION"));
        println!("Type 'exit' to quit, 'help' for help.");

        let result = if io::stdin().is_terminal() {
            self.run_raw()
        } else {
            self.run_line_buffered()
        };

        save_history(&self.history);
        result
    }

    fn run_line_buffered(&mut self) -> Result<(), String> {
        loop {
            print!("lion> ");
            io::stdout().flush().map_err(|e| e.to_string())?;

            let mut input = String::new();
            io::stdin().read_line(&mut input).map_err(|e| e.to_string())?;
            let input = input.trim().to_string();

            if input.is_empty() {
                continue;
            }

            if !self.handle_builtin(&input) {
                break;
            }

            let trimmed = input.trim().to_string();
            if !trimmed.is_empty() {
                self.history.push(trimmed.clone());
                self.history_idx = self.history.len();
            }

            let full_input = if needs_more_input(&input) {
                let mut full = input;
                loop {
                    print!("... ");
                    io::stdout().flush().map_err(|e| e.to_string())?;
                    let mut line = String::new();
                    io::stdin().read_line(&mut line).map_err(|e| e.to_string())?;
                    full.push('\n');
                    full.push_str(line.trim_end());
                    if !needs_more_input(&full) {
                        break;
                    }
                }
                full
            } else {
                input
            };

            match self.loader.execute_source(&full_input) {
                Ok(result) => {
                    if !result.is_empty() && result != "nil" {
                        println!("\x1b[32m=> {}\x1b[0m", result);
                    }
                }
                Err(e) => {
                    eprintln!("\x1b[31m(!) {}\x1b[0m", e);
                }
            }
        }
        Ok(())
    }

    fn run_raw(&mut self) -> Result<(), String> {
        enable_raw_mode()?;
        let result = self.run_raw_inner();
        let _ = disable_raw_mode();
        result
    }

    fn run_raw_inner(&mut self) -> Result<(), String> {
        let stdout = io::stdout();
        let mut stdout = stdout.lock();

        loop {
            let input = self.read_line(&mut stdout, "lion> ")?;
            let input = input.trim().to_string();

            if input.is_empty() {
                continue;
            }

            if !self.handle_builtin_raw(&input, &mut stdout)? {
                woutln!(stdout, "\rGoodbye!");
                flush!(stdout);
                break;
            }

            let full_input = if needs_more_input(&input) {
                self.read_continuation(&mut stdout, &input)?
            } else {
                input
            };

            if full_input == "exit" {
                woutln!(stdout, "\rGoodbye!");
                flush!(stdout);
                break;
            }

            if !full_input.trim().is_empty() {
                self.history.push(full_input.trim().to_string());
                self.history_idx = self.history.len();
            }

            match self.loader.execute_source(&full_input) {
                Ok(result) => {
                    if !result.is_empty() && result != "nil" {
                        woutln!(stdout, "\r\x1b[32m=> {}\x1b[0m", result);
                    }
                }
                Err(e) => {
                    woutln!(stdout, "\r\x1b[31m(!) {}\x1b[0m", e);
                }
            }
            flush!(stdout);
        }
        Ok(())
    }

    fn handle_builtin(&mut self, input: &str) -> bool {
        match input {
            "exit" | "quit" => {
                println!("Goodbye!");
                false
            }
            "help" => {
                println!("Lion REPL commands:");
                println!("  exit/quit - Exit the REPL");
                println!("  help      - Show this help");
                println!("  history   - Show command history");
                true
            }
            "history" => {
                for (i, cmd) in self.history.iter().enumerate() {
                    println!("  {}: {}", i + 1, cmd);
                }
                true
            }
            _ => true,
        }
    }

    fn handle_builtin_raw(
        &mut self,
        input: &str,
        stdout: &mut io::StdoutLock<'_>,
    ) -> Result<bool, String> {
        match input {
            "exit" | "quit" => Ok(false),
            "help" => {
                woutln!(stdout, "\rLion REPL commands:");
                woutln!(stdout, "\r  exit/quit - Exit the REPL");
                woutln!(stdout, "\r  help      - Show this help");
                woutln!(stdout, "\r  history   - Show command history");
                flush!(stdout);
                Ok(true)
            }
            "history" => {
                for (i, cmd) in self.history.iter().enumerate() {
                    woutln!(stdout, "\r  {}: {}", i + 1, cmd);
                }
                flush!(stdout);
                Ok(true)
            }
            _ => Ok(true),
        }
    }

    fn find_completions(&self, prefix: &str) -> Vec<String> {
        let prefix = prefix.trim();
        if prefix.is_empty() {
            return Vec::new();
        }

        let mut matches: Vec<String> = self.completions.iter()
            .filter(|c| c.starts_with(prefix))
            .cloned()
            .collect();
        matches.sort();
        matches
    }

    fn read_line(
        &mut self,
        stdout: &mut io::StdoutLock<'_>,
        prompt: &str,
    ) -> Result<String, String> {
        let stdin = io::stdin();
        let mut stdin = stdin.lock();
        let mut buf = String::new();
        let mut cursor = 0;
        let mut history_idx = self.history.len();

        wout!(stdout, "\r{}", prompt);
        flush!(stdout);

        loop {
            let byte = match read_byte(&mut stdin) {
                Ok(b) => b,
                Err(_) => return Ok(buf),
            };

            match byte {
                b'\n' | b'\r' => {
                    wout!(stdout, "\r\n");
                    return Ok(buf);
                }
                b'\x7f' | b'\x08' => {
                    if cursor > 0 && !buf.is_empty() {
                        cursor -= 1;
                        buf.remove(cursor);
                        redraw_line(stdout, prompt, &buf, cursor)?;
                    }
                }
                b'\t' => {
                    // tab completion: find word left of cursor
                    let before = &buf[..cursor];
                    let word_start = before.rfind(|c: char| c.is_whitespace() || c == '(' || c == ')' || c == ',' || c == '"' || c == '\'')
                        .map(|i| i + 1)
                        .unwrap_or(0);
                    let prefix = &buf[word_start..cursor];
                    let matches = self.find_completions(prefix);
                    if matches.len() == 1 {
                        // replace prefix with the full match
                        let rest = &buf[cursor..];
                        buf = format!("{}{}{}", &buf[..word_start], matches[0], rest);
                        cursor = word_start + matches[0].len();
                        redraw_line(stdout, prompt, &buf, cursor)?;
                    } else if matches.len() > 1 {
                        // show all matches
                        woutln!(stdout, "");
                        for m in &matches {
                            woutln!(stdout, "\r\x1b[2m{}\x1b[0m", m);
                        }
                        redraw_line(stdout, prompt, &buf, cursor)?;
                    }
                }
                b'\x1b' => {
                    let seq = read_escape(&mut stdin);
                    match seq.as_slice() {
                        [b'[', b'A'] => {
                            if !self.history.is_empty() && history_idx > 0 {
                                history_idx -= 1;
                                buf = self.history[history_idx].clone();
                                cursor = buf.len();
                                redraw_line(stdout, prompt, &buf, cursor)?;
                            }
                        }
                        [b'[', b'B'] => {
                            if history_idx < self.history.len() {
                                history_idx += 1;
                                if history_idx == self.history.len() {
                                    buf.clear();
                                } else {
                                    buf = self.history[history_idx].clone();
                                }
                                cursor = buf.len();
                                redraw_line(stdout, prompt, &buf, cursor)?;
                            }
                        }
                        [b'[', b'C'] => {
                            if cursor < buf.len() {
                                cursor += 1;
                                wout!(stdout, "\x1b[C");
                                flush!(stdout);
                            }
                        }
                        [b'[', b'D'] => {
                            if cursor > 0 {
                                cursor -= 1;
                                wout!(stdout, "\x1b[D");
                                flush!(stdout);
                            }
                        }
                        [b'[', b'H'] | [b'[', b'1', b'~'] => {
                            cursor = 0;
                            wout!(stdout, "\r\x1b[{}G", 1);
                            flush!(stdout);
                        }
                        [b'[', b'F'] | [b'[', b'4', b'~'] => {
                            cursor = buf.len();
                            redraw_line(stdout, prompt, &buf, cursor)?;
                        }
                        [b'[', b'3', b'~'] => {
                            if cursor < buf.len() {
                                buf.remove(cursor);
                                redraw_line(stdout, prompt, &buf, cursor)?;
                            }
                        }
                        _ => {}
                    }
                }
                b'\x03' | b'\x04' => {
                    woutln!(stdout, "");
                    return Ok("exit".to_string());
                }
                0x01 => {
                    cursor = 0;
                    wout!(stdout, "\r\x1b[{}G", 1);
                    flush!(stdout);
                }
                0x05 => {
                    cursor = buf.len();
                    redraw_line(stdout, prompt, &buf, cursor)?;
                }
                0x0b => {
                    buf.truncate(cursor);
                    redraw_line(stdout, prompt, &buf, cursor)?;
                }
                0x15 => {
                    buf.clear();
                    cursor = 0;
                    redraw_line(stdout, prompt, &buf, cursor)?;
                }
                _b @ 0x08..=0x1a => {}
                c => {
                    buf.insert(cursor, c as char);
                    cursor += 1;
                    redraw_line(stdout, prompt, &buf, cursor)?;
                }
            }
        }
    }

    fn read_continuation(
        &mut self,
        stdout: &mut io::StdoutLock<'_>,
        first: &str,
    ) -> Result<String, String> {
        let mut full = first.to_string();
        loop {
            let line = self.read_line(stdout, "... ")?;
            if line.trim() == "exit" {
                return Ok("exit".to_string());
            }
            full.push('\n');
            full.push_str(&line);
            if !needs_more_input(&full) {
                return Ok(full);
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn enable_raw_mode() -> Result<(), String> {
    let status = std::process::Command::new("stty")
        .args(["raw", "-echo"])
        .status()
        .map_err(|e| format!("stty: {}", e))?;
    if !status.success() {
        return Err("stty failed to enable raw mode".to_string());
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn disable_raw_mode() -> Result<(), String> {
    let status = std::process::Command::new("stty")
        .args(["sane"])
        .status()
        .map_err(|e| format!("stty: {}", e))?;
    if !status.success() {
        return Err("stty failed to restore terminal".to_string());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn enable_raw_mode() -> Result<(), String> {
    unsafe {
        let console = GetStdHandle(STD_INPUT_HANDLE);
        if console == INVALID_HANDLE_VALUE {
            return Err("GetStdHandle failed".to_string());
        }
        let mut mode: u32 = 0;
        if GetConsoleMode(console, &mut mode) == 0 {
            return Err("GetConsoleMode failed".to_string());
        }
        // Disable ENABLE_LINE_INPUT and ENABLE_ECHO_INPUT
        let new_mode = mode & !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT);
        if SetConsoleMode(console, new_mode) == 0 {
            return Err("SetConsoleMode failed".to_string());
        }
        // Store original mode for restoration
        let _ = raw_mode_store(mode, true);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn disable_raw_mode() -> Result<(), String> {
    unsafe {
        let console = GetStdHandle(STD_INPUT_HANDLE);
        if console == INVALID_HANDLE_VALUE {
            return Err("GetStdHandle failed".to_string());
        }
        if let Some(original_mode) = raw_mode_store(0, false) {
            if SetConsoleMode(console, original_mode) == 0 {
                return Err("SetConsoleMode failed".to_string());
            }
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn raw_mode_store(mode: u32, set: bool) -> Option<u32> {
    use std::sync::Mutex;
    static MODE: Mutex<Option<u32>> = Mutex::new(None);
    let mut guard = MODE.lock().unwrap();
    if set {
        *guard = Some(mode);
        None
    } else {
        guard.take()
    }
}

#[cfg(target_os = "windows")]
extern "system" {
    fn GetStdHandle(nStdHandle: u32) -> isize;
    fn GetConsoleMode(hConsoleHandle: isize, lpMode: *mut u32) -> u32;
    fn SetConsoleMode(hConsoleHandle: isize, dwMode: u32) -> u32;
}

#[cfg(target_os = "windows")]
const STD_INPUT_HANDLE: u32 = 0xFFFFFFF6;
#[cfg(target_os = "windows")]
const INVALID_HANDLE_VALUE: isize = -1;
#[cfg(target_os = "windows")]
const ENABLE_LINE_INPUT: u32 = 0x0002;
#[cfg(target_os = "windows")]
const ENABLE_ECHO_INPUT: u32 = 0x0004;

fn read_byte(stdin: &mut io::StdinLock<'_>) -> Result<u8, String> {
    let mut buf = [0u8; 1];
    stdin.read_exact(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf[0])
}

fn read_escape(stdin: &mut io::StdinLock<'_>) -> Vec<u8> {
    let mut seq = vec![b'\x1b'];
    let b = read_byte(stdin).unwrap_or(b'\0');
    seq.push(b);
    if b == b'[' {
        loop {
            let b = read_byte(stdin).unwrap_or(b'\0');
            seq.push(b);
            if b.is_ascii_alphabetic() || b == b'~' || b == b'\0' {
                break;
            }
        }
    } else if b == b'O' {
        let b = read_byte(stdin).unwrap_or(b'\0');
        seq.push(b);
    }
    seq
}

fn redraw_line(
    stdout: &mut io::StdoutLock<'_>,
    prompt: &str,
    buf: &str,
    cursor: usize,
) -> Result<(), String> {
    let prompt_len = prompt.len();
    wout!(stdout, "\r{}{}\x1b[K", prompt, buf);
    if cursor < buf.len() {
        let col = prompt_len + cursor + 1;
        wout!(stdout, "\r\x1b[{}G", col);
    }
    flush!(stdout);
    Ok(())
}

fn count_open_braces(s: &str) -> i32 {
    let mut count = 0;
    for c in s.chars() {
        match c {
            '{' | '(' | '[' => count += 1,
            _ => {}
        }
    }
    count
}

fn count_close_braces(s: &str) -> i32 {
    let mut count = 0;
    for c in s.chars() {
        match c {
            '}' | ')' | ']' => count += 1,
            _ => {}
        }
    }
    count
}

fn needs_more_input(s: &str) -> bool {
    count_open_braces(s) > count_close_braces(s)
}
