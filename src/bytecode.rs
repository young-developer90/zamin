#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    Halt = 0,
    Pop,
    Dup,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    IntDiv,
    Neg,
    Not,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    Concat,
    Return,
    Nil,
    True,
    False,
    Print,
    PrintLn,
    LoadConst,
    LoadLocal,
    StoreLocal,
    LoadGlobal,
    StoreGlobal,
    LoadUpvalue,
    StoreUpvalue,
    Jump,
    JumpIfTrue,
    JumpIfFalse,
    JumpIfNil,
    Call,
    MakeFunc,
    MakeClosure,
    CloseUpvalue,
    BuildList,
    BuildDict,
    BuildSet,
    BuildTuple,
    ListAppend,
    DictSet,
    SetAdd,
    LoadIndex,
    StoreIndex,
    LoadAttr,
    StoreAttr,
    PushScope,
    PopScope,
    Throw,
    Try,
    EndTry,
    ForPrep,
    ForIter,
    CheckMatch,
    EnterLoop,
    LeaveLoop,
    Break,
    Continue,
    BuildRange,
    MakeIter,
    NextIter,
    Len,
    MakeStruct,
    NewStructInstance,
    StructSetField,
    StructGetField,
    In,
}

impl OpCode {
    pub fn from_u8(n: u8) -> Option<OpCode> {
        use OpCode::*;
        Some(match n {
            0 => Halt,
            1 => Pop,
            2 => Dup,
            3 => Add,
            4 => Sub,
            5 => Mul,
            6 => Div,
            7 => Mod,
            8 => Pow,
            9 => IntDiv,
            10 => Neg,
            11 => Not,
            12 => Eq,
            13 => Ne,
            14 => Lt,
            15 => Gt,
            16 => Le,
            17 => Ge,
            18 => And,
            19 => Or,
            20 => Concat,
            21 => Return,
            22 => Nil,
            23 => True,
            24 => False,
            25 => Print,
            26 => PrintLn,
            27 => LoadConst,
            28 => LoadLocal,
            29 => StoreLocal,
            30 => LoadGlobal,
            31 => StoreGlobal,
            32 => LoadUpvalue,
            33 => StoreUpvalue,
            34 => Jump,
            35 => JumpIfTrue,
            36 => JumpIfFalse,
            37 => JumpIfNil,
            38 => Call,
            39 => MakeFunc,
            40 => MakeClosure,
            41 => CloseUpvalue,
            42 => BuildList,
            43 => BuildDict,
            44 => BuildSet,
            45 => BuildTuple,
            46 => ListAppend,
            47 => DictSet,
            48 => SetAdd,
            49 => LoadIndex,
            50 => StoreIndex,
            51 => LoadAttr,
            52 => StoreAttr,
            53 => PushScope,
            54 => PopScope,
            55 => Throw,
            56 => Try,
            57 => EndTry,
            58 => ForPrep,
            59 => ForIter,
            60 => CheckMatch,
            61 => EnterLoop,
            62 => LeaveLoop,
            63 => Break,
            64 => Continue,
            65 => BuildRange,
            66 => MakeIter,
            67 => NextIter,
            68 => Len,
            69 => MakeStruct,
            70 => NewStructInstance,
            71 => StructSetField,
            72 => StructGetField,
            73 => In,
            _ => return None,
        })
    }

    pub fn operand_count(&self) -> usize {
        use OpCode::*;
        match self {
            LoadConst | LoadLocal | StoreLocal | LoadGlobal | StoreGlobal
            | LoadUpvalue | StoreUpvalue | Jump | JumpIfTrue | JumpIfFalse
            | JumpIfNil | Call | MakeFunc | MakeClosure | BuildList
            | BuildDict | BuildSet | BuildTuple | Try | ForPrep | ForIter
            |             CheckMatch | MakeIter | NextIter | LoadAttr | StoreAttr | Len
            | MakeStruct | NewStructInstance | StructSetField | StructGetField => 1,
            _ => 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: Vec<super::gc::Value>,
    pub string_constants: Vec<Option<String>>,
    pub string_pool: Vec<String>,
    pub locals: usize,
    pub upvalues: Vec<UpvalueInfo>,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpvalueInfo {
    pub name: String,
    pub is_local: bool,
    pub index: usize,
}

impl Chunk {
    pub fn new(name: Option<String>) -> Self {
        Chunk {
            code: Vec::new(),
            constants: Vec::new(),
            string_constants: Vec::new(),
            string_pool: Vec::new(),
            locals: 0,
            upvalues: Vec::new(),
            name,
        }
    }

    pub fn emit(&mut self, op: OpCode) {
        self.code.push(op as u8);
    }

    pub fn emit_u16(&mut self, val: u16) {
        self.code.extend_from_slice(&val.to_le_bytes());
    }

    pub fn emit_u32(&mut self, val: u32) {
        self.code.extend_from_slice(&val.to_le_bytes());
    }

    pub fn add_constant(&mut self, val: super::gc::Value) -> u16 {
        let idx = self.constants.len();
        self.constants.push(val);
        self.string_constants.push(None);
        idx as u16
    }

    pub fn add_string_constant(&mut self, val: super::gc::Value, content: String) -> u16 {
        let idx = self.constants.len();
        self.constants.push(val);
        self.string_constants.push(Some(content));
        idx as u16
    }

    pub fn intern_string(&mut self, s: &str) -> u16 {
        if let Some((i, _)) = self.string_pool.iter().enumerate().find(|(_, sp)| sp == &s) {
            i as u16
        } else {
            let idx = self.string_pool.len() as u16;
            self.string_pool.push(s.to_string());
            idx
        }
    }

    pub fn emit_const(&mut self, val: super::gc::Value) {
        let idx = self.add_constant(val);
        self.emit(OpCode::LoadConst);
        self.emit_u16(idx);
    }

    pub fn disassemble(&self) -> String {
        let mut output = String::new();
        let name = self.name.as_deref().unwrap_or("<anonymous>");
        output.push_str(&format!("== {} ==\n", name));

        let mut i = 0;
        while i < self.code.len() {
            let op = OpCode::from_u8(self.code[i]).unwrap_or(OpCode::Halt);
            output.push_str(&format!("{:04}  {:?}", i, op));

            match op {
                OpCode::LoadConst | OpCode::LoadLocal | OpCode::StoreLocal
                | OpCode::LoadGlobal | OpCode::StoreGlobal | OpCode::LoadUpvalue
                | OpCode::StoreUpvalue | OpCode::Jump | OpCode::JumpIfTrue
                | OpCode::JumpIfFalse | OpCode::JumpIfNil => {
                    let val = u16::from_le_bytes([self.code[i+1], self.code[i+2]]);
                    if matches!(op, OpCode::LoadConst) {
                        if let Some(c) = self.constants.get(val as usize) {
                            output.push_str(&format!(" {} ({:?})", val, c));
                        } else {
                            output.push_str(&format!(" {}", val));
                        }
                    } else {
                        output.push_str(&format!(" {}", val));
                    }
                    i += 3;
                    continue;
                }
                OpCode::Call | OpCode::BuildList | OpCode::BuildDict
                | OpCode::BuildSet | OpCode::BuildTuple | OpCode::BuildRange
                | OpCode::Try
                | OpCode::ForPrep | OpCode::ForIter | OpCode::CheckMatch
                | OpCode::MakeFunc | OpCode::MakeClosure | OpCode::MakeIter
                | OpCode::NextIter
                | OpCode::Len | OpCode::LoadAttr | OpCode::StoreAttr
                | OpCode::MakeStruct | OpCode::NewStructInstance
                | OpCode::StructSetField | OpCode::StructGetField => {
                    let val = u16::from_le_bytes([self.code[i+1], self.code[i+2]]);
                    output.push_str(&format!(" {}", val));
                    i += 3;
                    continue;
                }
                _ => {
                    i += 1;
                }
            }
            output.push('\n');
        }
        output
    }
}
