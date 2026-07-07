# Lion Programming Language

[![Rust](https://img.shields.io/badge/Rust-1.80%2B-dea584?logo=rust)](https://rustup.rs/)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Version](https://img.shields.io/badge/version-1.5.6-green)](https://github.com/young-developer90/lion/releases)
[![Build](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/young-developer90/lion/actions)
[![PRs](https://img.shields.io/badge/PRs-welcome-orange)](https://github.com/young-developer90/lion/pulls)

Lion is a simple, expressive scripting language with a Rust-based interpreter (v1.5.6). It combines modern language features — closures, pattern matching, string interpolation, and a module system — with a lightweight bytecode VM and optional GPU acceleration.

```
print("Hello, Lion!");
```

## Philosophy

- **Readable** — syntax inspired by Swift, Kotlin, and Lua. No sigils, no ceremony.
- **Expressive** — first-class functions, closures, pattern matching, ternaries, list comprehensions.
- **Approachable** — you can learn the whole language in an afternoon.
- **Self-contained** — batteries included: HTTP client, JSON/CSV/HTML parsers, stats, regex, datetime, logging, subprocess, hashlib, collections, itertools, unit testing, and native GUI toolkits for **Windows** (leopard) and **Linux** (panther).
- **Extensible** — module system with import/export, optional Python interop, optional CUDA GPU acceleration, and C extension API.

## Quick Start

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
| `cargo build --features panther` | Enable Linux GUI toolkit (GTK4) |
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
for i in 0..10 { print(f"fib({i}) = {fibonacci(i)}"); }
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

### GUI (Windows with leopard)

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

### GUI (Linux with panther)

Build with `--features panther` and GTK4 development libraries:

```bash
# Ubuntu/Debian
sudo apt install libgtk-4-dev
cargo build --release --features panther
```

```lion
let root = panther.Leo("App", 400, 300);
let label = panther.Label(root, "Hello from Panther!");
panther.pack(label);
let btn = panther.Button(root, "Click", func() {
    panther.config(label, "text", "Clicked!");
});
panther.pack(btn);
panther.mainloop(root);
```

### JSON

```lion
let data = json.parse('{"name": "Lion", "version": 1.0}');
print(data["name"]);
let encoded = json.stringify(data);
print(encoded);
```

### Python Interop

Build with `--features python`:

```bash
cargo build --release --features python
```

```lion
import py
let np = py.import("numpy")
let arr = np.array([1, 2, 3])
print(arr)  // [1 2 3]
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
```

### Collections

```lion
let list = [1, 2, 3];  list.push(4);
let dict = {"name": "Lion", "version": 1.0};
let set = {1, 2, 3};  set.add(4);
let tuple = (1, "hello", true);
```

### Control Flow

```lion
if x > 0 { print("positive"); } elif x < 0 { print("negative"); } else { print("zero"); }
while count > 0 { count -= 1; }
for i in 0..10 { print(i); }
for i in 0..100..5 { print(i); }
let max = a > b ? a : b;
```

### Functions

```lion
func greet(name) { return f"Hello, {name}!"; }
let double = |x| x * 2;
func sum(...nums) { let t = 0; for n in nums { t += n; } return t; }
func connect(host, port = 8080) { print(f"{host}:{port}"); }
```

### Pattern Matching

```lion
match value {
    0 => print("zero"),
    1..10 => print("small"),
    42 => print("answer!"),
    _ => print("something else"),
}
```

### Error Handling

```lion
try { let result = risky_operation(); } catch e { print(f"caught: {e}"); }
throw "something went wrong";
```

### Structs

```lion
struct Counter {
    func new() { return Counter{ count = 0 }; }
    func increment(self) { self.count += 1; }
    func value(self) { return self.count; }
}
let c = Counter.new();  c.increment();  print(c.value());  // 1
```

### Modules

```lion
// import.lion
export func hello() { print("hi"); }
// main.lion
import "import.lion" as mymod;  mymod.hello();
```

### Comprehensions

```lion
let squares = [x * x for x in 0..10];
let evens   = [x for x in 0..20 if x % 2 == 0];
```

## Standard Library Reference

| Module | Functions | Description |
|--------|-----------|-------------|
| `math` | `abs`, `sqrt`, `sin`, `cos`, `tan`, `floor`, `ceil`, `round`, `min`, `max`, `pow`, `log`, `pi`, `e` | Math utilities |
| `time` | `sleep`, `now` | Time utilities |
| `rand` | `int`, `float`, `shuffle`, `choice` | Random number generation |
| `fs` | `read`, `write`, `append`, `exists`, `remove`, `mkdir`, `read_dir`, `stat`, `copy`, `rename` | File system |
| `os` | `args`, `env`, `set_env`, `cwd`, `chdir`, `exit`, `platform`, `type` | OS interface |
| `json` | `parse`, `stringify` | JSON encoding/decoding |
| `csv` | `parse`, `stringify` | CSV parsing |
| `html` | `parse`, `query`, `inner_text`, `inner_html`, `attr`, `children` | HTML parsing |
| `http` | `get`, `post`, `put`, `delete`, `patch`, `head`, `options` | HTTP client |
| `url` | `encode`, `decode`, `parse`, `build` | URL utilities |
| `stats` | `mean`, `median`, `mode`, `std`, `variance`, `min`, `max`, `sum`, `correlation`, `regression` | Statistics |
| `re` | `find_all`, `sub`, `split`, `match`, `search` | Regular expressions |
| `datetime` | `now`, `from_unix`, `format`, `parse`, `unix` | Date/time |
| `logging` | `info`, `warn`, `error`, `debug`, `set_level` | Logging |
| `subprocess` | `run`, `run_shell`, `capture` | Subprocess |
| `path` | `join`, `basename`, `dirname`, `ext`, `exists`, `is_file`, `is_dir`, `abs` | Path manipulation |
| `hashlib` | `sha256`, `sha512`, `sha1`, `md5`, `base64_encode`, `base64_decode`, `hex_encode`, `hex_decode` | Hashing & encoding |
| `collections` | `Counter`, `deque` | Specialized collections |
| `itertools` | `sorted`, `unique`, `group_by`, `flatten`, `chunks`, `zip`, `enumerate`, `cycle`, `repeat`, `take`, `skip` | Iterator utilities |
| `test` | `assert_eq`, `assert_ne`, `assert_true`, `assert_false`, `assert_lt`, `assert_gt`, `assert_approx` | Unit testing |
| **Windows** | | |
| `leopard` | `Leo`, `Button`, `Label`, `Entry`, `Frame`, `pack`, `place`, `config`, `get`, `insert`, `delete`, `title`, `geometry`, `mainloop`, `destroy`, `messagebox` | Native GUI (Win32) |
| **Linux** | | |
| `panther` | `Leo`, `Button`, `Label`, `Entry`, `Frame`, `pack`, `place`, `config`, `get`, `insert`, `delete`, `title`, `geometry`, `mainloop`, `destroy`, `messagebox` | Native GUI (GTK4, feature=panther) |

## Performance Benchmarks

Benchmarks comparing Lion 1.5.6 (release build) against Python 3.12 on the same workloads. Lower is better.

| Benchmark | Lion (ms) | Python (ms) | vs Python |
|-----------|-----------|-------------|-----------|
| `re.find_all` — 10k lines | 2.06 | 1.51 | ~1.4× slower |
| `re.sub_all` — 10k lines | 4.10 | 8.88 | **~2.2× faster** |
| `re.split` — 10k lines | 1.06 | 0.52 | ~2.0× slower |
| `collections.Counter` — 50k words | 1.94 | 1.13 | ~1.7× slower |
| `itertools.unique` — 20k items | 0.47 | 0.17 | ~2.8× slower |
| `itertools.sorted` — 10k items | 0.07 | 0.07 | ~1.0× (on par) |
| `datetime.now` — 10k calls | 10.60 | 2.04 | ~5.2× slower |
| `datetime.format` — 10k calls | 15.24 | 13.98 | ~1.1× slower |
| `hashlib.sha256` — 1k strings | 1.46 | 0.33 | ~4.4× slower |
| `hashlib.base64` — 1k strings | 1.22 | 0.27 | ~4.5× slower |
| `subprocess.run_shell` — 100 calls | 56.74 | 77.01 | **~1.4× faster** |

Lion is a young interpreted bytecode VM while Python benefits from decades of optimization and C-backed native libraries. Optimizations include: `&str`-based lexer (replacing `Vec<char>` allocation), `HashMap`-based string interning in the compiler (O(1) lookup), allocation-free `make_iterator`/`next_iterator` (stores original collection ref + index), `Vec::swap_remove` in dict operations, `get_str`/`get_str_owned` helpers for borrow-based string access (eliminating per-call clones), optimized `String.join` (single-pass with pre-allocated capacity), single-pass HTML encode/decode, `Rc<Regex>` caching (eliminating `Regex::clone` on cache hit), shared HTTP `Agent` via `OnceLock`, and `Value::string_len` for capacity estimation.

Run benchmarks yourself:

```bash
cargo build --release --bin lion
./target/release/lion run benchmarks/bench_lion.lion
python3 benchmarks/bench_python.py
```

### Key Optimizations in 1.5.6

| Optimization | Impact |
|---|---|
| `get_str` / `get_str_owned` helpers | Borrow strings from GC instead of cloning — speeds up datetime.format (~6%), sha256 (~12%), base64 (~5%) |
| `&str`-based lexer | Eliminates `Vec<char>` allocation in tokenizer |
| `HashMap`-based string interning | O(1) identifier lookup vs O(n) linear scan |
| `#[inline(always)]` on VM hot paths | Reduces dispatch overhead in bytecode loop |
| Shared HTTP `Agent` (OnceLock) | Connection pool reuse across requests |
| Borrow-based list iteration | Avoids `Vec<Value>::clone` for chain/reverse/unique/take/drop/slice |

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
| Built-in HTML parser | Yes | `bs4` | `DOMParser` | No |
| Cross-platform GUI | Win32 + Linux (GTK4) | tkinter/Qt | Web | No |
| GPU acceleration | Optional CUDA | NumPy/CuPy | WebGL | No |
| Python interop | Optional PyO3 | — | — | — |
| Android / Termux | Yes | Yes | No | Yes |
| REPL | Yes | Yes | Node.js | Yes |
| LSP support | Yes | pylance | tsserver | No |

## CLI

| Command | Description |
|---------|-------------|
| `lion run <file>` | Run a script |
| `lion repl` | Interactive REPL |
| `lion run --disassemble <file>` | Show bytecode |
| `lion fmt <file>` | Format source code |
| `lion test [path]` | Run tests (default `./tests/`) |
| `lion version` | Show version |

## Advanced Builds

### Linux GUI (Panther)

Requires GTK4 development headers:

```bash
# Ubuntu/Debian
sudo apt install libgtk-4-dev
# Fedora
sudo dnf install gtk4-devel
# Arch
sudo pacman -S gtk4
```

Build:

```bash
cargo build --release --features panther
```

```lion
let root = panther.Leo("Hello", 400, 300);
let label = panther.Label(root, "Welcome to Panther!");
panther.pack(label);
panther.mainloop(root);
```

### Python Interop

```bash
cargo build --release --features python
```

```lion
import py
let math = py.import("math")
print(math.sqrt(144))  // 12.0
```

### CUDA Support

```bash
set CUDA_PATH=/usr/local/cuda-12
cargo build --release
```

### LSP Server

```bash
cargo build --bin lion-lsp
```

### VS Code Extension

```bash
cd vscode-lion && npm install && cd ..
code --install-extension vscode-lion/
```

## Running Tests

```bash
cargo build --release --bin lion
./target/release/lion test tests/
```

## Project Structure

```
src/           # Rust source (lexer, parser, compiler, VM, GC, stdlib, modules)
examples/      # Example .lion scripts
tests/         # Test .lion scripts
benchmarks/    # Performance benchmarks (Lion + Python)
vscode-lion/   # VS Code extension (syntax highlighting + LSP client)
include/       # C header for native extensions
modules/       # C extension shared libraries (.dll/.so/.dylib)
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `cargo build` warns about CUDA | This is a warning, not an error. Build succeeds without CUDA. |
| Python interop not working | Build with `--features python`, ensure Python dev headers installed. |
| Slow performance | Always use `cargo build --release`. Debug builds are ~50× slower. |
| Tests fail | Build release binary first: `cargo build --release --bin lion` |

## License

MIT — see [LICENSE](LICENSE).
