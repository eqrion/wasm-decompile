use crate::ir::*;

#[derive(Clone, Copy)]
struct Ctx<'b> {
    func: &'b Func,
}

impl Block {
    fn pretty<'b, D, A>(
        &'b self,
        index: NodeIndex,
        ctx: Ctx<'b>,
        allocator: &'b D,
    ) -> DocBuilder<'b, D, A>
    where
        D: DocAllocator<'b, A>,
        D::Doc: Clone,
        A: Clone,
    {
        let mut instructions = vec![];
        for statement in &self.statements {
            instructions.push(statement.pretty(ctx, allocator));
        }
        instructions.push(self.terminator.pretty(ctx, allocator));

        let params = self.params.iter().enumerate().map(|(i, param)| {
            allocator
                .text(format!("b{}:", i))
                .append(allocator.space())
                .append(allocator.text(param.to_string()))
        });

        allocator
            .text(format!("block{}", index.index()))
            .append(if self.params.is_empty() {
                allocator.nil()
            } else {
                allocator.intersperse(params, allocator.text(", ")).parens()
            })
            .append(allocator.text(":"))
            .append(allocator.hardline())
            .append(
                allocator
                    .intersperse(instructions, allocator.hardline())
                    .indent(2),
            )
    }
}

impl Terminator {
    fn pretty<'b, D, A>(&'b self, ctx: Ctx<'b>, allocator: &'b D) -> DocBuilder<'b, D, A>
    where
        D: DocAllocator<'b, A>,
        D::Doc: Clone,
        A: Clone,
    {
        match self {
            Terminator::Unknown => allocator.text("unknown"),
            Terminator::Unreachable => allocator.text("unreachable"),
            Terminator::Return(params) => allocator
                .text("return")
                .append(allocator.space())
                .append(allocator.intersperse(
                    params.iter().map(|param| param.pretty(ctx, allocator)),
                    allocator.text(", "),
                )),
            Terminator::Br(target, params) => allocator
                .text(format!("jump block{}", target.index()))
                .append(if params.is_empty() {
                    allocator.nil()
                } else {
                    allocator
                        .intersperse(
                            params.iter().map(|param| param.pretty(ctx, allocator)),
                            allocator.text(", "),
                        )
                        .parens()
                }),
            Terminator::BrIf(condition, true_target, false_target, params) => {
                let params = if params.is_empty() {
                    allocator.nil()
                } else {
                    allocator.intersperse(
                        params.iter().map(|param| param.pretty(ctx, allocator)),
                        allocator.text(", "),
                    )
                };

                allocator
                    .text("if")
                    .append(allocator.space())
                    .append(condition.pretty(ctx, allocator))
                    .append(allocator.hardline())
                    .append(
                        allocator
                            .text(format!(" jump block{}", true_target.index()))
                            .append(params.clone())
                            .indent(2),
                    )
                    .append(allocator.hardline())
                    .append("else")
                    .append(allocator.hardline())
                    .append(
                        allocator
                            .text(format!("jump block{}", false_target.index()))
                            .append(params)
                            .indent(2),
                    )
            }
            Terminator::BrTable(targets, default_target, params) => {
                let params = allocator.intersperse(
                    params.iter().map(|param| param.pretty(ctx, allocator)),
                    allocator.text(", "),
                );
                let targets = allocator.intersperse(
                    targets
                        .iter()
                        .map(|x| allocator.text(format!("block{}", x.index()))),
                    allocator.text(", "),
                );
                allocator
                    .text("jump_table")
                    .append(
                        targets
                            .append(
                                allocator.text(" default ").append(
                                    allocator.text(format!("block{}", default_target.index())),
                                ),
                            )
                            .parens(),
                    )
                    .append(allocator.space())
                    .append(params)
            }
        }
    }
}

impl Statement {
    fn pretty<'b, D, A>(&'b self, ctx: Ctx<'b>, allocator: &'b D) -> DocBuilder<'b, D, A>
    where
        D: DocAllocator<'b, A>,
        D::Doc: Clone,
        A: Clone,
    {
        match self {
            Statement::Nop => allocator.text("nop"),
            Statement::Drop(expr) => allocator
                .text("drop")
                .append(expr.pretty(ctx, allocator).parens()),
            Statement::LocalSet(stmt) => stmt.pretty(ctx, allocator),
            Statement::GlobalSet(stmt) => stmt.pretty(ctx, allocator),
            Statement::MemoryStore(stmt) => stmt.pretty(ctx, allocator),
            Statement::Call(expr) => expr.pretty(ctx, allocator),
            Statement::CallIndirect(expr) => expr.pretty(ctx, allocator),
        }
    }
}

impl LocalSetStatement {
    fn pretty<'b, D, A>(&'b self, ctx: Ctx<'b>, allocator: &'b D) -> DocBuilder<'b, D, A>
    where
        D: DocAllocator<'b, A>,
        D::Doc: Clone,
        A: Clone,
    {
        allocator
            .text(&ctx.func.locals[self.index as usize].name)
            .append(allocator.space())
            .append(allocator.text("="))
            .append(allocator.space())
            .append(self.value.pretty(ctx, allocator))
    }
}

impl GlobalSetStatement {
    fn pretty<'b, D, A>(&'b self, ctx: Ctx<'b>, allocator: &'b D) -> DocBuilder<'b, D, A>
    where
        D: DocAllocator<'b, A>,
        D::Doc: Clone,
        A: Clone,
    {
        allocator
            .text(format!("global[{}] = ", self.index))
            .append(self.value.pretty(ctx, allocator))
    }
}

impl MemoryStoreStatement {
    fn pretty<'b, D, A>(&'b self, ctx: Ctx<'b>, allocator: &'b D) -> DocBuilder<'b, D, A>
    where
        D: DocAllocator<'b, A>,
        D::Doc: Clone,
        A: Clone,
    {
        // TODO: offset
        allocator
            .text("*")
            .append(self.index.pretty(ctx, allocator).parens())
            .append(allocator.space())
            .append(allocator.text("="))
            .append(allocator.space())
            .append(self.value.pretty(ctx, allocator))
    }
}

impl Expression {
    fn pretty<'b, D, A>(&'b self, ctx: Ctx<'b>, allocator: &'b D) -> DocBuilder<'b, D, A>
    where
        D: DocAllocator<'b, A>,
        D::Doc: Clone,
        A: Clone,
    {
        match self {
            Expression::I32Const { value } => allocator.text(value.to_string()),
            Expression::I64Const { value } => allocator.text(value.to_string()),
            Expression::F32Const { value } => {
                // TODO: Not correct for NaNs
                allocator.text(f32::from_bits(value.bits()).to_string())
            }
            Expression::F64Const { value } => {
                // TODO: Not correct for NaNs
                allocator.text(f64::from_bits(value.bits()).to_string())
            }
            Expression::BlockParam(index) => allocator.text(format!("b{}", index)),
            Expression::Unary(op, value) => allocator
                .text(op.to_string())
                .append(value.pretty(ctx, allocator).parens()),
            Expression::Binary(op, lhs, rhs) => {
                let (text, is_infix) = op.to_string_and_infix();
                if is_infix {
                    lhs.pretty(ctx, allocator)
                        .append(allocator.space())
                        .append(allocator.text(text))
                        .append(allocator.space())
                        .append(rhs.pretty(ctx, allocator))
                } else {
                    allocator
                        .text(text)
                        .append(allocator.space())
                        .append(lhs.pretty(ctx, allocator))
                        .append(allocator.space())
                        .append(rhs.pretty(ctx, allocator))
                }
            }
            Expression::Call(expr) => expr.pretty(ctx, allocator),
            Expression::CallIndirect(expr) => expr.pretty(ctx, allocator),
            Expression::GetLocal(expr) => expr.pretty(ctx, allocator),
            Expression::GetGlobal(expr) => expr.pretty(ctx, allocator),
            Expression::Select(expr) => expr.pretty(ctx, allocator),
            Expression::MemoryLoad(expr) => expr.pretty(ctx, allocator),
            Expression::MemorySize => allocator.text("memory.size"),
            Expression::MemoryGrow(expr) => expr.pretty(ctx, allocator),

            // Should be eliminated by dead code removal
            Expression::Bottom => allocator.text("bottom"),
        }
    }
}

impl CallExpression {
    fn pretty<'b, D, A>(&'b self, ctx: Ctx<'b>, allocator: &'b D) -> DocBuilder<'b, D, A>
    where
        D: DocAllocator<'b, A>,
        D::Doc: Clone,
        A: Clone,
    {
        allocator.text(format!("func{}", self.func_index)).append(
            allocator
                .intersperse(
                    self.params.iter().map(|param| param.pretty(ctx, allocator)),
                    allocator.text(", "),
                )
                .parens(),
        )
    }
}

impl CallIndirectExpression {
    fn pretty<'b, D, A>(&'b self, ctx: Ctx<'b>, allocator: &'b D) -> DocBuilder<'b, D, A>
    where
        D: DocAllocator<'b, A>,
        D::Doc: Clone,
        A: Clone,
    {
        self.callee_index.pretty(ctx, allocator).append(
            allocator
                .intersperse(
                    self.params.iter().map(|param| param.pretty(ctx, allocator)),
                    allocator.text(", "),
                )
                .parens(),
        )
    }
}

impl GetLocalExpression {
    fn pretty<'b, D, A>(&'b self, ctx: Ctx<'b>, allocator: &'b D) -> DocBuilder<'b, D, A>
    where
        D: DocAllocator<'b, A>,
        D::Doc: Clone,
        A: Clone,
    {
        allocator.text(&ctx.func.locals[self.local_index as usize].name)
    }
}

impl GetGlobalExpression {
    fn pretty<'b, D, A>(&'b self, ctx: Ctx<'b>, allocator: &'b D) -> DocBuilder<'b, D, A>
    where
        D: DocAllocator<'b, A>,
        D::Doc: Clone,
        A: Clone,
    {
        // TODO: Assign pretty names to globals
        allocator
            .text("globals")
            .append(allocator.text(self.global_index.to_string()).brackets())
    }
}

impl SelectExpression {
    fn pretty<'b, D, A>(&'b self, ctx: Ctx<'b>, allocator: &'b D) -> DocBuilder<'b, D, A>
    where
        D: DocAllocator<'b, A>,
        D::Doc: Clone,
        A: Clone,
    {
        self.condition
            .pretty(ctx, allocator)
            .append(allocator.space())
            .append(allocator.text("?"))
            .append(self.on_true.pretty(ctx, allocator))
            .append(allocator.text(":"))
            .append(self.on_false.pretty(ctx, allocator))
    }
}

impl MemoryLoadExpression {
    fn pretty<'b, D, A>(&'b self, ctx: Ctx<'b>, allocator: &'b D) -> DocBuilder<'b, D, A>
    where
        D: DocAllocator<'b, A>,
        D::Doc: Clone,
        A: Clone,
    {
        // TODO: offset
        allocator
            .text("memory")
            .append(self.index.pretty(ctx, allocator).brackets())
    }
}

impl MemoryGrowExpression {
    fn pretty<'b, D, A>(&'b self, ctx: Ctx<'b>, allocator: &'b D) -> DocBuilder<'b, D, A>
    where
        D: DocAllocator<'b, A>,
        D::Doc: Clone,
        A: Clone,
    {
        allocator
            .text("memory_grow")
            .append(self.value.pretty(ctx, allocator).parens())
    }
}

impl Func {
    fn pretty<'b, D, A>(&'b self, allocator: &'b D) -> DocBuilder<'b, D, A>
    where
        D: DocAllocator<'b, A>,
        D::Doc: Clone,
        A: Clone,
    {
        let mut items = vec![];
        for local in &self.locals {
            items.push(
                allocator
                    .text(&local.name)
                    .append(allocator.text(": "))
                    .append(allocator.text(local.ty.to_string())),
            );
        }
        for block in self.blocks.node_indices() {
            items.push(self.blocks.node_weight(block).unwrap().pretty(
                block,
                Ctx { func: self },
                allocator,
            ));
        }

        allocator
            .text(format!("func {}()", self.index))
            .append(allocator.space())
            .append(
                allocator
                    .intersperse(items, allocator.hardline().append(allocator.hardline()))
                    .indent(2)
                    .enclose(allocator.hardline(), allocator.hardline())
                    .braces(),
            )
    }
}

impl Module {
    pub fn pretty<'b, D, A>(&'b self, allocator: &'b D) -> DocBuilder<'b, D, A>
    where
        D: DocAllocator<'b, A>,
        D::Doc: Clone,
        A: Clone,
    {
        allocator
            .text("module")
            .append(allocator.space())
            .append(
                allocator
                    .intersperse(
                        self.funcs.iter().map(|func| func.pretty(allocator)),
                        allocator.hardline().append(allocator.hardline()),
                    )
                    .enclose(
                        allocator.hardline().append(allocator.hardline()),
                        allocator.hardline().append(allocator.hardline()),
                    )
                    .braces(),
            )
            .append(allocator.hardline())
    }
}
