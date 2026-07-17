use std::collections::HashMap;

use crate::ast::*;
use crate::bytecode::*;
use crate::gc::Value;

pub struct Compiler {
    pub chunks: Vec<Chunk>,
    pub current_chunk: usize,
    pub scopes: Vec<Scope>,
    pub loop_stack: Vec<LoopInfo>,
    #[allow(dead_code)]
    pub strings: HashMap<String, u16>,
    pub current_func_depth: usize,
}

pub struct Scope {
    pub locals: Vec<LocalInfo>,
    pub depth: usize,
    pub func_depth: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Variable {
    Local(u16),
    Upvalue(u16),
    Global,
}

pub struct LocalInfo {
    pub name: String,
    #[allow(dead_code)]
    pub depth: usize,
    pub captured: bool,
}

pub struct LoopInfo {
    pub start: usize,
    #[allow(dead_code)]
    pub break_target: usize,
    pub end_targets: Vec<usize>,
}

impl Compiler {
    pub fn new() -> Self {
        let mut chunks = Vec::new();
        chunks.push(Chunk::new(Some("__main__".to_string())));
        Compiler {
            chunks,
            current_chunk: 0,
            scopes: vec![Scope {
                locals: Vec::new(),
                depth: 0,
                func_depth: 0,
            }],
            loop_stack: Vec::new(),
            strings: HashMap::new(),
            current_func_depth: 0,
        }
    }

    pub fn chunk(&mut self) -> &mut Chunk {
        let idx = self.current_chunk;
        &mut self.chunks[idx]
    }

    pub fn add_chunk(&mut self, name: Option<String>) -> usize {
        let idx = self.chunks.len();
        self.chunks.push(Chunk::new(name));
        idx
    }

    pub fn compile(&mut self, program: &Program) -> Result<(), String> {
        for stmt in &program.stmts {
            self.compile_stmt(stmt)?;
        }
        // Auto-call main() if defined
        let main_idx = self.add_global("main");
        self.chunk().emit(OpCode::LoadGlobal);
        self.chunk().emit_u16(main_idx);
        let jump_past = self.chunk().code.len();
        self.chunk().emit(OpCode::JumpIfNil);
        self.chunk().emit_u16(0);
        self.chunk().emit(OpCode::Call);
        self.chunk().emit_u16(0);
        self.chunk().emit(OpCode::Pop);
        let done = self.chunk().code.len() as u16;
        let patch = jump_past + 1;
        self.chunk().code[patch..patch + 2].copy_from_slice(&done.to_le_bytes());
        self.chunk().emit(OpCode::Nil);
        self.chunk().emit(OpCode::Return);
        // Peephole optimizations
        for chunk in &mut self.chunks {
            Self::peephole_optimize(chunk);
        }
        Ok(())
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(Scope {
            locals: Vec::new(),
            depth: self.scopes.last().map(|s| s.depth + 1).unwrap_or(0),
            func_depth: self.current_func_depth,
        });
    }

    pub fn leave_scope(&mut self) {
        let scope = self.scopes.pop().unwrap();
        let mut close_count = 0;
        for local in &scope.locals {
            if local.captured {
                self.chunk().emit(OpCode::CloseUpvalue);
                close_count += 1;
            }
        }
        // Pop non-captured locals from stack
        let pop_count = scope.locals.len() - close_count;
        if pop_count > 0 {
            self.chunk().emit(OpCode::PopScope);
            self.chunk().emit_u16(pop_count as u16);
        }
    }

    pub fn add_local(&mut self, name: &str) -> u16 {
        let depth = self.scopes.last().map(|s| s.depth).unwrap_or(0);
        let func_local_count: usize = self.scopes.iter()
            .filter(|s| s.func_depth == self.current_func_depth)
            .map(|s| s.locals.len())
            .sum();
        let idx = func_local_count;
        self.scopes.last_mut().unwrap().locals.push(LocalInfo {
            name: name.to_string(),
            depth,
            captured: false,
        });
        idx as u16
    }

    pub fn resolve_variable(&mut self, name: &str) -> Variable {
        let total_locals: usize = self.scopes.iter().map(|s| s.locals.len()).sum();
        let outer_locals: usize = self.scopes.iter()
            .filter(|s| s.func_depth < self.current_func_depth)
            .map(|s| s.locals.len())
            .sum();
        let mut cumulative = total_locals;
        for scope in self.scopes.iter().rev() {
            cumulative -= scope.locals.len();
            for (i, local) in scope.locals.iter().enumerate().rev() {
                if local.name == name {
                    if scope.func_depth == self.current_func_depth {
                        return Variable::Local(((cumulative - outer_locals) + i) as u16);
                    } else {
                        return self.add_upvalue(name, cumulative + i);
                    }
                }
            }
        }
        Variable::Global
    }

    pub fn add_upvalue(&mut self, name: &str, local_index: usize) -> Variable {
        let chunk = &self.chunks[self.current_chunk];
        let existing = chunk.upvalues.iter().position(|uv| uv.name == name);
        if let Some(idx) = existing {
            return Variable::Upvalue(idx as u16);
        }
        let idx = self.chunks[self.current_chunk].upvalues.len();
        self.chunks[self.current_chunk].upvalues.push(UpvalueInfo {
            name: name.to_string(),
            is_local: true,
            index: local_index,
        });
        for scope in self.scopes.iter_mut().rev() {
            for local in scope.locals.iter_mut().rev() {
                if local.name == name {
                    local.captured = true;
                    return Variable::Upvalue(idx as u16);
                }
            }
        }
        Variable::Upvalue(idx as u16)
    }

    pub fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Expr(expr) => {
                self.compile_expr(expr)?;
                self.chunk().emit(OpCode::Pop);
            }
            Stmt::Let { name, value, .. } => {
                self.compile_expr(value)?;
                let idx = self.add_local(name);
                self.chunk().emit(OpCode::StoreLocal);
                self.chunk().emit_u16(idx);
            }
            Stmt::Assign { target, value } => {
                match target {
                    Expr::Identifier(name) => {
                        self.compile_expr(value)?;
                        match self.resolve_variable(name) {
                            Variable::Local(idx) => {
                                self.chunk().emit(OpCode::StoreLocal);
                                self.chunk().emit_u16(idx);
                            }
                            Variable::Upvalue(idx) => {
                                self.chunk().emit(OpCode::StoreUpvalue);
                                self.chunk().emit_u16(idx);
                            }
                            Variable::Global => {
                                let gidx = self.add_global(name);
                                self.chunk().emit(OpCode::StoreGlobal);
                                self.chunk().emit_u16(gidx);
                            }
                        }
                    }
                    Expr::Index { obj, index } => {
                        self.compile_expr(value)?;
                        self.compile_expr(obj)?;
                        self.compile_expr(index)?;
                        self.chunk().emit(OpCode::StoreIndex);
                        self.chunk().emit_u16(0);
                    }
                    Expr::Attr { obj, name } => {
                        self.compile_expr(value)?;
                        self.compile_expr(obj)?;
                        self.chunk().emit(OpCode::StoreAttr);
                        let sidx = self.intern_string(name);
                        self.chunk().emit_u16(sidx);
                    }
                    _ => return Err("invalid assignment target".to_string()),
                }
            }
            Stmt::OpAssign { op, target, value } => {
                if self.try_emit_inc_dec(*op, target, value) {
                    return Ok(());
                }
                self.compile_expr(target)?;
                self.compile_expr(value)?;
                self.emit_binary_op(*op)?;
                match target {
                    Expr::Identifier(name) => {
                        match self.resolve_variable(name) {
                            Variable::Local(idx) => {
                                self.chunk().emit(OpCode::StoreLocal);
                                self.chunk().emit_u16(idx);
                            }
                            Variable::Upvalue(idx) => {
                                self.chunk().emit(OpCode::StoreUpvalue);
                                self.chunk().emit_u16(idx);
                            }
                            Variable::Global => {
                                let gidx = self.add_global(name);
                                self.chunk().emit(OpCode::StoreGlobal);
                                self.chunk().emit_u16(gidx);
                            }
                        }
                    }
                    _ => return Err("invalid op-assignment target".to_string()),
                }
            }
            Stmt::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
            } => {
                self.compile_expr(condition)?;
                let mut jump_false = self.chunk().code.len();
                self.chunk().emit(OpCode::JumpIfFalsePop);
                self.chunk().emit_u16(0);

                for stmt in then_branch {
                    self.compile_stmt(stmt)?;
                }

                let has_else = !elif_branches.is_empty() || else_branch.is_some();
                let mut jump_end = 0;
                if has_else {
                    let pos = self.chunk().code.len();
                    self.chunk().emit(OpCode::Jump);
                    self.chunk().emit_u16(0);
                    jump_end = pos;
                }

                let endif = self.chunk().code.len() as u16;
                let jf_pos = jump_false + 1;
                self.chunk().code[jf_pos..jf_pos + 2].copy_from_slice(&endif.to_le_bytes());

                for (cond, body) in elif_branches {
                    self.compile_expr(cond)?;
                    jump_false = self.chunk().code.len();
                    self.chunk().emit(OpCode::JumpIfFalsePop);
                    self.chunk().emit_u16(0);

                    for stmt in body {
                        self.compile_stmt(stmt)?;
                    }

                    if has_else {
                        let pos = self.chunk().code.len();
                        self.chunk().emit(OpCode::Jump);
                        self.chunk().emit_u16(0);
                        let prev_end = jump_end + 1;
                        self.chunk().code[prev_end..prev_end + 2]
                            .copy_from_slice(&(pos as u16).to_le_bytes());
                        jump_end = pos;
                    }

                    let endif = self.chunk().code.len() as u16;
                    let jf_pos = jump_false + 1;
                    self.chunk().code[jf_pos..jf_pos + 2]
                        .copy_from_slice(&endif.to_le_bytes());
                }

                if let Some(else_branch) = else_branch {
                    self.chunk().emit(OpCode::Pop);
                    for stmt in else_branch {
                        self.compile_stmt(stmt)?;
                    }
                }

                if has_else && jump_end > 0 {
                    let end_pos = jump_end + 1;
                    let pos = self.chunk().code.len() as u16;
                    self.chunk().code[end_pos..end_pos + 2]
                        .copy_from_slice(&pos.to_le_bytes());
                }
            }
            Stmt::While { condition, body } => {
                let loop_start = self.chunk().code.len();
                let loop_info = LoopInfo {
                    start: loop_start,
                    break_target: 0,
                    end_targets: Vec::new(),
                };
                self.loop_stack.push(loop_info);

                self.compile_expr(condition)?;
                let jump_false = self.chunk().code.len();
                self.chunk().emit(OpCode::JumpIfFalsePop);
                self.chunk().emit_u16(0);

                self.enter_scope();
                for stmt in body {
                    self.compile_stmt(stmt)?;
                }
                self.leave_scope();

                let start = self.loop_stack.last().unwrap().start as u16;
                self.chunk().emit(OpCode::Jump);
                self.chunk().emit_u16(start);

                let loop_end = self.chunk().code.len() as u16;

                let loop_info = self.loop_stack.pop().unwrap();
                for target in &loop_info.end_targets {
                    let pos = *target + 1;
                    self.chunk().code[pos..pos + 2]
                        .copy_from_slice(&loop_end.to_le_bytes());
                }

                let jf_pos = jump_false + 1;
                self.chunk().code[jf_pos..jf_pos + 2]
                    .copy_from_slice(&loop_end.to_le_bytes());
            }
            Stmt::For {
                variable,
                iterable,
                body,
            } => {
                // Specialize for-range loops: for i in start..end
                if let Expr::Range { start, end, step } = iterable {
                    let step_val = *step;
                    let var_idx = self.add_local(variable);
                    let end_idx = self.add_local("__end__");

                    // Emit start value
                    self.compile_expr(start)?;
                    self.chunk().emit(OpCode::StoreLocal);
                    self.chunk().emit_u16(var_idx);

                    // Emit end value (evaluated once)
                    self.compile_expr(end)?;
                    self.chunk().emit(OpCode::StoreLocal);
                    self.chunk().emit_u16(end_idx);

                    let loop_start = self.chunk().code.len();
                    let loop_info = LoopInfo {
                        start: loop_start,
                        break_target: 0,
                        end_targets: Vec::new(),
                    };
                    self.loop_stack.push(loop_info);

                    // SPECIALIZED: IntJumpIfNotLt/Gt replaces LoadLocal+LoadLocal+Lt/Gt+JumpIfFalsePop
                    let jump_exit;
                    if step_val > 0 {
                        self.chunk().emit(OpCode::IntJumpIfNotLt);
                        self.chunk().emit_u16(var_idx);
                        self.chunk().emit_u16(end_idx);
                        jump_exit = self.chunk().code.len();
                        self.chunk().emit_u16(0); // placeholder target
                    } else {
                        self.chunk().emit(OpCode::IntJumpIfNotGt);
                        self.chunk().emit_u16(var_idx);
                        self.chunk().emit_u16(end_idx);
                        jump_exit = self.chunk().code.len();
                        self.chunk().emit_u16(0); // placeholder target
                    }

                    self.enter_scope();
                    for stmt in body {
                        self.compile_stmt(stmt)?;
                    }
                    self.leave_scope();

                    // SPECIALIZED: IntInc/IntDec/IntAddLocal (no push — fixes stack leak)
                    match step_val {
                        1 => {
                            self.chunk().emit(OpCode::IntInc);
                            self.chunk().emit_u16(var_idx);
                        },
                        -1 => {
                            self.chunk().emit(OpCode::IntDec);
                            self.chunk().emit_u16(var_idx);
                        },
                        n => {
                            // IntAddLocal: local += immediate
                            self.chunk().emit(OpCode::IntAddLocal);
                            self.chunk().emit_u16(var_idx);
                            self.chunk().emit_i16(n as i16);
                        },
                    }

                    let loop_start_u16 = loop_start as u16;
                    self.chunk().emit(OpCode::Jump);
                    self.chunk().emit_u16(loop_start_u16);

                    self.loop_stack.pop().unwrap();
                    let done_pos = self.chunk().code.len() as u16;
                    self.chunk().code[jump_exit..jump_exit + 2]
                        .copy_from_slice(&done_pos.to_le_bytes());
                } else {
                    self.compile_expr(iterable)?;
                    let iter_idx = self.add_local("__iter__");
                    self.chunk().emit(OpCode::MakeIter);
                    self.chunk().emit_u16(0);
                    self.chunk().emit(OpCode::StoreLocal);
                    self.chunk().emit_u16(iter_idx);

                    let loop_start = self.chunk().code.len();
                    let loop_info = LoopInfo {
                        start: loop_start,
                        break_target: 0,
                        end_targets: Vec::new(),
                    };
                    self.loop_stack.push(loop_info);

                    self.chunk().emit(OpCode::LoadLocal);
                    self.chunk().emit_u16(iter_idx);
                    let for_iter_done = self.chunk().code.len();
                    self.chunk().emit(OpCode::ForIter);
                    self.chunk().emit_u16(0);

                    let var_idx = self.add_local(variable);
                    self.chunk().emit(OpCode::StoreLocal);
                    self.chunk().emit_u16(var_idx);

                    self.enter_scope();
                    for stmt in body {
                        self.compile_stmt(stmt)?;
                    }
                    self.leave_scope();

                    let start = self.loop_stack.last().unwrap().start as u16;
                    self.chunk().emit(OpCode::Jump);
                    self.chunk().emit_u16(start);

                    self.loop_stack.pop().unwrap();
                    let done_pos = self.chunk().code.len() as u16;
                    let fi_pos = for_iter_done + 1;
                    self.chunk().code[fi_pos..fi_pos + 2]
                        .copy_from_slice(&done_pos.to_le_bytes());
                }
            }
            Stmt::Match { value, arms } => {
                self.compile_expr(value)?;
                let match_val_idx = self.add_local("__match_val");
                self.chunk().emit(OpCode::StoreLocal);
                self.chunk().emit_u16(match_val_idx);

                let mut end_jumps = Vec::new();

                for (pattern, body) in arms {
                    self.chunk().emit(OpCode::LoadLocal);
                    self.chunk().emit_u16(match_val_idx);
                    self.compile_match_pattern(pattern.clone())?;
                    self.chunk().emit(OpCode::CheckMatch);
                    self.chunk().emit_u16(0);

                    let jump_next = self.chunk().code.len();
                    self.chunk().emit(OpCode::JumpIfFalse);
                    self.chunk().emit_u16(0);
                    self.chunk().emit(OpCode::Pop);

                    for stmt in body {
                        self.compile_stmt(stmt)?;
                    }

                    let end_jump = self.chunk().code.len();
                    self.chunk().emit(OpCode::Jump);
                    self.chunk().emit_u16(0);
                    end_jumps.push(end_jump);

                    let next_pos = self.chunk().code.len() as u16;
                    let jn_pos = jump_next + 1;
                    self.chunk().code[jn_pos..jn_pos + 2]
                        .copy_from_slice(&next_pos.to_le_bytes());
                    self.chunk().emit(OpCode::Pop);
                }

                let end_pos = self.chunk().code.len() as u16;
                for jump in &end_jumps {
                    let pos = *jump + 1;
                    self.chunk().code[pos..pos + 2]
                        .copy_from_slice(&end_pos.to_le_bytes());
                }
            }
            Stmt::Return(expr_opt) => {
                if let Some(expr) = expr_opt {
                    self.compile_expr(expr)?;
                } else {
                    self.chunk().emit(OpCode::Nil);
                }
                self.chunk().emit(OpCode::Return);
            }
            Stmt::FuncDef {
                name,
                params,
                is_vararg,
                body,
                ..
            } => {
                let func_chunk = self.add_chunk(Some(name.clone()));
                {
                    let chunk = &mut self.chunks[func_chunk];
                    chunk.params = (*params).clone();
                    chunk.is_vararg = *is_vararg;
                }
                let prev_chunk = self.current_chunk;
                self.current_chunk = func_chunk;
                let prev_depth = self.current_func_depth;
                self.current_func_depth += 1;

                self.enter_scope();
                for param in params.iter() {
                    self.add_local(param);
                }
                for stmt in body {
                    self.compile_stmt(stmt)?;
                }
                self.chunk().emit(OpCode::Nil);
                self.chunk().emit(OpCode::Return);
                self.leave_scope();

                self.current_func_depth = prev_depth;
                self.current_chunk = prev_chunk;

                let has_upvalues = self.chunks[func_chunk].upvalues.len() > 0;
                if has_upvalues {
                    self.chunk().emit(OpCode::MakeClosure);
                } else {
                    self.chunk().emit(OpCode::MakeFunc);
                }
                self.chunk().emit_u16(func_chunk as u16);
                if self.scopes.len() == 1 {
                    let gidx = self.add_global(name);
                    self.chunk().emit(OpCode::StoreGlobal);
                    self.chunk().emit_u16(gidx);
                } else {
                    let fname_idx = self.add_local(name);
                    self.chunk().emit(OpCode::StoreLocal);
                    self.chunk().emit_u16(fname_idx);
                }
            }
            Stmt::StructDef { name, methods } => {
                let _struct_name_idx = self.intern_string(name);
                // compile methods first, collect their chunks
                let mut method_infos: Vec<(String, usize)> = Vec::new();
                for method in methods {
                    if let Stmt::FuncDef { name: mname, params, is_vararg: _, body, .. } = method {
                        let chunk_idx = self.add_chunk(Some(format!("{}.{}", name, mname)));
                        let prev_chunk = self.current_chunk;
                        self.current_chunk = chunk_idx;
                        let prev_depth = self.current_func_depth;
                        self.current_func_depth += 1;
                        self.enter_scope();
                        for param in params { self.add_local(param); }
                        for stmt in body { self.compile_stmt(stmt)?; }
                        self.chunk().emit(OpCode::Nil);
                        self.chunk().emit(OpCode::Return);
                        self.leave_scope();
                        self.current_func_depth = prev_depth;
                        self.current_chunk = prev_chunk;
                        method_infos.push((mname.clone(), chunk_idx));
                    }
                }
                // Push method data onto stack first (method names + chunk indices)
                for (mname, chunk_idx) in &method_infos {
                    let mname_const = self.chunk().add_string_constant(Value::Nil, mname.clone());
                    self.chunk().emit(OpCode::LoadConst);
                    self.chunk().emit_u16(mname_const);
                    let cidx_const = self.chunk().add_constant(Value::Int(*chunk_idx as i64));
                    self.chunk().emit(OpCode::LoadConst);
                    self.chunk().emit_u16(cidx_const);
                }
                // Push struct name, then MakeStruct pops all
                let name_const = self.chunk().add_string_constant(Value::Nil, name.clone());
                self.chunk().emit(OpCode::LoadConst);
                self.chunk().emit_u16(name_const);
                self.chunk().emit(OpCode::MakeStruct);
                self.chunk().emit_u16(method_infos.len() as u16);
                // Store in globals
                let gidx = self.chunk().intern_string(name);
                self.chunk().emit(OpCode::StoreGlobal);
                self.chunk().emit_u16(gidx);
            }
            Stmt::Export { .. } => {}
            Stmt::Import { .. } => {
                self.chunk().emit(OpCode::Nil);
                self.chunk().emit(OpCode::Pop);
            }
            Stmt::Throw(expr) => {
                self.compile_expr(expr)?;
                self.chunk().emit(OpCode::Throw);
            }
            Stmt::Try {
                body,
                catch_var,
                catch_body,
            } => {
                let try_op_pos = self.chunk().code.len();
                self.chunk().emit(OpCode::Try);
                self.chunk().emit_u16(0);

                for stmt in body {
                    self.compile_stmt(stmt)?;
                }
                self.chunk().emit(OpCode::EndTry);
                let try_done_jump = self.chunk().code.len();
                self.chunk().emit(OpCode::Jump);
                self.chunk().emit_u16(0);

                let catch_pos = self.chunk().code.len() as u16;
                let patch_pos = try_op_pos + 1;
                self.chunk().code[patch_pos..patch_pos + 2]
                    .copy_from_slice(&catch_pos.to_le_bytes());

                let err_idx = self.add_local(catch_var);
                self.chunk().emit(OpCode::StoreLocal);
                self.chunk().emit_u16(err_idx);

                for stmt in catch_body {
                    self.compile_stmt(stmt)?;
                }

                let done_pos = self.chunk().code.len() as u16;
                let td_pos = try_done_jump + 1;
                self.chunk().code[td_pos..td_pos + 2]
                    .copy_from_slice(&done_pos.to_le_bytes());
            }
            Stmt::Block(stmts) => {
                for s in stmts {
                    self.compile_stmt(s)?;
                }
            }
            Stmt::Break => {
                let pos = self.chunk().code.len();
                self.chunk().emit(OpCode::Jump);
                self.chunk().emit_u16(0);
                if let Some(loop_info) = self.loop_stack.last_mut() {
                    loop_info.end_targets.push(pos);
                }
            }
            Stmt::Continue => {
                if let Some(loop_info) = self.loop_stack.last() {
                    let start = loop_info.start as u16;
                    self.chunk().emit(OpCode::Jump);
                    self.chunk().emit_u16(start);
                }
            }
        }
        Ok(())
    }

    pub fn compile_expr(&mut self, expr: &Expr) -> Result<(), String> {
        match expr {
            Expr::Int(n) => self.chunk().emit_const(Value::Int(*n)),
            Expr::UInt(n) => self.chunk().emit_const(Value::UInt(*n)),
            Expr::Float(n) => self.chunk().emit_const(Value::Float(*n)),
            Expr::String(s) => {
                let idx = self.chunk().add_string_constant(Value::Nil, s.clone());
                self.chunk().emit(OpCode::LoadConst);
                self.chunk().emit_u16(idx);
            }
            Expr::Bool(b) => {
                if *b { self.chunk().emit(OpCode::True); }
                else { self.chunk().emit(OpCode::False); }
            }
            Expr::Nil => self.chunk().emit(OpCode::Nil),
            Expr::Identifier(name) => {
                match self.resolve_variable(name) {
                    Variable::Local(idx) => {
                        self.chunk().emit(OpCode::LoadLocal);
                        self.chunk().emit_u16(idx);
                    }
                    Variable::Upvalue(idx) => {
                        self.chunk().emit(OpCode::LoadUpvalue);
                        self.chunk().emit_u16(idx);
                    }
                    Variable::Global => {
                        let gidx = self.add_global(name);
                        self.chunk().emit(OpCode::LoadGlobal);
                        self.chunk().emit_u16(gidx);
                    }
                }
            }
            Expr::List(items) => {
                for item in items {
                    self.compile_expr(item)?;
                }
                self.chunk().emit(OpCode::BuildList);
                self.chunk().emit_u16(items.len() as u16);
            }
            Expr::Dict(entries) => {
                self.chunk().emit(OpCode::BuildDict);
                self.chunk().emit_u16(0);
                for (key, val) in entries {
                    self.compile_expr(key)?;
                    self.compile_expr(val)?;
                    self.chunk().emit(OpCode::DictSet);
                    self.chunk().emit_u16(0);
                }
            }
            Expr::Set(items) => {
                self.chunk().emit(OpCode::BuildSet);
                self.chunk().emit_u16(0);
                for item in items {
                    self.compile_expr(item)?;
                    self.chunk().emit(OpCode::SetAdd);
                    self.chunk().emit_u16(0);
                }
            }
            Expr::Tuple(items) => {
                for item in items {
                    self.compile_expr(item)?;
                }
                self.chunk().emit(OpCode::BuildTuple);
                self.chunk().emit_u16(items.len() as u16);
            }
            Expr::BinaryOp { op, left, right } => {
                if self.try_constant_fold(*op, left, right) {
                    return Ok(());
                }
                self.compile_expr(left)?;
                self.compile_expr(right)?;
                self.emit_binary_op(*op)?;
            }
            Expr::UnaryOp { op, operand } => {
                self.compile_expr(operand)?;
                match op {
                    UnaryOpKind::Neg => self.chunk().emit(OpCode::Neg),
                    UnaryOpKind::Not => self.chunk().emit(OpCode::Not),
                }
            }
            Expr::Assign { target, value } => {
                self.compile_expr(value)?;
                self.chunk().emit(OpCode::Dup);
                match target.as_ref() {
                    Expr::Identifier(name) => {
                        match self.resolve_variable(name) {
                            Variable::Local(idx) => {
                                self.chunk().emit(OpCode::StoreLocal);
                                self.chunk().emit_u16(idx);
                            }
                            Variable::Upvalue(idx) => {
                                self.chunk().emit(OpCode::StoreUpvalue);
                                self.chunk().emit_u16(idx);
                            }
                            Variable::Global => {
                                let gidx = self.add_global(name);
                                self.chunk().emit(OpCode::StoreGlobal);
                                self.chunk().emit_u16(gidx);
                            }
                        }
                    }
                    Expr::Attr { obj, name } => {
                        // Stack: [value, value_dup] from earlier
                        self.compile_expr(obj)?;
                        let sidx = self.intern_string(name);
                        self.chunk().emit(OpCode::StoreAttr);
                        self.chunk().emit_u16(sidx);
                    }
                    Expr::Index { obj, index } => {
                        self.compile_expr(obj)?;
                        self.compile_expr(index)?;
                        self.chunk().emit(OpCode::StoreIndex);
                        self.chunk().emit_u16(0);
                    }
                    _ => {}
                }
            }
            Expr::OpAssign { op, target, value } => {
                if self.try_emit_inc_dec(*op, target, value) {
                    return Ok(());
                }
                self.compile_expr(target)?;
                self.compile_expr(value)?;
                self.emit_binary_op(*op)?;
                self.chunk().emit(OpCode::Dup);
                match target.as_ref() {
                    Expr::Identifier(name) => {
                        match self.resolve_variable(name) {
                            Variable::Local(idx) => {
                                self.chunk().emit(OpCode::StoreLocal);
                                self.chunk().emit_u16(idx);
                            }
                            Variable::Upvalue(idx) => {
                                self.chunk().emit(OpCode::StoreUpvalue);
                                self.chunk().emit_u16(idx);
                            }
                            Variable::Global => {
                                let gidx = self.add_global(name);
                                self.chunk().emit(OpCode::StoreGlobal);
                                self.chunk().emit_u16(gidx);
                            }
                        }
                    }
                    Expr::Attr { obj, name } => {
                        self.compile_expr(obj)?;
                        let sidx = self.intern_string(name);
                        self.chunk().emit(OpCode::StoreAttr);
                        self.chunk().emit_u16(sidx);
                    }
                    Expr::Index { obj, index } => {
                        self.compile_expr(obj)?;
                        self.compile_expr(index)?;
                        self.chunk().emit(OpCode::StoreIndex);
                        self.chunk().emit_u16(0);
                    }
                    _ => {}
                }
            }
            Expr::Call { callee, args, is_method } => {
                if *is_method {
                    self.compile_expr(callee)?;
                } else {
                    self.compile_expr(callee)?;
                }
                let mut stack_args = 0;
                for arg in args {
                    self.compile_expr(arg)?;
                    stack_args += match arg {
                        Expr::NamedArg { .. } => 2,
                        _ => 1,
                    };
                }
                self.chunk().emit(OpCode::Call);
                self.chunk().emit_u16(stack_args);
            }
            Expr::Index { obj, index } => {
                self.compile_expr(obj)?;
                self.compile_expr(index)?;
                self.chunk().emit(OpCode::LoadIndex);
                self.chunk().emit_u16(0);
            }
            Expr::Attr { obj, name } => {
                self.compile_expr(obj)?;
                let sidx = self.intern_string(name);
                self.chunk().emit(OpCode::LoadAttr);
                self.chunk().emit_u16(sidx);
            }
            Expr::Func {
                name,
                params,
                is_vararg,
                body,
            } => {
                let func_chunk = self.add_chunk(name.clone());
                {
                    let chunk = &mut self.chunks[func_chunk];
                    chunk.params = (*params).clone();
                    chunk.is_vararg = *is_vararg;
                }
                let prev_chunk = self.current_chunk;
                self.current_chunk = func_chunk;
                let prev_depth = self.current_func_depth;
                self.current_func_depth += 1;

                self.enter_scope();
                for param in params.iter() {
                    self.add_local(param);
                }
                for stmt in body {
                    self.compile_stmt(stmt)?;
                }
                self.chunk().emit(OpCode::Nil);
                self.chunk().emit(OpCode::Return);
                self.leave_scope();

                self.current_func_depth = prev_depth;
                self.current_chunk = prev_chunk;
                let has_upvalues = self.chunks[func_chunk].upvalues.len() > 0;
                if has_upvalues {
                    self.chunk().emit(OpCode::MakeClosure);
                } else {
                    self.chunk().emit(OpCode::MakeFunc);
                }
                self.chunk().emit_u16(func_chunk as u16);
            }
            Expr::Lambda { params, body } => {
                let func_chunk = self.add_chunk(None);
                let prev_chunk = self.current_chunk;
                self.current_chunk = func_chunk;
                let prev_depth = self.current_func_depth;
                self.current_func_depth += 1;

                self.enter_scope();
                for param in params {
                    self.add_local(param);
                }
                self.compile_expr(body)?;
                self.chunk().emit(OpCode::Return);
                self.leave_scope();

                self.current_func_depth = prev_depth;
                self.current_chunk = prev_chunk;
                let has_upvalues = self.chunks[func_chunk].upvalues.len() > 0;
                if has_upvalues {
                    self.chunk().emit(OpCode::MakeClosure);
                } else {
                    self.chunk().emit(OpCode::MakeFunc);
                }
                self.chunk().emit_u16(func_chunk as u16);
            }
            Expr::Range { start, end, step } => {
                self.compile_expr(start)?;
                self.compile_expr(end)?;
                self.chunk().emit_const(Value::Int(*step));
                self.chunk().emit(OpCode::BuildRange);
                self.chunk().emit_u16(0);
            }
            Expr::Ternary { condition, then_expr, else_expr } => {
                self.compile_expr(condition)?;
                self.chunk().emit(OpCode::JumpIfFalsePop);
                let else_jump = self.chunk().code.len();
                self.chunk().emit_u16(0);
                self.compile_expr(then_expr)?;
                self.chunk().emit(OpCode::Jump);
                let end_jump = self.chunk().code.len();
                self.chunk().emit_u16(0);
                let else_pos = self.chunk().code.len() as u16;
                self.chunk().code[else_jump..else_jump+2].copy_from_slice(&else_pos.to_le_bytes());
                self.compile_expr(else_expr)?;
                let end_pos = self.chunk().code.len() as u16;
                self.chunk().code[end_jump..end_jump+2].copy_from_slice(&end_pos.to_le_bytes());
            }
            Expr::FString(parts) => {
                // Build result by concatenating parts
                for (i, part) in parts.iter().enumerate() {
                    match part {
                        crate::ast::FStringPart::Literal(s) => {
                            let idx = self.chunk().add_string_constant(Value::Nil, s.clone());
                            self.chunk().emit(OpCode::LoadConst);
                            self.chunk().emit_u16(idx);
                        }
                        crate::ast::FStringPart::Expr(e) => {
                            self.compile_expr(e)?;
                            // Ensure non-string values are converted to string via Concat with ""
                        }
                    }
                    if i > 0 {
                        self.chunk().emit(OpCode::Concat);
                    }
                }
                if parts.is_empty() {
                    self.chunk().emit(OpCode::Nil);
                }
            }
            Expr::InterpolatedString(parts) => {
                for part in parts {
                    self.compile_expr(part)?;
                }
            }
            Expr::NamedArg { name, value } => {
                let name_const = self.chunk().add_string_constant(Value::Nil, name.clone());
                self.chunk().emit(OpCode::LoadConst);
                self.chunk().emit_u16(name_const);
                self.compile_expr(value)?;
            }
            Expr::MatchExpr { value, arms } => {
                self.compile_expr(value)?;

                let tmp_name = "__match_tmp__";
                let tmp_idx = self.chunk().intern_string(tmp_name);
                self.chunk().emit(OpCode::StoreGlobal);
                self.chunk().emit_u16(tmp_idx);

                let mut end_jumps = Vec::new();
                let num_arms = arms.len();

                for (_i, (pattern, body)) in arms.iter().enumerate() {
                    self.chunk().emit(OpCode::LoadGlobal);
                    self.chunk().emit_u16(tmp_idx);
                    self.compile_match_pattern(pattern.clone())?;
                    self.chunk().emit(OpCode::CheckMatch);
                    self.chunk().emit_u16(0);

                    let jump_next = self.chunk().code.len();
                    self.chunk().emit(OpCode::JumpIfFalse);
                    self.chunk().emit_u16(0);
                    self.chunk().emit(OpCode::Pop);

                    let len = body.len();
                    for (j, stmt) in body.iter().enumerate() {
                        if j == len - 1 {
                            self.compile_expr_body(stmt)?;
                        } else {
                            self.compile_stmt(stmt)?;
                        }
                    }

                    let end_jump = self.chunk().code.len();
                    self.chunk().emit(OpCode::Jump);
                    self.chunk().emit_u16(0);
                    end_jumps.push(end_jump);

                    let next_pos = self.chunk().code.len() as u16;
                    let jn_pos = jump_next + 1;
                    self.chunk().code[jn_pos..jn_pos + 2]
                        .copy_from_slice(&next_pos.to_le_bytes());
                    self.chunk().emit(OpCode::Pop);
                }

                // If no arm matched, push the match value itself as result
                self.chunk().emit(OpCode::LoadGlobal);
                self.chunk().emit_u16(tmp_idx);

                let end_pos = self.chunk().code.len() as u16;
                for jump in &end_jumps {
                    let pos = *jump + 1;
                    self.chunk().code[pos..pos + 2]
                        .copy_from_slice(&end_pos.to_le_bytes());
                }
            }
        }
        Ok(())
    }

    fn compile_match_pattern(&mut self, pattern: Expr) -> Result<(), String> {
        match &pattern {
            Expr::Identifier(name) if name == "_" => {
                let idx = self.chunk().add_string_constant(Value::Nil, "_".to_string());
                self.chunk().emit(OpCode::LoadConst);
                self.chunk().emit_u16(idx);
            }
            _ => self.compile_expr(&pattern)?,
        }
        Ok(())
    }

    fn compile_expr_body(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Expr(expr) => self.compile_expr(expr)?,
            _ => {
                self.compile_stmt(stmt)?;
                self.chunk().emit(OpCode::Nil);
            }
        }
        Ok(())
    }

    fn emit_binary_op(&mut self, op: BinaryOpKind) -> Result<(), String> {
        match op {
            BinaryOpKind::Add => self.chunk().emit(OpCode::Add),
            BinaryOpKind::Sub => self.chunk().emit(OpCode::Sub),
            BinaryOpKind::Mul => self.chunk().emit(OpCode::Mul),
            BinaryOpKind::Div => self.chunk().emit(OpCode::Div),
            BinaryOpKind::Mod => self.chunk().emit(OpCode::Mod),
            BinaryOpKind::Pow => self.chunk().emit(OpCode::Pow),
            BinaryOpKind::IntDiv => self.chunk().emit(OpCode::IntDiv),
            BinaryOpKind::Eq => self.chunk().emit(OpCode::Eq),
            BinaryOpKind::Ne => self.chunk().emit(OpCode::Ne),
            BinaryOpKind::Lt => self.chunk().emit(OpCode::Lt),
            BinaryOpKind::Gt => self.chunk().emit(OpCode::Gt),
            BinaryOpKind::Le => self.chunk().emit(OpCode::Le),
            BinaryOpKind::Ge => self.chunk().emit(OpCode::Ge),
            BinaryOpKind::And => self.chunk().emit(OpCode::And),
            BinaryOpKind::Or => self.chunk().emit(OpCode::Or),
            BinaryOpKind::Concat => self.chunk().emit(OpCode::Concat),
            BinaryOpKind::In => self.chunk().emit(OpCode::In),
        }
        Ok(())
    }

    fn try_constant_fold(&mut self, op: BinaryOpKind, left: &Expr, right: &Expr) -> bool {
        use BinaryOpKind::*;
        // String concatenation: "hello" + " world" → folded to single LoadConst
        if let (Expr::String(a), Expr::String(b)) = (left, right) {
            if matches!(op, Add) {
                let combined = format!("{}{}", a, b);
                let idx = self.chunk().add_string_constant(Value::Nil, combined);
                self.chunk().emit(OpCode::LoadConst);
                self.chunk().emit_u16(idx);
                return true;
            }
        }
        let l = match left {
            Expr::Int(n) => Some(Value::Int(*n)),
            Expr::UInt(n) => Some(Value::UInt(*n)),
            Expr::Float(n) => Some(Value::Float(*n)),
            Expr::Bool(b) => Some(Value::Bool(*b)),
            Expr::Nil => Some(Value::Nil),
            _ => None,
        };
        let r = match right {
            Expr::Int(n) => Some(Value::Int(*n)),
            Expr::UInt(n) => Some(Value::UInt(*n)),
            Expr::Float(n) => Some(Value::Float(*n)),
            Expr::Bool(b) => Some(Value::Bool(*b)),
            Expr::Nil => Some(Value::Nil),
            _ => None,
        };
        let (Some(lv), Some(rv)) = (l, r) else { return false; };
        // Can't fold comparisons on heap types - eq() would crash on empty heap
        let is_heap_type = |v: &Value| matches!(v, Value::String(_) | Value::List(_) | Value::Dict(_) | Value::Set(_) | Value::Tuple(_) | Value::Function(_) | Value::Closure(_) | Value::Error(_) | Value::Range(_) | Value::Matrix(_) | Value::Struct(_) | Value::StructInstance(_) | Value::Image(_));
        if is_heap_type(&lv) || is_heap_type(&rv) { return false; }
        let result = match (op, &lv, &rv) {
            (Add, Value::Int(a), Value::Int(b)) => Some(Value::Int(a + b)),
            (Add, Value::Float(a), Value::Float(b)) => Some(Value::Float(a + b)),
            (Add, Value::Int(a), Value::Float(b)) => Some(Value::Float(*a as f64 + b)),
            (Add, Value::Float(a), Value::Int(b)) => Some(Value::Float(a + *b as f64)),
            (Sub, Value::Int(a), Value::Int(b)) => Some(Value::Int(a - b)),
            (Sub, Value::Float(a), Value::Float(b)) => Some(Value::Float(a - b)),
            (Mul, Value::Int(a), Value::Int(b)) => Some(Value::Int(a * b)),
            (Mul, Value::Float(a), Value::Float(b)) => Some(Value::Float(a * b)),
            (Div, Value::Int(a), Value::Int(b)) if *b != 0 => Some(Value::Float(*a as f64 / *b as f64)),
            (Div, Value::Float(a), Value::Float(b)) if *b != 0.0 => Some(Value::Float(a / b)),
            (Mod, Value::Int(a), Value::Int(b)) if *b != 0 => Some(Value::Int(a % b)),
            (IntDiv, Value::Int(a), Value::Int(b)) if *b != 0 => Some(Value::Int(a / b)),
            (Eq, _, _) => Some(Value::Bool(lv.eq(&rv, &crate::gc::GcHeap::new()))),
            (Ne, _, _) => Some(Value::Bool(!lv.eq(&rv, &crate::gc::GcHeap::new()))),
            _ => None,
        };
        match result {
            Some(val) => { self.chunk().emit_const(val); true }
            None => false,
        }
    }

    fn try_emit_inc_dec(&mut self, op: BinaryOpKind, target: &Expr, value: &Expr) -> bool {
        if !matches!(op, BinaryOpKind::Add | BinaryOpKind::Sub) { return false; }
        let is_sub = matches!(op, BinaryOpKind::Sub);
        let one = match value {
            Expr::Int(1) => true,
            Expr::UInt(1) => true,
            _ => false,
        };
        if !one { return false; }
        match target {
            Expr::Identifier(name) => {
                match self.resolve_variable(name) {
                    Variable::Local(idx) => {
                        if is_sub {
                            self.chunk().emit(OpCode::Dec);
                        } else {
                            self.chunk().emit(OpCode::Inc);
                        }
                        self.chunk().emit_u16(idx);
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn add_global(&mut self, name: &str) -> u16 {
        self.chunk().intern_string(name)
    }

    fn intern_string(&mut self, s: &str) -> u16 {
        self.chunk().intern_string(s)
    }

    fn peephole_optimize(chunk: &mut Chunk) {
        fn insn_size_at(code: &[u8], offset: usize) -> usize {
            if offset >= code.len() { return 1; }
            let op = OpCode::from_u8(code[offset]).unwrap_or(OpCode::Halt);
            1 + op.operand_count() * 2
        }
        fn is_jump_with_target(op: u8) -> bool {
            matches!(OpCode::from_u8(op), Some(
                OpCode::Jump | OpCode::JumpIfTrue | OpCode::JumpIfFalse
                | OpCode::JumpIfNil | OpCode::JumpIfFalsePop | OpCode::JumpIfTruePop
                | OpCode::ForPrep | OpCode::ForIter
            ))
        }
        fn is_int_jump_with_target(op: u8) -> bool {
            matches!(OpCode::from_u8(op), Some(
                OpCode::IntJumpIfNotLt | OpCode::IntJumpIfNotGt
            ))
        }

        let len = chunk.code.len();
        let mut live: Vec<bool> = vec![true; len];
        let mut changed = false;

        // Collect all jump-target offsets (byte positions that any instruction jumps to)
        let mut targets = std::collections::HashSet::new();
        {
            let mut i = 0;
            while i < len {
                let op = chunk.code[i];
                if is_jump_with_target(op) {
                    let t = u16::from_le_bytes([chunk.code[i + 1], chunk.code[i + 2]]) as usize;
                    targets.insert(t);
                } else if is_int_jump_with_target(op) {
                    let t = u16::from_le_bytes([chunk.code[i + 5], chunk.code[i + 6]]) as usize;
                    targets.insert(t);
                }
                i += insn_size_at(&chunk.code, i);
            }
        }

        // Phase 1: dead code after Return (stops at any known jump target)
        {
            let mut i = 0;
            while i < len {
                if chunk.code[i] == OpCode::Return as u8 {
                    let mut j = i + 1;
                    while j < len && !targets.contains(&j) {
                        live[j] = false;
                        changed = true;
                        j += 1;
                    }
                    i = j;
                } else {
                    i += insn_size_at(&chunk.code, i);
                }
            }
        }

        // Phase 2: Jump threading — redirect Jump/JumpIf* that point to another Jump
        {
            let mut jump_threaded = true;
            while jump_threaded {
                jump_threaded = false;
                let snapshot = chunk.code.clone();
                let mut i = 0;
                while i < len {
                    if !live[i] { i += 1; continue; }
                    let op = snapshot[i];
                    if is_jump_with_target(op) {
                        let target = u16::from_le_bytes([snapshot[i + 1], snapshot[i + 2]]) as usize;
                        if target < len && live[target] && snapshot[target] == OpCode::Jump as u8 {
                            let redirect = u16::from_le_bytes([snapshot[target + 1], snapshot[target + 2]]) as usize;
                            if redirect != target && live[redirect] {
                                chunk.code[i + 1] = (redirect & 0xFF) as u8;
                                chunk.code[i + 2] = ((redirect >> 8) & 0xFF) as u8;
                                jump_threaded = true;
                                changed = true;
                            }
                        }
                    }
                    i += insn_size_at(&chunk.code, i);
                }
            }
        }

        // Phase 3: Remove self-jumps (jump to next instruction)
        {
            let mut i = 0;
            while i < len {
                if !live[i] { i += 1; continue; }
                let op = chunk.code[i];
                if is_jump_with_target(op) {
                    let target = u16::from_le_bytes([chunk.code[i + 1], chunk.code[i + 2]]) as usize;
                    if target == i + insn_size_at(&chunk.code, i) {
                        match OpCode::from_u8(op).unwrap() {
                            OpCode::Jump => {
                                live[i] = false; live[i+1] = false; live[i+2] = false;
                                changed = true;
                            }
                            OpCode::JumpIfFalsePop | OpCode::JumpIfTruePop => {
                                chunk.code[i] = OpCode::Pop as u8;
                                live[i] = true; live[i+1] = false; live[i+2] = false;
                                changed = true;
                            }
                            OpCode::JumpIfNil | OpCode::JumpIfTrue | OpCode::JumpIfFalse => {
                                live[i] = false; live[i+1] = false; live[i+2] = false;
                                changed = true;
                            }
                            _ => {}
                        }
                    }
                } else if is_int_jump_with_target(op) {
                    let target = u16::from_le_bytes([chunk.code[i + 5], chunk.code[i + 6]]) as usize;
                    if target == i + insn_size_at(&chunk.code, i) {
                        live[i] = false; live[i+1] = false; live[i+2] = false;
                        live[i+3] = false; live[i+4] = false; live[i+5] = false; live[i+6] = false;
                        changed = true;
                    }
                }
                i += insn_size_at(&chunk.code, i);
            }
        }

        // Phase 4: Squash consecutive Pop-Pop
        {
            let mut i = 0;
            while i < len {
                if !live[i] { i += 1; continue; }
                if chunk.code[i] == OpCode::Pop as u8 {
                    let mut j = i + 1;
                    while j < len && live[j] && chunk.code[j] == OpCode::Pop as u8 {
                        live[j] = false;
                        changed = true;
                        j += 1;
                    }
                }
                i += insn_size_at(&chunk.code, i);
            }
        }

        if !changed { return; }

        // Phase 5: Compact — remove dead bytes, remap jump targets
        let mut new_code = Vec::with_capacity(len);
        let mut old_to_new: Vec<u16> = vec![u16::MAX; len];
        for idx in 0..len {
            if live[idx] {
                old_to_new[idx] = new_code.len() as u16;
                new_code.push(chunk.code[idx]);
            }
        }

        let new_len = new_code.len();
        let mut j = 0;
        while j < new_len {
            let op = new_code[j];
            if is_jump_with_target(op) {
                let old = u16::from_le_bytes([new_code[j + 1], new_code[j + 2]]) as usize;
                if old < old_to_new.len() {
                    let new = old_to_new[old];
                    new_code[j + 1] = (new & 0xFF) as u8;
                    new_code[j + 2] = ((new >> 8) & 0xFF) as u8;
                }
            } else if is_int_jump_with_target(op) {
                let old = u16::from_le_bytes([new_code[j + 5], new_code[j + 6]]) as usize;
                if old < old_to_new.len() {
                    let new = old_to_new[old];
                    new_code[j + 5] = (new & 0xFF) as u8;
                    new_code[j + 6] = ((new >> 8) & 0xFF) as u8;
                }
            }
            j += insn_size_at(&new_code, j);
        }
        chunk.code = new_code;
    }
}
