# Lion Programming Language

[![Rust](https://img.shields.io/badge/Rust-1.80%2B-dea584?logo=rust)](https://rustup.rs/)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Version](https://img.shields.io/badge/version-1.4.2-green)](https://github.com/young-developer90/lion/releases)
[![Build](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/young-developer90/lion/actions)
[![PRs](https://img.shields.io/badge/PRs-welcome-orange)](https://github.com/young-developer90/lion/pulls)

Lion is a simple, expressive scripting language with a Rust-based interpreter. It combines modern language features — closures, pattern matching, string interpolation, and a module system — with a lightweight bytecode VM and optional CUDA GPU acceleration.

```
print("Hello, Lion!");
```

## Philosophy

Lion is designed to be:

- **Readable** — syntax inspired by Swift, Kotlin, and Lua. No sigils, no ceremony.
- **Expressive** — first-class functions, closures, pattern matching, ternaries, list comprehensions.
- **Approachable** — you can learn the whole language in an afternoon.
- **Self-contained** — batteries included: HTTP client, JSON/CSV/HTML parsers, stats module, file I/O.
- **Extensible** — module system with import/export, optional Python interop, optional CUDA GPU acceleration.

## Quick Start

```bash
git clone https://github.com/young-developer90/lion.git
cd lion
cargo build --release
./target/release/lion run examples/hello.lion
```

## Examples

### Fibonacci

```lion
func fibonacci(n) {
    if n <= 1 { return n; }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

for i in 0..10 {
    print(f"fib({i}) = {fibonacci(i)}");
}
```

### HTTP Request

```lion
let resp = http.get("https://api.github.com/repos/young-developer90/lion");
print(resp.status);
print(resp.json()["description"]);
```

### File I/O

```lion
fs.write("hello.txt", "Hello, Lion!");
print(fs.read("hello.txt"));
fs.exists("hello.txt");  // true
```

### JSON

```lion
let data = json.parse('{"name": "Lion", "version": 1.0}');
print(data["name"]);

let encoded = json.stringify(data);
print(encoded);
```

## Features

| Category | Details |
|----------|---------|
| **Syntax** | Clean, modern — inspired by Swift, Kotlin, and Lua |
| **Functions** | First-class, closures, lambdas (`\|x\| x * 2`), variadic and named args |
| **Types** | Int, Float, String, Bool, List, Dict, Set, Tuple, ranges |
| **Strings** | Interpolation (`f"Hello, {name}!"`), multi-line (triple quotes) |
| **Control Flow** | `if`/`elif`/`else`, `while`, `for..in`, ternary `? :`, `match` |
| **Error Handling** | `try`/`catch`/`throw` |
| **Modules** | `import`/`export` with aliases |
| **Standard Library** | `math`, `time`, `rand`, `fs`, `os`, `json`, `csv`, `html`, `http`, `url`, `stats` |
| **GPU** | Optional CUDA acceleration for matrix operations |
| **Tooling** | REPL, bytecode disassembler, formatter, test runner |
| **Editor Support** | VS Code extension with LSP (diagnostics, completions, hover) |
| **Cross-platform** | Windows, macOS, Linux |

## Comparison

| Feature | Lion | Python | JavaScript | Lua |
|---------|------|--------|------------|-----|
| Closures | Yes | Yes | Yes | Yes |
| Pattern matching | Yes | 3.10+ | No | No |
| String interpolation | Native | f-strings | Template literals | No |
| Type annotations | Optional | Optional | TypeScript | No |
| Built-in HTTP | Yes | `requests` | `fetch` | No |
| Built-in JSON | Yes | Yes | Yes | No |
| Built-in CSV | Yes | `csv` | No | No |
| Built-in HTML/parser | Yes | `bs4` | `DOMParser` | No |
| GPU acceleration | Optional (CUDA) | NumPy/CuPy | WebGL | No |
| REPL | Yes | Yes | Node.js | Yes |
| LSP support | Yes | pylance | tsserver | No |

## Usage

```bash
lion run <file>              # Run a script
lion repl                    # Interactive REPL
lion run --disassemble <file> # Show bytecode disassembly
lion fmt <file>              # Format source code
lion test [path]             # Run tests
lion version                 # Show version
```

## Building

### Prerequisites

- [Rust](https://rustup.rs/) 1.80+ (edition 2021)

### Release build

```bash
cargo build --release
```

### LSP server

```bash
cargo build --bin lion-lsp
```

### VS Code extension

```bash
cd vscode-lion && npm install && cd ..
code --install-extension vscode-lion/
```

### CUDA support (optional)

Install the [CUDA Toolkit](https://developer.nvidia.com/cuda-downloads) and set the `CUDA_PATH` environment variable. The build script detects it automatically.

```bash
set CUDA_PATH=C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.x
cargo build --release --features cuda
```

## Running Tests

```bash
cargo build --release
./target/release/lion test tests/
```

## Project Structure

```
src/           # Rust source
  ├── lexer.rs      # Tokenizer
  ├── parser.rs     # Recursive descent parser
  ├── ast.rs        # AST definitions
  ├── compiler.rs   # Bytecode compiler
  ├── bytecode.rs   # Instruction set & chunk format
  ├── vm.rs         # Stack-based virtual machine
  ├── gc.rs         # Mark-and-sweep garbage collector
  ├── jit.rs        # JIT compilation framework
  ├── module.rs     # Module loader & stdlib setup
  ├── stdlib.rs     # Built-in standard library
  ├── cli.rs        # Command-line interface
  ├── repl.rs       # Interactive REPL
  ├── lsp.rs        # Language server
  ├── main.rs       # Entry point
  ├── cuda.rs       # CUDA acceleration
  ├── linum.rs      # Linear algebra module
  └── *_mod.rs      # Utility modules (csv, html, json, stats, string, url)
examples/      # Example .lion scripts
tests/         # Test .lion scripts
vscode-lion/   # VS Code extension (syntax highlighting + LSP client)
```

## Documentation

- [Full Tutorial](TUTORIAL.md) — comprehensive guide covering all language features
- [Examples](examples/) — runnable example scripts
- [Tests](tests/) — test suite demonstrating various features

## Contributing

Contributions are welcome! Here's how to get started:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

Please make sure tests pass before submitting:

```bash
cargo build --release
./target/release/lion test tests/
```

## License

Distributed under the MIT License. See [LICENSE](LICENSE) for more information.
