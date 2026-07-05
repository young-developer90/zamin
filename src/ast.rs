#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    UInt(u64),
    Float(f64),
    String(String),
    FString(Vec<FStringPart>),
    Bool(bool),
    Nil,
    Identifier(String),
    List(Vec<Expr>),
    Dict(Vec<(Expr, Expr)>),
    Set(Vec<Expr>),
    Tuple(Vec<Expr>),
    BinaryOp {
        op: BinaryOpKind,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    UnaryOp {
        op: UnaryOpKind,
        operand: Box<Expr>,
    },
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
    },
    OpAssign {
        op: BinaryOpKind,
        target: Box<Expr>,
        value: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        is_method: bool,
    },
    Index {
        obj: Box<Expr>,
        index: Box<Expr>,
    },
    Attr {
        obj: Box<Expr>,
        name: String,
    },
    Func {
        name: Option<String>,
        params: Vec<String>,
        is_vararg: bool,
        body: Vec<Stmt>,
    },
    Lambda {
        params: Vec<String>,
        body: Box<Expr>,
    },
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        step: i64,
    },
    InterpolatedString(Vec<Expr>),
    Ternary {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },
    NamedArg {
        name: String,
        value: Box<Expr>,
    },
}

#[derive(Debug, Clone)]
pub enum FStringPart {
    Literal(String),
    Expr(Expr),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOpKind {
    Add, Sub, Mul, Div, Mod, Pow, IntDiv,
    Eq, Ne, Lt, Gt, Le, Ge,
    And, Or,
    Concat, In,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOpKind {
    Neg, Not,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expr(Expr),
    Let {
        name: String,
        type_annotation: Option<String>,
        value: Box<Expr>,
        is_const: bool,
    },
    Assign {
        target: Expr,
        value: Expr,
    },
    OpAssign {
        op: BinaryOpKind,
        target: Expr,
        value: Expr,
    },
    If {
        condition: Expr,
        then_branch: Vec<Stmt>,
        elif_branches: Vec<(Expr, Vec<Stmt>)>,
        else_branch: Option<Vec<Stmt>>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    For {
        variable: String,
        iterable: Expr,
        body: Vec<Stmt>,
    },
    Match {
        value: Expr,
        arms: Vec<(Expr, Vec<Stmt>)>,
    },
    Return(Option<Expr>),
    FuncDef {
        name: String,
        params: Vec<String>,
        is_vararg: bool,
        body: Vec<Stmt>,
        is_export: bool,
    },
    Export {
        names: Vec<String>,
    },
    Import {
        module: String,
        alias: Option<String>,
        symbols: Vec<(String, Option<String>)>,
    },
    Throw(Expr),
    Try {
        body: Vec<Stmt>,
        catch_var: String,
        catch_body: Vec<Stmt>,
    },
    Break,
    Continue,
    StructDef {
        name: String,
        methods: Vec<Stmt>,
    },
}

#[derive(Debug, Clone)]
pub enum TypeAnnotation {
    Int, UInt, Float, String, Bool, Nil,
    List, Dict, Set, Tuple, Function,
    Named(String),
}

#[derive(Debug, Clone)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}

impl Program {
    pub fn new() -> Self {
        Program { stmts: Vec::new() }
    }
}
