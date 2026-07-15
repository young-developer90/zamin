pub mod ast;
pub mod bytecode;
pub mod cext;
pub mod cheetah_mod;
pub mod cli;
pub mod collections_mod;
pub mod compiler;
pub mod csv_mod;
pub mod datetime_mod;
pub mod gc;
pub mod hashlib_mod;
pub mod html_mod;
pub mod http;
pub mod http_module;
pub mod itertools_mod;
pub mod jaguar_mod;
pub mod json_mod;
#[cfg(target_os = "windows")]
pub mod leopard_mod;
#[cfg(all(not(target_os = "windows"), feature = "panther"))]
pub mod panther_mod;
pub mod lexer;
pub mod logging_mod;
pub mod module;
pub mod parser;
pub mod path_mod;
pub mod re_mod;
pub mod repl;
pub mod stats_mod;
pub mod stdlib;
pub mod string_mod;
pub mod subprocess_mod;
pub mod test_mod;
pub mod url_mod;
pub mod vm;

#[cfg(cuda_support)]
pub mod cuda;
#[cfg(cuda_support)]
pub mod linum;

#[cfg(feature = "python")]
pub mod py;

pub use vm::Vm;
pub use gc::{GcHeap, Value, GcObj, ObjRef, VmContext, NativeFunc, make_string, make_string_owned, make_list, make_dict, make_set, make_tuple, make_error, make_range, make_matrix, make_struct_def, make_struct_instance, to_f64, to_i64, get_str, get_str_owned};
pub use bytecode::{Chunk, OpCode, UpvalueInfo};
pub use compiler::Compiler;
pub use parser::Parser;
pub use lexer::Lexer;
pub use module::ModuleLoader;
pub use ast::{Program, Stmt, Expr, BinaryOpKind, UnaryOpKind};

pub fn execute_source(source: &str) -> Result<String, String> {
    let mut loader = ModuleLoader::new();
    loader.execute_source(source)
}

pub fn execute_file(path: &str) -> Result<(), String> {
    let mut loader = ModuleLoader::new();
    loader.execute_file(path)
}
