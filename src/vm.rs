use std::collections::HashMap;
use std::rc::Rc;

use crate::bytecode::*;
use crate::gc::*;

pub struct Vm {
    pub chunks: Vec<Chunk>,
    pub heap: GcHeap,
    pub stack: Vec<Value>,
    pub globals: Vec<(String, Value)>,
    pub frames: Vec<Frame>,
    pub ip: usize,
    pub chunk_idx: usize,
    pub modules: HashMap<String, Value>,
    pub try_frames: Vec<(usize, usize)>,
}

pub struct Frame {
    pub chunk_idx: usize,
    pub ip: usize,
    pub stack_start: usize,
    pub locals: usize,
    pub closure: Option<ObjRef>,
}

impl Vm {
    pub fn new(mut chunks: Vec<Chunk>) -> Self {
        let mut heap = GcHeap::new();
        for chunk in &mut chunks {
            for (i, content) in chunk.string_constants.iter().enumerate() {
                if let Some(s) = content {
                    chunk.constants[i] = Value::String(heap.alloc(GcObj::String(s.clone())));
                }
            }
        }
        Vm {
            chunks,
            heap,
            stack: Vec::new(),
            globals: Vec::new(),
            frames: Vec::new(),
            ip: 0,
            chunk_idx: 0,
            modules: HashMap::new(),
            try_frames: Vec::new(),
        }
    }

    pub fn read_u16(&self, pos: usize) -> u16 {
        u16::from_le_bytes([
            self.chunks[self.chunk_idx].code[pos],
            self.chunks[self.chunk_idx].code[pos + 1],
        ])
    }

    pub fn chunk(&self) -> &Chunk {
        &self.chunks[self.chunk_idx]
    }

    pub fn run(&mut self) -> Result<Value, String> {
        loop {
            if self.ip >= self.chunk().code.len() {
                return Err("program counter out of bounds".to_string());
            }

            let op = OpCode::from_u8(self.chunk().code[self.ip])
                .ok_or(format!("unknown opcode at {}", self.ip))?;
            self.ip += 1;

            match op {
                OpCode::Halt => return Ok(Value::Nil),
                OpCode::Pop => { self.stack.pop().ok_or("stack empty on pop")?; }
                OpCode::Dup => {
                    let val = self.stack.last().ok_or("stack empty on dup")?.clone();
                    self.stack.push(val);
                }
                OpCode::Nil => self.stack.push(Value::Nil),
                OpCode::True => self.stack.push(Value::Bool(true)),
                OpCode::False => self.stack.push(Value::Bool(false)),
                OpCode::LoadConst => {
                    let idx = self.read_u16(self.ip) as usize;
                    self.ip += 2;
                    self.stack.push(self.chunk().constants[idx].clone());
                }
                OpCode::LoadLocal => {
                    let idx = self.read_u16(self.ip) as usize;
                    self.ip += 2;
                    let stack_idx = self.frames.last().map(|f| f.stack_start + idx).unwrap_or(idx);
                    self.stack.push(self.stack[stack_idx].clone());
                }
                OpCode::LoadUpvalue => {
                    let idx = self.read_u16(self.ip) as usize;
                    self.ip += 2;
                    let cl_ref = self.frames.last()
                        .and_then(|f| f.closure)
                        .ok_or("load_upvalue: no closure")?;
                    if let GcObj::Closure { ref upvalues, .. } = self.heap.get(cl_ref) {
                        let val = upvalues[idx].value.clone().unwrap_or(Value::Nil);
                        self.stack.push(val);
                    } else {
                        return Err("load_upvalue: not a closure".to_string());
                    }
                }
                OpCode::StoreUpvalue => {
                    let idx = self.read_u16(self.ip) as usize;
                    self.ip += 2;
                    let val = self.stack.pop().ok_or("stack empty on store_upvalue")?;
                    let cl_ref = self.frames.last()
                        .and_then(|f| f.closure)
                        .ok_or("store_upvalue: no closure")?;
                    if let GcObj::Closure { ref mut upvalues, .. } = self.heap.get_mut(cl_ref) {
                        upvalues[idx].value = Some(val);
                    } else {
                        return Err("store_upvalue: not a closure".to_string());
                    }
                }
                OpCode::StoreLocal => {
                    let idx = self.read_u16(self.ip) as usize;
                    self.ip += 2;
                    let val = self.stack.pop().ok_or("stack empty on store_local")?;
                    let stack_idx = self.frames.last().map(|f| f.stack_start + idx).unwrap_or(idx);
                    if stack_idx < self.stack.len() {
                        self.stack[stack_idx] = val;
                    } else {
                        self.stack.resize(stack_idx + 1, Value::Nil);
                        self.stack[stack_idx] = val;
                    }
                }
                OpCode::LoadGlobal => {
                    let sidx = self.read_u16(self.ip) as usize;
                    self.ip += 2;
                    let name = self.chunk().string_pool.get(sidx).ok_or("invalid global name index")?.clone();
                    if let Some((_, val)) = self.globals.iter().find(|(n, _)| n == &name) {
                        self.stack.push(val.clone());
                    } else {
                        return Err(format!("undefined global '{}'", name));
                    }
                }
                OpCode::StoreGlobal => {
                    let sidx = self.read_u16(self.ip) as usize;
                    self.ip += 2;
                    let name = self.chunk().string_pool.get(sidx).ok_or("invalid global name index")?.clone();
                    let val = self.stack.pop().ok_or("stack empty on store_global")?;
                    if let Some((_, entry)) = self.globals.iter_mut().find(|(n, _)| n == &name) {
                        *entry = val;
                    } else {
                        self.globals.push((name, val));
                    }
                }
                OpCode::Add => {
                    let b = self.stack.pop().ok_or("stack empty")?;
                    let a = self.stack.pop().ok_or("stack empty")?;
                    let result = match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => Value::Int(x + y),
                        (Value::Float(x), Value::Float(y)) => Value::Float(x + y),
                        (Value::Int(x), Value::Float(y)) => Value::Float(*x as f64 + y),
                        (Value::Float(x), Value::Int(y)) => Value::Float(x + *y as f64),
                        _ => add_values(&a, &b, &mut self.heap)?,
                    };
                    self.stack.push(result);
                }
                OpCode::Sub => {
                    let b = self.stack.pop().ok_or("stack empty")?;
                    let a = self.stack.pop().ok_or("stack empty")?;
                    let result = match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => Value::Int(x - y),
                        (Value::Float(x), Value::Float(y)) => Value::Float(x - y),
                        (Value::Int(x), Value::Float(y)) => Value::Float(*x as f64 - y),
                        (Value::Float(x), Value::Int(y)) => Value::Float(x - *y as f64),
                        _ => sub_values(&a, &b)?,
                    };
                    self.stack.push(result);
                }
                OpCode::Mul => {
                    let b = self.stack.pop().ok_or("stack empty")?;
                    let a = self.stack.pop().ok_or("stack empty")?;
                    let result = match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => Value::Int(x * y),
                        (Value::Float(x), Value::Float(y)) => Value::Float(x * y),
                        (Value::Int(x), Value::Float(y)) => Value::Float(*x as f64 * y),
                        (Value::Float(x), Value::Int(y)) => Value::Float(x * *y as f64),
                        _ => mul_values(&a, &b)?,
                    };
                    self.stack.push(result);
                }
                OpCode::Div => {
                    let b = self.stack.pop().ok_or("stack empty")?;
                    let a = self.stack.pop().ok_or("stack empty")?;
                    let result = match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => Value::Float(*x as f64 / *y as f64),
                        (Value::Float(x), Value::Float(y)) => Value::Float(x / y),
                        (Value::Int(x), Value::Float(y)) => Value::Float(*x as f64 / y),
                        (Value::Float(x), Value::Int(y)) => Value::Float(x / *y as f64),
                        _ => div_values(&a, &b)?,
                    };
                    self.stack.push(result);
                }
                OpCode::Mod => {
                    let b = self.stack.pop().ok_or("stack empty")?;
                    let a = self.stack.pop().ok_or("stack empty")?;
                    self.stack.push(mod_values(&a, &b)?);
                }
                OpCode::Pow => {
                    let b = self.stack.pop().ok_or("stack empty")?;
                    let a = self.stack.pop().ok_or("stack empty")?;
                    self.stack.push(pow_values(&a, &b)?);
                }
                OpCode::IntDiv => {
                    let b = self.stack.pop().ok_or("stack empty")?;
                    let a = self.stack.pop().ok_or("stack empty")?;
                    self.stack.push(intdiv_values(&a, &b)?);
                }
                OpCode::Neg => {
                    let a = self.stack.pop().ok_or("stack empty")?;
                    self.stack.push(neg_value(&a)?);
                }
                OpCode::Not => {
                    let a = self.stack.pop().ok_or("stack empty")?;
                    self.stack.push(Value::Bool(!a.is_truthy()));
                }
                OpCode::Eq => {
                    let b = self.stack.pop().ok_or("stack empty")?;
                    let a = self.stack.pop().ok_or("stack empty")?;
                    self.stack.push(Value::Bool(a.eq(&b, &self.heap)));
                }
                OpCode::Ne => {
                    let b = self.stack.pop().ok_or("stack empty")?;
                    let a = self.stack.pop().ok_or("stack empty")?;
                    self.stack.push(Value::Bool(!a.eq(&b, &self.heap)));
                }
                OpCode::Lt => {
                    let b = self.stack.pop().ok_or("stack empty")?;
                    let a = self.stack.pop().ok_or("stack empty")?;
                    self.stack.push(Value::Bool(cmp_lt(&a, &b)?));
                }
                OpCode::Gt => {
                    let b = self.stack.pop().ok_or("stack empty")?;
                    let a = self.stack.pop().ok_or("stack empty")?;
                    self.stack.push(Value::Bool(cmp_lt(&b, &a)?));
                }
                OpCode::Le => {
                    let b = self.stack.pop().ok_or("stack empty")?;
                    let a = self.stack.pop().ok_or("stack empty")?;
                    self.stack.push(Value::Bool(!cmp_lt(&b, &a)?));
                }
                OpCode::Ge => {
                    let b = self.stack.pop().ok_or("stack empty")?;
                    let a = self.stack.pop().ok_or("stack empty")?;
                    self.stack.push(Value::Bool(!cmp_lt(&a, &b)?));
                }
                OpCode::And => {
                    let b = self.stack.pop().ok_or("stack empty")?;
                    let a = self.stack.pop().ok_or("stack empty")?;
                    self.stack.push(if !a.is_truthy() { a } else { b });
                }
                OpCode::Or => {
                    let b = self.stack.pop().ok_or("stack empty")?;
                    let a = self.stack.pop().ok_or("stack empty")?;
                    self.stack.push(if a.is_truthy() { a } else { b });
                }
                OpCode::Concat => {
                    let b = self.stack.pop().ok_or("stack empty")?;
                    let a = self.stack.pop().ok_or("stack empty")?;
                    let sa = value_display(&a, &self.heap);
                    let sb = value_display(&b, &self.heap);
                    self.stack.push(make_string(&mut self.heap, &format!("{}{}", sa, sb)));
                }
                OpCode::In => {
                    let right = self.stack.pop().ok_or("stack empty")?;
                    let left = self.stack.pop().ok_or("stack empty")?;
                    let result = contains_check(&left, &right, &mut self.heap)?;
                    self.stack.push(Value::Bool(result));
                }
                OpCode::Return => {
                    let val = self.stack.pop().ok_or("stack empty on return")?;
                    if let Some(frame) = self.frames.pop() {
                        self.stack.truncate(frame.stack_start);
                        self.stack.push(val);
                        self.chunk_idx = frame.chunk_idx;
                        self.ip = frame.ip;
                    } else {
                        return Ok(val);
                    }
                }
                OpCode::Print => {
                    if let Some(val) = self.stack.pop() {
                        print!("{}", val.to_string(&self.heap));
                    }
                }
                OpCode::PrintLn => {
                    if let Some(val) = self.stack.pop() {
                        println!("{}", val.to_string(&self.heap));
                    } else { println!(); }
                }
                OpCode::Jump => { self.ip = self.read_u16(self.ip) as usize; }
                OpCode::JumpIfTrue => {
                    let target = self.read_u16(self.ip) as usize;
                    self.ip += 2;
                    if let Some(val) = self.stack.last() {
                        if val.is_truthy() { self.ip = target; }
                    }
                }
                OpCode::JumpIfFalse => {
                    let target = self.read_u16(self.ip) as usize;
                    self.ip += 2;
                    if let Some(val) = self.stack.last() {
                        if !val.is_truthy() { self.ip = target; }
                    }
                }
                OpCode::JumpIfNil => {
                    let target = self.read_u16(self.ip) as usize;
                    self.ip += 2;
                    if let Some(val) = self.stack.last() {
                        if matches!(val, Value::Nil) { self.ip = target; }
                    }
                }
                OpCode::Call => {
                    let argc = self.read_u16(self.ip) as usize;
                    self.ip += 2;

                    let args: Vec<Value> = if argc > 0 {
                        let start = self.stack.len() - argc;
                        self.stack[start..].to_vec()
                    } else { Vec::new() };
                    for _ in 0..argc { self.stack.pop(); }

                    let callee = self.stack.pop().ok_or("stack empty on call")?;

                    match callee {
                        Value::NativeFunc(f) => {
                            let mut ctx = VmContext {
                                heap: &mut self.heap,
                                globals: &mut self.globals,
                                modules: &mut Vec::new(),
                                chunks: &self.chunks,
                                try_frames: &mut self.try_frames,
                            };
                            let result = (f.func)(&args, &mut ctx)
                                .map_err(|e| format!("{}", e));
                            match result {
                                Ok(val) => self.stack.push(val),
                                Err(e) => {
                                    if let Some((depth, catch_ip)) = self.try_frames.pop() {
                                        self.stack.truncate(depth);
                                        self.stack.push(make_error(&mut self.heap, &e));
                                        self.ip = catch_ip;
                                    } else {
                                        return Err(e);
                                    }
                                }
                            }
                        }
                        Value::Function(func_ref) => {
                            let func_obj = self.heap.get(func_ref).clone();
                            if let GcObj::Function { body_chunk, .. } = func_obj {
                                let frame = Frame {
                                    chunk_idx: self.chunk_idx,
                                    ip: self.ip,
                                    stack_start: self.stack.len(),
                                    locals: args.len(),
                                    closure: None,
                                };
                                self.frames.push(frame);
                                for arg in args { self.stack.push(arg); }
                                self.chunk_idx = body_chunk;
                                self.ip = 0;
                            }
                        }
                        Value::Closure(cl_ref) => {
                            let closure = self.heap.get(cl_ref).clone();
                            if let GcObj::Closure { function, .. } = closure {
                                let func = self.heap.get(function).clone();
                                if let GcObj::Function { body_chunk, .. } = func {
                                    let frame = Frame {
                                        chunk_idx: self.chunk_idx,
                                        ip: self.ip,
                                        stack_start: self.stack.len(),
                                        locals: args.len(),
                                        closure: Some(cl_ref),
                                    };
                                    self.frames.push(frame);
                                    for arg in args { self.stack.push(arg); }
                                    self.chunk_idx = body_chunk;
                                    self.ip = 0;
                                }
                            }
                        }
                        Value::Struct(r) => {
                            // Struct constructor call: args are (key, value) pairs
                            let struct_ref = r;
                            let mut fields = Vec::new();
                            let mut init_args = Vec::new();
                            let mut i = 0;
                            while i + 1 < args.len() {
                                fields.push((args[i].clone(), args[i + 1].clone()));
                                init_args.push(args[i + 1].clone());
                                i += 2;
                            }
                            let instance = make_struct_instance(&mut self.heap, struct_ref, fields);
                            // Call init if it exists: pass (this, field_values...)
                            let struct_def = self.heap.get(struct_ref).clone();
                            if let GcObj::StructDef { ref methods, .. } = struct_def {
                                if let Some((_, init_chunk)) = methods.iter().find(|(n, _)| n == "init") {
                            let mut init_args_with_this = vec![instance.clone()];
                            init_args_with_this.extend(init_args);
                            let save_ip = self.ip;
                            let save_chunk = self.chunk_idx;
                            let mut ctx = VmContext {
                                heap: &mut self.heap,
                                globals: &mut self.globals,
                                modules: &mut Vec::new(),
                                chunks: &self.chunks,
                                try_frames: &mut self.try_frames,
                            };
                            execute_chunk(*init_chunk, &init_args_with_this, &mut ctx)?;
                            self.ip = save_ip;
                            self.chunk_idx = save_chunk;
                        }
                    }
                    self.stack.push(instance);
                }
                other => return Err(format!("cannot call {}", other.type_name())),
                    }
                }
                OpCode::MakeFunc => {
                    let chunk_idx = self.read_u16(self.ip) as usize;
                    self.ip += 2;
                    let func_ref = self.heap.alloc(GcObj::Function {
                        name: self.chunks[chunk_idx].name.clone(),
                        params: Vec::new(),
                        is_vararg: false,
                        body_chunk: chunk_idx,
                        upvalue_count: 0,
                    });
                    self.stack.push(Value::Function(func_ref));
                }
                OpCode::CloseUpvalue => {
                    self.ip += 2;
                }
                OpCode::MakeClosure => {
                    let chunk_idx = self.read_u16(self.ip) as usize;
                    self.ip += 2;
                    let upvalue_infos = self.chunks[chunk_idx].upvalues.clone();
                    let mut captured = Vec::new();
                    let stack_start = self.frames.last().map(|f| f.stack_start).unwrap_or(0);
                    for uv in &upvalue_infos {
                        if uv.is_local {
                            let stack_idx = stack_start + uv.index;
                            let val = self.stack[stack_idx].clone();
                            captured.push(Upvalue { is_local: true, index: uv.index, value: Some(val) });
                        } else {
                            captured.push(Upvalue { is_local: false, index: uv.index, value: None });
                        }
                    }
                    let func_ref = self.heap.alloc(GcObj::Function {
                        name: self.chunks[chunk_idx].name.clone(),
                        params: Vec::new(),
                        is_vararg: false,
                        body_chunk: chunk_idx,
                        upvalue_count: upvalue_infos.len(),
                    });
                    let closure_ref = self.heap.alloc(GcObj::Closure {
                        function: func_ref,
                        upvalues: captured,
                    });
                    self.stack.push(Value::Closure(closure_ref));
                }
                OpCode::BuildList => {
                    let count = self.read_u16(self.ip) as usize;
                    self.ip += 2;
                    let start = self.stack.len().saturating_sub(count);
                    let items: Vec<Value> = self.stack.drain(start..).collect();
                    self.stack.push(make_list(&mut self.heap, items));
                }
                OpCode::BuildDict => {
                    self.ip += 2;
                    self.stack.push(make_dict(&mut self.heap, Vec::new()));
                }
                OpCode::BuildSet => {
                    self.ip += 2;
                    self.stack.push(make_set(&mut self.heap, Vec::new()));
                }
                OpCode::BuildTuple => {
                    let count = self.read_u16(self.ip) as usize;
                    self.ip += 2;
                    let start = self.stack.len().saturating_sub(count);
                    let items: Vec<Value> = self.stack.drain(start..).collect();
                    self.stack.push(make_tuple(&mut self.heap, items));
                }
                OpCode::ListAppend => {
                    self.ip += 2;
                    let val = self.stack.pop().ok_or("stack empty")?;
                    if let Some(Value::List(r)) = self.stack.pop() {
                        if let GcObj::List(ref mut items) = self.heap.get_mut(r) {
                            items.push(val);
                        }
                    }
                }
                OpCode::DictSet => {
                    self.ip += 2;
                    let val = self.stack.pop().ok_or("stack empty")?;
                    let key = self.stack.pop().ok_or("stack empty")?;
                    if let Some(Value::Dict(r)) = self.stack.pop() {
                        let key_clone = key.clone();
                        let mut indices_to_remove: Vec<usize> = Vec::new();
                        {
                            let entries = match self.heap.get(r) {
                                GcObj::Dict(ref entries) => entries,
                                _ => return Err("not a dict".to_string()),
                            };
                            for (i, (k, _)) in entries.iter().enumerate() {
                                if k.eq(&key_clone, &self.heap) {
                                    indices_to_remove.push(i);
                                }
                            }
                        }
                        {
                            let entries = match self.heap.get_mut(r) {
                                GcObj::Dict(ref mut entries) => entries,
                                _ => return Err("not a dict".to_string()),
                            };
                            for i in indices_to_remove.into_iter().rev() {
                                entries.remove(i);
                            }
                            entries.push((key, val));
                        }
                        self.stack.push(Value::Dict(r));
                    }
                }
                OpCode::SetAdd => {
                    self.ip += 2;
                    let val = self.stack.pop().ok_or("stack empty")?;
                    if let Some(Value::Set(r)) = self.stack.pop() {
                        let val_clone = val.clone();
                        let exists = {
                            let items = match self.heap.get(r) {
                                GcObj::Set(ref items) => items,
                                _ => return Err("not a set".to_string()),
                            };
                            items.iter().any(|x| x.eq(&val_clone, &self.heap))
                        };
                        if !exists {
                            if let GcObj::Set(ref mut items) = self.heap.get_mut(r) {
                                items.push(val);
                            }
                        }
                        self.stack.push(Value::Set(r));
                    }
                }
                OpCode::LoadIndex => {
                    self.ip += 2;
                    let index = self.stack.pop().ok_or("stack empty")?;
                    let obj = self.stack.pop().ok_or("stack empty")?;
                    let result = match (&obj, &index) {
                        (Value::List(r), Value::Int(n)) => {
                            let i = if *n < 0 { let len = { let items = match self.heap.get(*r) { GcObj::List(items) => items, _ => return Err("not a list".to_string()) }; items.len() }; len as i64 + n } else { *n };
                            if i < 0 { return Err("list index out of bounds".to_string()); }
                            match self.heap.get(*r) { GcObj::List(items) => { if (i as usize) >= items.len() { return Err("list index out of bounds".to_string()); } items[i as usize].clone() }, _ => return Err("not a list".to_string()) }
                        }
                        _ => load_index(&obj, &index, &mut self.heap)?,
                    };
                    self.stack.push(result);
                }
                OpCode::StoreIndex => {
                    self.ip += 2;
                    let index = self.stack.pop().ok_or("stack empty")?;
                    let obj = self.stack.pop().ok_or("stack empty")?;
                    let val = self.stack.pop().ok_or("stack empty")?;
                    match (&obj, &index) {
                        (Value::List(r), Value::Int(n)) => {
                            let i = if *n < 0 {
                                let len = { let items = match self.heap.get(*r) { GcObj::List(items) => items, _ => return Err("not a list".to_string()) }; items.len() };
                                len as i64 + n
                            } else { *n };
                            if i < 0 { return Err("list index out of bounds".to_string()); }
                            match self.heap.get_mut(*r) { GcObj::List(ref mut items) => { if (i as usize) >= items.len() { return Err("list index out of bounds".to_string()); } items[i as usize] = val; }, _ => return Err("not a list".to_string()) }
                        }
                        _ => store_index(obj, index, val, &mut self.heap)?,
                    }
                }
                OpCode::LoadAttr => {
                    let sidx = self.read_u16(self.ip) as usize;
                    self.ip += 2;
                    let name = self.chunk().string_pool.get(sidx).ok_or("invalid attr name index")?.clone();
                    let obj = self.stack.pop().ok_or("stack empty")?;
                    self.stack.push(load_attr(&obj, &name, &mut self.heap)?);
                }
                OpCode::StoreAttr => {
                    let sidx = self.read_u16(self.ip) as usize;
                    self.ip += 2;
                    let name = self.chunk().string_pool.get(sidx).ok_or("invalid attr name index")?.clone();
                    let obj = self.stack.pop().ok_or("stack empty")?;
                    let val = self.stack.pop().ok_or("stack empty")?;
                    store_attr(&obj, &name, val, &mut self.heap)?;
                }
                OpCode::Throw => {
                    let val = self.stack.pop().ok_or("stack empty")?;
                    let msg = val.to_string(&self.heap);
                    if let Some((depth, catch_ip)) = self.try_frames.pop() {
                        self.stack.truncate(depth);
                        self.stack.push(val);
                        self.ip = catch_ip;
                    } else {
                        return Err(format!("Error: {}", msg));
                    }
                }
                OpCode::Try => {
                    let catch_ip = self.read_u16(self.ip) as usize;
                    self.ip += 2;
                    self.try_frames.push((self.stack.len(), catch_ip));
                }
                OpCode::EndTry => {
                    self.try_frames.pop();
                }
                OpCode::CheckMatch => {
                    self.ip += 2;
                    let val = self.stack.pop().ok_or("stack empty")?;
                    let pattern = self.stack.pop().ok_or("stack empty")?;
                    let matched = val.eq(&pattern, &self.heap)
                        || matches!(&pattern, Value::String(r) if {
                            matches!(self.heap.get(*r), GcObj::String(s) if s == "_")
                        });
                    self.stack.push(Value::Bool(matched));
                }
                OpCode::BuildRange => {
                    self.ip += 2;
                    let step = self.stack.pop().ok_or("stack empty")?;
                    let end = self.stack.pop().ok_or("stack empty")?;
                    let start = self.stack.pop().ok_or("stack empty")?;
                    let s = match start { Value::Int(n) => n, _ => return Err("range start must be int".to_string()) };
                    let e = match end { Value::Int(n) => n, _ => return Err("range end must be int".to_string()) };
                    let st = match step { Value::Int(n) => n, _ => return Err("range step must be int".to_string()) };
                    self.stack.push(make_range(&mut self.heap, s, e, st));
                }
                OpCode::MakeIter => {
                    self.ip += 2;
                    let obj = self.stack.pop().ok_or("stack empty")?;
                    self.stack.push(make_iterator(&obj, &mut self.heap)?);
                }
                OpCode::NextIter => {
                    self.ip += 2;
                    let obj = self.stack.pop().ok_or("stack empty")?;
                    let (has_next, next_val) = next_iterator(&obj, &mut self.heap)?;
                    self.stack.push(obj);
                    if has_next {
                        self.stack.push(next_val);
                        self.stack.push(Value::Bool(true));
                    } else {
                        self.stack.push(Value::Nil);
                        self.stack.push(Value::Bool(false));
                    }
                }
                OpCode::ForIter => {
                    let target = self.read_u16(self.ip) as usize;
                    self.ip += 2;
                    let obj = self.stack.pop().ok_or("stack empty")?;
                    let (has_next, next_val) = next_iterator(&obj, &mut self.heap)?;
                    if has_next {
                        self.stack.push(obj);
                        self.stack.push(next_val);
                    } else {
                        self.ip = target;
                    }
                }
                OpCode::MakeStruct => {
                    let count = self.read_u16(self.ip) as usize;
                    self.ip += 2;
                    let name_val = self.stack.pop().ok_or("stack empty")?;
                    let name = value_display(&name_val, &self.heap);
                    let mut methods = Vec::new();
                    for _ in 0..count {
                        let chunk_idx_val = self.stack.pop().ok_or("stack empty")?;
                        let mname_val = self.stack.pop().ok_or("stack empty")?;
                        let mname = value_display(&mname_val, &self.heap);
                        let chunk_idx = match chunk_idx_val { Value::Int(n) => n as usize, _ => return Err("invalid method chunk index".to_string()) };
                        methods.push((mname, chunk_idx));
                    }
                    self.stack.push(make_struct_def(&mut self.heap, &name, methods));
                }
                OpCode::NewStructInstance => {
                    let named_count = self.read_u16(self.ip) as usize;
                    self.ip += 2;
                    let struct_val = self.stack.pop().ok_or("stack empty")?;
                    let struct_ref = match struct_val { Value::Struct(r) => r, _ => return Err("expected struct".to_string()) };
                    let mut fields = Vec::new();
                    for _ in 0..named_count {
                        let val = self.stack.pop().ok_or("stack empty")?;
                        let key = self.stack.pop().ok_or("stack empty")?;
                        fields.push((key, val));
                    }
                    fields.reverse();
                    let instance = make_struct_instance(&mut self.heap, struct_ref, fields);
                    // Call init if it exists
                    if let GcObj::StructDef { ref methods, .. } = self.heap.get(struct_ref).clone() {
                        if let Some((_, init_chunk)) = methods.iter().find(|(n, _)| n == "init") {
                            let inst_clone = instance.clone();
                            let save_ip = self.ip;
                            let save_chunk = self.chunk_idx;
                            let mut ctx = VmContext {
                                heap: &mut self.heap,
                                globals: &mut self.globals,
                                modules: &mut Vec::new(),
                                chunks: &self.chunks,
                                try_frames: &mut self.try_frames,
                            };
                            execute_chunk(*init_chunk, &[inst_clone], &mut ctx)?;
                            self.ip = save_ip;
                            self.chunk_idx = save_chunk;
                        }
                    }
                    self.stack.push(instance);
                }
                OpCode::StructSetField => {
                    self.ip += 2;
                }
                OpCode::StructGetField => {
                    self.ip += 2;
                }
                OpCode::Len => {
                    self.ip += 2;
                    let obj = self.stack.pop().ok_or("stack empty")?;
                    let len = match &obj {
                        Value::String(r) => match self.heap.get(*r) { GcObj::String(s) => s.len() as i64, _ => 0 },
                        Value::List(r) => match self.heap.get(*r) { GcObj::List(items) => items.len() as i64, _ => 0 },
                        Value::Dict(r) => match self.heap.get(*r) { GcObj::Dict(entries) => entries.len() as i64, _ => 0 },
                        Value::Set(r) => match self.heap.get(*r) { GcObj::Set(items) => items.len() as i64, _ => 0 },
                        Value::Tuple(r) => match self.heap.get(*r) { GcObj::Tuple(items) => items.len() as i64, _ => 0 },
                        _ => return Err(format!("cannot get length of {}", obj.type_name())),
                    };
                    self.stack.push(Value::Int(len));
                }
                _ => {}
            }
        }
    }
}

fn add_values(a: &Value, b: &Value, heap: &mut GcHeap) -> Result<Value, String> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x + y)),
        (Value::Int(x), Value::Float(y)) => Ok(Value::Float(*x as f64 + y)),
        (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x + *y as f64)),
        (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x + y)),
        (Value::UInt(x), Value::UInt(y)) => Ok(Value::UInt(x + y)),
        (Value::String(x), Value::String(y)) => {
            let sx = match heap.get(*x) { GcObj::String(s) => s.clone(), _ => return Err("not a string".to_string()) };
            let sy = match heap.get(*y) { GcObj::String(s) => s.clone(), _ => return Err("not a string".to_string()) };
            Ok(make_string(heap, &format!("{}{}", sx, sy)))
        }
        _ => Err(format!("cannot add {} and {}", a.type_name(), b.type_name())),
    }
}

fn sub_values(a: &Value, b: &Value) -> Result<Value, String> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x - y)),
        (Value::Int(x), Value::Float(y)) => Ok(Value::Float(*x as f64 - y)),
        (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x - *y as f64)),
        (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x - y)),
        _ => Err(format!("cannot subtract {} and {}", a.type_name(), b.type_name())),
    }
}

fn mul_values(a: &Value, b: &Value) -> Result<Value, String> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x * y)),
        (Value::Int(x), Value::Float(y)) => Ok(Value::Float(*x as f64 * y)),
        (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x * *y as f64)),
        (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x * y)),
        _ => Err(format!("cannot multiply {} and {}", a.type_name(), b.type_name())),
    }
}

fn div_values(a: &Value, b: &Value) -> Result<Value, String> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => {
            if *y == 0 { return Err("division by zero".to_string()); }
            Ok(Value::Float(*x as f64 / *y as f64))
        }
        (Value::Float(x), Value::Float(y)) => {
            if *y == 0.0 { return Err("division by zero".to_string()); }
            Ok(Value::Float(x / y))
        }
        (Value::Int(x), Value::Float(y)) => {
            if *y == 0.0 { return Err("division by zero".to_string()); }
            Ok(Value::Float(*x as f64 / y))
        }
        (Value::Float(x), Value::Int(y)) => {
            if *y == 0 { return Err("division by zero".to_string()); }
            Ok(Value::Float(x / *y as f64))
        }
        _ => Err(format!("cannot divide {} and {}", a.type_name(), b.type_name())),
    }
}

fn mod_values(a: &Value, b: &Value) -> Result<Value, String> {
    match (a, b) { (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x % y)), _ => Err(format!("cannot mod {} and {}", a.type_name(), b.type_name())) }
}

fn pow_values(a: &Value, b: &Value) -> Result<Value, String> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Ok(Value::Float((*x as f64).powf(*y as f64))),
        (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x.powf(*y as f64))),
        (Value::Int(x), Value::Float(y)) => Ok(Value::Float((*x as f64).powf(*y))),
        (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x.powf(*y))),
        _ => Err(format!("cannot pow {} and {}", a.type_name(), b.type_name())),
    }
}

fn intdiv_values(a: &Value, b: &Value) -> Result<Value, String> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => {
            if *y == 0 { return Err("division by zero".to_string()); }
            Ok(Value::Int(x / y))
        }
        _ => Err(format!("cannot integer divide {} and {}", a.type_name(), b.type_name())),
    }
}

fn neg_value(a: &Value) -> Result<Value, String> {
    match a { Value::Int(x) => Ok(Value::Int(-x)), Value::Float(x) => Ok(Value::Float(-x)), _ => Err(format!("cannot negate {}", a.type_name())) }
}

fn cmp_lt(a: &Value, b: &Value) -> Result<bool, String> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Ok(x < y),
        (Value::Int(x), Value::Float(y)) => Ok((*x as f64) < *y),
        (Value::Float(x), Value::Int(y)) => Ok(*x < *y as f64),
        (Value::Float(x), Value::Float(y)) => Ok(x < y),
        _ => Err(format!("cannot compare {} and {}", a.type_name(), b.type_name())),
    }
}

fn value_display(val: &Value, heap: &GcHeap) -> String {
    match val { Value::String(r) => match heap.get(*r) { GcObj::String(s) => s.clone(), _ => "<string>".to_string() }, other => other.to_string(heap) }
}

fn contains_check(left: &Value, right: &Value, heap: &GcHeap) -> Result<bool, String> {
    match right {
        Value::List(r) => {
            let items = match heap.get(*r) { GcObj::List(items) => items, _ => return Err("not a list".to_string()) };
            Ok(items.iter().any(|x| x.eq(left, heap)))
        }
        Value::Dict(r) => {
            let entries = match heap.get(*r) { GcObj::Dict(entries) => entries, _ => return Err("not a dict".to_string()) };
            Ok(entries.iter().any(|(k, _)| k.eq(left, heap)))
        }
        Value::Set(r) => {
            let items = match heap.get(*r) { GcObj::Set(items) => items, _ => return Err("not a set".to_string()) };
            Ok(items.iter().any(|x| x.eq(left, heap)))
        }
        Value::String(r) => {
            let s = match heap.get(*r) { GcObj::String(s) => s, _ => return Err("not a string".to_string()) };
            let left_str = left.to_string(heap);
            Ok(s.contains(&left_str))
        }
        Value::Range(r) => {
            let range = match heap.get(*r) { GcObj::Range { start, end, step } => (*start, *end, *step), _ => return Err("not a range".to_string()) };
            let n = match left { Value::Int(n) => *n, _ => return Ok(false) };
            let (start, end, step) = range;
            if step > 0 {
                Ok(n >= start && n < end && (n - start) % step == 0)
            } else {
                Ok(n <= start && n > end && (start - n) % (-step) == 0)
            }
        }
        _ => Err(format!("cannot use 'in' with {}", right.type_name())),
    }
}

fn load_index(obj: &Value, index: &Value, heap: &mut GcHeap) -> Result<Value, String> {
    match obj {
        Value::List(r) => {
            let idx = match index { Value::Int(n) => *n, _ => return Err("list index must be an integer".to_string()) };
            let items = match heap.get(*r) { GcObj::List(items) => items, _ => return Err("not a list".to_string()) };
            let i = if idx < 0 { items.len() as i64 + idx } else { idx };
            if i < 0 || i >= items.len() as i64 { return Err("list index out of bounds".to_string()); }
            Ok(items[i as usize].clone())
        }
        Value::Dict(r) => {
            let entries = match heap.get(*r) { GcObj::Dict(entries) => entries, _ => return Err("not a dict".to_string()) };
            for (k, v) in entries { if k.eq(index, heap) { return Ok(v.clone()); } }
            Err("key not found".to_string())
        }
        Value::String(r) => {
            let idx = match index { Value::Int(n) => *n, _ => return Err("string index must be an integer".to_string()) };
            let s = match heap.get(*r) { GcObj::String(s) => s.clone(), _ => return Err("not a string".to_string()) };
            let chars: Vec<char> = s.chars().collect();
            let i = if idx < 0 { chars.len() as i64 + idx } else { idx };
            if i < 0 || i >= chars.len() as i64 { return Err("string index out of bounds".to_string()); }
            Ok(make_string(heap, &chars[i as usize].to_string()))
        }
        Value::Tuple(r) => {
            let idx = match index { Value::Int(n) => *n, _ => return Err("tuple index must be an integer".to_string()) };
            let items = match heap.get(*r) { GcObj::Tuple(items) => items, _ => return Err("not a tuple".to_string()) };
            let i = if idx < 0 { items.len() as i64 + idx } else { idx };
            if i < 0 || i >= items.len() as i64 { return Err("tuple index out of bounds".to_string()); }
            Ok(items[i as usize].clone())
        }
        _ => Err(format!("cannot index {}", obj.type_name())),
    }
}

fn store_index(obj: Value, index: Value, val: Value, heap: &mut GcHeap) -> Result<(), String> {
    match obj {
        Value::List(r) => {
            let idx = match &index { Value::Int(n) => *n, _ => return Err("list index must be an integer".to_string()) };
            let items = match heap.get_mut(r) { GcObj::List(ref mut items) => items, _ => return Err("not a list".to_string()) };
            let i = if idx < 0 { items.len() as i64 + idx } else { idx };
            if i < 0 || i >= items.len() as i64 { return Err("list index out of bounds".to_string()); }
            items[i as usize] = val;
            Ok(())
        }
        Value::Dict(r) => {
            let key_clone = index.clone();
            let mut indices: Vec<usize> = Vec::new();
            {
                let entries = match heap.get(r) { GcObj::Dict(entries) => entries, _ => return Err("not a dict".to_string()) };
                for (i, (k, _)) in entries.iter().enumerate() {
                    if k.eq(&key_clone, heap) { indices.push(i); }
                }
            }
            {
                let entries = match heap.get_mut(r) { GcObj::Dict(ref mut entries) => entries, _ => return Err("not a dict".to_string()) };
                for i in indices.into_iter().rev() { entries.remove(i); }
                entries.push((index, val));
            }
            Ok(())
        }
        _ => Err("cannot index into this type".to_string()),
    }
}

fn store_attr(obj: &Value, name: &str, val: Value, heap: &mut GcHeap) -> Result<(), String> {
    match obj {
        Value::StructInstance(r) => {
            let r = *r;
            let key = make_string(heap, name);
            let existing_idx = {
                if let GcObj::StructInstance { fields, .. } = heap.get(r) {
                    fields.iter().position(|(k, _)| k.eq(&key, heap))
                } else {
                    return Err("not a struct instance".to_string());
                }
            };
            if let GcObj::StructInstance { ref mut fields, .. } = heap.get_mut(r) {
                if let Some(idx) = existing_idx {
                    fields[idx].1 = val;
                } else {
                    fields.push((key, val));
                }
                Ok(())
            } else {
                Err("not a struct instance".to_string())
            }
        }
        _ => Err(format!("cannot set attribute '{}' on {}", name, obj.type_name())),
    }
}

fn load_attr(obj: &Value, name: &str, heap: &mut GcHeap) -> Result<Value, String> {
    match obj {
        Value::StructInstance(r) => {
            let r = *r;
            // check fields first
            let fields = match heap.get(r) {
                GcObj::StructInstance { ref fields, .. } => fields.clone(),
                _ => return Err("not a struct instance".to_string()),
            };
            let key = make_string(heap, name);
            for (k, v) in &fields {
                if k.eq(&key, heap) {
                    return Ok(v.clone());
                }
            }
            // check methods from struct definition
            let struct_ref = match heap.get(r) {
                GcObj::StructInstance { struct_ref, .. } => *struct_ref,
                _ => return Err("not a struct instance".to_string()),
            };
            let methods = match heap.get(struct_ref) {
                GcObj::StructDef { ref methods, .. } => methods.clone(),
                _ => return Err("not a struct def".to_string()),
            };
            if let Some((_, chunk_idx)) = methods.iter().find(|(n, _)| n == name) {
                let chunk_idx = *chunk_idx;
                let inst_val = Value::StructInstance(r);
                return Ok(Value::NativeFunc(NativeFunc {
                    name: format!("<{}.{}>", name, name),
                    func: std::rc::Rc::new(move |args, ctx| {
                        let mut all_args = vec![inst_val.clone()];
                        all_args.extend_from_slice(args);
                        execute_chunk(chunk_idx, &all_args, ctx)
                    }),
                }));
            }
            Err(format!("struct instance has no attribute '{}'", name))
        }
        Value::Struct(r) => {
            let r = *r;
            match heap.get(r) {
                GcObj::StructDef { ref name, .. } => {
                    Err(format!("cannot access attribute '{}' on struct {}", name, name))
                }
                _ => Err("not a struct".to_string()),
            }
        }
        Value::List(r) => {
            let r = *r;
            match name {
                "push" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<list.push>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.is_empty() { return Err("push requires an argument".to_string()); }
                        if let GcObj::List(ref mut items) = ctx.heap.get_mut(r) {
                            items.push(args[0].clone());
                        }
                        Ok(Value::Nil)
                    }),
                })),
                "pop" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<list.pop>".to_string(),
                    func: Rc::new(move |_, ctx| {
                        if let GcObj::List(ref mut items) = ctx.heap.get_mut(r) {
                            items.pop().ok_or("pop from empty list".to_string())
                        } else { Err("not a list".to_string()) }
                    }),
                })),
                "len" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<list.len>".to_string(),
                    func: Rc::new(move |_, ctx| {
                        if let GcObj::List(ref items) = ctx.heap.get(r) {
                            Ok(Value::Int(items.len() as i64))
                        } else { Err("not a list".to_string()) }
                    }),
                })),
                "clear" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<list.clear>".to_string(),
                    func: Rc::new(move |_, ctx| {
                        if let GcObj::List(ref mut items) = ctx.heap.get_mut(r) { items.clear(); }
                        Ok(Value::Nil)
                    }),
                })),
                "map" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<list.map>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.is_empty() { return Err("map requires a function".to_string()); }
                        let func = &args[0];
                        let items = if let GcObj::List(ref items) = ctx.heap.get(r) { items.clone() } else { return Err("not a list".to_string()); };
                        let mut result = Vec::new();
                        for item in &items { result.push(call_func_closure(func, &[item.clone()], ctx)?); }
                        Ok(make_list(ctx.heap, result))
                    }),
                })),
                "filter" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<list.filter>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.is_empty() { return Err("filter requires a function".to_string()); }
                        let func = &args[0];
                        let items = if let GcObj::List(ref items) = ctx.heap.get(r) { items.clone() } else { return Err("not a list".to_string()); };
                        let mut result = Vec::new();
                        for item in &items { if call_func_closure(func, &[item.clone()], ctx)?.is_truthy() { result.push(item.clone()); } }
                        Ok(make_list(ctx.heap, result))
                    }),
                })),
                "reduce" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<list.reduce>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.len() < 2 { return Err("reduce requires a function and initial value".to_string()); }
                        let func = &args[0];
                        let mut acc = args[1].clone();
                        let items = if let GcObj::List(ref items) = ctx.heap.get(r) { items.clone() } else { return Err("not a list".to_string()); };
                        for item in &items { acc = call_func_closure(func, &[acc, item.clone()], ctx)?; }
                        Ok(acc)
                    }),
                })),
                "sort" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<list.sort>".to_string(),
                    func: Rc::new(move |_, ctx| {
                        if let GcObj::List(ref mut items) = ctx.heap.get_mut(r) {
                            items.sort_by(|a, b| {
                                if cmp_lt(a, b).unwrap_or(false) { std::cmp::Ordering::Less }
                                else if cmp_lt(b, a).unwrap_or(false) { std::cmp::Ordering::Greater }
                                else { std::cmp::Ordering::Equal }
                            });
                        }
                        Ok(Value::Nil)
                    }),
                })),
                "insert" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<list.insert>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.len() < 2 { return Err("insert requires index and value".to_string()); }
                        let index = if let Value::Int(i) = args[0] { i as usize } else { return Err("insert requires integer index".to_string()); };
                        if let GcObj::List(ref mut items) = ctx.heap.get_mut(r) {
                            if index <= items.len() { items.insert(index, args[1].clone()); }
                        }
                        Ok(Value::Nil)
                    }),
                })),
                "remove" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<list.remove>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.is_empty() { return Err("remove requires an index".to_string()); }
                        let index = if let Value::Int(i) = args[0] { i as usize } else { return Err("remove requires integer index".to_string()); };
                        if let GcObj::List(ref mut items) = ctx.heap.get_mut(r) {
                            if index < items.len() { items.remove(index); }
                        }
                        Ok(Value::Nil)
                    }),
                })),
                "reverse" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<list.reverse>".to_string(),
                    func: Rc::new(move |_, ctx| {
                        if let GcObj::List(ref mut items) = ctx.heap.get_mut(r) { items.reverse(); }
                        Ok(Value::Nil)
                    }),
                })),
                "contains" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<list.contains>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.is_empty() { return Err("contains requires an argument".to_string()); }
                        let items = if let GcObj::List(ref items) = ctx.heap.get(r) { items.clone() } else { return Err("not a list".to_string()) };
                        Ok(Value::Bool(items.iter().any(|item| item.eq(&args[0], ctx.heap))))
                    }),
                })),
                "indexOf" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<list.indexOf>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.is_empty() { return Err("indexOf requires an argument".to_string()); }
                        let items = if let GcObj::List(ref items) = ctx.heap.get(r) { items.clone() } else { return Err("not a list".to_string()) };
                        let idx = items.iter().position(|item| item.eq(&args[0], ctx.heap));
                        Ok(match idx { Some(i) => Value::Int(i as i64), None => Value::Int(-1) })
                    }),
                })),
                "find" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<list.find>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.is_empty() { return Err("find requires a function".to_string()); }
                        let func = &args[0];
                        let items = if let GcObj::List(ref items) = ctx.heap.get(r) { items.clone() } else { return Err("not a list".to_string()) };
                        for item in &items {
                            if call_func_closure(func, &[item.clone()], ctx)?.is_truthy() {
                                return Ok(item.clone());
                            }
                        }
                        Ok(Value::Nil)
                    }),
                })),
                "any" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<list.any>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.is_empty() { return Err("any requires a function".to_string()); }
                        let func = &args[0];
                        let items = if let GcObj::List(ref items) = ctx.heap.get(r) { items.clone() } else { return Err("not a list".to_string()) };
                        for item in &items {
                            if call_func_closure(func, &[item.clone()], ctx)?.is_truthy() {
                                return Ok(Value::Bool(true));
                            }
                        }
                        Ok(Value::Bool(false))
                    }),
                })),
                "all" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<list.all>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.is_empty() { return Err("all requires a function".to_string()); }
                        let func = &args[0];
                        let items = if let GcObj::List(ref items) = ctx.heap.get(r) { items.clone() } else { return Err("not a list".to_string()) };
                        for item in &items {
                            if !call_func_closure(func, &[item.clone()], ctx)?.is_truthy() {
                                return Ok(Value::Bool(false));
                            }
                        }
                        Ok(Value::Bool(true))
                    }),
                })),
                "sum" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<list.sum>".to_string(),
                    func: Rc::new(move |_, ctx| {
                        let items = if let GcObj::List(ref items) = ctx.heap.get(r) { items.clone() } else { return Err("not a list".to_string()) };
                        let mut total = Value::Int(0);
                        for item in &items { total = add_values(&total, item, ctx.heap)?; }
                        Ok(total)
                    }),
                })),
                "foreach" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<list.foreach>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.is_empty() { return Err("foreach requires a function".to_string()); }
                        let func = &args[0];
                        let items = if let GcObj::List(ref items) = ctx.heap.get(r) { items.clone() } else { return Err("not a list".to_string()) };
                        for item in &items { call_func_closure(func, &[item.clone()], ctx)?; }
                        Ok(Value::Nil)
                    }),
                })),
                "slice" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<list.slice>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if let GcObj::List(ref items) = ctx.heap.get(r) {
                            let len = items.len();
                            let start = if args.len() > 0 {
                                if let Value::Int(i) = args[0] {
                                    if i < 0 { (len as i64 + i).max(0) as usize } else { (i as usize).min(len) }
                                } else { 0 }
                            } else { 0 };
                            let end = if args.len() > 1 {
                                if let Value::Int(i) = args[1] {
                                    if i < 0 { (len as i64 + i).max(0) as usize } else { (i as usize).min(len) }
                                } else { len }
                            } else { len };
                            Ok(make_list(ctx.heap, items[start..end].to_vec()))
                        } else { Err("not a list".to_string()) }
                    }),
                })),
                "join" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<list.join>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        let sep = if !args.is_empty() { value_display(&args[0], ctx.heap) } else { String::new() };
                        if let GcObj::List(ref items) = ctx.heap.get(r) {
                            let parts: Vec<String> = items.iter().map(|v| value_display(v, ctx.heap)).collect();
                            Ok(make_string(ctx.heap, &parts.join(&sep)))
                        } else { Err("not a list".to_string()) }
                    }),
                })),
                "unique" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<list.unique>".to_string(),
                    func: Rc::new(move |_, ctx| {
                        let items = if let GcObj::List(ref items) = ctx.heap.get(r) { items.clone() } else { return Err("not a list".to_string()) };
                        let mut result = Vec::new();
                        for item in &items {
                            if !result.iter().any(|s: &Value| s.eq(item, ctx.heap)) {
                                result.push(item.clone());
                            }
                        }
                        Ok(make_list(ctx.heap, result))
                    }),
                })),
                _ => Err(format!("list has no attribute '{}'", name)),
            }
        }
        Value::Dict(r) => {
            let r = *r;
            match name {
                "keys" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<dict.keys>".to_string(),
                    func: Rc::new(move |_, ctx| {
                        if let GcObj::Dict(ref entries) = ctx.heap.get(r) {
                            Ok(make_list(ctx.heap, entries.iter().map(|(k, _)| k.clone()).collect()))
                        } else { Err("not a dict".to_string()) }
                    }),
                })),
                "values" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<dict.values>".to_string(),
                    func: Rc::new(move |_, ctx| {
                        if let GcObj::Dict(ref entries) = ctx.heap.get(r) {
                            Ok(make_list(ctx.heap, entries.iter().map(|(_, v)| v.clone()).collect()))
                        } else { Err("not a dict".to_string()) }
                    }),
                })),
                "contains" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<dict.contains>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.is_empty() { return Err("contains requires an argument".to_string()); }
                        if let GcObj::Dict(ref entries) = ctx.heap.get(r) {
                            Ok(Value::Bool(entries.iter().any(|(k, _)| k.eq(&args[0], ctx.heap))))
                        } else { Err("not a dict".to_string()) }
                    }),
                })),
                _ => {
                    let key = make_string(heap, name);
                    if let GcObj::Dict(ref entries) = heap.get(r) {
                        for (k, v) in entries {
                            if k.eq(&key, heap) {
                                return Ok(v.clone());
                            }
                        }
                    }
                    Err(format!("dict has no attribute '{}'", name))
                }
            }
        }
        Value::Set(r) => {
            let r = *r;
            match name {
                "insert" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<set.insert>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.is_empty() { return Err("insert requires an argument".to_string()); }
                        let has = {
                            if let GcObj::Set(items) = ctx.heap.get(r) {
                                items.iter().any(|x| x.eq(&args[0], ctx.heap))
                            } else { false }
                        };
                        if !has {
                            if let GcObj::Set(ref mut items) = ctx.heap.get_mut(r) {
                                items.push(args[0].clone());
                            }
                        }
                        Ok(Value::Nil)
                    }),
                })),
                "remove" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<set.remove>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.is_empty() { return Err("remove requires an argument".to_string()); }
                        let idx = {
                            if let GcObj::Set(items) = ctx.heap.get(r) {
                                items.iter().position(|x| x.eq(&args[0], ctx.heap))
                            } else { None }
                        };
                        if let Some(i) = idx {
                            if let GcObj::Set(ref mut items) = ctx.heap.get_mut(r) {
                                items.remove(i);
                            }
                        }
                        Ok(Value::Nil)
                    }),
                })),
                "contains" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<set.contains>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.is_empty() { return Err("contains requires an argument".to_string()); }
                        if let GcObj::Set(ref items) = ctx.heap.get(r) {
                            Ok(Value::Bool(items.iter().any(|x| x.eq(&args[0], ctx.heap))))
                        } else { Err("not a set".to_string()) }
                    }),
                })),
                "union" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<set.union>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.is_empty() { return Err("union requires a set".to_string()); }
                        let other = &args[0];
                        let other_r = match other { Value::Set(r) => *r, _ => return Err("expected set".to_string()) };
                        let mut result_items = Vec::new();
                        if let GcObj::Set(ref items) = ctx.heap.get(r) { result_items = items.clone(); }
                        if let GcObj::Set(ref other_items) = ctx.heap.get(other_r) {
                            for item in other_items {
                                if !result_items.iter().any(|x| x.eq(item, ctx.heap)) { result_items.push(item.clone()); }
                            }
                        }
                        Ok(make_set(ctx.heap, result_items))
                    }),
                })),
                "intersection" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<set.intersection>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.is_empty() { return Err("intersection requires a set".to_string()); }
                        let other = &args[0];
                        let other_r = match other { Value::Set(r) => *r, _ => return Err("expected set".to_string()) };
                        let mut result_items = Vec::new();
                        if let GcObj::Set(ref items) = ctx.heap.get(r) {
                            if let GcObj::Set(ref other_items) = ctx.heap.get(other_r) {
                                for item in items {
                                    if other_items.iter().any(|x| x.eq(item, ctx.heap)) { result_items.push(item.clone()); }
                                }
                            }
                        }
                        Ok(make_set(ctx.heap, result_items))
                    }),
                })),
                "difference" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<set.difference>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.is_empty() { return Err("difference requires a set".to_string()); }
                        let other = &args[0];
                        let other_r = match other { Value::Set(r) => *r, _ => return Err("expected set".to_string()) };
                        let mut result_items = Vec::new();
                        if let GcObj::Set(ref items) = ctx.heap.get(r) {
                            if let GcObj::Set(ref other_items) = ctx.heap.get(other_r) {
                                for item in items {
                                    if !other_items.iter().any(|x| x.eq(item, ctx.heap)) { result_items.push(item.clone()); }
                                }
                            }
                        }
                        Ok(make_set(ctx.heap, result_items))
                    }),
                })),
                _ => Err(format!("set has no attribute '{}'", name)),
            }
        }
        Value::String(r) => {
            let r = *r;
            match name {
                "len" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<string.len>".to_string(),
                    func: Rc::new(move |_, ctx| {
                        if let GcObj::String(ref s) = ctx.heap.get(r) {
                            Ok(Value::Int(s.len() as i64))
                        } else { Err("not a string".to_string()) }
                    }),
                })),
                _ => Err(format!("string has no attribute '{}'", name)),
            }
        },
        Value::Range(r) => match heap.get(*r) {
            GcObj::Range { start, end, .. } => match name {
                "start" => Ok(Value::Int(*start)),
                "end" => Ok(Value::Int(*end)),
                _ => Err(format!("range has no attribute '{}'", name)),
            }
            _ => Err("not a range".to_string()),
        },
        Value::Matrix(r) => {
            let r = *r;
            match name {
                "shape" => {
                    let (nrows, ncols) = match heap.get(r) {
                        GcObj::Matrix { rows, cols, .. } => (*rows, *cols),
                        _ => return Err("not a matrix".to_string()),
                    };
                    let items = vec![Value::Int(nrows as i64), Value::Int(ncols as i64)];
                    Ok(make_tuple(heap, items))
                }
                "T" => {
                    let (nrows, ncols, data_clone) = match heap.get(r) {
                        GcObj::Matrix { rows, cols, data } => (*rows, *cols, data.clone()),
                        _ => return Err("not a matrix".to_string()),
                    };
                    let mut new_data = vec![0.0; nrows * ncols];
                    for i in 0..nrows {
                        for j in 0..ncols {
                            new_data[j * nrows + i] = data_clone[i * ncols + j];
                        }
                    }
                    Ok(make_matrix(heap, ncols, nrows, new_data))
                }
                "add" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<matrix.add>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.is_empty() { return Err("add requires a matrix argument".to_string()); }
                        let other = &args[0];
                        let other_r = match other { Value::Matrix(r) => *r, _ => return Err("expected matrix".to_string()) };
                        if let GcObj::Matrix { rows, cols, data } = ctx.heap.get(r) {
                            if let GcObj::Matrix { rows: or, cols: oc, data: od } = ctx.heap.get(other_r) {
                                if *rows != *or || *cols != *oc { return Err("matrix dimensions must match for addition".to_string()); }
                                let new_data: Vec<f64> = data.iter().zip(od.iter()).map(|(a, b)| a + b).collect();
                                Ok(make_matrix(ctx.heap, *rows, *cols, new_data))
                            } else { Err("expected matrix".to_string()) }
                        } else { Err("not a matrix".to_string()) }
                    }),
                })),
                "sub" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<matrix.sub>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.is_empty() { return Err("sub requires a matrix argument".to_string()); }
                        let other = &args[0];
                        let other_r = match other { Value::Matrix(r) => *r, _ => return Err("expected matrix".to_string()) };
                        if let GcObj::Matrix { rows, cols, data } = ctx.heap.get(r) {
                            if let GcObj::Matrix { rows: or, cols: oc, data: od } = ctx.heap.get(other_r) {
                                if *rows != *or || *cols != *oc { return Err("matrix dimensions must match for subtraction".to_string()); }
                                let new_data: Vec<f64> = data.iter().zip(od.iter()).map(|(a, b)| a - b).collect();
                                Ok(make_matrix(ctx.heap, *rows, *cols, new_data))
                            } else { Err("expected matrix".to_string()) }
                        } else { Err("not a matrix".to_string()) }
                    }),
                })),
                "mul" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<matrix.mul>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.is_empty() { return Err("mul requires a matrix argument".to_string()); }
                        let other = &args[0];
                        if let Value::Matrix(other_r) = other {
                            let other_r = *other_r;
                            if let GcObj::Matrix { rows, cols, data } = ctx.heap.get(r) {
                                if let GcObj::Matrix { rows: or, cols: oc, data: od } = ctx.heap.get(other_r) {
                                    if *cols != *or { return Err(format!("matrix dimensions incompatible for multiplication: ({}x{}) x ({}x{})", rows, cols, or, oc)); }
                                    let mut new_data = vec![0.0; rows * oc];
                                    for i in 0..*rows {
                                        for j in 0..*oc {
                                            let mut sum = 0.0;
                                            for k in 0..*cols {
                                                sum += data[i * cols + k] * od[k * oc + j];
                                            }
                                            new_data[i * oc + j] = sum;
                                        }
                                    }
                                    Ok(make_matrix(ctx.heap, *rows, *oc, new_data))
                                } else { Err("expected matrix".to_string()) }
                            } else { Err("not a matrix".to_string()) }
                        } else if let Value::Float(scalar) = other {
                            if let GcObj::Matrix { rows, cols, data } = ctx.heap.get(r) {
                                let new_data: Vec<f64> = data.iter().map(|x| x * scalar).collect();
                                Ok(make_matrix(ctx.heap, *rows, *cols, new_data))
                            } else { Err("not a matrix".to_string()) }
                        } else if let Value::Int(scalar) = other {
                            if let GcObj::Matrix { rows, cols, data } = ctx.heap.get(r) {
                                let s = *scalar as f64;
                                let new_data: Vec<f64> = data.iter().map(|x| x * s).collect();
                                Ok(make_matrix(ctx.heap, *rows, *cols, new_data))
                            } else { Err("not a matrix".to_string()) }
                        } else {
                            Err("mul expected a matrix or scalar".to_string())
                        }
                    }),
                })),
                "get" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<matrix.get>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.len() < 2 { return Err("get requires row and column".to_string()); }
                        let row = match &args[0] { Value::Int(n) => *n as usize, _ => return Err("row must be an integer".to_string()) };
                        let col = match &args[1] { Value::Int(n) => *n as usize, _ => return Err("col must be an integer".to_string()) };
                        if let GcObj::Matrix { rows, cols, data } = ctx.heap.get(r) {
                            if row >= *rows { return Err("row out of bounds".to_string()); }
                            if col >= *cols { return Err("col out of bounds".to_string()); }
                            Ok(Value::Float(data[row * cols + col]))
                        } else { Err("not a matrix".to_string()) }
                    }),
                })),
                "set" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<matrix.set>".to_string(),
                    func: Rc::new(move |args, ctx| {
                        if args.len() < 3 { return Err("set requires row, col, and value".to_string()); }
                        let row = match &args[0] { Value::Int(n) => *n as usize, _ => return Err("row must be an integer".to_string()) };
                        let col = match &args[1] { Value::Int(n) => *n as usize, _ => return Err("col must be an integer".to_string()) };
                        let val = match &args[2] { Value::Float(n) => *n, Value::Int(n) => *n as f64, _ => return Err("value must be a number".to_string()) };
                        if let GcObj::Matrix { rows, cols, ref mut data } = ctx.heap.get_mut(r) {
                            if row >= *rows { return Err("row out of bounds".to_string()); }
                            if col >= *cols { return Err("col out of bounds".to_string()); }
                            data[row * *cols + col] = val;
                            Ok(Value::Nil)
                        } else { Err("not a matrix".to_string()) }
                    }),
                })),
                "rows_list" => Ok(Value::NativeFunc(NativeFunc {
                    name: "<matrix.rows_list>".to_string(),
                    func: Rc::new(move |_, ctx| {
                        let (nrows, ncols, cloned_data) = match ctx.heap.get(r) {
                            GcObj::Matrix { rows, cols, data } => (*rows, *cols, data.clone()),
                            _ => return Err("not a matrix".to_string()),
                        };
                        let mut row_list = Vec::new();
                        for r in 0..nrows {
                            let mut row_data = Vec::new();
                            for c in 0..ncols {
                                row_data.push(Value::Float(cloned_data[r * ncols + c]));
                            }
                            row_list.push(make_list(ctx.heap, row_data));
                        }
                        Ok(make_list(ctx.heap, row_list))
                    }),
                })),
                _ => Err(format!("matrix has no attribute '{}'", name)),
            }
        }
        _ => Err(format!("cannot access attribute '{}' on {}", name, obj.type_name())),
    }
}

fn make_iterator(obj: &Value, heap: &mut GcHeap) -> Result<Value, String> {
    match obj {
        Value::List(r) => {
            let items = match heap.get(*r) { GcObj::List(items) => items.clone(), _ => return Err("not a list".to_string()) };
            let iter_list = make_list(heap, items);
            Ok(make_list(heap, vec![iter_list, Value::Int(0)]))
        }
        Value::Range(r) => {
            let (start, end, _step) = match heap.get(*r) {
                GcObj::Range { start, end, step } => (*start, *end, *step),
                _ => return Err("not a range".to_string()),
            };
            Ok(make_tuple(heap, vec![Value::Int(start), Value::Int(end), Value::Int(start)]))
        }
        Value::String(r) => {
            let s = match heap.get(*r) { GcObj::String(s) => s.clone(), _ => return Err("not a string".to_string()) };
            let chars: Vec<Value> = s.chars().map(|c| make_string(heap, &c.to_string())).collect();
            let char_list = make_list(heap, chars);
            Ok(make_list(heap, vec![char_list, Value::Int(0)]))
        }
        _ => Err(format!("cannot iterate {}", obj.type_name())),
    }
}

fn next_iterator(iter: &Value, heap: &mut GcHeap) -> Result<(bool, Value), String> {
    match iter {
        Value::List(ir) => {
            let items = match heap.get(*ir) { GcObj::List(items) => items.clone(), _ => return Err("invalid iterator".to_string()) };
            if items.len() < 2 { return Err("invalid iterator".to_string()); }
            let collection = &items[0];
            let idx = match &items[1] { Value::Int(n) => *n, _ => return Err("invalid iterator state".to_string()) };
            let coll_items = match collection {
                Value::List(r) => match heap.get(*r) { GcObj::List(items) => items.clone(), _ => return Err("invalid collection".to_string()) },
                _ => return Err("cannot iterate".to_string()),
            };
            if (idx as usize) < coll_items.len() {
                let val = coll_items[idx as usize].clone();
                if let GcObj::List(ref mut state) = heap.get_mut(*ir) { state[1] = Value::Int(idx + 1); }
                Ok((true, val))
            } else { Ok((false, Value::Nil)) }
        }
        Value::Tuple(tr) => {
            let items = match heap.get(*tr) { GcObj::Tuple(items) => items.clone(), _ => return Err("invalid range iterator".to_string()) };
            if items.len() < 3 { return Err("invalid range iterator".to_string()); }
            let start = match &items[0] { Value::Int(n) => *n, _ => 0 };
            let end = match &items[1] { Value::Int(n) => *n, _ => 0 };
            let current = match &items[2] { Value::Int(n) => *n, _ => start };
            if current < end {
                let val = Value::Int(current);
                if let GcObj::Tuple(ref mut t) = heap.get_mut(*tr) { t[2] = Value::Int(current + 1); }
                Ok((true, val))
            } else { Ok((false, Value::Nil)) }
        }
        _ => Err("cannot iterate".to_string()),
    }
}

pub fn call_func_closure(func: &Value, args: &[Value], ctx: &mut VmContext) -> Result<Value, String> {
    match func {
        Value::NativeFunc(f) => (f.func)(args, ctx),
        Value::Function(func_ref) => {
            let func_obj = ctx.heap.get(*func_ref).clone();
            if let GcObj::Function { body_chunk, .. } = func_obj {
                execute_chunk(body_chunk, args, ctx)
            } else {
                Err("not a function".to_string())
            }
        }
        Value::Closure(cl_ref) => {
            let closure = ctx.heap.get(*cl_ref).clone();
            if let GcObj::Closure { function, ref upvalues } = closure {
                let func_obj = ctx.heap.get(function).clone();
                if let GcObj::Function { body_chunk, .. } = func_obj {
                    execute_closure_chunk(body_chunk, args, upvalues.clone(), ctx)
                } else {
                    Err("not a function in closure".to_string())
                }
            } else {
                Err("not a closure".to_string())
            }
        }
        _ => Ok(args.first().cloned().unwrap_or(Value::Nil)),
    }
}

fn execute_chunk(chunk_idx: usize, args: &[Value], ctx: &mut VmContext) -> Result<Value, String> {
    let chunk = ctx.chunks.get(chunk_idx).ok_or("invalid chunk")?;
    let mut stack: Vec<Value> = Vec::new();
    for arg in args {
        stack.push(arg.clone());
    }
    let mut ip: usize = 0;
    loop {
        if ip >= chunk.code.len() {
            return Err("pc out of bounds".to_string());
        }
        let op = OpCode::from_u8(chunk.code[ip]).ok_or(format!("unknown opcode at {}", ip))?;
        ip += 1;
        match op {
            OpCode::Return => {
                let val = stack.pop().unwrap_or(Value::Nil);
                return Ok(val);
            }
            OpCode::Pop => { stack.pop(); }
            OpCode::Nil => stack.push(Value::Nil),
            OpCode::True => stack.push(Value::Bool(true)),
            OpCode::False => stack.push(Value::Bool(false)),
            OpCode::LoadConst => {
                let idx = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                stack.push(chunk.constants[idx].clone());
            }
            OpCode::LoadGlobal => {
                let sidx = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                let name = chunk.string_pool.get(sidx).ok_or("invalid global name index")?.clone();
                if let Some((_, val)) = ctx.globals.iter().find(|(n, _)| n == &name) {
                    stack.push(val.clone());
                } else {
                    return Err(format!("undefined global '{}'", name));
                }
            }
            OpCode::StoreGlobal => {
                let sidx = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                let name = chunk.string_pool.get(sidx).ok_or("invalid global name index")?.clone();
                let val = stack.pop().ok_or("stack empty")?;
                if let Some((_, entry)) = ctx.globals.iter_mut().find(|(n, _)| n == &name) {
                    *entry = val;
                } else {
                    ctx.globals.push((name, val));
                }
            }
            OpCode::LoadLocal => {
                let idx = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                stack.push(stack[idx].clone());
            }
            OpCode::StoreLocal => {
                let idx = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                let val = stack.pop().ok_or("stack empty")?;
                if idx < stack.len() { stack[idx] = val; }
                else { stack.resize(idx + 1, Value::Nil); stack[idx] = val; }
            }
            OpCode::Dup => {
                if let Some(v) = stack.last() {
                    stack.push(v.clone());
                }
            }
            OpCode::Jump => {
                let target = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                ip = target;
            }
            OpCode::JumpIfFalse => {
                let target = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                if let Some(val) = stack.last() {
                    if !val.is_truthy() { ip = target; }
                }
            }
            OpCode::JumpIfTrue => {
                let target = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                if let Some(val) = stack.last() {
                    if val.is_truthy() { ip = target; }
                }
            }
            OpCode::JumpIfNil => {
                let target = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                if let Some(val) = stack.last() {
                    if matches!(val, Value::Nil) { ip = target; }
                }
            }
            OpCode::Add => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(add_values(&a, &b, ctx.heap)?);
            }
            OpCode::Sub => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(sub_values(&a, &b)?);
            }
            OpCode::Mul => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(mul_values(&a, &b)?);
            }
            OpCode::Div => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(div_values(&a, &b)?);
            }
            OpCode::Mod => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(mod_values(&a, &b)?);
            }
            OpCode::Pow => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(pow_values(&a, &b)?);
            }
            OpCode::IntDiv => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(intdiv_values(&a, &b)?);
            }
            OpCode::Neg => {
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(neg_value(&a)?);
            }
            OpCode::Not => {
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(Value::Bool(!a.is_truthy()));
            }
            OpCode::Eq => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(Value::Bool(a.eq(&b, ctx.heap)));
            }
            OpCode::Ne => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(Value::Bool(!a.eq(&b, ctx.heap)));
            }
            OpCode::Lt => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(Value::Bool(cmp_lt(&a, &b)?));
            }
            OpCode::Gt => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(Value::Bool(cmp_lt(&b, &a)?));
            }
            OpCode::Le => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(Value::Bool(!cmp_lt(&b, &a)?));
            }
            OpCode::Ge => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(Value::Bool(!cmp_lt(&a, &b)?));
            }
            OpCode::And => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(if !a.is_truthy() { a } else { b });
            }
            OpCode::Or => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(if a.is_truthy() { a } else { b });
            }
            OpCode::Concat => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                let sa = value_display(&a, ctx.heap);
                let sb = value_display(&b, ctx.heap);
                stack.push(make_string(ctx.heap, &format!("{}{}", sa, sb)));
            }
            OpCode::In => {
                ip += 2;
                let right = stack.pop().ok_or("stack empty")?;
                let left = stack.pop().ok_or("stack empty")?;
                stack.push(Value::Bool(contains_check(&left, &right, ctx.heap)?));
            }
            OpCode::LoadAttr => {
                let sidx = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                let name = chunk.string_pool.get(sidx).ok_or("invalid attr name index")?.clone();
                let obj = stack.pop().ok_or("stack empty")?;
                stack.push(load_attr(&obj, &name, ctx.heap)?);
            }
            OpCode::StoreAttr => {
                let sidx = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                let name = chunk.string_pool.get(sidx).ok_or("invalid attr name index")?.clone();
                let obj = stack.pop().ok_or("stack empty")?;
                let val = stack.pop().ok_or("stack empty")?;
                store_attr(&obj, &name, val, ctx.heap)?;
            }
            OpCode::LoadUpvalue => {
                let idx = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                stack.push(Value::Nil);
            }
            OpCode::StoreUpvalue => {
                let idx = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                stack.pop();
            }
            OpCode::Call => {
                let argc = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                let args: Vec<Value> = if argc > 0 {
                    let start = stack.len() - argc;
                    stack.drain(start..).collect()
                } else { Vec::new() };
                let callee = stack.pop().ok_or("stack empty on call")?;
                let result = call_func_closure(&callee, &args, ctx);
                match result {
                    Ok(val) => stack.push(val),
                    Err(e) => {
                        if let Some((depth, catch_ip)) = ctx.try_frames.pop() {
                            stack.truncate(depth);
                            stack.push(make_error(ctx.heap, &e));
                            ip = catch_ip;
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
            OpCode::Halt => return Ok(Value::Nil),
            OpCode::Print => { if let Some(v) = stack.pop() { print!("{}", v.to_string(ctx.heap)); } }
            OpCode::PrintLn => { if let Some(v) = stack.pop() { println!("{}", v.to_string(ctx.heap)); } else { println!(); } }
            OpCode::BuildList => {
                let count = u16::from_le_bytes([chunk.code[ip], chunk.code[ip + 1]]) as usize;
                ip += 2;
                let start = stack.len().saturating_sub(count);
                let items: Vec<Value> = stack.drain(start..).collect();
                stack.push(make_list(ctx.heap, items));
            }
            OpCode::BuildDict => { ip += 2; stack.push(make_dict(ctx.heap, Vec::new())); }
            OpCode::BuildSet => { ip += 2; stack.push(make_set(ctx.heap, Vec::new())); }
            OpCode::BuildTuple => {
                let count = u16::from_le_bytes([chunk.code[ip], chunk.code[ip + 1]]) as usize;
                ip += 2;
                let start = stack.len().saturating_sub(count);
                let items: Vec<Value> = stack.drain(start..).collect();
                stack.push(make_tuple(ctx.heap, items));
            }
            OpCode::ListAppend => {
                ip += 2;
                let val = stack.pop().ok_or("stack empty")?;
                if let Some(Value::List(r)) = stack.pop() {
                    if let GcObj::List(ref mut items) = ctx.heap.get_mut(r) { items.push(val); }
                    stack.push(Value::List(r));
                }
            }
            OpCode::DictSet => {
                ip += 2;
                let val = stack.pop().ok_or("stack empty")?;
                let key = stack.pop().ok_or("stack empty")?;
                if let Some(Value::Dict(r)) = stack.pop() {
                    let key_clone = key.clone();
                    let mut to_remove = Vec::new();
                    {
                        let entries = match ctx.heap.get(r) { GcObj::Dict(e) => e, _ => return Err("not a dict".to_string()) };
                        for (i, (k, _)) in entries.iter().enumerate() { if k.eq(&key_clone, ctx.heap) { to_remove.push(i); } }
                    }
                    {
                        let entries = match ctx.heap.get_mut(r) { GcObj::Dict(ref mut e) => e, _ => return Err("not a dict".to_string()) };
                        for i in to_remove.into_iter().rev() { entries.remove(i); }
                        entries.push((key, val));
                    }
                    stack.push(Value::Dict(r));
                }
            }
            OpCode::SetAdd => {
                ip += 2;
                let val = stack.pop().ok_or("stack empty")?;
                if let Some(Value::Set(r)) = stack.pop() {
                    let val_c = val.clone();
                    let exists = { let items = match ctx.heap.get(r) { GcObj::Set(i) => i, _ => return Err("not a set".to_string()) }; items.iter().any(|x| x.eq(&val_c, ctx.heap)) };
                    if !exists { if let GcObj::Set(ref mut items) = ctx.heap.get_mut(r) { items.push(val); } }
                    stack.push(Value::Set(r));
                }
            }
            OpCode::LoadIndex => {
                ip += 2;
                let index = stack.pop().ok_or("stack empty")?;
                let obj = stack.pop().ok_or("stack empty")?;
                stack.push(load_index(&obj, &index, ctx.heap)?);
            }
            OpCode::StoreIndex => {
                ip += 2;
                let index = stack.pop().ok_or("stack empty")?;
                let obj = stack.pop().ok_or("stack empty")?;
                let val = stack.pop().ok_or("stack empty")?;
                store_index(obj, index, val, ctx.heap)?;
            }
            OpCode::Throw => {
                let v = stack.pop().ok_or("stack empty")?;
                let msg = v.to_string(ctx.heap);
                if let Some((depth, catch_ip)) = ctx.try_frames.pop() {
                    stack.truncate(depth);
                    stack.push(v);
                    ip = catch_ip;
                } else {
                    return Err(format!("Error: {}", msg));
                }
            }
            OpCode::CheckMatch => {
                ip += 2;
                let val = stack.pop().ok_or("stack empty")?;
                let pattern = stack.pop().ok_or("stack empty")?;
                let matched = val.eq(&pattern, ctx.heap) || matches!(&pattern, Value::String(r) if matches!(ctx.heap.get(*r), GcObj::String(s) if s == "_"));
                stack.push(Value::Bool(matched));
            }
            OpCode::BuildRange => {
                ip += 2;
                let step = stack.pop().ok_or("stack empty")?;
                let end = stack.pop().ok_or("stack empty")?;
                let start = stack.pop().ok_or("stack empty")?;
                let s = match start { Value::Int(n) => n, _ => return Err("range start must be int".to_string()) };
                let e = match end { Value::Int(n) => n, _ => return Err("range end must be int".to_string()) };
                let st = match step { Value::Int(n) => n, _ => return Err("range step must be int".to_string()) };
                stack.push(make_range(ctx.heap, s, e, st));
            }
            OpCode::MakeIter => {
                ip += 2;
                let obj = stack.pop().ok_or("stack empty")?;
                stack.push(make_iterator(&obj, ctx.heap)?);
            }
            OpCode::NextIter => {
                ip += 2;
                let obj = stack.pop().ok_or("stack empty")?;
                let (has_next, next_val) = next_iterator(&obj, ctx.heap)?;
                if has_next {
                    stack.push(obj);
                    stack.push(next_val);
                    stack.push(Value::Bool(true));
                } else {
                    stack.push(Value::Nil);
                    stack.push(Value::Bool(false));
                }
            }
            OpCode::MakeFunc => {
                let cidx = u16::from_le_bytes([chunk.code[ip], chunk.code[ip + 1]]) as usize;
                ip += 2;
                let func_ref = ctx.heap.alloc(GcObj::Function {
                    name: ctx.chunks[cidx].name.clone(), params: Vec::new(), is_vararg: false,
                    body_chunk: cidx, upvalue_count: 0,
                });
                stack.push(Value::Function(func_ref));
            }
            OpCode::CloseUpvalue => { ip += 2; }
            OpCode::Len => {
                ip += 2;
                let obj = stack.pop().ok_or("stack empty")?;
                let len = match &obj {
                    Value::String(r) => match ctx.heap.get(*r) { GcObj::String(s) => s.len() as i64, _ => 0 },
                    Value::List(r) => match ctx.heap.get(*r) { GcObj::List(items) => items.len() as i64, _ => 0 },
                    Value::Dict(r) => match ctx.heap.get(*r) { GcObj::Dict(entries) => entries.len() as i64, _ => 0 },
                    Value::Set(r) => match ctx.heap.get(*r) { GcObj::Set(items) => items.len() as i64, _ => 0 },
                    Value::Tuple(r) => match ctx.heap.get(*r) { GcObj::Tuple(items) => items.len() as i64, _ => 0 },
                    _ => return Err(format!("cannot get length of {}", obj.type_name())),
                };
                stack.push(Value::Int(len));
            }
            OpCode::PushScope => { ip += 2; }
            OpCode::PopScope => { ip += 2; }
            OpCode::ForPrep => { ip += 2; }
            OpCode::ForIter => {
                let target = u16::from_le_bytes([chunk.code[ip], chunk.code[ip + 1]]) as usize;
                ip += 2;
                let obj = stack.pop().ok_or("stack empty")?;
                let (has_next, next_val) = next_iterator(&obj, ctx.heap)?;
                if has_next { stack.push(obj); stack.push(next_val); }
                else { ip = target; }
            }
            OpCode::EnterLoop => { ip += 2; }
            OpCode::LeaveLoop => { ip += 2; }
            OpCode::Try => {
                let catch_ip = u16::from_le_bytes([chunk.code[ip], chunk.code[ip + 1]]) as usize;
                ip += 2;
                ctx.try_frames.push((stack.len(), catch_ip));
            }
            OpCode::EndTry => { ctx.try_frames.pop(); }
            OpCode::MakeStruct => { ip += 2; }
            OpCode::NewStructInstance => { ip += 2; }
            OpCode::StructSetField => { ip += 2; }
            OpCode::StructGetField => { ip += 2; }
            _ => {
                let count = op.operand_count();
                ip += count * 2;
            }
        }
    }
}

fn execute_closure_chunk(chunk_idx: usize, args: &[Value], upvalues: Vec<Upvalue>, ctx: &mut VmContext) -> Result<Value, String> {
    let chunk = ctx.chunks.get(chunk_idx).ok_or("invalid chunk")?;
    let mut stack: Vec<Value> = Vec::new();
    for arg in args {
        stack.push(arg.clone());
    }
    let mut ip: usize = 0;
    let mut upvalues = upvalues;
    loop {
        if ip >= chunk.code.len() {
            return Err("pc out of bounds".to_string());
        }
        let op = OpCode::from_u8(chunk.code[ip]).ok_or(format!("unknown opcode at {}", ip))?;
        ip += 1;
        match op {
            OpCode::Return => {
                let val = stack.pop().unwrap_or(Value::Nil);
                return Ok(val);
            }
            OpCode::Pop => { stack.pop(); }
            OpCode::Nil => stack.push(Value::Nil),
            OpCode::True => stack.push(Value::Bool(true)),
            OpCode::False => stack.push(Value::Bool(false)),
            OpCode::LoadConst => {
                let idx = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                stack.push(chunk.constants[idx].clone());
            }
            OpCode::LoadGlobal => {
                let sidx = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                let name = chunk.string_pool.get(sidx).ok_or("invalid global name index")?.clone();
                if let Some((_, val)) = ctx.globals.iter().find(|(n, _)| n == &name) {
                    stack.push(val.clone());
                } else {
                    return Err(format!("undefined global '{}'", name));
                }
            }
            OpCode::StoreGlobal => {
                let sidx = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                let name = chunk.string_pool.get(sidx).ok_or("invalid global name index")?.clone();
                let val = stack.pop().ok_or("stack empty")?;
                if let Some((_, entry)) = ctx.globals.iter_mut().find(|(n, _)| n == &name) {
                    *entry = val;
                } else {
                    ctx.globals.push((name, val));
                }
            }
            OpCode::LoadLocal => {
                let idx = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                stack.push(stack[idx].clone());
            }
            OpCode::StoreLocal => {
                let idx = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                let val = stack.pop().ok_or("stack empty")?;
                if idx < stack.len() { stack[idx] = val; }
                else { stack.resize(idx + 1, Value::Nil); stack[idx] = val; }
            }
            OpCode::LoadAttr => {
                let sidx = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                let name = chunk.string_pool.get(sidx).ok_or("invalid attr name index")?.clone();
                let obj = stack.pop().ok_or("stack empty")?;
                stack.push(load_attr(&obj, &name, ctx.heap)?);
            }
            OpCode::StoreAttr => {
                let sidx = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                let name = chunk.string_pool.get(sidx).ok_or("invalid attr name index")?.clone();
                let obj = stack.pop().ok_or("stack empty")?;
                let val = stack.pop().ok_or("stack empty")?;
                store_attr(&obj, &name, val, ctx.heap)?;
            }
            OpCode::Dup => {
                if let Some(v) = stack.last() {
                    stack.push(v.clone());
                }
            }
            OpCode::Jump => {
                let target = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                ip = target;
            }
            OpCode::JumpIfFalse => {
                let target = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                if let Some(val) = stack.last() {
                    if !val.is_truthy() { ip = target; }
                }
            }
            OpCode::JumpIfTrue => {
                let target = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                if let Some(val) = stack.last() {
                    if val.is_truthy() { ip = target; }
                }
            }
            OpCode::JumpIfNil => {
                let target = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                if let Some(val) = stack.last() {
                    if matches!(val, Value::Nil) { ip = target; }
                }
            }
            OpCode::LoadUpvalue => {
                let idx = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                let val = upvalues[idx].value.clone().unwrap_or(Value::Nil);
                stack.push(val);
            }
            OpCode::StoreUpvalue => {
                let idx = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                let val = stack.pop().ok_or("stack empty")?;
                upvalues[idx].value = Some(val);
            }
            OpCode::Add => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(add_values(&a, &b, ctx.heap)?);
            }
            OpCode::Sub => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(sub_values(&a, &b)?);
            }
            OpCode::Mul => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(mul_values(&a, &b)?);
            }
            OpCode::Div => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(div_values(&a, &b)?);
            }
            OpCode::Mod => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(mod_values(&a, &b)?);
            }
            OpCode::Pow => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(pow_values(&a, &b)?);
            }
            OpCode::IntDiv => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(intdiv_values(&a, &b)?);
            }
            OpCode::Neg => {
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(neg_value(&a)?);
            }
            OpCode::Not => {
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(Value::Bool(!a.is_truthy()));
            }
            OpCode::Eq => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(Value::Bool(a.eq(&b, ctx.heap)));
            }
            OpCode::Ne => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(Value::Bool(!a.eq(&b, ctx.heap)));
            }
            OpCode::Lt => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(Value::Bool(cmp_lt(&a, &b)?));
            }
            OpCode::Gt => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(Value::Bool(cmp_lt(&b, &a)?));
            }
            OpCode::Le => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(Value::Bool(!cmp_lt(&b, &a)?));
            }
            OpCode::Ge => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(Value::Bool(!cmp_lt(&a, &b)?));
            }
            OpCode::And => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(if !a.is_truthy() { a } else { b });
            }
            OpCode::Or => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                stack.push(if a.is_truthy() { a } else { b });
            }
            OpCode::Concat => {
                let b = stack.pop().ok_or("stack empty")?;
                let a = stack.pop().ok_or("stack empty")?;
                let sa = value_display(&a, ctx.heap);
                let sb = value_display(&b, ctx.heap);
                stack.push(make_string(ctx.heap, &format!("{}{}", sa, sb)));
            }
            OpCode::In => {
                ip += 2;
                let right = stack.pop().ok_or("stack empty")?;
                let left = stack.pop().ok_or("stack empty")?;
                stack.push(Value::Bool(contains_check(&left, &right, ctx.heap)?));
            }
            OpCode::Call => {
                let argc = u16::from_le_bytes([chunk.code[ip], chunk.code[ip+1]]) as usize;
                ip += 2;
                let args: Vec<Value> = if argc > 0 {
                    let start = stack.len() - argc;
                    stack.drain(start..).collect()
                } else { Vec::new() };
                let callee = stack.pop().ok_or("stack empty on call")?;
                let result = call_func_closure(&callee, &args, ctx);
                match result {
                    Ok(val) => stack.push(val),
                    Err(e) => {
                        if let Some((depth, catch_ip)) = ctx.try_frames.pop() {
                            stack.truncate(depth);
                            stack.push(make_error(ctx.heap, &e));
                            ip = catch_ip;
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
            OpCode::Halt => return Ok(Value::Nil),
            OpCode::NextIter => {
                ip += 2;
                let obj = stack.pop().ok_or("stack empty")?;
                let (has_next, next_val) = next_iterator(&obj, ctx.heap)?;
                stack.push(obj);
                if has_next {
                    stack.push(next_val);
                    stack.push(Value::Bool(true));
                } else {
                    stack.push(Value::Nil);
                    stack.push(Value::Bool(false));
                }
            }
            OpCode::ForIter => {
                let target = u16::from_le_bytes([chunk.code[ip], chunk.code[ip + 1]]) as usize;
                ip += 2;
                let obj = stack.pop().ok_or("stack empty")?;
                let (has_next, next_val) = next_iterator(&obj, ctx.heap)?;
                if has_next { stack.push(obj); stack.push(next_val); }
                else { ip = target; }
            }
            OpCode::Throw => {
                let v = stack.pop().ok_or("stack empty")?;
                let msg = v.to_string(ctx.heap);
                if let Some((depth, catch_ip)) = ctx.try_frames.pop() {
                    stack.truncate(depth);
                    stack.push(v);
                    ip = catch_ip;
                } else {
                    return Err(format!("Error: {}", msg));
                }
            }
            OpCode::Try => {
                let catch_ip = u16::from_le_bytes([chunk.code[ip], chunk.code[ip + 1]]) as usize;
                ip += 2;
                ctx.try_frames.push((stack.len(), catch_ip));
            }
            OpCode::EndTry => { ctx.try_frames.pop(); }
            _ => {
                let count = op.operand_count();
                ip += count * 2;
            }
        }
    }
}
