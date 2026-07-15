# Lion Language Tutorial

A step-by-step guide to learning Lion, from zero to building real applications.

---

## Table of Contents

1. [Installation](#1-installation)
2. [Hello, Lion!](#2-hello-lion)
3. [Variables & Types](#3-variables--types)
4. [Strings](#4-strings)
5. [Collections](#5-collections)
6. [Control Flow](#6-control-flow)
7. [Functions & Closures](#7-functions--closures)
8. [Pattern Matching](#8-pattern-matching)
9. [Error Handling](#9-error-handling)
10. [Modules & Imports](#10-modules--imports)
11. [Standard Library](#11-standard-library)
12. [Building a GUI App](#12-building-a-gui-app)
13. [What's Next](#13-whats-next)

---

## 1. Installation

### Prerequisites

- **Rust** 1.80 or later ([rustup.rs](https://rustup.rs/))
- **Git**

### Build Lion

```bash
git clone https://github.com/young-developer90/lion.git
cd lion
cargo build --release
```

For the Linux GUI toolkit (panther), also install GTK4:

```bash
# Ubuntu / Debian
sudo apt install libgtk-4-dev

# Fedora
sudo dnf install gtk4-devel

# Arch
sudo pacman -S gtk4

# Build with GUI support
cargo build --release --features panther
```

### Verify

```bash
./target/release/lion version
```

You should see `lion 1.6.2`.

---

## 2. Hello, Lion!

Create a file called `hello.lion`:

```lion
print("Hello, Lion!");
```

Run it:

```bash
./target/release/lion run hello.lion
```

You can also use the REPL for quick experiments:

```bash
./target/release/lion repl
```

```
Lion> print("hello from REPL");
hello from REPL
```

`print()` outputs text to the console. Multiple values are space-separated:

```lion
print("the answer is", 42);
// the answer is 42
```

---

## 3. Variables & Types

### Variables

Use `let` to declare a mutable variable:

```lion
let name = "Lion";
name = "updated";         // reassign ok
```

Use `const` for immutable bindings:

```lion
const pi = 3.14159;
pi = 3;                   // error: cannot assign to const
```

### Built-in Types

```lion
let integer = 42;         // Int (64-bit signed)
let unsigned = 42u;       // UInt
let float = 3.14;         // Float (64-bit)
let boolean = true;       // Bool
let text = "hello";       // String
let nothing = nil;        // Nil (null)
```

Check a value's type:

```lion
print(type(42));          // int
print(type("hi"));        // string
print(type(true));        // bool
print(type([1, 2]));      // list
```

### Type Conversion

```lion
let s = string(42);        // "42"
let n = int("42");         // 42
let f = float("3.14");     // 3.14
```

---

## 4. Strings

### Basics

```lion
let s = "hello";
let multi = """line one
line two
line three""";
```

### Concatenation

```lion
let combined = "hello" + " world";   // "hello world"
let repeated = "ha" * 3;             // "hahaha"
```

### Interpolation (f-strings)

Use `f"..."` with `{}` for embedded expressions:

```lion
let name = "Lion";
let version = 1.6;
let msg = f"{name} v{version} is {2 + 2 == 4}";
print(msg);   // Lion v1.6 is true
```

Any expression works inside `{}`:

```lion
let list = [1, 2, 3];
print(f"sum: {list[0] + list[1] + list[2]}");   // sum: 6
```

### String Utilities

```lion
string.len("hello");                            // 5
string.upper("hello");                          // "HELLO"
string.lower("HELLO");                          // "hello"
string.trim("  hi  ");                          // "hi"
string.contains("hello world", "world");        // true
string.replace("a-b-c", "-", ":");              // "a:b:c"
string.split("a,b,c", ",");                     // ["a", "b", "c"]
string.join(["a", "b", "c"], ",");              // "a,b,c"
string.reverse("abc");                          // "cba"
string.repeat("x", 3);                          // "xxx"
string.substring("hello", 1, 4);                // "ell"
```

---

## 5. Collections

### Lists

```lion
let fruits = ["apple", "banana", "cherry"];
fruits.push("date");                   // append
let first = fruits[0];                 // "apple"
fruits[1] = "blueberry";               // update
let len = fruits.len();                // 4
let sliced = fruits[1..3];             // ["blueberry", "cherry"]
```

### Dicts

```lion
let config = {"host": "localhost", "port": 8080};
print(config["host"]);                 // "localhost"
config["port"] = 9090;                 // update
config["ssl"] = true;                  // add key
config.remove("ssl");                  // remove key (via dict.contains + dict.keys)
```

### Sets

```lion
let tags = {"lion", "rust", "script"};
tags.add("language");
tags.has("lion");                      // true
tags.remove("rust");
```

### Tuples

```lion
let point = (10, 20);
print(point.0);                        // 10
print(point.1);                        // 20
```

### Comprehensions

```lion
let squares = [x * x for x in 0..10];
let evens = [x for x in 0..20 if x % 2 == 0];
let doubled = {x * 2 for x in [1, 2, 3]};   // set comprehension
```

---

## 6. Control Flow

### If / Elif / Else

```lion
if x > 0 {
    print("positive");
} elif x < 0 {
    print("negative");
} else {
    print("zero");
}
```

### Ternary

```lion
let max = a > b ? a : b;
let status = age >= 18 ? "adult" : "minor";
```

### While Loops

```lion
let i = 0;
while i < 5 {
    print(i);
    i += 1;
}
```

### For Loops

```lion
// range
for i in 0..10 { print(i); }           // 0 to 9
for i in 0..100..5 { print(i); }       // step by 5

// list
let items = ["a", "b", "c"];
for item in items { print(item); }

// dict
let d = {"x": 1, "y": 2};
for key in d { print(key); }           // keys only

// with index
for i, item in items { print(f"{i}: {item}"); }
```

### Break / Continue

```lion
for i in 0..10 {
    if i == 3 { continue; }
    if i == 7 { break; }
    print(i);                          // 0, 1, 2, 4, 5, 6
}
```

---

## 7. Functions & Closures

### Named Functions

```lion
func add(a, b) {
    return a + b;
}
print(add(3, 4));                      // 7
```

### Anonymous Functions (Lambdas)

```lion
let double = |x| x * 2;
print(double(5));                      // 10

let add = |a, b| a + b;
print(add(2, 3));                      // 5
```

### Default Arguments

```lion
func connect(host, port = 8080) {
    print(f"{host}:{port}");
}
connect("localhost");                  // localhost:8080
connect("localhost", 3000);            // localhost:3000
```

### Variadic Arguments

```lion
func sum(...nums) {
    let total = 0;
    for n in nums { total += n; }
    return total;
}
print(sum(1, 2, 3, 4));               // 10
```

### Closures

Functions capture their surrounding scope:

```lion
func make_counter(start) {
    let count = start;
    func inc() {
        count += 1;
        return count;
    }
    return inc;
}

let counter = make_counter(0);
print(counter());                      // 1
print(counter());                      // 2
print(counter());                      // 3
```

---

## 8. Pattern Matching

```lion
match value {
    0 => print("zero"),
    1..10 => print("small"),
    42 => print("answer!"),
    _ => print("something else"),
}
```

Pattern matching returns a value too:

```lion
let label = match code {
    200 => "OK",
    404 => "Not Found",
    500 => "Server Error",
    _ => "Unknown",
};
```

---

## 9. Error Handling

### Try / Catch

```lion
try {
    let result = risky_operation();
} catch e {
    print(f"caught: {e}");
}
```

### Throw

```lion
func divide(a, b) {
    if b == 0 { throw "division by zero"; }
    return a / b;
}
```

---

## 10. Modules & Imports

### Export

```lion
// math.lion
export func add(a, b) { return a + b; }
export const PI = 3.14159;
```

### Import

```lion
// main.lion
import "math.lion" as math;
print(math.add(2, 3));                  // 5
print(math.PI);                         // 3.14159
```

### Standard Modules

Built-in modules are available without import:

```lion
print(fs.read("file.txt"));
print(json.parse('{"key": "value"}'));
print(string.upper("hello"));
```

---

## 11. Standard Library

### File System (`fs`)

```lion
fs.write("test.txt", "Hello, Lion!");
let content = fs.read("test.txt");
fs.append("test.txt", " more text");
let exists = fs.exists("test.txt");     // true
fs.copy("test.txt", "backup.txt");
fs.remove("backup.txt");
```

### JSON (`json`)

```lion
let parsed = json.parse('{"name": "Lion"}');
print(parsed["name"]);                  // Lion

let encoded = json.stringify(parsed);
print(encoded);                         // {"name":"Lion"}

let pretty = json.pretty(parsed, 2);    // pretty-print

// file I/O
let data = json.load("config.json");
json.dump(data, "config_backup.json");
```

### HTTP (`http`)

```lion
let resp = http.get("https://api.github.com/repos/young-developer90/lion");
print(resp.status);                     // 200
print(resp.json()["description"]);

// with headers
let resp2 = http.post("https://httpbin.org/post",
    {"Content-Type": "application/json"},
    json.stringify({"key": "value"})
);
```

### Regular Expressions (`re`)

```lion
let text = "Contact: user@example.com, admin@test.com";
let emails = re.find_all(r"[\w.]+@[\w.]+", text);
print(emails);                          // ["user@example.com", "admin@test.com"]

let cleaned = re.sub(r"\d+", "#", "abc 123 def 456");
print(cleaned);                         // "abc # def #"

let parts = re.split(r"[,;]", "a,b;c");
print(parts);                           // ["a", "b", "c"]
```

### Path (`path`)

```lion
path.join("dir", "sub", "file.txt");           // "dir/sub/file.txt"
path.basename("/usr/local/bin");               // "bin"
path.dirname("/usr/local/bin");                // "/usr/local"
path.ext("document.txt");                      // "txt"
path.abs(".");                                 // "/home/user/project"
path.list_dir(".");                            // list of files
```

### Hashing (`hashlib`)

```lion
hashlib.sha256("hello");              // hex string
hashlib.md5("hello");
hashlib.base64_encode("hello");       // "aGVsbG8="
hashlib.base64_decode("aGVsbG8=");    // "hello"
```

### Date & Time (`datetime`)

```lion
let now = datetime.now();
print(now["year"]);                          // 2026

let formatted = datetime.format(now, "%Y-%m-%d %H:%M:%S");
print(formatted);                            // "2026-07-07 12:34:56"

let parsed = datetime.parse("2026-01-01", "%Y-%m-%d");
print(parsed["unix"]);                       // timestamp
```

### Statistics (`stats`)

```lion
let data = [1.0, 2.0, 3.0, 4.0, 5.0];
stats.mean(data);                      // 3.0
stats.median(data);                    // 3.0
stats.std(data);                       // ~1.58
stats.correlation(x, y);               // Pearson correlation
```

### Subprocess (`subprocess`)

```lion
let result = subprocess.run("echo", ["hello"]);
print(result["stdout"]);               // "hello\n"

let output = subprocess.run_output("whoami");
print(output);                         // current username
```

### Unit Testing (`test`)

```lion
test.assert_eq(add(2, 3), 5, "should add two numbers");
test.assert_true(is_even(4));
test.assert_approx(3.14159, 3.14, 0.01);
test.assert_lt(1, 2);
```

---

## 12. Building a GUI App

### Linux: Panther (GTK4)

Build with GTK4 support:

```bash
cargo build --release --features panther
```

#### Hello Window

```lion
let win = panther.Leo("Hello", 400, 300);
let label = panther.Label(win, "Welcome to Panther!");
panther.pack(label);
panther.mainloop(win);
```

#### Interactive App with Callback

```lion
let win = panther.Leo("Counter", 300, 200);
let display = panther.Label(win, "0");
panther.pack(display);

let btn = panther.Button(win, "Increment", func() {
    let current = panther.get(display);
    panther.config(display, "text", string(current + 1));
});
panther.pack(btn);

panther.mainloop(win);
```

#### Text Editor (full example at `examples/textedit.lion`)

```lion
let root = panther.Leo("Lion NotePad", 720, 320);
panther.pack(panther.Label(root, "Lion NotePad  —  single-line text editor"));

let file_frame = panther.Frame(root);
panther.pack(panther.Label(file_frame, "File path"));
let file_path = panther.Entry(file_frame);
panther.config(file_path, "text", "untitled.txt");
panther.pack(file_path);

let body_frame = panther.Frame(root);
panther.pack(panther.Label(body_frame, "Text content"));
let body = panther.Entry(body_frame);
panther.pack(body);

let btn_frame = panther.Frame(root);
panther.pack(panther.Label(btn_frame, "Actions"));

let load_btn = panther.Button(btn_frame, "Load", func() {
    let fpath = panther.get(file_path);
    let text = fs.read(fpath);
    panther.config(body, "text", text);
    panther.title(root, f"NotePad — {path.basename(fpath)}");
    panther.config(status, f"Loaded {path.basename(fpath)} ({string.len(text)} chars)");
});
panther.pack(load_btn);

let save_btn = panther.Button(btn_frame, "Save", func() {
    let fpath = panther.get(file_path);
    let text = panther.get(body);
    let _ = fs.write(fpath, text);
    panther.config(status, f"Saved to {path.basename(fpath)}");
});
panther.pack(save_btn);

let status = panther.Label(root, "Ready");
panther.pack(status);

panther.mainloop(root);
```

#### Available panther Widgets

| Function | Purpose |
|----------|---------|
| `Leo(title, w, h)` | Create a window |
| `Label(parent, text)` | Static text label |
| `Button(parent, text, callback)` | Clickable button |
| `Entry(parent)` | Single-line text input |
| `Frame(parent)` | Container with border |
| `pack(widget)` | Show a widget |
| `config(widget, prop, val)` | Change widget properties |
| `get(widget)` | Get text from Entry or Label |
| `insert(widget, pos, text)` | Insert text at position |
| `delete(widget, start, end)` | Delete text range |
| `title(widget, text)` | Set window title |
| `geometry(widget, w, h)` | Resize window |
| `mainloop(widget)` | Start the GUI event loop |
| `click(button)` | Programmatically click a button |
| `destroy(widget)` | Remove a widget |
| `messagebox(text, title)` | Show a message dialog |

### Windows: Leopard (Win32)

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

---

## 13. What's Next

Explore the examples in the `examples/` directory:

| File | Topic |
|------|-------|
| `basics.lion` | Math, booleans, strings, nil |
| `control_flow.lion` | If, while, for, match |
| `functions.lion` | Named functions, lambdas, closures |
| `data_structs.lion` | Lists, dicts, sets, tuples |
| `stdlib.lion` | Standard library tour |
| `stdlib_extended.lion` | Advanced stdlib |
| `textedit.lion` | GUI text editor with panther |
| `panther_test.lion` | Panther GUI test |
| `panda_demo.lion` | panda numerical library demo |
| `http.lion` | HTTP client examples |

Run the test suite:

```bash
cargo build --release --bin lion
./target/release/lion test tests/
```

Write your own modules:

```lion
// mylib.lion
export func greet(name) { return f"Hi, {name}!"; }
export const VERSION = "1.0";

// main.lion
import "mylib.lion" as lib;
print(lib.greet("Lion"));           // Hi, Lion!
```
