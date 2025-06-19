use std::collections::HashMap;

use pretty::block;

use crate::ir::*;

struct Frame {
    kind: FrameKind,
    blockty: wasm::BlockType,
    unreachable: bool,
    stack_height: usize,
}

enum FrameKind {
    Func,
    Block {
        join_block: BlockIndex,
    },
    Loop {
        header_block: BlockIndex,
        join_block: BlockIndex,
    },
    If {
        true_block: BlockIndex,
        false_block: BlockIndex,
        join_block: BlockIndex,
    },
    Else {
        true_block: BlockIndex,
        false_block: BlockIndex,
        join_block: BlockIndex,
    },
}

impl FrameKind {
    fn is_func(&self) -> bool {
        matches!(self, FrameKind::Func)
    }

    fn branch_target_block(&self) -> BlockIndex {
        match self {
            FrameKind::Block { join_block } => *join_block,
            FrameKind::Loop { header_block, .. } => *header_block,
            FrameKind::If { join_block, .. } => *join_block,
            FrameKind::Else { join_block, .. } => *join_block,
            // Callers must handle this to manually emit a return
            FrameKind::Func => unreachable!(),
        }
    }
}

struct Builder {
    func_index: u32,
    func_type: wasm::FuncType,
    locals: Vec<Local>,
    temp_count: u32,
    frames: Vec<Frame>,
    stack: Vec<Expression>,
    validator: wasm::FuncValidator<wasm::ValidatorResources>,
    blocks: HashMap<BlockIndex, Block>,
    start_block: BlockIndex,
    current_block: BlockIndex,
    return_block: BlockIndex,
    next_block_index: BlockIndex,
}

impl Builder {
    fn new(
        func_index: u32,
        mut locals: Vec<Local>,
        validator: wasm::FuncValidator<wasm::ValidatorResources>,
    ) -> Self {
        let func_type = validator
            .resources()
            .sub_type_at(
                validator
                    .resources()
                    .type_index_of_function(func_index)
                    .unwrap(),
            )
            .unwrap()
            .composite_type
            .unwrap_func()
            .clone();

        let mut blocks = HashMap::new();

        let start_block_index = BlockIndex(0);
        let start_block = Block {
            params: Vec::new(),
            statements: Vec::new(),
            terminator: Terminator::Unknown,
        };
        blocks.insert(start_block_index, start_block);

        let return_block_results = func_type
            .results()
            .iter()
            .enumerate()
            .map(|(i, ty)| Expression::BlockParam(i as u32))
            .collect();
        let return_block_index = BlockIndex(1);
        let return_block = Block {
            params: func_type.results().to_vec(),
            statements: Vec::new(),
            terminator: Terminator::Return(return_block_results),
        };
        blocks.insert(return_block_index, return_block);

        let mut locals_with_args = Vec::new();
        for (i, param) in func_type.params().iter().enumerate() {
            locals_with_args.push(Local {
                ty: *param,
                name: format!("arg{}", i),
            });
        }
        locals_with_args.append(&mut locals);

        Self {
            func_index,
            func_type,
            locals: locals_with_args,
            temp_count: 0,
            frames: vec![Frame {
                kind: FrameKind::Func,
                unreachable: false,
                stack_height: 0,
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
            start_block: start_block_index,
            current_block: start_block_index,
            return_block: return_block_index,
            next_block_index: BlockIndex(2),
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

    fn expr_type(&self, expression: &Expression, in_block: &Block) -> Vec<wasm::ValType> {
        match expression {
            Expression::I32Const { .. } => vec![wasm::ValType::I32],
            Expression::I64Const { .. } => vec![wasm::ValType::I64],
            Expression::F32Const { .. } => vec![wasm::ValType::F32],
            Expression::F64Const { .. } => vec![wasm::ValType::F64],
            Expression::GetLocal(GetLocalExpression { local_index }) => {
                vec![self.locals[*local_index as usize].ty]
            }
            Expression::GetLocalN(GetLocalNExpression { local_indices }) => local_indices
                .iter()
                .map(|x| self.locals[*x as usize].ty)
                .collect(),
            Expression::GetGlobal(GetGlobalExpression { global_index }) => {
                vec![
                    self.validator
                        .resources()
                        .global_at(*global_index)
                        .unwrap()
                        .content_type,
                ]
            }
            Expression::Call(CallExpression { func_index, .. }) => {
                self.type_of_func(*func_index).results().to_vec()
            }
            Expression::CallIndirect(CallIndirectExpression {
                func_type_index, ..
            }) => self.func_type(*func_type_index).results().to_vec(),
            Expression::MemorySize => {
                // TODO
                vec![wasm::ValType::I32]
            }
            Expression::MemoryGrow(_) => vec![wasm::ValType::I32],
            Expression::MemoryLoad(MemoryLoadExpression { arg, .. }) => {
                // TODO
                vec![wasm::ValType::I32]
            }
            Expression::Unary(op, _) => vec![op.result_type()],
            Expression::Binary(op, _, _) => vec![op.result_type()],
            Expression::Select(op) => {
                let on_true = self.expr_type(&op.on_true, in_block);
                let on_false = self.expr_type(&op.on_false, in_block);
                assert_eq!(on_true, on_false);
                on_true
            }
            Expression::BlockParam(i) => {
                vec![in_block.params[*i as usize]]
            }
            Expression::Bottom => vec![],
        }
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

    fn blockty_params_count(&self, blockty: wasm::BlockType) -> usize {
        match blockty {
            wasm::BlockType::Empty => 0,
            wasm::BlockType::FuncType(type_index) => self.func_type(type_index).params().len(),
            wasm::BlockType::Type(_) => 0,
        }
    }

    fn blockty_results_count(&self, blockty: wasm::BlockType) -> usize {
        match blockty {
            wasm::BlockType::Empty => 0,
            wasm::BlockType::FuncType(type_index) => self.func_type(type_index).results().len(),
            wasm::BlockType::Type(_) => 1,
        }
    }

    fn add_block(&mut self, node: Block) -> BlockIndex {
        let block_index = self.next_block_index;
        self.next_block_index = BlockIndex(block_index.0 + 1);
        self.blocks.insert(block_index, node);
        block_index
    }

    fn push_frame(&mut self, frame: Frame) {
        self.frames.push(frame);
    }

    fn pop_frame(&mut self) -> Frame {
        self.frames.pop().unwrap()
    }

    fn frame_at(&self, relative_depth: u32) -> &Frame {
        assert!((relative_depth as usize) < self.frames.len());
        let index = self.frames.len() - relative_depth as usize - 1;
        &self.frames[index]
    }

    fn frame_at_mut(&mut self, relative_depth: u32) -> &mut Frame {
        assert!((relative_depth as usize) < self.frames.len());
        let index = self.frames.len() - relative_depth as usize - 1;
        &mut self.frames[index]
    }

    fn frame_unreachable(&self, relative_depth: u32) -> bool {
        self.frame_at(relative_depth).unreachable
    }

    fn branch_target_block(&self, relative_depth: u32) -> BlockIndex {
        let frame = self.frame_at(relative_depth);
        if frame.kind.is_func() {
            self.return_block
        } else {
            frame.kind.branch_target_block()
        }
    }

    fn push_block_params(&mut self, n: usize) {
        for i in 0..n {
            self.stack.push(Expression::BlockParam(i as u32));
        }
    }

    fn pop_branch_params(&mut self, relative_depth: u32) -> Vec<Expression> {
        let frame = self.frame_at(relative_depth);
        let count = match frame.kind {
            FrameKind::Block { join_block: _ } => self.blockty_results_count(frame.blockty),
            FrameKind::Loop {
                header_block: _,
                join_block: _,
            } => self.blockty_params_count(frame.blockty),
            FrameKind::If {
                true_block: _,
                false_block: _,
                join_block: _,
            } => self.blockty_results_count(frame.blockty),
            FrameKind::Else {
                true_block: _,
                false_block: _,
                join_block: _,
            } => self.blockty_results_count(frame.blockty),
            FrameKind::Func => self.func_type.results().len(),
        };
        self.popn(count)
    }

    fn after_unconditional_branch(&mut self) {
        let frame = self.frames.last_mut().unwrap();
        assert!(!frame.unreachable);
        let block = self.blocks.get_mut(&self.current_block).unwrap();

        // TODO
        // assert!(block.terminator != Terminator::Unknown);

        // Emit drop statements for all the expressions on the stack
        // that would get clobbered by the unconditional branch
        let dropped_values = self.stack.drain(frame.stack_height..);
        for value in dropped_values {
            block.statements.push(Statement::Drop(value));
        }

        // We don't need to truncate after manually dropping all those expressions
        assert!(self.stack.len() == frame.stack_height);
        frame.unreachable = true;
    }

    fn pop(&mut self) -> Expression {
        if self.stack.is_empty() {
            assert!(self.frame_unreachable(0));
            Expression::Bottom
        } else {
            self.stack.pop().unwrap()
        }
    }

    fn popn(&mut self, n: usize) -> Vec<Expression> {
        if n == 0 {
            return Vec::new();
        }
        let mut result = Vec::new();
        result.reserve(n);
        for _ in 0..n {
            if self.stack.is_empty() {
                assert!(self.frame_unreachable(0));
                result.push(Expression::Bottom);
            } else {
                result.push(self.stack.pop().unwrap());
            }
        }
        result.reverse();
        result
    }

    fn sync_stack_before_statement(&mut self) {
        let frame = self.frames.last_mut().unwrap();
        for i in frame.stack_height..self.stack.len() {
            let expr_type = self.expr_type(
                &self.stack[i],
                self.blocks.get(&self.current_block).unwrap(),
            );
            if expr_type.is_empty() {
                assert!(matches!(self.stack[i], Expression::Bottom));
                continue;
            }

            let temps_needed = expr_type.len() as u32;
            let local_start_index = self.locals.len() as u32;
            let temp_start_index = self.temp_count;
            self.temp_count += temps_needed;

            // Add temp locals for all the expressions on the stack
            let mut local_indices = Vec::new();
            for l in 0..temps_needed {
                self.locals.push(Local {
                    ty: expr_type[l as usize],
                    name: format!("temp{}", temp_start_index + l),
                });
                local_indices.push(local_start_index + l);
            }

            // Replace the expression on the stack with a GetLocalN expression
            let replacement_expr = Expression::GetLocalN(GetLocalNExpression {
                local_indices: local_indices.clone(),
            });
            // Swap it in on the expression stack, grabbing the original value
            let init_temp_value = std::mem::replace(&mut self.stack[i], replacement_expr);

            // Add a LocalSetN statement to initialize the temp local
            let block = self.blocks.get_mut(&self.current_block).unwrap();
            block
                .statements
                .push(Statement::LocalSetN(LocalSetNStatement {
                    index: local_indices,
                    value: Box::new(init_temp_value),
                }));
        }
    }

    fn check_stack_for_block(&mut self, block_params: usize) -> Vec<Expression> {
        let results = self.popn(block_params);
        self.sync_stack_before_statement();
        self.push_block_params(block_params);
        results
    }

    fn visit_op(
        &mut self,
        op_offset: usize,
        current_offset: usize,
        op: wasm::Operator,
    ) -> anyhow::Result<()> {
        self.validator.op(op_offset, &op)?;

        match op {
            wasm::Operator::Block { blockty } => {
                self.visit_block_op(blockty);
            }
            wasm::Operator::Loop { blockty } => {
                self.visit_loop_op(blockty);
            }
            wasm::Operator::If { blockty } => {
                self.visit_if_op(blockty);
            }
            wasm::Operator::Else => {
                self.visit_else_op();
            }
            wasm::Operator::End => {
                self.visit_end_op(current_offset)?;
            }
            wasm::Operator::Unreachable => {
                // If our current frame is in unreachable code, don't codegen anything
                if self.frame_unreachable(0) {
                    return Ok(());
                }

                self.visit_unreachable_op();
            }
            wasm::Operator::Return => {
                // If our current frame is in unreachable code, don't codegen anything
                if self.frame_unreachable(0) {
                    return Ok(());
                }

                self.visit_return_op();
            }
            wasm::Operator::Br { relative_depth } => {
                // If our current frame is in unreachable code, don't codegen anything
                if self.frame_unreachable(0) {
                    return Ok(());
                }

                self.visit_br_op(relative_depth);
            }
            wasm::Operator::BrIf { relative_depth } => {
                // If our current frame is in unreachable code, don't codegen anything
                if self.frame_unreachable(0) {
                    return Ok(());
                }

                self.visit_br_if_op(relative_depth);
            }
            wasm::Operator::BrTable { targets } => {
                // If our current frame is in unreachable code, don't codegen anything
                if self.frame_unreachable(0) {
                    return Ok(());
                }

                self.visit_br_table_op(targets)?;
            }
            _ => {
                // If our current frame is in unreachable code, don't codegen anything
                if self.frame_unreachable(0) {
                    return Ok(());
                }

                self.visit_statement_op(op);
            }
        }

        Ok(())
    }

    fn visit_block_op(&mut self, blockty: wasm::BlockType) {
        let block_params = self.blockty_params(blockty);
        let block_results = self.blockty_results(blockty);
        let block_params_count = block_params.len();

        // Create the inner block that will contain the block's body
        let inner_block = self.add_block(Block {
            params: block_params,
            statements: Vec::new(),
            terminator: Terminator::Unknown,
        });

        // Create a join block
        let join_block = self.add_block(Block {
            params: block_results,
            statements: Vec::new(),
            terminator: Terminator::Unknown,
        });

        // Get the block params and the value stack height
        let results = self.check_stack_for_block(block_params_count);
        let stack_height = self.stack.len() - block_params_count;

        // Jump to the inner block
        let current_block_ref = self.blocks.get_mut(&self.current_block).unwrap();
        current_block_ref.terminator = Terminator::Br(inner_block, results);
        self.current_block = inner_block;

        // Push the block frame
        self.push_frame(Frame {
            kind: FrameKind::Block { join_block },
            unreachable: false,
            stack_height,
            blockty,
        });
    }

    fn visit_loop_op(&mut self, blockty: wasm::BlockType) {
        let block_params = self.blockty_params(blockty);
        let block_results = self.blockty_results(blockty);
        let block_params_count = block_params.len();

        // Create the 'loop_header' block
        let header_block = self.add_block(Block {
            params: block_params,
            statements: Vec::new(),
            terminator: Terminator::Unknown,
        });

        // Create a join block
        let join_block = self.add_block(Block {
            params: block_results,
            statements: Vec::new(),
            terminator: Terminator::Unknown,
        });

        // Get the block params and the value stack height
        let results = self.check_stack_for_block(block_params_count);
        let stack_height = self.stack.len() - block_params_count;

        // Move to the loop header block
        let current_block_ref = self.blocks.get_mut(&self.current_block).unwrap();
        current_block_ref.terminator = Terminator::Br(header_block, results);
        self.current_block = header_block;

        // Push the loop frame
        self.push_frame(Frame {
            kind: FrameKind::Loop {
                header_block,
                join_block,
            },
            unreachable: false,
            stack_height,
            blockty,
        });
    }

    fn visit_if_op(&mut self, blockty: wasm::BlockType) {
        let block_params = self.blockty_params(blockty);
        let block_results = self.blockty_results(blockty);
        let block_params_count = block_params.len();

        // Create the true, false, and join blocks
        let true_block = self.add_block(Block {
            params: block_params.clone(),
            statements: Vec::new(),
            terminator: Terminator::Unknown,
        });
        let false_block = self.add_block(Block {
            params: block_params,
            statements: Vec::new(),
            terminator: Terminator::Unknown,
        });
        let join_block = self.add_block(Block {
            params: block_results,
            statements: Vec::new(),
            terminator: Terminator::Unknown,
        });

        let condition = self.pop();

        // Get the block params and the value stack height
        let results = self.check_stack_for_block(block_params_count);
        let stack_height = self.stack.len() - block_params_count;

        // Terminate the if predecessor block with br_if(true, false) and then move to the 'true_block'
        let current_block_ref = self.blocks.get_mut(&self.current_block).unwrap();
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
            unreachable: false,
            stack_height,
            blockty,
        });
    }

    fn visit_else_op(&mut self) {
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
            unreachable: false,
            stack_height: frame.stack_height,
            blockty: frame.blockty,
        });

        // Pop this block's results and pass them to the join block.
        let results = self.popn(block_results_count);
        // Reset the value stack to the height it was at the start of the if block.
        if frame.unreachable {
            self.stack.truncate(frame.stack_height);
        } else {
            assert!(self.stack.len() == frame.stack_height);
        }
        // Re-push this block's params.
        self.push_block_params(block_params_count);

        // Terminate the true block with br(join) and then move to the 'false_block'
        let current_block_ref = self.blocks.get_mut(&self.current_block).unwrap();
        current_block_ref.terminator = Terminator::Br(join_block, results);
        self.current_block = false_block;
    }

    fn visit_end_op(&mut self, current_offset: usize) -> anyhow::Result<()> {
        let frame = self.pop_frame();
        let block_results_count = self.blockty_results(frame.blockty).len();
        let results = self.popn(block_results_count);
        // Reset the value stack to the height it was at the start of the block.
        if frame.unreachable {
            self.stack.truncate(frame.stack_height);
        } else {
            assert!(self.stack.len() == frame.stack_height);
        }

        match frame.kind {
            FrameKind::Func => {
                // Terminate the function with a return
                let current_block_ref = self.blocks.get_mut(&self.current_block).unwrap();
                if !frame.unreachable {
                    current_block_ref.terminator = Terminator::Return(results);
                } else {
                    // TODO
                    // assert!(current_block_ref.terminator != Terminator::Unknown);
                }

                self.validator.finish(current_offset)?;
            }
            FrameKind::Block { join_block } => {
                // Terminate with a br to the join block
                let current_block_ref = self.blocks.get_mut(&self.current_block).unwrap();
                if !frame.unreachable {
                    current_block_ref.terminator = Terminator::Br(join_block, results);
                } else {
                    // TODO
                    // assert!(current_block_ref.terminator != Terminator::Unknown);
                }
                self.current_block = join_block;
                self.push_block_params(block_results_count);
            }
            FrameKind::Loop {
                header_block: _,
                join_block,
            } => {
                // Terminate with a br to the join block
                let current_block_ref = self.blocks.get_mut(&self.current_block).unwrap();
                if !frame.unreachable {
                    current_block_ref.terminator = Terminator::Br(join_block, results);
                } else {
                    // TODO
                    // assert!(current_block_ref.terminator != Terminator::Unknown);
                }
                self.current_block = join_block;
                self.push_block_params(block_results_count);
            }
            FrameKind::If {
                true_block: _,
                false_block,
                join_block,
            } => {
                // Terminate the true block with a br(join_block)
                let current_block_ref = self.blocks.get_mut(&self.current_block).unwrap();
                if !frame.unreachable {
                    current_block_ref.terminator = Terminator::Br(join_block, results);
                } else {
                    // TODO
                    // assert!(current_block_ref.terminator != Terminator::Unknown);
                }

                // There was no 'else', so create an empty false block with
                // just a br(join_block). It's only valid to omit an 'else'
                // if the block has the same number of params as results,
                // so use that to create the block params and results for
                // the block.
                let block_params_count = block_results_count;
                self.push_block_params(block_params_count);
                let results = self.popn(block_results_count);
                let false_block_ref = self.blocks.get_mut(&false_block).unwrap();
                false_block_ref.terminator = Terminator::Br(join_block, results);

                // Move to the join block
                self.current_block = join_block;
                self.push_block_params(block_results_count);
            }
            FrameKind::Else {
                true_block: _,
                false_block: _,
                join_block,
            } => {
                // Terminate with a br(join_block) and move to the join block
                let current_block_ref = self.blocks.get_mut(&self.current_block).unwrap();
                if !frame.unreachable {
                    current_block_ref.terminator = Terminator::Br(join_block, results);
                } else {
                    // TODO
                    // assert!(current_block_ref.terminator != Terminator::Unknown);
                }
                self.current_block = join_block;
                self.push_block_params(block_results_count);
            }
        }

        Ok(())
    }

    fn visit_unreachable_op(&mut self) {
        let block = self.blocks.get_mut(&self.current_block).unwrap();
        block.terminator = Terminator::Unreachable;

        self.after_unconditional_branch();
    }

    fn visit_return_op(&mut self) {
        let func_frame_depth = self.frames.len() - 1;
        self.visit_br_op(func_frame_depth as u32);
    }

    fn visit_br_op(&mut self, relative_depth: u32) {
        let branch_params = self.pop_branch_params(relative_depth);
        let target_frame = self.frame_at(relative_depth);
        if target_frame.kind.is_func() {
            let block = self.blocks.get_mut(&self.current_block).unwrap();
            block.terminator = Terminator::Return(branch_params);
        } else {
            let target_block = target_frame.kind.branch_target_block();
            let block = self.blocks.get_mut(&self.current_block).unwrap();
            block.terminator = Terminator::Br(target_block, branch_params);
        }

        self.after_unconditional_branch();
    }

    fn visit_br_if_op(&mut self, relative_depth: u32) {
        let condition = self.pop();
        let branch_params = self.pop_branch_params(relative_depth);
        self.sync_stack_before_statement();

        let target_frame = self.frame_at(relative_depth);
        let target_block = if target_frame.kind.is_func() {
            self.return_block
        } else {
            target_frame.kind.branch_target_block()
        };

        let branch_param_types = branch_params
            .iter()
            .flat_map(|x| self.expr_type(x, self.blocks.get(&self.current_block).unwrap()))
            .collect();
        let fallthrough_block = self.add_block(Block {
            params: branch_param_types,
            statements: Vec::new(),
            terminator: Terminator::Unknown,
        });

        let block = self.blocks.get_mut(&self.current_block).unwrap();
        block.terminator =
            Terminator::BrIf(condition, target_block, fallthrough_block, branch_params);

        self.after_unconditional_branch();
        self.current_block = fallthrough_block;
    }

    fn visit_br_table_op<'a>(&mut self, br_table: wasm::BrTable<'a>) -> anyhow::Result<()> {
        let default_target_depth = br_table.default();
        let default_target = self.branch_target_block(default_target_depth);
        let branch_params = self.pop_branch_params(default_target_depth);

        let mut targets = Vec::new();
        for relative_depth in br_table.targets() {
            targets.push(self.branch_target_block(relative_depth?));
        }

        let block = self.blocks.get_mut(&self.current_block).unwrap();
        block.terminator = Terminator::BrTable(targets, default_target, branch_params);

        self.after_unconditional_branch();
        Ok(())
    }

    fn visit_statement_op(&mut self, op: wasm::Operator) {
        // We only parse statements if we're not in dead code
        assert!(!self.frame_unreachable(0));

        let statement = match op {
            wasm::Operator::Nop => Statement::Nop,
            wasm::Operator::Drop => {
                let value = self.pop();
                Statement::Drop(value)
            }
            wasm::Operator::LocalSet { local_index } => {
                let value = self.pop();

                Statement::LocalSet(LocalSetStatement {
                    index: local_index,
                    value: Box::new(value),
                })
            }
            wasm::Operator::LocalTee { local_index } => {
                let value = self.pop();

                self.stack.push(Expression::GetLocal(GetLocalExpression {
                    local_index: local_index,
                }));

                Statement::LocalSet(LocalSetStatement {
                    index: local_index,
                    value: Box::new(value),
                })
            }
            wasm::Operator::GlobalSet { global_index } => {
                let value = self.pop();

                Statement::GlobalSet(GlobalSetStatement {
                    index: global_index,
                    value: Box::new(value),
                })
            }
            wasm::Operator::I32Store { memarg }
            | wasm::Operator::I32Store16 { memarg }
            | wasm::Operator::I32Store8 { memarg }
            | wasm::Operator::I64Store { memarg }
            | wasm::Operator::I64Store32 { memarg }
            | wasm::Operator::I64Store16 { memarg }
            | wasm::Operator::F32Store { memarg }
            | wasm::Operator::F64Store { memarg } => {
                let value = self.pop();
                let index = self.pop();
                Statement::MemoryStore(MemoryStoreStatement {
                    arg: memarg,
                    index: Box::new(index),
                    value: Box::new(value),
                })
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
                    Statement::Call(call)
                } else {
                    if result_count == 1 {
                        self.stack.push(Expression::Call(call));
                    } else {
                        unimplemented!()
                    }
                    return;
                }
            }
            wasm::Operator::CallIndirect {
                type_index,
                table_index,
            } => {
                let callee_index = Box::new(self.pop());
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
                    Statement::CallIndirect(call)
                } else {
                    if result_count == 1 {
                        self.stack.push(Expression::CallIndirect(call));
                    } else {
                        unimplemented!()
                    }
                    return;
                }
            }
            _ => {
                self.expr_op(op);
                return;
            }
        };

        self.sync_stack_before_statement();

        let current_block_ref = self.blocks.get_mut(&self.current_block).unwrap();
        current_block_ref.statements.push(statement);
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
                // Handled in visit_statement_op
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
                let cond = self.pop();
                let false_expr = self.pop();
                let true_expr = self.pop();
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
                let index = self.pop();
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
                let value = self.pop();
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
                let value = Box::new(self.pop());
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
                let rhs = Box::new(self.pop());
                let lhs = Box::new(self.pop());
                self.stack.push(Expression::Binary(op.into(), lhs, rhs));
            }

            _ => todo!("unimplemented op: {:?}", op),
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
            builder.visit_op(offset, operator_reader.original_position(), op)?;
        }
        operator_reader.ensure_end()?;

        builder.finish()
    }
}
