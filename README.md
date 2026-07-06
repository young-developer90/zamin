# Lion Programming Language

[![Rust](https://img.shields.io/badge/Rust-1.80%2B-dea584?logo=rust)](https://rustup.rs/)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Version](https://img.shields.io/badge/version-1.5.6-green)](https://github.com/young-developer90/lion/releases)
[![Build](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/young-developer90/lion/actions)
[![PRs](https://img.shields.io/badge/PRs-welcome-orange)](https://github.com/young-developer90/lion/pulls)

Lion is a simple, expressive scripting language with a Rust-based interpreter (v1.5.6). It combines modern language features — closures, pattern matching, string interpolation, and a module system — with a lightweight bytecode VM and optional CUDA GPU acceleration.

```
print("Hello, Lion!");
```

## Philosophy

Lion is designed to be:

- **Readable** — syntax inspired by Swift, Kotlin, and Lua. No sigils, no ceremony.
- **Expressive** — first-class functions, closures, pattern matching, ternaries, list comprehensions.
- **Approachable** — you can learn the whole language in an afternoon.
- **Self-contained** — batteries included: HTTP client, JSON/CSV/HTML parsers, stats module, file I/O, regex, datetime, logging, subprocess, pathlib, hashlib/crypto, collections, itertools, unit test assertions, and a **native GUI toolkit** (leopard, Win32).
- **Extensible** — module system with import/export, optional Python interop, optional CUDA GPU acceleration, and C extension API for native modules.

## Installation

### Prerequisites

- **Rust** 1.80+ (edition 2021) — install via [rustup](https://rustup.rs/)
- **Git** — for cloning the repository
- **Python** 3.10+ (optional) — for running Python benchmarks and Python interop
- **CUDA Toolkit** 12.x (optional) — for GPU-accelerated matrix operations

### Quick Start

```bash
git clone https://github.com/young-developer90/lion.git
cd lion
cargo build --release
./target/release/lion run examples/hello.lion
```

### Build Options

| Command | Description |
|---------|-------------|
| `cargo build` | Debug build (fast compile, slow execution) |
| `cargo build --release` | Release build (slow compile, fast execution) |
| `cargo build --bin lion` | Build only the interpreter (excludes LSP) |
| `cargo build --bin lion-lsp` | Build only the LSP server |
| `cargo build --features python` | Enable Python interop via PyO3 |
| `cargo build --features cuda` | Enable CUDA GPU acceleration |

### Run Your First Script

```bash
echo 'print("Hello, Lion!")' > hello.lion
./target/release/lion run hello.lion
```

### Start the REPL

```bash
./target/release/lion repl
```

Try it out:
```
Lion> let x = 42;
Lion> print(f"the answer is {x}");
the answer is 42
Lion> func fib(n) { if n <= 1 { return n; } return fib(n-1) + fib(n-2); }
Lion> print(fib(20));
6765
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

### GUI (Windows)

```lion
let root = leopard.Leo("App", 400, 300);
let label = leopard.Label(root, "Hello from Leopard!");
leopard.pack(label, "top", 0, 10);
let btn = leopard.Button(root, "Click", func() {
    leopard.config(label, "text", "Clicked!");
});
leopard.pack(btn, "top", 0, 5);
leopard.mainloop(root);
```

### JSON

```lion
let data = json.parse('{"name": "Lion", "version": 1.0}');
print(data["name"]);

let encoded = json.stringify(data);
print(encoded);
```

## Language Tour

### Variables & Constants

```lion
let name = "Lion";           // mutable
const pi = 3.14159;          // immutable
let count = 42;              // Int
let price = 19.99;           // Float
let active = true;           // Bool
let data = nil;              // Nil
```

### Strings

```lion
let s = "hello";
let multi = """line one
line two
line three""";
let interpolated = f"value: {s}, sum: {2 + 2}";
let len = string.len(s);
let upper = string.upper(s);
let parts = string.split("a,b,c", ",");
```

### Collections

```lion
let list = [1, 2, 3];
list.push(4);
let first = list[0];

let dict = {"name": "Lion", "version": 1.0};
dict["author"] = "you";
let v = dict["name"];

let set = {1, 2, 3};
set.add(4);

let tuple = (1, "hello", true);
```

### Control Flow

```lion
// if/elif/else
if x > 0 {
    print("positive");
} elif x < 0 {
    print("negative");
} else {
    print("zero");
}

// while
while count > 0 {
    count -= 1;
}

// for..in ranges
for i in 0..10 {
    print(i);
}

// for..in with step
for i in 0..100..5 {
    print(i);
}

// for..in collections
for item in list {
    print(item);
}

// ternary
let max = a > b ? a : b;
```

### Functions

```lion
// named function
func greet(name) {
    return f"Hello, {name}!";
}

// lambda
let double = |x| x * 2;

// closure
func make_counter() {
    let i = 0;
    return func() { i += 1; return i; };
}

// variadic args
func sum(...nums) {
    let total = 0;
    for n in nums { total += n; }
    return total;
}

// named/default args
func connect(host, port = 8080) {
    print(f"{host}:{port}");
}
```

### Pattern Matching

```lion
let value = 42;
match value {
    0 => print("zero"),
    1..10 => print("small"),
    42 => print("answer!"),
    _ => print("something else"),
}

match status {
    "ok" => print("success"),
    "error" => print("failed"),
    _ => print("unknown"),
}
```

### Error Handling

```lion
try {
    let result = risky_operation();
    print(result);
} catch e {
    print(f"caught: {e}");
}

throw "something went wrong";
```

### Structs

```lion
struct Counter {
    func new() {
        return Counter{ count = 0 };
    }

    func increment(self) {
        self.count += 1;
    }

    func value(self) {
        return self.count;
    }
}

let c = Counter.new();
c.increment();
print(c.value());  // 1
```

### Modules

```lion
// import.lion
export func hello() { print("hi"); }

// main.lion
import "import.lion" as mymod;
mymod.hello();
```

### Comprehensions

```lion
let squares = [x * x for x in 0..10];
let evens = [x for x in 0..20 if x % 2 == 0];
```

### C Extensions

Lion can load native C extensions — shared libraries (`.dll`/`.so`/`.dylib`) placed in the `modules/` directory. Each extension must define `lion_module_init` and return an array of functions.

**C header** (`include/lion.h`):
```c
#include "lion.h"

static LionValue add(int argc, const LionValue* args) {
    long long a = (argc > 0 && args[0].tag == LION_INT) ? args[0].data.as_int : 0;
    long long b = (argc > 1 && args[1].tag == LION_INT) ? args[1].data.as_int : 0;
    LionValue r; r.tag = LION_INT; r.data.as_int = a + b;
    return r;
}

static LionModuleFunc funcs[] = {{"add", add}};

int lion_module_init(int* count, LionModuleFunc** out) {
    *count = 1; *out = funcs; return 0;
}
```

**Compile:**
```bash
gcc -O2 -shared -o modules/example.dll modules/example.c -Iinclude
```

**Use in Lion:**
```lion
import example;
print(example.add(3, 4));  // 7
```

Supported types: `LION_NIL`, `LION_INT`, `LION_FLOAT`, `LION_BOOL`, `LION_STRING`.

## Standard Library Reference

| Module | Functions | Description |
|--------|-----------|-------------|
| `math` | `abs`, `sqrt`, `sin`, `cos`, `tan`, `floor`, `ceil`, `round`, `min`, `max`, `pow`, `log`, `pi`, `e` | Math utilities |
| `time` | `sleep`, `now` | Time utilities |
| `rand` | `int`, `float`, `shuffle`, `choice` | Random number generation |
| `fs` | `read`, `write`, `append`, `exists`, `remove`, `mkdir`, `read_dir`, `stat`, `copy`, `rename` | File system operations |
| `os` | `args`, `env`, `set_env`, `cwd`, `chdir`, `exit`, `platform`, `type` | Operating system interface |
| `json` | `parse`, `stringify` | JSON encoding/decoding |
| `csv` | `parse`, `stringify` | CSV parsing/formatting |
| `html` | `parse`, `query`, `inner_text`, `inner_html`, `attr`, `children` | HTML parsing with CSS selectors |
| `http` | `get`, `post`, `put`, `delete`, `patch`, `head`, `options` | HTTP client |
| `url` | `encode`, `decode`, `parse`, `build` | URL utilities |
| `stats` | `mean`, `median`, `mode`, `std`, `variance`, `min`, `max`, `sum`, `correlation`, `regression` | Statistics |
| `re` | `find_all`, `sub`, `split`, `match`, `search` | Regular expressions |
| `datetime` | `now`, `from_unix`, `format`, `parse`, `unix` | Date/time handling |
| `logging` | `info`, `warn`, `error`, `debug`, `set_level` | Logging |
| `subprocess` | `run`, `run_shell`, `capture` | Subprocess execution |
| `path` | `join`, `basename`, `dirname`, `ext`, `exists`, `is_file`, `is_dir`, `abs` | Path manipulation |
| `hashlib` | `sha256`, `sha512`, `sha1`, `md5`, `base64_encode`, `base64_decode`, `hex_encode`, `hex_decode` | Hashing & encoding |
| `collections` | `Counter`, `deque` | Specialized collections |
| `itertools` | `sorted`, `unique`, `group_by`, `flatten`, `chunks`, `zip`, `enumerate`, `cycle`, `repeat`, `take`, `skip` | Iterator utilities |
| `test` | `assert_eq`, `assert_ne`, `assert_true`, `assert_false`, `assert_lt`, `assert_gt`, `assert_approx` | Unit testing |
| `leopard` | `Leo`, `Button`, `Label`, `Entry`, `Frame`, `pack`, `place`, `config`, `get`, `insert`, `delete`, `title`, `geometry`, `mainloop`, `destroy`, `messagebox` | Native GUI toolkit (Win32, tkinter-like) |

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
| **Standard Library** | 20+ built-in modules, including native GUI (leopard) |
| **Python Interop** | Optional — import and call any Python module (NumPy, PyTorch, pandas, etc.) |
| **GPU** | Optional CUDA acceleration for matrix operations |
| **Tooling** | REPL, bytecode disassembler, formatter, test runner |
| **Editor Support** | VS Code extension with LSP (diagnostics, completions, hover) |
| **Cross-platform** | Windows, macOS, Linux |

## Comparison

| Feature | Lion | Python | JavaScript | Lua | Mojo |
|---------|------|--------|------------|-----|------|
| Closures | Yes | Yes | Yes | Yes | Yes |
| Pattern matching | Yes | 3.10+ | No | No | **No** |
| String interpolation | Native | f-strings | Template literals | No | Yes |
| Type annotations | Optional | Optional | TypeScript | No | Yes |
| Built-in HTTP | Yes | `requests` | `fetch` | No | **No** |
| Built-in JSON | Yes | Yes | Yes | No | **No** |
| Built-in CSV | Yes | `csv` | No | No | **No** |
| Built-in HTML/parser | Yes | `bs4` | `DOMParser` | No | **No** |
| File I/O (native) | **Yes** | Yes | Yes | Yes | **No** ❌ |
| Cross-platform | **Yes** | Yes | Yes | Yes | **No** (Linux only) |
| Android / Termux | **Yes** | Yes | No | Yes | **No** |
| GPU acceleration | Optional (CUDA) | NumPy/CuPy | WebGL | No | Yes |
| REPL | Yes | Yes | Node.js | Yes | Yes |
| LSP support | Yes | pylance | tsserver | No | Yes |

## CLI Usage

```bash
lion run <file>                # Run a script
lion repl                      # Interactive REPL
lion run --disassemble <file>  # Show bytecode disassembly
lion fmt <file>                # Format source code
lion test [path]               # Run tests (default: ./tests/)
lion version                   # Show version
```

## Advanced Builds

### LSP server

```bash
cargo build --bin lion-lsp
```

### VS Code extension

```bash
cd vscode-lion && npm install && cd ..
code --install-extension vscode-lion/
```

### Python interop (optional)

Lion can import and call Python modules directly via [PyO3](https://pyo3.rs/). Enable with the `python` feature.

```bash
cargo build --release --features python
```

```lion
import py

// Get Python version
let sys = py.import("sys")
print("Python version:", sys.version)

// Use Python's math module
let math = py.import("math")
print("sqrt(144) =", math.sqrt(144))
print("pi =", math.pi)

// OS info with chained attribute access
let os = py.import("os")
print("cwd:", os.getcwd())
print("abspath('.') =", os.path.abspath("."))

// Random numbers
let random = py.import("random")
print("random int:", random.randint(1, 100))

// NumPy (if installed)
let np = py.import("numpy")
let arr = np.array([1, 2, 3])
print(arr)
```

See [`examples/python_interop.lion`](examples/python_interop.lion) for a runnable example.

**Type conversion:** Int, Float, String, Bool, List, Dict, Nil map to their Python equivalents automatically.

**Attribute access:** Chained attribute access works (`os.path.abspath(".")`). Python objects are lazily wrapped — attributes are resolved dynamically when accessed, not eagerly, so large modules (NumPy, PyTorch) load instantly.

**Calling Python objects:** Any Python callable (functions, classes, methods) can be called with Lion syntax. Arguments are converted automatically, and the return value is converted back to a Lion value.

### CUDA support (optional)

Install the [CUDA Toolkit](https://developer.nvidia.com/cuda-downloads) and set the `CUDA_PATH` environment variable. The build script detects it automatically.

```bash
set CUDA_PATH=C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.x
cargo build --release
```

## Performance Benchmarks

Benchmarks comparing Lion 1.5.6 (release build) against Python 3.14 on the same workloads. Lower is better.

| Benchmark | Lion 1.5.6 | Python 3.14 | vs Python |
|-----------|-------------|-------------|-----------|
| `re.find_all` — 10k lines | 2.2 ms | 1.8 ms | ~1.2× slower |
| `re.sub_all` — 10k lines | 3.3 ms | 10.0 ms | ~3.0× faster |
| `re.split` — 10k lines | 1.2 ms | 0.6 ms | ~2× slower |
| `collections.Counter` — 50k words | 1.9 ms | 1.1 ms | ~1.7× slower |
| `itertools.unique` — 20k items | 0.6 ms | 0.2 ms | ~3× slower |
| `itertools.sorted` — 10k items | 0.13 ms | 0.04 ms | ~3× slower |
| `datetime.now` — 10k calls | 16.8 ms | 1.7 ms | ~9.9× slower |
| `datetime.format` — 10k calls | 28.3 ms | 16.5 ms | ~1.7× slower |
| `hashlib.sha256` — 1k strings | 2.7 ms | 0.6 ms | ~4.5× slower |
| `hashlib.base64` — 1k strings | 2.4 ms | 0.4 ms | ~6× slower |
| `subprocess.run_shell` — 100 calls | 1.50 s | 1.47 s | ~1× (on par) |

Lion is an interpreted bytecode VM while Python benefits from decades of optimization and C-backed native implementations. Optimizations in 1.5.6 include: `&str`-based lexer (replacing `Vec<char>` allocation in the lexer), `HashMap`-based string interning in the compiler (O(1) lookup vs O(n)), allocation-free `make_iterator`/`next_iterator` (stores original collection ref + index instead of cloning items into a `Vec`), allocation-free GC gray marking (inline marking instead of per-object `Vec` allocation), direct `&str` access in string module (eliminating per-call string clones via `get_str`/`get_str_owned` helpers), optimized `String.join` (single-pass with pre-allocated capacity using `string_len`), single-pass HTML encode/decode (eliminating multi-pass `.replace()` chains), conditional path separator normalization (avoids unnecessary allocations on paths without backslashes), direct byte read in HTTP client (eliminating String→bytes round-trip via `into_reader`), shared HTTP `Agent` via `OnceLock` (connection pool reuse across requests), `Rc<Regex>` caching (eliminating `Regex::clone` on cache hit), direct buffer logging (eliminating intermediate `Vec<String>` allocation), inlining in stats module (avoiding `Vec<f64>` allocation for sum/mean/min/max via `fold_f64s`/`sum_len` helpers), borrow-based list iteration in itertools (avoiding `Vec<Value>::clone` per operation for chain/reverse/unique/take/drop/slice), `Value::string_len` method for capacity estimation, and all prior 1.4.x–1.5.01 optimizations.

Benchmarks are in [`benchmarks/`](benchmarks/) and can be run with:
```bash
cargo build --release --bin lion
./target/release/lion run benchmarks/bench_lion.lion
python benchmarks/bench_python.py
```

## Running Tests

```bash
cargo build --release --bin lion
./target/release/lion test tests/
```

Run a single test file:

```bash
./target/release/lion test tests/test_datetime.lion
```

Run all tests including the LSP test:

```bash
./target/release/lion test tests/ --include-lsp
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
  ├── gc.rs         # Mark-and-sweep garbage collector (also Value impl with string_len, to_string, eq, hash, etc.)
  ├── jit.rs        # JIT compilation framework
  ├── module.rs     # Module loader & stdlib setup
  ├── stdlib.rs     # Built-in standard library
  ├── cli.rs        # Command-line interface
  ├── repl.rs       # Interactive REPL
  ├── lsp.rs        # Language server
  ├── main.rs       # Entry point
  ├── cuda.rs       # CUDA acceleration
  ├── linum.rs      # Linear algebra module
  ├── http.rs       # HTTP client (shared agent, direct byte reads)
  ├── cext.rs       # C extension loader (FFI)
  ├── py.rs         # Python interop (PyO3)
  └── *_mod.rs      # Utility modules (csv, html, json, stats, string, url, re, datetime, logging, subprocess, path, hashlib, collections, itertools, test, leopard, cheetah, jaguar)
examples/      # Example .lion scripts
tests/         # Test .lion scripts
vscode-lion/   # VS Code extension (syntax highlighting + LSP client)
```

## Documentation

- **[Language Tour](#language-tour)** — quick overview of all language features (above)
- [Full Tutorial](TUTORIAL.md) — comprehensive guide covering all language features in depth
- [Examples](examples/) — runnable example scripts (hello world, HTTP, JSON, Python interop, CUDA, etc.)
- [Tests](tests/) — test suite demonstrating various features (also great as reference)
- [Benchmarks](benchmarks/) — performance benchmarks comparing Lion vs Python
- [VS Code Extension](vscode-lion/) — syntax highlighting and LSP client for VS Code

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `cargo build` fails with "CUDA toolkit not found" | This is a warning, not an error. The build succeeds without CUDA. |
| `lion-lsp.exe` locked during build | Run `cargo build --bin lion` to build only the interpreter, or kill any running LSP process. |
| Python interop not working | Ensure you build with `--features python` and have Python installed with `pyo3`-compatible headers. |
| Slow performance | Always use `cargo build --release` for benchmarks and production use. Debug builds are ~50× slower. |
| Tests fail with "cannot run test" | Build the release binary first: `cargo build --release --bin lion` |
| REPL not working on Windows | Use the default terminal (cmd, PowerShell, or Windows Terminal). Some third-party terminals may have issues. |

## Contributing

Contributions are welcome! Here's how to get started:

1. **Fork** the repository on GitHub
2. **Clone** your fork: `git clone https://github.com/your-username/lion.git`
3. **Create a feature branch**: `git checkout -b feature/amazing-feature`
4. **Make your changes** — follow the existing code style (no trailing whitespace, 4-space indentation)
5. **Run tests** to verify nothing is broken (see below)
6. **Commit** your changes with a descriptive message: `git commit -m 'Add amazing feature'`
7. **Push** to your branch: `git push origin feature/amazing-feature`
8. **Open a Pull Request** on the original repository

### Before submitting

```bash
# Build and run all tests
cargo build --release --bin lion
./target/release/lion test tests/

# Run benchmarks to check for regressions
./target/release/lion run benchmarks/bench_lion.lion
```

### Code style

- 4-space indentation, no tabs
- Opening braces on the same line (`func foo() {`)
- Semicolons after statements
- Comments with `//` (not `#`)
- Match the naming conventions in the surrounding code
- No trailing whitespace at end of lines
- Newline at end of file

## License

Distributed under the MIT License. See [LICENSE](LICENSE) for more information.
