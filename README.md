# Zamin Programming Language

[![Rust](https://img.shields.io/badge/Rust-1.80%2B-dea584?logo=rust)](https://rustup.rs/)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Version](https://img.shields.io/badge/version-1.7.0-green)](https://github.com/young-developer90/zamin/releases)
[![Build](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/young-developer90/zamin/actions)
[![PRs](https://img.shields.io/badge/PRs-welcome-orange)](https://github.com/young-developer90/zamin/pulls)

Zamin is a simple, expressive scripting language with a Rust-based interpreter. It combines modern language features -- closures, pattern matching, string interpolation, and a module system -- with a lightweight bytecode VM, optional GPU acceleration, and a built-in project manager.

```
print("Hello, Zamin!");
```

## Documentation

**[Full Documentation](docs/index.html)** -- language guide, standard library reference, GUI toolkit guide, OpenCV guide, and more.

| Page | Description |
|------|-------------|
| [Getting Started](docs/getting-started.html) | Installation, setup, and first script |
| [Language Guide](docs/language-guide.html) | Complete language reference |
| [Standard Library](docs/standard-library.html) | All 25+ built-in modules |
| [GUI Toolkit](docs/gui-guide.html) | Sol (Windows) and Luna (Linux) |
| [OpenCV](docs/opencv-guide.html) | Computer vision and image processing |
| [Advanced Guide](docs/advanced-guide.html) | C extensions, Python interop, CUDA, embedding |
| [Project Management](docs/project-management.html) | CLI commands and project structure |
| [Performance](docs/performance.html) | Benchmarks and optimization tips |
| [FAQ](docs/faq.html) | Frequently asked questions |

## Quick Start

```bash
git clone https://github.com/young-developer90/zamin.git
cd zamin
cargo build --release
./target/release/zamin run examples/hello.zamin
```

### Start the REPL

```bash
./target/release/zamin repl
```

```
Zamin> let x = 42;
Zamin> print(f"the answer is {x}");
the answer is 42
Zamin> func fib(n) { if n <= 1 { return n; } return fib(n-1) + fib(n-2); }
Zamin> print(fib(20));
6765
```

## Examples

### Fibonacci

```lion
func fibonacci(n) {
    if n <= 1 { return n; }
    return fibonacci(n - 1) + fibonacci(n - 2);
}
for i in 0..10 { print(f"fib({i}) = {fibonacci(i)}"); }
```

### HTTP Request

```lion
let resp = http.get("https://api.github.com/repos/young-developer90/zamin");
print(resp.status);
```

### File I/O

```lion
fs.write("hello.txt", "Hello, Zamin!");
let content = fs.read("hello.txt");
print(content);
```

### GUI (Linux with Luna)

```bash
sudo apt install libgtk-4-dev
cargo build --release --features luna
```

```lion
let win = luna.Leo("App", 400, 300);
let label = luna.Label(win, "Hello from Luna!");
luna.pack(label);
let btn = luna.Button(win, "Click", func() {
    luna.config(label, "text", "Clicked!");
});
luna.pack(btn);
luna.mainloop(win);
```

## CLI

| Command | Description |
|---------|-------------|
| `zamin run <file>` | Run a script |
| `zamin repl` | Interactive REPL |
| `zamin fmt <file>` | Format source code |
| `zamin test [filter]` | Run tests |
| `zamin new <name>` | Create a new project |
| `zamin init` | Initialize project in current dir |
| `zamin build` | Type-check all `.zamin` files |
| `zamin version` | Show version |
| `zamin-rs <file>` | Quick-run without subcommands |

## Performance

Zamin's bytecode VM achieves significant speedups through raw byte dispatch, specialized integer opcodes, and a peephole optimizer:

| Benchmark | Speedup |
|-----------|---------|
| Integer loop (10M iterations) | **11.1x** |
| Local variable access (5M) | **10.0x** |
| Function calls (500K) | **7.2x** |
| List push (100K) | **5.3x** |
| String concat (100K) | **2.9x** |

See [Performance](docs/performance.html) for detailed benchmarks.

## Build Options

```bash
# Default (no optional features)
cargo build --release

# With OpenCV
cargo build --release --features opencv

# With Luna (Linux GUI)
cargo build --release --features luna

# With Python interop
cargo build --release --features python
```

## License

MIT -- see [LICENSE](LICENSE).
