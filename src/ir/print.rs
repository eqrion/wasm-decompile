use crate::ir::*;

#[derive(Clone, Copy)]
struct Ctx<'b> {
    func: &'b Func,
}

impl Block {
    fn pretty<'b, D, A>(
        &'b self,
        func: &Func,
        index: BlockIndex,
        is_last_block: bool,
        ctx: Ctx<'b>,
        allocator: &'b D,
    ) -> DocBuilder<'b, D, A>
    where
        D: DocAllocator<'b, A>,
        D::Doc: Clone,
        A: Clone,
    {
        // The entry block is guaranteed to be printed first. See assertion in printing Func.
        let is_entry_block = index == func.entry_block;
        // The entry block cannot have params, so we can skip printing the block label.
        assert!(!is_entry_block || self.params.is_empty());

        let mut instructions = vec![];
        for statement in &self.statements {
            instructions.push(statement.pretty(ctx, allocator));
        }
        // Skip an empty return in the last block
        if !is_last_block || !self.terminator.is_empty_return() {
            instructions.push(self.terminator.pretty(ctx, allocator));
        }

        let params = self.params.iter().enumerate().map(|(i, param)| {
            allocator
                .text(format!("b{}:", i))
                .append(allocator.space())
                .append(allocator.text(param.to_string()))
        });

        let label = if is_entry_block {
            allocator.nil()
        } else {
            allocator
                .text(format!("@{}", index.0))
                .append(if self.params.is_empty() {
                    allocator.nil()
                } else {
                    allocator.intersperse(params, allocator.text(", ")).parens()
                })
                .append(allocator.text(":"))
                .append(allocator.hardline())
        };

        label.append(
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
            Terminator::Br(target, params) => {
                let params = if params.is_empty() {
                    allocator.nil()
                } else {
                    allocator
                        .space()
                        .append(allocator.text("with"))
                        .append(allocator.space())
                        .append(
                            allocator
                                .intersperse(
                                    params.iter().map(|param| param.pretty(ctx, allocator)),
                                    allocator.text(", "),
                                )
                                .parens(),
                        )
                };

                allocator.text(format!("br @{}", target.0)).append(params)
            }
            Terminator::BrIf(condition, true_target, false_target, params) => {
                let params = if params.is_empty() {
                    allocator.nil()
                } else {
                    allocator
                        .space()
                        .append(allocator.text("with"))
                        .append(allocator.space())
                        .append(
                            allocator
                                .intersperse(
                                    params.iter().map(|param| param.pretty(ctx, allocator)),
                                    allocator.text(", "),
                                )
                                .parens(),
                        )
                };

                allocator
                    .text("if")
                    .append(allocator.space())
                    .append(condition.pretty(ctx, allocator))
                    .append(allocator.hardline())
                    .append(
                        allocator
                            .text(format!(" br @{}", true_target.0))
                            .append(params.clone())
                            .indent(2),
                    )
                    .append(allocator.hardline())
                    .append(
                        allocator
                            .text(format!("br @{}", false_target.0))
                            .append(params),
                    )
            }
            Terminator::BrTable(targets, default_target, params) => {
                let params = if params.is_empty() {
                    allocator.nil()
                } else {
                    allocator
                        .space()
                        .append(allocator.text("with"))
                        .append(allocator.space())
                        .append(
                            allocator
                                .intersperse(
                                    params.iter().map(|param| param.pretty(ctx, allocator)),
                                    allocator.text(", "),
                                )
                                .parens(),
                        )
                };

                let targets = allocator.intersperse(
                    targets.iter().map(|x| allocator.text(format!("@{}", x.0))),
                    allocator.text(", "),
                );

                allocator
                    .text("br_table")
                    .append(
                        targets
                            .append(
                                allocator
                                    .text(" default ")
                                    .append(allocator.text(format!("@{}", default_target.0))),
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
            Statement::LocalSetN(stmt) => stmt.pretty(ctx, allocator),
            Statement::GlobalSet(stmt) => stmt.pretty(ctx, allocator),
            Statement::MemoryStore(stmt) => stmt.pretty(ctx, allocator),
            Statement::If(stmt) => stmt.pretty(ctx, allocator),
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

impl LocalSetNStatement {
    fn pretty<'b, D, A>(&'b self, ctx: Ctx<'b>, allocator: &'b D) -> DocBuilder<'b, D, A>
    where
        D: DocAllocator<'b, A>,
        D::Doc: Clone,
        A: Clone,
    {
        allocator
            .intersperse(
                self.index
                    .iter()
                    .map(|x| allocator.text(&ctx.func.locals[*x as usize].name)),
                allocator.text(", "),
            )
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

impl IfStatement {
    fn pretty<'b, D, A>(&'b self, ctx: Ctx<'b>, allocator: &'b D) -> DocBuilder<'b, D, A>
    where
        D: DocAllocator<'b, A>,
        D::Doc: Clone,
        A: Clone,
    {
        allocator
            .text("if")
            .append(allocator.space())
            .append(self.condition.pretty(ctx, allocator).parens())
            .append(allocator.space())
            .append(
                allocator
                    .intersperse(
                        self.true_statements
                            .iter()
                            .map(|x| x.pretty(ctx, allocator)),
                        allocator.hardline(),
                    )
                    .indent(2)
                    .enclose(allocator.hardline(), allocator.hardline())
                    .braces(),
            )
            .append(allocator.space())
            .append(allocator.text("else"))
            .append(allocator.space())
            .append(
                allocator
                    .intersperse(
                        self.false_statements
                            .iter()
                            .map(|x| x.pretty(ctx, allocator)),
                        allocator.hardline(),
                    )
                    .indent(2)
                    .enclose(allocator.hardline(), allocator.hardline())
                    .braces(),
            )
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
            Expression::GetLocalN(expr) => expr.pretty(ctx, allocator),
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

impl GetLocalNExpression {
    fn pretty<'b, D, A>(&'b self, ctx: Ctx<'b>, allocator: &'b D) -> DocBuilder<'b, D, A>
    where
        D: DocAllocator<'b, A>,
        D::Doc: Clone,
        A: Clone,
    {
        allocator.intersperse(
            self.local_indices
                .iter()
                .map(|x| allocator.text(&ctx.func.locals[*x as usize].name)),
            allocator.text(", "),
        )
    }
}

impl GetGlobalExpression {
    fn pretty<'b, D, A>(&'b self, _ctx: Ctx<'b>, allocator: &'b D) -> DocBuilder<'b, D, A>
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
    pub fn pretty<'b, D, A>(&'b self, allocator: &'b D) -> DocBuilder<'b, D, A>
    where
        D: DocAllocator<'b, A>,
        D::Doc: Clone,
        A: Clone,
    {
        let params = self.ty.params();
        let num_params = params.len();

        let param_group = if params.is_empty() {
            allocator.nil()
        } else {
            let mut param_items = vec![];
            for param in &self.locals[0..num_params] {
                param_items.push(
                    allocator
                        .text(&param.name)
                        .append(allocator.text(": "))
                        .append(allocator.text(param.ty.to_string())),
                );
            }
            allocator.intersperse(param_items, allocator.text(", "))
        };

        let local_group = if self.locals.is_empty() {
            allocator.nil()
        } else {
            let mut local_items = vec![];
            for local in &self.locals[num_params..self.locals.len()] {
                local_items.push(
                    allocator
                        .text(&local.name)
                        .append(allocator.text(": "))
                        .append(allocator.text(local.ty.to_string())),
                );
            }
            allocator
                .intersperse(local_items, allocator.hardline())
                .indent(2)
                .enclose(allocator.hardline(), allocator.hardline())
        };

        let block_group = if self.blocks.is_empty() {
            allocator.nil()
        } else {
            let mut block_items = vec![];

            let visual_block_order = self.visual_block_order();
            assert!(self.entry_block == visual_block_order[0]);
            for index in &visual_block_order {
                let block = self.blocks.get(index).unwrap();
                let is_last_block = *index == visual_block_order[visual_block_order.len() - 1];
                block_items.push(block.pretty(
                    self,
                    *index,
                    is_last_block,
                    Ctx { func: self },
                    allocator,
                ));
            }

            allocator
                .intersperse(
                    block_items,
                    allocator.hardline().append(allocator.hardline()),
                )
                .enclose(allocator.hardline(), allocator.hardline())
        };

        let func_body = local_group.append(block_group).braces();

        allocator
            .text(format!("func {}", self.index))
            .append(param_group.parens())
            .append(allocator.space())
            .append(func_body)
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
