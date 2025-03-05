use std::any;

use anyhow::Ok;
use wasmparser::component_types::ResourceId;

use crate::ir::*;

struct Frame {
    kind: FrameKind,
    blockty: wasm::BlockType,
}

enum FrameKind {
    Func,
    Block {
        join_block: NodeIndex,
    },
    Loop {
        header_block: NodeIndex,
        join_block: NodeIndex,
    },
    If {
        true_block: NodeIndex,
        false_block: NodeIndex,
        join_block: NodeIndex,
    },
    Else {
        true_block: NodeIndex,
        false_block: NodeIndex,
        join_block: NodeIndex,
    },
}

struct Builder {
    func_index: u32,
    locals: Vec<Local>,
    frames: Vec<Frame>,
    stack: Vec<Expression>,
    validator: wasm::FuncValidator<wasm::ValidatorResources>,
    blocks: Graph<Block, ()>,
    start_block: NodeIndex,
    current_block: NodeIndex,
}

impl Builder {
    fn new(
        func_index: u32,
        locals: Vec<Local>,
        validator: wasm::FuncValidator<wasm::ValidatorResources>,
    ) -> Self {
        let mut blocks = Graph::new();
        let start_block = blocks.add_node(Block {
            params: Vec::new(),
            statements: Vec::new(),
            terminator: Terminator::Unknown,
        });

        Self {
            func_index,
            locals,
            frames: vec![Frame {
                kind: FrameKind::Func,
                blockty: wasm::BlockType::FuncType(
                    validator
                        .resources()
                        .type_index_of_function(func_index)
                        .unwrap(),
                ),
            }],
            stack: Vec::new(),
            validator,
            blocks,
            start_block,
            current_block: start_block,
        }
    }

    fn func_type(&self, type_index: u32) -> &wasm::FuncType {
        self.validator
            .resources()
            .sub_type_at(type_index)
            .unwrap()
            .composite_type
            .unwrap_func()
    }

    fn type_of_func(&self, func_index: u32) -> &wasm::FuncType {
        self.func_type(
            self.validator
                .resources()
                .type_index_of_function(func_index)
                .unwrap(),
        )
    }

    fn blockty_params(&self, blockty: wasm::BlockType) -> Vec<wasm::ValType> {
        match blockty {
            wasm::BlockType::Empty => vec![],
            wasm::BlockType::FuncType(type_index) => self.func_type(type_index).params().to_vec(),
            wasm::BlockType::Type(_) => vec![],
        }
    }

    fn blockty_results(&self, blockty: wasm::BlockType) -> Vec<wasm::ValType> {
        match blockty {
            wasm::BlockType::Empty => vec![],
            wasm::BlockType::FuncType(type_index) => self.func_type(type_index).results().to_vec(),
            wasm::BlockType::Type(ty) => vec![ty],
        }
    }

    fn push_frame(&mut self, frame: Frame) {
        self.frames.push(frame);
    }

    fn pop_frame(&mut self) -> Frame {
        self.frames.pop().unwrap()
    }

    fn push_block_params(&mut self, n: usize) {
        for i in 0..n {
            self.stack.push(Expression::BlockParam(i as u32));
        }
    }

    fn popn(&mut self, n: usize) -> Vec<Expression> {
        if n == 0 {
            return Vec::new();
        }
        let mut result = Vec::new();
        result.reserve(n);
        for _ in 0..n {
            result.push(self.stack.pop().unwrap());
        }
        result.reverse();
        result
    }

    fn op(
        &mut self,
        op_offset: usize,
        current_offset: usize,
        op: wasm::Operator,
    ) -> anyhow::Result<()> {
        let was_dead_code = self.validator.get_control_frame(0).unwrap().unreachable;
        self.validator.op(op_offset, &op)?;
        let is_dead_code = self.validator.control_stack_height() > 0
            && self.validator.get_control_frame(0).unwrap().unreachable;

        // Only do anything for the transition to dead code, or the transition out of dead code
        if was_dead_code && is_dead_code {
            return Ok(());
        }

        if let wasm::Operator::Block { blockty } = op {
            // Entering a block does not make us reachable if we weren't
            // reachable, and does not much us unreachable if we were reachable.
            assert!(!was_dead_code && !is_dead_code);

            let block_params = self.blockty_params(blockty);
            let block_results = self.blockty_results(blockty);
            let block_params_count = block_params.len();

            // Create a join block and add a frame for this block
            let join_block = self.blocks.add_node(Block {
                params: block_results,
                statements: Vec::new(),
                terminator: Terminator::Unknown,
            });

            // Create and move to the 'inner_block' block
            let inner_block = self.blocks.add_node(Block {
                params: block_params,
                statements: Vec::new(),
                terminator: Terminator::Unknown,
            });

            // Pop the params that are passed to the inner block, and re-push them as block params
            let results = self.popn(block_params_count);
            self.push_block_params(block_params_count);

            // Jump to the inner block
            let current_block_ref = self.blocks.node_weight_mut(self.current_block).unwrap();
            current_block_ref.terminator = Terminator::Br(inner_block, results);
            self.current_block = inner_block;

            // Push the block frame
            self.push_frame(Frame {
                kind: FrameKind::Block { join_block },
                blockty,
            });
        } else if let wasm::Operator::Loop { blockty } = op {
            // Entering a loop does not make us reachable if we weren't
            // reachable, and does not much us unreachable if we were reachable.
            assert!(!was_dead_code && !is_dead_code);

            let block_params = self.blockty_params(blockty);
            let block_results = self.blockty_results(blockty);
            let block_params_count = block_params.len();

            // Create the 'loop_header' block
            let header_block = self.blocks.add_node(Block {
                params: block_params,
                statements: Vec::new(),
                terminator: Terminator::Unknown,
            });

            // Create a join block
            let join_block = self.blocks.add_node(Block {
                params: block_results,
                statements: Vec::new(),
                terminator: Terminator::Unknown,
            });

            // Pop the params that are passed to the loop header block, and re-push them as block params
            let results = self.popn(block_params_count);
            self.push_block_params(block_params_count);

            // Move to the loop header block
            let current_block_ref = self.blocks.node_weight_mut(self.current_block).unwrap();
            current_block_ref.terminator = Terminator::Br(header_block, results);
            self.current_block = header_block;

            // Push the loop frame
            self.push_frame(Frame {
                kind: FrameKind::Loop {
                    header_block,
                    join_block,
                },
                blockty,
            });
        } else if let wasm::Operator::If { blockty } = op {
            // Entering an if does not make us reachable if we weren't
            // reachable, and does not much us unreachable if we were reachable.
            assert!(!was_dead_code && !is_dead_code);

            let block_params = self.blockty_params(blockty);
            let block_results = self.blockty_results(blockty);
            let block_params_count = block_params.len();

            // Create the true, false, and join blocks
            let true_block = self.blocks.add_node(Block {
                params: block_params.clone(),
                statements: Vec::new(),
                terminator: Terminator::Unknown,
            });
            let false_block = self.blocks.add_node(Block {
                params: block_params,
                statements: Vec::new(),
                terminator: Terminator::Unknown,
            });
            let join_block = self.blocks.add_node(Block {
                params: block_results,
                statements: Vec::new(),
                terminator: Terminator::Unknown,
            });

            let condition = self.stack.pop().unwrap();
            // Pop the params to the true block, and re-push them as block params
            let results = self.popn(block_params_count);
            self.push_block_params(block_params_count);

            // Terminate the if predecessor block with br_if(true, false) and then move to the 'true_block'
            let current_block_ref = self.blocks.node_weight_mut(self.current_block).unwrap();
            current_block_ref.terminator =
                Terminator::BrIf(condition, true_block, false_block, results);
            self.current_block = true_block;

            // Push the if frame
            self.push_frame(Frame {
                kind: FrameKind::If {
                    true_block,
                    false_block,
                    join_block,
                },
                blockty,
            });
        } else if let wasm::Operator::Else = op {
            // Entering an if may change us from being in dead code to being in
            // reachable code.
            assert!(!is_dead_code);

            let frame = self.pop_frame();
            let block_params_count = self.blockty_params(frame.blockty).len();
            let block_results_count = self.blockty_results(frame.blockty).len();
            let (true_block, false_block, join_block) = match frame.kind {
                FrameKind::If {
                    true_block,
                    false_block,
                    join_block,
                    ..
                } => (true_block, false_block, join_block),
                _ => unreachable!(),
            };
            self.push_frame(Frame {
                kind: FrameKind::Else {
                    true_block,
                    false_block,
                    join_block,
                },
                blockty: frame.blockty,
            });

            // TODO: handle unreachable

            // Pop this block's results and pass them to the join block.
            let results = self.popn(block_results_count);
            // Re-push this block's params.
            self.push_block_params(block_params_count);

            // Terminate the true block with br(join) and then move to the 'false_block'
            let current_block_ref = self.blocks.node_weight_mut(self.current_block).unwrap();
            current_block_ref.terminator = Terminator::Br(join_block, results);
            self.current_block = false_block;
        } else if let wasm::Operator::End = op {
            // We must now be in reachable code
            assert!(!is_dead_code);

            let frame = self.pop_frame();
            let block_results_count = self.blockty_results(frame.blockty).len();
            let results = self.popn(block_results_count);

            match frame.kind {
                FrameKind::Func => {
                    // Terminate the function with a return
                    let current_block_ref =
                        self.blocks.node_weight_mut(self.current_block).unwrap();
                    current_block_ref.terminator = Terminator::Return(results);

                    self.validator.finish(current_offset)?;
                }
                FrameKind::Block { join_block } => {
                    // Terminate with a br to the join block
                    let current_block_ref =
                        self.blocks.node_weight_mut(self.current_block).unwrap();
                    current_block_ref.terminator = Terminator::Br(join_block, results);
                    self.current_block = join_block;
                    self.push_block_params(block_results_count);
                }
                FrameKind::Loop {
                    header_block: _,
                    join_block,
                } => {
                    // Terminate with a br to the join block
                    let current_block_ref =
                        self.blocks.node_weight_mut(self.current_block).unwrap();
                    current_block_ref.terminator = Terminator::Br(join_block, results);
                    self.current_block = join_block;
                    self.push_block_params(block_results_count);
                }
                FrameKind::If {
                    true_block: _,
                    false_block,
                    join_block,
                } => {
                    // There was no 'else', so finish up that block with just a br(join_block)
                    let false_block_ref = self.blocks.node_weight_mut(false_block).unwrap();
                    false_block_ref.terminator = Terminator::Br(join_block, results);

                    // Only valid to omit an 'else' if the block has the same number of params as results
                    let block_params_count = block_results_count;
                    self.push_block_params(block_params_count);
                    let results = self.popn(block_results_count);

                    // Terminate with a br(join_block) and move to the join block
                    let current_block_ref =
                        self.blocks.node_weight_mut(self.current_block).unwrap();
                    current_block_ref.terminator = Terminator::Br(join_block, results);
                    self.current_block = join_block;
                    self.push_block_params(block_results_count);
                }
                FrameKind::Else {
                    true_block: _,
                    false_block: _,
                    join_block,
                } => {
                    // Terminate with a br(join_block) and move to the join block
                    let current_block_ref =
                        self.blocks.node_weight_mut(self.current_block).unwrap();
                    current_block_ref.terminator = Terminator::Br(join_block, results);
                    self.current_block = join_block;
                    self.push_block_params(block_results_count);
                }
            }
        } else {
            assert!(!was_dead_code && !is_dead_code);

            if let Some(statement) = self.statement_op(op)? {
                let current_block_ref = self.blocks.node_weight_mut(self.current_block).unwrap();
                current_block_ref.statements.push(statement);
            }
        }

        Ok(())
    }

    fn statement_op(&mut self, op: wasm::Operator) -> anyhow::Result<Option<Statement>> {
        match op {
            wasm::Operator::Nop => Ok(Some(Statement::Nop)),
            wasm::Operator::Drop => {
                let value = self.stack.pop().unwrap();
                Ok(Some(Statement::Drop(value)))
            }
            wasm::Operator::LocalSet { local_index } => {
                let value = self.stack.pop().unwrap();

                Ok(Some(Statement::LocalSet(LocalSetStatement {
                    index: local_index,
                    value: Box::new(value),
                })))
            }
            wasm::Operator::GlobalSet { global_index } => {
                let value = self.stack.pop().unwrap();

                Ok(Some(Statement::GlobalSet(GlobalSetStatement {
                    index: global_index,
                    value: Box::new(value),
                })))
            }
            wasm::Operator::I32Store { memarg }
            | wasm::Operator::I32Store16 { memarg }
            | wasm::Operator::I32Store8 { memarg }
            | wasm::Operator::I64Store { memarg }
            | wasm::Operator::I64Store32 { memarg }
            | wasm::Operator::I64Store16 { memarg }
            | wasm::Operator::F32Store { memarg }
            | wasm::Operator::F64Store { memarg } => {
                let value = self.stack.pop().unwrap();
                let index = self.stack.pop().unwrap();
                Ok(Some(Statement::MemoryStore(MemoryStoreStatement {
                    arg: memarg,
                    index: Box::new(index),
                    value: Box::new(value),
                })))
            }
            wasm::Operator::Call { function_index } => {
                let func_type = self.type_of_func(function_index);
                let result_count = func_type.results().len();
                let params = self.popn(func_type.params().len());

                let call = CallExpression {
                    func_index: function_index,
                    params,
                };

                if result_count == 0 {
                    Ok(Some(Statement::Call(call)))
                } else {
                    if result_count == 1 {
                        self.stack.push(Expression::Call(call));
                    } else {
                        unimplemented!()
                    }
                    Ok(None)
                }
            }
            wasm::Operator::CallIndirect {
                type_index,
                table_index,
            } => {
                let callee_index = Box::new(self.stack.pop().unwrap());
                let func_type = self.func_type(type_index);
                let result_count = func_type.results().len();
                let params = self.popn(func_type.params().len());

                let call = CallIndirectExpression {
                    func_type_index: type_index,
                    table_index,
                    callee_index,
                    params,
                };

                if result_count == 0 {
                    Ok(Some(Statement::CallIndirect(call)))
                } else {
                    if result_count == 1 {
                        self.stack.push(Expression::CallIndirect(call));
                    } else {
                        unimplemented!()
                    }
                    Ok(None)
                }
            }
            _ => {
                self.expr_op(op);
                Ok(None)
            }
        }
    }

    fn expr_op(&mut self, op: wasm::Operator) {
        match op {
            wasm::Operator::I32Const { value } => {
                self.stack.push(Expression::I32Const { value });
            }
            wasm::Operator::I64Const { value } => {
                self.stack.push(Expression::I64Const { value });
            }
            wasm::Operator::F32Const { value } => {
                self.stack.push(Expression::F32Const { value });
            }
            wasm::Operator::F64Const { value } => {
                self.stack.push(Expression::F64Const { value });
            }
            wasm::Operator::Call { .. } | wasm::Operator::CallIndirect { .. } => {
                // Handled in statement_op
                unreachable!()
            }
            wasm::Operator::LocalGet { local_index } => {
                self.stack
                    .push(Expression::GetLocal(GetLocalExpression { local_index }));
            }
            wasm::Operator::GlobalGet { global_index } => {
                self.stack
                    .push(Expression::GetGlobal(GetGlobalExpression { global_index }));
            }
            wasm::Operator::Select => {
                let cond = self.stack.pop().unwrap();
                let false_expr = self.stack.pop().unwrap();
                let true_expr = self.stack.pop().unwrap();
                self.stack.push(Expression::Select(SelectExpression {
                    condition: Box::new(cond),
                    on_false: Box::new(false_expr),
                    on_true: Box::new(true_expr),
                }));
            }
            wasm::Operator::I32Load { memarg }
            | wasm::Operator::I32Load8S { memarg }
            | wasm::Operator::I32Load8U { memarg }
            | wasm::Operator::I32Load16S { memarg }
            | wasm::Operator::I32Load16U { memarg }
            | wasm::Operator::I64Load { memarg }
            | wasm::Operator::I64Load8S { memarg }
            | wasm::Operator::I64Load8U { memarg }
            | wasm::Operator::I64Load16S { memarg }
            | wasm::Operator::I64Load16U { memarg }
            | wasm::Operator::I64Load32S { memarg }
            | wasm::Operator::I64Load32U { memarg } => {
                let index = self.stack.pop().unwrap();
                self.stack
                    .push(Expression::MemoryLoad(MemoryLoadExpression {
                        arg: memarg,
                        index: Box::new(index),
                    }));
            }
            wasm::Operator::MemorySize { mem } => {
                self.stack.push(Expression::MemorySize);
            }
            wasm::Operator::MemoryGrow { mem } => {
                let value = self.stack.pop().unwrap();
                self.stack
                    .push(Expression::MemoryGrow(MemoryGrowExpression {
                        value: Box::new(value),
                    }));
            }

            // Unary operators
            wasm::Operator::I32Eqz
            | wasm::Operator::I64Eqz
            | wasm::Operator::I32Clz
            | wasm::Operator::I32Ctz
            | wasm::Operator::I32Popcnt
            | wasm::Operator::I64Clz
            | wasm::Operator::I64Ctz
            | wasm::Operator::I64Popcnt
            | wasm::Operator::F32Abs
            | wasm::Operator::F32Neg
            | wasm::Operator::F32Ceil
            | wasm::Operator::F32Floor
            | wasm::Operator::F32Trunc
            | wasm::Operator::F32Nearest
            | wasm::Operator::F32Sqrt
            | wasm::Operator::F32Copysign
            | wasm::Operator::F64Abs
            | wasm::Operator::F64Neg
            | wasm::Operator::F64Ceil
            | wasm::Operator::F64Floor
            | wasm::Operator::F64Trunc
            | wasm::Operator::F64Nearest
            | wasm::Operator::F64Sqrt
            | wasm::Operator::F64Copysign
            | wasm::Operator::I32WrapI64
            | wasm::Operator::I32TruncF32S
            | wasm::Operator::I32TruncF32U
            | wasm::Operator::I32TruncF64S
            | wasm::Operator::I32TruncF64U
            | wasm::Operator::I64ExtendI32S
            | wasm::Operator::I64ExtendI32U
            | wasm::Operator::I64TruncF32S
            | wasm::Operator::I64TruncF32U
            | wasm::Operator::I64TruncF64S
            | wasm::Operator::I64TruncF64U
            | wasm::Operator::F32ConvertI32S
            | wasm::Operator::F32ConvertI32U
            | wasm::Operator::F32ConvertI64S
            | wasm::Operator::F32ConvertI64U
            | wasm::Operator::F32DemoteF64
            | wasm::Operator::F64ConvertI32S
            | wasm::Operator::F64ConvertI32U
            | wasm::Operator::F64ConvertI64S
            | wasm::Operator::F64ConvertI64U
            | wasm::Operator::F64PromoteF32
            | wasm::Operator::I32ReinterpretF32
            | wasm::Operator::I64ReinterpretF64
            | wasm::Operator::F32ReinterpretI32
            | wasm::Operator::F64ReinterpretI64
            | wasm::Operator::I32Extend8S
            | wasm::Operator::I32Extend16S
            | wasm::Operator::I64Extend8S
            | wasm::Operator::I64Extend16S
            | wasm::Operator::I64Extend32S => {
                let value = Box::new(self.stack.pop().unwrap());
                self.stack.push(Expression::Unary(op.into(), value));
            }

            // Binary operators
            wasm::Operator::I32Eq
            | wasm::Operator::I32Ne
            | wasm::Operator::I32LtS
            | wasm::Operator::I32LtU
            | wasm::Operator::I32GtS
            | wasm::Operator::I32GtU
            | wasm::Operator::I32LeS
            | wasm::Operator::I32LeU
            | wasm::Operator::I32GeS
            | wasm::Operator::I32GeU
            | wasm::Operator::I64Eq
            | wasm::Operator::I64Ne
            | wasm::Operator::I64LtS
            | wasm::Operator::I64LtU
            | wasm::Operator::I64GtS
            | wasm::Operator::I64GtU
            | wasm::Operator::I64LeS
            | wasm::Operator::I64LeU
            | wasm::Operator::I64GeS
            | wasm::Operator::I64GeU
            | wasm::Operator::F32Eq
            | wasm::Operator::F32Ne
            | wasm::Operator::F32Lt
            | wasm::Operator::F32Gt
            | wasm::Operator::F32Le
            | wasm::Operator::F32Ge
            | wasm::Operator::F64Eq
            | wasm::Operator::F64Ne
            | wasm::Operator::F64Lt
            | wasm::Operator::F64Gt
            | wasm::Operator::F64Le
            | wasm::Operator::F64Ge
            | wasm::Operator::I32Add
            | wasm::Operator::I32Sub
            | wasm::Operator::I32Mul
            | wasm::Operator::I32DivS
            | wasm::Operator::I32DivU
            | wasm::Operator::I32RemS
            | wasm::Operator::I32RemU
            | wasm::Operator::I32And
            | wasm::Operator::I32Or
            | wasm::Operator::I32Xor
            | wasm::Operator::I32Shl
            | wasm::Operator::I32ShrS
            | wasm::Operator::I32ShrU
            | wasm::Operator::I32Rotl
            | wasm::Operator::I32Rotr
            | wasm::Operator::I64Add
            | wasm::Operator::I64Sub
            | wasm::Operator::I64Mul
            | wasm::Operator::I64DivS
            | wasm::Operator::I64DivU
            | wasm::Operator::I64RemS
            | wasm::Operator::I64RemU
            | wasm::Operator::I64And
            | wasm::Operator::I64Or
            | wasm::Operator::I64Xor
            | wasm::Operator::I64Shl
            | wasm::Operator::I64ShrS
            | wasm::Operator::I64ShrU
            | wasm::Operator::I64Rotl
            | wasm::Operator::I64Rotr
            | wasm::Operator::F32Add
            | wasm::Operator::F32Sub
            | wasm::Operator::F32Mul
            | wasm::Operator::F32Div
            | wasm::Operator::F32Min
            | wasm::Operator::F32Max
            | wasm::Operator::F64Add
            | wasm::Operator::F64Sub
            | wasm::Operator::F64Mul
            | wasm::Operator::F64Div
            | wasm::Operator::F64Min
            | wasm::Operator::F64Max => {
                let rhs = Box::new(self.stack.pop().unwrap());
                let lhs = Box::new(self.stack.pop().unwrap());
                self.stack.push(Expression::Binary(op.into(), lhs, rhs));
            }

            _ => todo!(),
        }
    }

    fn finish(self) -> anyhow::Result<Func> {
        Ok(Func {
            index: self.func_index,
            ty: self.type_of_func(self.func_index).clone(),
            locals: self.locals,
            blocks: self.blocks,
            entry_block: self.start_block,
        })
    }
}

impl Func {
    pub fn decode(
        body: wasm::FunctionBody,
        func_to_validate: wasm::FuncToValidate<wasm::ValidatorResources>,
    ) -> anyhow::Result<Self> {
        let index = func_to_validate.index;
        let mut body_validator =
            func_to_validate.into_validator(FuncValidatorAllocations::default());

        let locals_reader = body.get_locals_reader()?;
        let mut locals = Vec::new();
        for local in locals_reader {
            let (count, ty) = local?;
            for _ in 0..count {
                let prefix = match ty {
                    wasmparser::ValType::I32 | wasmparser::ValType::I64 => "i",
                    wasmparser::ValType::F32 | wasmparser::ValType::F64 => "f",
                    wasmparser::ValType::V128 => "v",
                    wasmparser::ValType::Ref(_) => "r",
                };
                let name = format!("{}{}", prefix, locals.len());
                locals.push(Local { ty, name });
            }
            body_validator.define_locals(body.get_binary_reader().current_position(), count, ty)?;
        }

        let mut builder = Builder::new(index, locals, body_validator);

        let mut operator_reader = body.get_operators_reader()?;
        while !operator_reader.eof() {
            let (op, offset) = operator_reader.read_with_offset()?;
            builder.op(offset, operator_reader.original_position(), op)?;
        }
        operator_reader.ensure_end()?;

        builder.finish()
    }
}
