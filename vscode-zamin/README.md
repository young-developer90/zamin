# Zamin Language — VS Code Extension

Syntax highlighting, snippets, and LSP support for the [Zamin](https://github.com/young-developer90/zamin) scripting language.

## Features

- **Syntax highlighting** — keywords, strings (including f-strings), numbers, comments, operators, built-in types and modules
- **Code snippets** — `func`, `if`, `for`, `match`, `try`, `struct`, `import`, list comprehensions, f-strings, HTTP requests, file I/O, and GUI patterns
- **Language Server** — diagnostics, completions, and hover info (requires `zamin-lsp` binary)

## Install

1. Build the extension: `cd vscode-zamin && npm install`
2. Install in VS Code: `code --install-extension .`

Or from the VS Code Marketplace (future).

## LSP Setup

For diagnostics and completions, build the language server:

```bash
cargo build --bin zamin-lsp
```

The extension searches for `zamin-lsp` in your `PATH`, then in `target/debug/` and `target/release/`.

## Snippets

| Prefix | Description |
|--------|-------------|
| `func` | Function definition |
| `fn` / `lam` | Lambda expression |
| `if` / `ife` | If / if-elif-else |
| `for` / `forr` / `fori` | For loop over list / range / with index |
| `while` | While loop |
| `match` | Pattern matching |
| `try` | Try-catch |
| `struct` | Struct definition |
| `import` / `export` | Module import/export |
| `comp` / `compf` | List comprehension |
| `fstr` | F-string |
| `httpget` | HTTP GET request |
| `fsread` / `fswrite` | File read/write |
| `jsonparse` | JSON parse |
| `assert` | Unit test assertion |
| `luna` / `lbtn` | GUI window / button |
