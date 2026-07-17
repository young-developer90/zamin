#![allow(dead_code)]

use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::Mutex;

use crate::bytecode::Chunk;

/// Fast hash-based lookup for dict/set operations.
/// Stores entries in insertion order Vec for GC tracing, but uses a HashMap
/// from hash values to indices for O(1)-average lookups.
#[derive(Debug, Clone)]
pub struct DictMap {
    pub entries: Vec<(Value, Value)>,
    hash_index: HashMap<u64, Vec<usize>>,
}

impl DictMap {
    pub fn new() -> Self {
        DictMap { entries: Vec::new(), hash_index: HashMap::new() }
    }
    pub fn with_capacity(cap: usize) -> Self {
        DictMap { entries: Vec::with_capacity(cap), hash_index: HashMap::with_capacity(cap) }
    }
    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
    pub fn iter(&self) -> impl Iterator<Item = &(Value, Value)> { self.entries.iter() }
    pub fn clear(&mut self) { self.entries.clear(); self.hash_index.clear(); }
    pub fn get(&self, key: &Value, heap: &GcHeap) -> Option<&Value> {
        let h = key.hash(heap);
        if let Some(indices) = self.hash_index.get(&h) {
            for &idx in indices {
                if self.entries[idx].0.eq(key, heap) {
                    return Some(&self.entries[idx].1);
                }
            }
        }
        None
    }
    pub fn contains(&self, key: &Value, heap: &GcHeap) -> bool {
        let h = key.hash(heap);
        if let Some(indices) = self.hash_index.get(&h) {
            indices.iter().any(|&idx| self.entries[idx].0.eq(key, heap))
        } else {
            false
        }
    }
    pub fn insert(&mut self, key: Value, val: Value, heap: &GcHeap) {
        let h = key.hash(heap);
        // Remove existing entry with same key
        if let Some(indices) = self.hash_index.get(&h).map(|v| v.clone()) {
            for idx in indices.iter().rev() {
                if self.entries[*idx].0.eq(&key, heap) {
                    self.entries[*idx].1 = val;
                    return;
                }
            }
        }
        let idx = self.entries.len();
        self.entries.push((key, val));
        self.hash_index.entry(h).or_default().push(idx);
    }
    pub fn remove(&mut self, key: &Value, heap: &GcHeap) {
        let h = key.hash(heap);
        let indices = match self.hash_index.get(&h).map(|v| v.clone()) {
            Some(v) => v,
            None => return,
        };
        for idx in indices.iter().rev() {
            if self.entries[*idx].0.eq(key, heap) {
                self.hash_index.entry(h).or_default().retain(|&i| i != *idx);
                self.entries.remove(*idx);
                // Update hash_index for shifted entries
                for (_, indices) in self.hash_index.iter_mut() {
                    for i in indices.iter_mut() {
                        if *i > *idx { *i -= 1; }
                    }
                }
                return;
            }
        }
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut (Value, Value)> {
        self.entries.iter_mut()
    }
}

impl From<Vec<(Value, Value)>> for DictMap {
    fn from(entries: Vec<(Value, Value)>) -> Self {
        let cap = entries.len();
        let mut map = DictMap { entries, hash_index: HashMap::with_capacity(cap) };
        for (i, (k, _)) in map.entries.iter().enumerate() {
            let h = k.hash_raw();
            map.hash_index.entry(h).or_default().push(i);
        }
        map
    }
}

type ResourceDropper = fn(i64);
static RESOURCE_DROPPER: Mutex<Option<ResourceDropper>> = Mutex::new(None);

/// Register a global callback that is invoked when an `OcvHandle` is freed by GC.
pub fn set_resource_dropper(f: ResourceDropper) {
    *RESOURCE_DROPPER.lock().unwrap() = Some(f);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjRef(pub usize);

#[derive(Debug, Clone)]
pub enum GcObj {
    String(String),
    List(Vec<Value>),
    Dict(Vec<(Value, Value)>),
    Set(Vec<Value>),
    Tuple(Vec<Value>),
    Function {
        name: Option<String>,
        params: Vec<String>,
        is_vararg: bool,
        body_chunk: usize,
        upvalue_count: usize,
    },
    Closure {
        function: ObjRef,
        upvalues: Vec<Upvalue>,
    },
    Error {
        message: String,
    },
    Range {
        start: i64,
        end: i64,
        step: i64,
    },
    Matrix {
        rows: usize,
        cols: usize,
        data: Vec<f64>,
    },
    StructDef {
        name: String,
        methods: Vec<(String, usize)>, // (method_name, chunk_index)
    },
    StructInstance {
        struct_ref: ObjRef,
        fields: Vec<(Value, Value)>,
    },
    /// External resource handle (e.g. OpenCV image). When collected by GC,
    /// the global resource dropper is called with this handle.
    OcvHandle(i64),
}

#[derive(Debug, Clone)]
pub struct Upvalue {
    pub is_local: bool,
    pub index: usize,
    pub value: Option<Value>,
}

pub struct GcHeap {
    pub objects: Vec<Option<GcObj>>,
    pub marks: Vec<bool>,
    free_list: Vec<usize>,
    pub permanent_roots: Vec<Value>,
    pub stats_total_allocated: usize,
    pub stats_collections: usize,
}

impl GcHeap {
    pub fn new() -> Self {
        GcHeap {
            objects: Vec::new(),
            marks: Vec::new(),
            free_list: Vec::new(),
            permanent_roots: Vec::new(),
            stats_total_allocated: 0,
            stats_collections: 0,
        }
    }

    pub fn alloc(&mut self, obj: GcObj) -> ObjRef {
        self.stats_total_allocated += 1;
        if let Some(idx) = self.free_list.pop() {
            self.objects[idx] = Some(obj);
            self.marks[idx] = false;
            ObjRef(idx)
        } else {
            let idx = self.objects.len();
            self.objects.push(Some(obj));
            self.marks.push(false);
            ObjRef(idx)
        }
    }

    pub fn get(&self, r: ObjRef) -> &GcObj {
        self.objects[r.0].as_ref().unwrap()
    }

    pub fn get_mut(&mut self, r: ObjRef) -> &mut GcObj {
        self.objects[r.0].as_mut().unwrap()
    }

    pub fn mark(&mut self, r: ObjRef) {
        self.marks[r.0] = true;
    }

    pub fn mark_value(&mut self, v: &Value) {
        match v {
            Value::String(r)
            | Value::List(r)
            | Value::Dict(r)
            | Value::Set(r)
            | Value::Tuple(r)
            | Value::Function(r)
            | Value::Closure(r)
            | Value::Error(r)
                | Value::Range(r)
                | Value::Matrix(r)
                | Value::Struct(r)
                | Value::StructInstance(r)
                | Value::Image(r) => self.mark(*r),
            _ => {}
        }
    }

    pub fn children_of(&self, r: ObjRef) -> Vec<ObjRef> {
        let obj = match self.objects[r.0].as_ref() { Some(o) => o, _ => return Vec::new() };
        match obj {
            GcObj::List(items) => items.iter().filter_map(|v| v.ref_or_nil()).collect(),
            GcObj::Dict(entries) => entries.iter().flat_map(|(k, v)| [k.ref_or_nil(), v.ref_or_nil()]).filter_map(|x| x).collect(),
            GcObj::Set(items) => items.iter().filter_map(|v| v.ref_or_nil()).collect(),
            GcObj::Tuple(items) => items.iter().filter_map(|v| v.ref_or_nil()).collect(),
            GcObj::Closure { function, upvalues } => {
                let mut refs = Vec::with_capacity(1 + upvalues.len());
                refs.push(*function);
                for uv in upvalues {
                    if let Some(ref val) = uv.value {
                        if let Some(r) = val.ref_or_nil() { refs.push(r); }
                    }
                }
                refs
            }
            GcObj::StructInstance { struct_ref, fields } => {
                let mut refs = Vec::with_capacity(1 + fields.len() * 2);
                refs.push(*struct_ref);
                for (k, v) in fields {
                    if let Some(r) = k.ref_or_nil() { refs.push(r); }
                    if let Some(r) = v.ref_or_nil() { refs.push(r); }
                }
                refs
            }
            _ => Vec::new(),
        }
    }

    pub fn mark_gray(&mut self, r: ObjRef) {
        for child in self.children_of(r) {
            self.mark(child);
        }
    }

    pub fn collect_garbage(&mut self, roots: &[Value]) {
        self.stats_collections += 1;
        for v in roots { self.mark_value(v); }
        let perm: Vec<Value> = self.permanent_roots.clone();
        for v in &perm { self.mark_value(v); }
        let len = self.objects.len();
        let mut gray: Vec<ObjRef> = Vec::new();
        for (i, marked) in self.marks.iter().enumerate() {
            if *marked { gray.push(ObjRef(i)); }
        }
        while let Some(r) = gray.pop() {
            for child in self.children_of(r) {
                if !self.marks[child.0] {
                    self.marks[child.0] = true;
                    gray.push(child);
                }
            }
        }
        let dropper = *RESOURCE_DROPPER.lock().unwrap();
        for i in 0..len {
            if !self.marks[i] {
                if let Some(GcObj::OcvHandle(h)) = &self.objects[i] {
                    if let Some(drop_fn) = dropper {
                        drop_fn(*h);
                    }
                }
                self.objects[i] = None;
                self.free_list.push(i);
            } else {
                self.marks[i] = false;
            }
        }
        // Compact free list - trim trailing Nones from objects
        while let Some(None) = self.objects.last() {
            self.objects.pop();
            self.marks.pop();
        }
    }

    pub fn stats(&self) -> (usize, usize) {
        let live = self.objects.iter().filter(|o| o.is_some()).count();
        (live, self.stats_collections)
    }
}

#[derive(Clone)]
pub struct NativeFunc {
    pub name: String,
    pub func: Rc<dyn Fn(&[Value], &mut VmContext) -> Result<Value, String>>,
}

impl fmt::Debug for NativeFunc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<native {}>", self.name)
    }
}

impl fmt::Display for NativeFunc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<native {}>", self.name)
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    UInt(u64),
    Float(f64),
    Bool(bool),
    Nil,
    String(ObjRef),
    List(ObjRef),
    Dict(ObjRef),
    Set(ObjRef),
    Tuple(ObjRef),
    Function(ObjRef),
    Closure(ObjRef),
    NativeFunc(NativeFunc),
    Error(ObjRef),
    Range(ObjRef),
    Matrix(ObjRef),
    Struct(ObjRef),
    StructInstance(ObjRef),
    Image(ObjRef),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::UInt(n) => write!(f, "{}", n),
            Value::Float(n) => {
                if n.fract() == 0.0 { write!(f, "{}.0", n) } else { write!(f, "{}", n) }
            }
            Value::Bool(b) => write!(f, "{}", b),
            Value::Nil => write!(f, "nil"),
            Value::String(_) => write!(f, "<string>"),
            Value::List(_) => write!(f, "<list>"),
            Value::Dict(_) => write!(f, "<dict>"),
            Value::Set(_) => write!(f, "<set>"),
            Value::Tuple(_) => write!(f, "<tuple>"),
            Value::Function(_) => write!(f, "<function>"),
            Value::Closure(_) => write!(f, "<closure>"),
            Value::NativeFunc(_) => write!(f, "<native>"),
            Value::Error(_) => write!(f, "<error>"),
            Value::Range(_) => write!(f, "<range>"),
            Value::Matrix(_) => write!(f, "<matrix>"),
            Value::Struct(_) => write!(f, "<struct>"),
            Value::StructInstance(_) => write!(f, "<instance>"),
            Value::Image(_) => write!(f, "<image>"),
        }
    }
}

impl Value {
    pub fn ref_or_nil(&self) -> Option<ObjRef> {
        match self {
            Value::String(r)
            | Value::List(r)
            | Value::Dict(r)
            | Value::Set(r)
            | Value::Tuple(r)
            | Value::Function(r)
            | Value::Closure(r)
            | Value::Error(r)
            | Value::Range(r)
            | Value::Matrix(r)
            | Value::Struct(r)
            | Value::StructInstance(r)
            | Value::Image(r) => Some(*r),
            _ => None,
        }
    }

    pub fn to_string(&self, heap: &GcHeap) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::UInt(n) => n.to_string(),
            Value::Float(n) => {
                if n.fract() == 0.0 { format!("{}.0", n) } else { format!("{}", n) }
            }
            Value::Bool(b) => b.to_string(),
            Value::Nil => "nil".to_string(),
            Value::String(r) => match heap.get(*r) { GcObj::String(s) => s.clone(), _ => "<string>".to_string() },
            Value::List(r) => {
                let items = match heap.get(*r) { GcObj::List(items) => items, _ => return "<list>".to_string() };
                let inner: Vec<String> = items.iter().map(|v| v.to_string(heap)).collect();
                format!("[{}]", inner.join(", "))
            }
            Value::Dict(r) => {
                let entries = match heap.get(*r) { GcObj::Dict(entries) => entries, _ => return "<dict>".to_string() };
                let inner: Vec<String> = entries.iter().map(|(k, v)| format!("{}: {}", k.to_string(heap), v.to_string(heap))).collect();
                format!("{{{}}}", inner.join(", "))
            }
            Value::Set(r) => {
                let items = match heap.get(*r) { GcObj::Set(items) => items, _ => return "<set>".to_string() };
                let inner: Vec<String> = items.iter().map(|v| v.to_string(heap)).collect();
                format!("{{{}}}", inner.join(", "))
            }
            Value::Tuple(r) => {
                let items = match heap.get(*r) { GcObj::Tuple(items) => items, _ => return "<tuple>".to_string() };
                let inner: Vec<String> = items.iter().map(|v| v.to_string(heap)).collect();
                format!("({})", inner.join(", "))
            }
            Value::Function(r) => match heap.get(*r) {
                GcObj::Function { name, .. } => name.as_deref().map(|n| format!("<fn {}>", n)).unwrap_or("<fn>".to_string()),
                _ => "<function>".to_string(),
            },
            Value::Closure(r) => match heap.get(*r) {
                GcObj::Closure { function, .. } => match heap.get(*function) {
                    GcObj::Function { name, .. } => name.as_deref().map(|n| format!("<fn {}>", n)).unwrap_or("<closure>".to_string()),
                    _ => "<closure>".to_string(),
                },
                _ => "<closure>".to_string(),
            },
            Value::NativeFunc(f) => f.name.clone(),
            Value::Error(r) => match heap.get(*r) { GcObj::Error { message } => format!("Error({})", message), _ => "<error>".to_string() },
            Value::Range(r) => match heap.get(*r) { GcObj::Range { start, end, .. } => format!("{}..{}", start, end), _ => "<range>".to_string() },
            Value::Struct(r) => match heap.get(*r) {
                GcObj::StructDef { name, .. } => format!("<struct {}>", name),
                _ => "<struct>".to_string(),
            },
            Value::StructInstance(r) => {
                let inst = match heap.get(*r) { GcObj::StructInstance { struct_ref, fields } => (struct_ref, fields), _ => return "<instance>".to_string() };
                let sname = match heap.get(*inst.0) { GcObj::StructDef { name, .. } => name.clone(), _ => "?".to_string() };
                let inner: Vec<String> = inst.1.iter().map(|(k, v)| format!("{}: {}", k.to_string(heap), v.to_string(heap))).collect();
                format!("{}({})", sname, inner.join(", "))
            }
            Value::Matrix(r) => match heap.get(*r) {
                GcObj::Matrix { rows, cols, data } => {
                    let mut s = format!("matrix({}x{}) [", rows, cols);
                    for r in 0..*rows {
                        if r > 0 { s.push_str("; "); }
                        s.push('[');
                        for c in 0..*cols {
                            if c > 0 { s.push_str(", "); }
                            s.push_str(&format!("{}", data[r * cols + c]));
                        }
                        s.push(']');
                    }
                    s.push(']');
                    s
                }
                _ => "<matrix>".to_string(),
            },
            Value::Image(r) => match heap.get(*r) {
                GcObj::OcvHandle(_) => "<image>".to_string(),
                _ => "<image>".to_string(),
            },
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Nil => false,
            Value::Int(n) => *n != 0,
            Value::UInt(n) => *n != 0,
            Value::Float(n) => *n != 0.0,
            _ => true,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::UInt(_) => "uint",
            Value::Float(_) => "float",
            Value::Bool(_) => "bool",
            Value::Nil => "nil",
            Value::String(_) => "string",
            Value::List(_) => "list",
            Value::Dict(_) => "dict",
            Value::Set(_) => "set",
            Value::Tuple(_) => "tuple",
            Value::Function(_) | Value::Closure(_) | Value::NativeFunc(_) => "function",
            Value::Error(_) => "error",
            Value::Range(_) => "range",
            Value::Matrix(_) => "matrix",
            Value::Struct(_) => "struct",
            Value::StructInstance(_) => "instance",
            Value::Image(_) => "image",
        }
    }

    pub fn string_len(&self, heap: &GcHeap) -> usize {
        match self {
            Value::String(r) => match heap.get(*r) {
                GcObj::String(s) => s.len(),
                _ => 8,
            },
            Value::Int(n) => n.to_string().len(),
            Value::UInt(n) => n.to_string().len(),
            Value::Float(_) => 16,
            Value::Bool(b) => if *b { 4 } else { 5 },
            Value::Nil => 3,
            Value::List(r) => match heap.get(*r) { GcObj::List(items) => items.len() * 8, _ => 4 },
            Value::Dict(r) => match heap.get(*r) { GcObj::Dict(e) => e.len() * 16, _ => 4 },
            _ => 8,
        }
    }

    pub fn eq(&self, other: &Value, heap: &GcHeap) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::UInt(a), Value::UInt(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::String(a), Value::String(b)) => match (heap.get(*a), heap.get(*b)) {
                (GcObj::String(sa), GcObj::String(sb)) => sa == sb,
                _ => false,
            },
            (Value::Int(a), Value::Float(b)) => *a as f64 == *b,
            (Value::Float(a), Value::Int(b)) => *a == *b as f64,
            (Value::Matrix(a), Value::Matrix(b)) => match (heap.get(*a), heap.get(*b)) {
                (GcObj::Matrix { rows: ra, cols: ca, data: da }, GcObj::Matrix { rows: rb, cols: cb, data: db }) => {
                    ra == rb && ca == cb && da == db
                }
                _ => false,
            },
            _ => false,
        }
    }

    pub fn hash(&self, heap: &GcHeap) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        match self {
            Value::Int(n) => n.hash(&mut h),
            Value::UInt(n) => n.hash(&mut h),
            Value::Float(n) => n.to_bits().hash(&mut h),
            Value::Bool(b) => b.hash(&mut h),
            Value::Nil => 0u64.hash(&mut h),
            Value::String(r) => {
                if let GcObj::String(s) = heap.get(*r) { s.hash(&mut h); }
            }
            _ => {}
        }
        h.finish()
    }

    /// Hash without heap access (for scalars only, returns 0 for heap types)
    pub fn hash_raw(&self) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        match self {
            Value::Int(n) => n.hash(&mut h),
            Value::UInt(n) => n.hash(&mut h),
            Value::Float(n) => n.to_bits().hash(&mut h),
            Value::Bool(b) => b.hash(&mut h),
            Value::Nil => 0u64.hash(&mut h),
            _ => {}
        }
        h.finish()
    }
}

pub struct VmContext<'a> {
    pub heap: &'a mut GcHeap,
    pub globals: &'a mut HashMap<String, Value>,
    pub chunks: &'a Vec<Chunk>,
    pub try_frames: &'a mut Vec<(usize, usize)>,
}

pub fn make_string(heap: &mut GcHeap, s: &str) -> Value {
    Value::String(heap.alloc(GcObj::String(s.to_string())))
}

pub fn make_string_owned(heap: &mut GcHeap, s: String) -> Value {
    Value::String(heap.alloc(GcObj::String(s)))
}

pub fn make_list(heap: &mut GcHeap, items: Vec<Value>) -> Value {
    Value::List(heap.alloc(GcObj::List(items)))
}

pub fn make_dict(heap: &mut GcHeap, entries: Vec<(Value, Value)>) -> Value {
    Value::Dict(heap.alloc(GcObj::Dict(entries)))
}

pub fn make_set(heap: &mut GcHeap, items: Vec<Value>) -> Value {
    Value::Set(heap.alloc(GcObj::Set(items)))
}

pub fn make_tuple(heap: &mut GcHeap, items: Vec<Value>) -> Value {
    Value::Tuple(heap.alloc(GcObj::Tuple(items)))
}

pub fn make_error(heap: &mut GcHeap, msg: &str) -> Value {
    Value::Error(heap.alloc(GcObj::Error { message: msg.to_string() }))
}

pub fn make_range(heap: &mut GcHeap, start: i64, end: i64, step: i64) -> Value {
    Value::Range(heap.alloc(GcObj::Range { start, end, step }))
}

pub fn make_matrix(heap: &mut GcHeap, rows: usize, cols: usize, data: Vec<f64>) -> Value {
    Value::Matrix(heap.alloc(GcObj::Matrix { rows, cols, data }))
}

pub fn make_struct_def(heap: &mut GcHeap, name: &str, methods: Vec<(String, usize)>) -> Value {
    Value::Struct(heap.alloc(GcObj::StructDef { name: name.to_string(), methods }))
}

pub fn make_struct_instance(heap: &mut GcHeap, struct_ref: ObjRef, fields: Vec<(Value, Value)>) -> Value {
    Value::StructInstance(heap.alloc(GcObj::StructInstance { struct_ref, fields }))
}

pub fn make_image(heap: &mut GcHeap, handle: i64) -> Value {
    Value::Image(heap.alloc(GcObj::OcvHandle(handle)))
}

pub fn to_f64(val: &Value) -> Result<f64, String> {
    match val {
        Value::Int(n) => Ok(*n as f64),
        Value::UInt(n) => Ok(*n as f64),
        Value::Float(n) => Ok(*n),
        _ => Err(format!("cannot convert {} to float", val.type_name())),
    }
}

pub fn to_i64(val: &Value) -> Result<i64, String> {
    match val {
        Value::Int(n) => Ok(*n),
        Value::UInt(n) => Ok(*n as i64),
        Value::Float(n) => Ok(*n as i64),
        _ => Err(format!("cannot convert {} to int", val.type_name())),
    }
}

pub fn get_str<'a>(val: &'a Value, heap: &'a GcHeap) -> Result<&'a str, String> {
    match val {
        Value::String(r) => match heap.get(*r) {
            GcObj::String(s) => Ok(s.as_str()),
            _ => Err("invalid string".to_string()),
        },
        _ => Err("expected string".to_string()),
    }
}

pub fn get_str_owned(val: &Value, heap: &GcHeap) -> Result<String, String> {
    match val {
        Value::String(r) => match heap.get(*r) {
            GcObj::String(s) => Ok(s.clone()),
            _ => Err("invalid string".to_string()),
        },
        other => Ok(other.to_string(heap)),
    }
}
