use petgraph::graph::NodeIndex;
use petgraph::Graph;
use pretty::{DocAllocator, DocBuilder};
use wasmparser::{self as wasm, FuncValidatorAllocations, Global, WasmModuleResources};

mod decode;
mod print;

pub struct Block {
    params: Vec<wasm::ValType>,
    statements: Vec<Statement>,
    terminator: Terminator,
}

pub enum Terminator {
    Unknown,
    Unreachable,
    Return(Vec<Expression>),
    Br(NodeIndex, Vec<Expression>),
    BrIf(Expression, NodeIndex, NodeIndex, Vec<Expression>),
    BrTable(Vec<NodeIndex>, NodeIndex, Vec<Expression>),
}

enum Statement {
    Nop,
    Drop(Expression),
    LocalSet(LocalSetStatement),
    GlobalSet(GlobalSetStatement),
    MemoryStore(MemoryStoreStatement),
    Call(CallExpression),
    CallIndirect(CallIndirectExpression),
}

pub struct LocalSetStatement {
    index: u32,
    value: Box<Expression>,
}

pub struct GlobalSetStatement {
    index: u32,
    value: Box<Expression>,
}

pub struct MemoryStoreStatement {
    arg: wasm::MemArg,
    index: Box<Expression>,
    value: Box<Expression>,
}

pub enum Expression {
    I32Const { value: i32 },
    I64Const { value: i64 },
    F32Const { value: wasm::Ieee32 },
    F64Const { value: wasm::Ieee64 },

    BlockParam(u32),

    Unary(UnaryExpression, Box<Expression>),
    Binary(BinaryExpression, Box<Expression>, Box<Expression>),
    Call(CallExpression),
    CallIndirect(CallIndirectExpression),
    GetLocal(GetLocalExpression),
    GetGlobal(GetGlobalExpression),
    Select(SelectExpression),
    MemoryLoad(MemoryLoadExpression),
    MemorySize,
    MemoryGrow(MemoryGrowExpression),

    Bottom,
}

pub enum UnaryExpression {
    I32Eqz,
    I64Eqz,
    I32Clz,
    I32Ctz,
    I32Popcnt,
    I64Clz,
    I64Ctz,
    I64Popcnt,
    F32Abs,
    F32Neg,
    F32Ceil,
    F32Floor,
    F32Trunc,
    F32Nearest,
    F32Sqrt,
    F32Copysign,
    F64Abs,
    F64Neg,
    F64Ceil,
    F64Floor,
    F64Trunc,
    F64Nearest,
    F64Sqrt,
    F64Copysign,
    I32WrapI64,
    I32TruncF32S,
    I32TruncF32U,
    I32TruncF64S,
    I32TruncF64U,
    I64ExtendI32S,
    I64ExtendI32U,
    I64TruncF32S,
    I64TruncF32U,
    I64TruncF64S,
    I64TruncF64U,
    F32ConvertI32S,
    F32ConvertI32U,
    F32ConvertI64S,
    F32ConvertI64U,
    F32DemoteF64,
    F64ConvertI32S,
    F64ConvertI32U,
    F64ConvertI64S,
    F64ConvertI64U,
    F64PromoteF32,
    I32ReinterpretF32,
    I64ReinterpretF64,
    F32ReinterpretI32,
    F64ReinterpretI64,
    I32Extend8S,
    I32Extend16S,
    I64Extend8S,
    I64Extend16S,
    I64Extend32S,
}

impl UnaryExpression {
    fn to_string(&self) -> &'static str {
        // TODO: these are suboptimal
        use UnaryExpression::*;
        match self {
            I32Eqz => "eqz",
            I64Eqz => "eqz",
            I32Clz => "clz",
            I32Ctz => "ctz",
            I32Popcnt => "popcnt",
            I64Clz => "clz",
            I64Ctz => "ctz",
            I64Popcnt => "popcnt",
            F32Abs => "abs",
            F32Neg => "neg",
            F32Ceil => "ceil",
            F32Floor => "floor",
            F32Trunc => "trunc",
            F32Nearest => "nearest",
            F32Sqrt => "sqrt",
            F32Copysign => "copysign",
            F64Abs => "abs",
            F64Neg => "neg",
            F64Ceil => "ceil",
            F64Floor => "floor",
            F64Trunc => "trunc",
            F64Nearest => "nearest",
            F64Sqrt => "sqrt",
            F64Copysign => "copysign",
            I32WrapI64 => "wrap_i64",
            I32TruncF32S => "trunc_f32s",
            I32TruncF32U => "trunc_f32u",
            I32TruncF64S => "trunc_f64s",
            I32TruncF64U => "trunc_f64u",
            I64ExtendI32S => "extend_i32s",
            I64ExtendI32U => "extend_i32u",
            I64TruncF32S => "trunc_f32s",
            I64TruncF32U => "trunc_f32u",
            I64TruncF64S => "trunc_f64s",
            I64TruncF64U => "trunc_f64u",
            F32ConvertI32S => "convert_i32s",
            F32ConvertI32U => "convert_i32u",
            F32ConvertI64S => "convert_i64s",
            F32ConvertI64U => "convert_i64u",
            F32DemoteF64 => "demote_f64",
            F64ConvertI32S => "convert_i32s",
            F64ConvertI32U => "convert_i32u",
            F64ConvertI64S => "convert_i64s",
            F64ConvertI64U => "convert_i64u",
            F64PromoteF32 => "promote_f32",
            I32ReinterpretF32 => "reinterpret_f32",
            I64ReinterpretF64 => "reinterpret_f64",
            F32ReinterpretI32 => "reinterpret_i32",
            F64ReinterpretI64 => "reinterpret_i64",
            I32Extend8S => "extend8s",
            I32Extend16S => "extend16s",
            I64Extend8S => "extend8s",
            I64Extend16S => "extend16s",
            I64Extend32S => "extend32s",
        }
    }
}

impl<'a> From<wasm::Operator<'a>> for UnaryExpression {
    fn from(value: wasm::Operator) -> Self {
        match value {
            wasm::Operator::I32Eqz => UnaryExpression::I32Eqz,
            wasm::Operator::I64Eqz => UnaryExpression::I64Eqz,
            wasm::Operator::I32Clz => UnaryExpression::I32Clz,
            wasm::Operator::I32Ctz => UnaryExpression::I32Ctz,
            wasm::Operator::I32Popcnt => UnaryExpression::I32Popcnt,
            wasm::Operator::I64Clz => UnaryExpression::I64Clz,
            wasm::Operator::I64Ctz => UnaryExpression::I64Ctz,
            wasm::Operator::I64Popcnt => UnaryExpression::I64Popcnt,
            wasm::Operator::F32Abs => UnaryExpression::F32Abs,
            wasm::Operator::F32Neg => UnaryExpression::F32Neg,
            wasm::Operator::F32Ceil => UnaryExpression::F32Ceil,
            wasm::Operator::F32Floor => UnaryExpression::F32Floor,
            wasm::Operator::F32Trunc => UnaryExpression::F32Trunc,
            wasm::Operator::F32Nearest => UnaryExpression::F32Nearest,
            wasm::Operator::F32Sqrt => UnaryExpression::F32Sqrt,
            wasm::Operator::F32Copysign => UnaryExpression::F32Copysign,
            wasm::Operator::F64Abs => UnaryExpression::F64Abs,
            wasm::Operator::F64Neg => UnaryExpression::F64Neg,
            wasm::Operator::F64Ceil => UnaryExpression::F64Ceil,
            wasm::Operator::F64Floor => UnaryExpression::F64Floor,
            wasm::Operator::F64Trunc => UnaryExpression::F64Trunc,
            wasm::Operator::F64Nearest => UnaryExpression::F64Nearest,
            wasm::Operator::F64Sqrt => UnaryExpression::F64Sqrt,
            wasm::Operator::F64Copysign => UnaryExpression::F64Copysign,
            wasm::Operator::I32WrapI64 => UnaryExpression::I32WrapI64,
            wasm::Operator::I32TruncF32S => UnaryExpression::I32TruncF32S,
            wasm::Operator::I32TruncF32U => UnaryExpression::I32TruncF32U,
            wasm::Operator::I32TruncF64S => UnaryExpression::I32TruncF64S,
            wasm::Operator::I32TruncF64U => UnaryExpression::I32TruncF64U,
            wasm::Operator::I64ExtendI32S => UnaryExpression::I64ExtendI32S,
            wasm::Operator::I64ExtendI32U => UnaryExpression::I64ExtendI32U,
            wasm::Operator::I64TruncF32S => UnaryExpression::I64TruncF32S,
            wasm::Operator::I64TruncF32U => UnaryExpression::I64TruncF32U,
            wasm::Operator::I64TruncF64S => UnaryExpression::I64TruncF64S,
            wasm::Operator::I64TruncF64U => UnaryExpression::I64TruncF64U,
            wasm::Operator::F32ConvertI32S => UnaryExpression::F32ConvertI32S,
            wasm::Operator::F32ConvertI32U => UnaryExpression::F32ConvertI32U,
            wasm::Operator::F32ConvertI64S => UnaryExpression::F32ConvertI64S,
            wasm::Operator::F32ConvertI64U => UnaryExpression::F32ConvertI64U,
            wasm::Operator::F32DemoteF64 => UnaryExpression::F32DemoteF64,
            wasm::Operator::F64ConvertI32S => UnaryExpression::F64ConvertI32S,
            wasm::Operator::F64ConvertI32U => UnaryExpression::F64ConvertI32U,
            wasm::Operator::F64ConvertI64S => UnaryExpression::F64ConvertI64S,
            wasm::Operator::F64ConvertI64U => UnaryExpression::F64ConvertI64U,
            wasm::Operator::F64PromoteF32 => UnaryExpression::F64PromoteF32,
            wasm::Operator::I32ReinterpretF32 => UnaryExpression::I32ReinterpretF32,
            wasm::Operator::I64ReinterpretF64 => UnaryExpression::I64ReinterpretF64,
            wasm::Operator::F32ReinterpretI32 => UnaryExpression::F32ReinterpretI32,
            wasm::Operator::F64ReinterpretI64 => UnaryExpression::F64ReinterpretI64,
            wasm::Operator::I32Extend8S => UnaryExpression::I32Extend8S,
            wasm::Operator::I32Extend16S => UnaryExpression::I32Extend16S,
            wasm::Operator::I64Extend8S => UnaryExpression::I64Extend8S,
            wasm::Operator::I64Extend16S => UnaryExpression::I64Extend16S,
            wasm::Operator::I64Extend32S => UnaryExpression::I64Extend32S,
            _ => unreachable!(),
        }
    }
}

pub enum BinaryExpression {
    I32Eq,
    I32Ne,
    I32LtS,
    I32LtU,
    I32GtS,
    I32GtU,
    I32LeS,
    I32LeU,
    I32GeS,
    I32GeU,
    I64Eq,
    I64Ne,
    I64LtS,
    I64LtU,
    I64GtS,
    I64GtU,
    I64LeS,
    I64LeU,
    I64GeS,
    I64GeU,
    F32Eq,
    F32Ne,
    F32Lt,
    F32Gt,
    F32Le,
    F32Ge,
    F64Eq,
    F64Ne,
    F64Lt,
    F64Gt,
    F64Le,
    F64Ge,
    I32Add,
    I32Sub,
    I32Mul,
    I32DivS,
    I32DivU,
    I32RemS,
    I32RemU,
    I32And,
    I32Or,
    I32Xor,
    I32Shl,
    I32ShrS,
    I32ShrU,
    I32Rotl,
    I32Rotr,
    I64Add,
    I64Sub,
    I64Mul,
    I64DivS,
    I64DivU,
    I64RemS,
    I64RemU,
    I64And,
    I64Or,
    I64Xor,
    I64Shl,
    I64ShrS,
    I64ShrU,
    I64Rotl,
    I64Rotr,
    F32Add,
    F32Sub,
    F32Mul,
    F32Div,
    F32Min,
    F32Max,
    F64Add,
    F64Sub,
    F64Mul,
    F64Div,
    F64Min,
    F64Max,
}

impl BinaryExpression {
    fn to_string_and_infix(&self) -> (&'static str, bool) {
        use BinaryExpression::*;
        match self {
            I32Eq => ("==", true),
            I32Ne => ("!=", true),
            I32LtS => ("<_s", true),
            I32LtU => ("<_u", true),
            I32GtS => (">_s", true),
            I32GtU => (">_u", true),
            I32LeS => ("<=_s", true),
            I32LeU => ("<=_u", true),
            I32GeS => (">=_s", true),
            I32GeU => (">=_u", true),
            I64Eq => ("==", true),
            I64Ne => ("!=", true),
            I64LtS => ("<_s", true),
            I64LtU => ("<_u", true),
            I64GtS => (">_s", true),
            I64GtU => (">_u", true),
            I64LeS => ("<=_s", true),
            I64LeU => ("<=_u", true),
            I64GeS => (">=_s", true),
            I64GeU => (">=_u", true),
            F32Eq => ("==", true),
            F32Ne => ("!=", true),
            F32Lt => ("<", true),
            F32Gt => (">", true),
            F32Le => ("<=", true),
            F32Ge => (">=", true),
            F64Eq => ("==", true),
            F64Ne => ("!=", true),
            F64Lt => ("<", true),
            F64Gt => (">", true),
            F64Le => ("<=", true),
            F64Ge => (">=", true),
            I32Add => ("+", true),
            I32Sub => ("-", true),
            I32Mul => ("*", true),
            I32DivS => ("/_s", true),
            I32DivU => ("/_u", true),
            I32RemS => ("%_s", true),
            I32RemU => ("%_u", true),
            I32And => ("&", true),
            I32Or => ("|", true),
            I32Xor => ("#xor", true),
            I32Shl => ("<<", true),
            I32ShrS => (">>_s", true),
            I32ShrU => (">>_u", true),
            I32Rotl => ("#rotl", true),
            I32Rotr => ("#rotr", true),
            I64Add => ("+", true),
            I64Sub => ("-", true),
            I64Mul => ("*", true),
            I64DivS => ("/_s", true),
            I64DivU => ("/_u", true),
            I64RemS => ("%_s", true),
            I64RemU => ("%_u", true),
            I64And => ("&", true),
            I64Or => ("|", true),
            I64Xor => ("#xor", true),
            I64Shl => ("<<", true),
            I64ShrS => (">>_s", true),
            I64ShrU => (">>_u", true),
            I64Rotl => ("#rotl", true),
            I64Rotr => ("#rotr", true),
            F32Add => ("+", true),
            F32Sub => ("-", true),
            F32Mul => ("*", true),
            F32Div => ("", true),
            F32Min => ("min", false),
            F32Max => ("max", false),
            F64Add => ("+", true),
            F64Sub => ("-", true),
            F64Mul => ("*", true),
            F64Div => ("/", true),
            F64Min => ("min", false),
            F64Max => ("max", false),
        }
    }
}

impl<'a> From<wasm::Operator<'a>> for BinaryExpression {
    fn from(value: wasm::Operator) -> Self {
        match value {
            wasm::Operator::I32Eq => BinaryExpression::I32Eq,
            wasm::Operator::I32Ne => BinaryExpression::I32Ne,
            wasm::Operator::I32LtS => BinaryExpression::I32LtS,
            wasm::Operator::I32LtU => BinaryExpression::I32LtU,
            wasm::Operator::I32GtS => BinaryExpression::I32GtS,
            wasm::Operator::I32GtU => BinaryExpression::I32GtU,
            wasm::Operator::I32LeS => BinaryExpression::I32LeS,
            wasm::Operator::I32LeU => BinaryExpression::I32LeU,
            wasm::Operator::I32GeS => BinaryExpression::I32GeS,
            wasm::Operator::I32GeU => BinaryExpression::I32GeU,
            wasm::Operator::I64Eq => BinaryExpression::I64Eq,
            wasm::Operator::I64Ne => BinaryExpression::I64Ne,
            wasm::Operator::I64LtS => BinaryExpression::I64LtS,
            wasm::Operator::I64LtU => BinaryExpression::I64LtU,
            wasm::Operator::I64GtS => BinaryExpression::I64GtS,
            wasm::Operator::I64GtU => BinaryExpression::I64GtU,
            wasm::Operator::I64LeS => BinaryExpression::I64LeS,
            wasm::Operator::I64LeU => BinaryExpression::I64LeU,
            wasm::Operator::I64GeS => BinaryExpression::I64GeS,
            wasm::Operator::I64GeU => BinaryExpression::I64GeU,
            wasm::Operator::F32Eq => BinaryExpression::F32Eq,
            wasm::Operator::F32Ne => BinaryExpression::F32Ne,
            wasm::Operator::F32Lt => BinaryExpression::F32Lt,
            wasm::Operator::F32Gt => BinaryExpression::F32Gt,
            wasm::Operator::F32Le => BinaryExpression::F32Le,
            wasm::Operator::F32Ge => BinaryExpression::F32Ge,
            wasm::Operator::F64Eq => BinaryExpression::F64Eq,
            wasm::Operator::F64Ne => BinaryExpression::F64Ne,
            wasm::Operator::F64Lt => BinaryExpression::F64Lt,
            wasm::Operator::F64Gt => BinaryExpression::F64Gt,
            wasm::Operator::F64Le => BinaryExpression::F64Le,
            wasm::Operator::F64Ge => BinaryExpression::F64Ge,
            wasm::Operator::I32Add => BinaryExpression::I32Add,
            wasm::Operator::I32Sub => BinaryExpression::I32Sub,
            wasm::Operator::I32Mul => BinaryExpression::I32Mul,
            wasm::Operator::I32DivS => BinaryExpression::I32DivS,
            wasm::Operator::I32DivU => BinaryExpression::I32DivU,
            wasm::Operator::I32RemS => BinaryExpression::I32RemS,
            wasm::Operator::I32RemU => BinaryExpression::I32RemU,
            wasm::Operator::I32And => BinaryExpression::I32And,
            wasm::Operator::I32Or => BinaryExpression::I32Or,
            wasm::Operator::I32Xor => BinaryExpression::I32Xor,
            wasm::Operator::I32Shl => BinaryExpression::I32Shl,
            wasm::Operator::I32ShrS => BinaryExpression::I32ShrS,
            wasm::Operator::I32ShrU => BinaryExpression::I32ShrU,
            wasm::Operator::I32Rotl => BinaryExpression::I32Rotl,
            wasm::Operator::I32Rotr => BinaryExpression::I32Rotr,
            wasm::Operator::I64Add => BinaryExpression::I64Add,
            wasm::Operator::I64Sub => BinaryExpression::I64Sub,
            wasm::Operator::I64Mul => BinaryExpression::I64Mul,
            wasm::Operator::I64DivS => BinaryExpression::I64DivS,
            wasm::Operator::I64DivU => BinaryExpression::I64DivU,
            wasm::Operator::I64RemS => BinaryExpression::I64RemS,
            wasm::Operator::I64RemU => BinaryExpression::I64RemU,
            wasm::Operator::I64And => BinaryExpression::I64And,
            wasm::Operator::I64Or => BinaryExpression::I64Or,
            wasm::Operator::I64Xor => BinaryExpression::I64Xor,
            wasm::Operator::I64Shl => BinaryExpression::I64Shl,
            wasm::Operator::I64ShrS => BinaryExpression::I64ShrS,
            wasm::Operator::I64ShrU => BinaryExpression::I64ShrU,
            wasm::Operator::I64Rotl => BinaryExpression::I64Rotl,
            wasm::Operator::I64Rotr => BinaryExpression::I64Rotr,
            wasm::Operator::F32Add => BinaryExpression::F32Add,
            wasm::Operator::F32Sub => BinaryExpression::F32Sub,
            wasm::Operator::F32Mul => BinaryExpression::F32Mul,
            wasm::Operator::F32Div => BinaryExpression::F32Div,
            wasm::Operator::F32Min => BinaryExpression::F32Min,
            wasm::Operator::F32Max => BinaryExpression::F32Max,
            wasm::Operator::F64Add => BinaryExpression::F64Add,
            wasm::Operator::F64Sub => BinaryExpression::F64Sub,
            wasm::Operator::F64Mul => BinaryExpression::F64Mul,
            wasm::Operator::F64Div => BinaryExpression::F64Div,
            wasm::Operator::F64Min => BinaryExpression::F64Min,
            wasm::Operator::F64Max => BinaryExpression::F64Max,
            _ => unreachable!(),
        }
    }
}

pub struct CallExpression {
    func_index: u32,
    params: Vec<Expression>,
}

pub struct CallIndirectExpression {
    func_type_index: u32,
    table_index: u32,
    callee_index: Box<Expression>,
    params: Vec<Expression>,
}

pub struct GetLocalExpression {
    local_index: u32,
}

pub struct GetGlobalExpression {
    global_index: u32,
}

pub struct SelectExpression {
    condition: Box<Expression>,
    on_true: Box<Expression>,
    on_false: Box<Expression>,
}

pub struct MemoryLoadExpression {
    arg: wasm::MemArg,
    index: Box<Expression>,
}

pub struct MemoryGrowExpression {
    value: Box<Expression>,
}

struct Local {
    ty: wasm::ValType,
    name: String,
}

struct Func {
    // name: String,
    index: u32,
    ty: wasm::FuncType,
    locals: Vec<Local>,
    blocks: Graph<Block, ()>,
    entry_block: NodeIndex,
}

pub struct Module {
    rec_groups: Vec<wasm::RecGroup>,
    types_of_funcs: Vec<u32>,
    funcs: Vec<Func>,
}

impl Module {
    pub fn from_buffer(buffer: &[u8]) -> anyhow::Result<Self> {
        let parser = wasm::Parser::new(0);
        let mut validator = wasm::Validator::new();
        let mut result = Self {
            rec_groups: Vec::new(),
            types_of_funcs: Vec::new(),
            funcs: Vec::new(),
        };

        for payload in parser.parse_all(&buffer) {
            match payload? {
                // Sections for WebAssembly modules
                wasm::Payload::Version {
                    num,
                    encoding,
                    range,
                } => {
                    validator.version(num, encoding, &range)?;
                }
                wasm::Payload::TypeSection(section) => {
                    validator.type_section(&section)?;
                    for rec_groups in section {
                        result.rec_groups.push(rec_groups?);
                    }
                }
                wasm::Payload::ImportSection(section) => {
                    validator.import_section(&section)?;
                }
                wasm::Payload::FunctionSection(section) => {
                    validator.function_section(&section)?;
                    for func in section {
                        result.types_of_funcs.push(func?);
                    }
                }
                wasm::Payload::TableSection(section) => {
                    validator.table_section(&section)?;
                }
                wasm::Payload::MemorySection(section) => {
                    validator.memory_section(&section)?;
                }
                wasm::Payload::TagSection(section) => {
                    validator.tag_section(&section)?;
                }
                wasm::Payload::GlobalSection(section) => {
                    validator.global_section(&section)?;
                }
                wasm::Payload::ExportSection(section) => {
                    validator.export_section(&section)?;
                }
                wasm::Payload::StartSection { func, range } => {
                    validator.start_section(func, &range)?;
                }
                wasm::Payload::ElementSection(section) => {
                    validator.element_section(&section)?;
                }
                wasm::Payload::DataCountSection { count, range } => {
                    validator.data_count_section(count, &range)?;
                }
                wasm::Payload::DataSection(section) => {
                    validator.data_section(&section)?;
                }

                // Here we know how many functions we'll be receiving as
                // `CodeSectionEntry`, so we can prepare for that, and
                // afterwards we can parse and handle each function
                // individually.
                wasm::Payload::CodeSectionStart {
                    count,
                    range,
                    size: _,
                } => {
                    validator.code_section_start(count, &range)?;
                }
                wasm::Payload::CodeSectionEntry(body) => {
                    let func_to_validate = validator.code_section_entry(&body)?;
                    let func = Func::decode(body, func_to_validate)?;
                    result.funcs.push(func);
                }

                wasm::Payload::CustomSection(_) => { /* ... */ }

                // Once we've reached the end of a parser we either resume
                // at the parent parser or the payload iterator is at its
                // end and we're done.
                wasm::Payload::End(offset) => {
                    validator.end(offset)?;
                }

                // most likely you'd return an error here, but if you want
                // you can also inspect the raw contents of unknown sections
                other => {
                    anyhow::bail!("unknown section: {:?}", other);
                }
            }
        }

        Ok(result)
    }

    pub fn write(&self, mut output: impl std::io::Write) -> anyhow::Result<()> {
        self.pretty::<_, ()>(&pretty::BoxAllocator)
            .render(80, &mut output)?;
        write!(output, "\n")?;
        Ok(())
    }
}
