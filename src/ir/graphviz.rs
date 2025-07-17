use crate::ir::*;

impl Func {
    pub fn to_graphviz(&self, output: &mut dyn std::io::Write) {
        writeln!(output, "digraph func_{} {{", self.index).unwrap();
        writeln!(output, "  rankdir=TB;").unwrap();
        writeln!(
            output,
            "  node [shape=box, style=filled, fillcolor=lightblue];"
        )
        .unwrap();
        writeln!(output, "").unwrap();

        // Write all blocks
        let block_order = self.visual_block_order();
        for block_index in &block_order {
            if let Some(block) = self.blocks.get(block_index) {
                write!(output, "  block_{} [label=\"", block_index.0).unwrap();
                write!(output, "Block {}\\l", block_index.0).unwrap();

                // Add block parameters if any
                if !block.params.is_empty() {
                    write!(output, "params: ").unwrap();
                    for (i, param_type) in block.params.iter().enumerate() {
                        if i > 0 {
                            write!(output, ", ").unwrap();
                        }
                        write!(output, "{:?}", param_type).unwrap();
                    }
                    write!(output, "\\l").unwrap();
                }

                // Add statements
                for stmt in &block.statements {
                    stmt.to_graphviz(output);
                }

                // Add terminator
                block.terminator.to_graphviz(output);

                writeln!(output, "\"];").unwrap();
            }
        }

        writeln!(output, "").unwrap();

        // Write edges between blocks
        for block_index in &block_order {
            if let Some(block) = self.blocks.get(block_index) {
                let successors = block.successors();
                for successor in successors {
                    writeln!(
                        output,
                        "  block_{} -> block_{};",
                        block_index.0, successor.0
                    )
                    .unwrap();
                }
            }
        }

        // Mark entry block differently
        writeln!(
            output,
            "  block_{} [fillcolor=lightgreen];",
            self.entry_block.0
        )
        .unwrap();

        writeln!(output, "}}").unwrap();
    }
}

impl Terminator {
    pub fn to_graphviz(&self, output: &mut dyn std::io::Write) {
        match self {
            Terminator::Unknown => {
                write!(output, "unknown\\l").unwrap();
            }
            Terminator::Unreachable => {
                write!(output, "unreachable\\l").unwrap();
            }
            Terminator::Return(values) => {
                write!(output, "return").unwrap();
                if !values.is_empty() {
                    write!(output, " ").unwrap();
                    for (i, value) in values.iter().enumerate() {
                        if i > 0 {
                            write!(output, ", ").unwrap();
                        }
                        value.to_graphviz(output);
                    }
                }
                write!(output, "\\l").unwrap();
            }
            Terminator::Br(target, values) => {
                write!(output, "br block_{}", target.0).unwrap();
                if !values.is_empty() {
                    write!(output, " ").unwrap();
                    for (i, value) in values.iter().enumerate() {
                        if i > 0 {
                            write!(output, ", ").unwrap();
                        }
                        value.to_graphviz(output);
                    }
                }
                write!(output, "\\l").unwrap();
            }
            Terminator::BrIf(condition, true_block, false_block, values) => {
                write!(output, "br_if ").unwrap();
                condition.to_graphviz(output);
                write!(
                    output,
                    " then block_{} else block_{}",
                    true_block.0, false_block.0
                )
                .unwrap();
                if !values.is_empty() {
                    write!(output, " ").unwrap();
                    for (i, value) in values.iter().enumerate() {
                        if i > 0 {
                            write!(output, ", ").unwrap();
                        }
                        value.to_graphviz(output);
                    }
                }
                write!(output, "\\l").unwrap();
            }
            Terminator::BrTable(targets, default_target, values) => {
                write!(output, "br_table [").unwrap();
                for (i, target) in targets.iter().enumerate() {
                    if i > 0 {
                        write!(output, ", ").unwrap();
                    }
                    write!(output, "block_{}", target.0).unwrap();
                }
                write!(output, "] default block_{}", default_target.0).unwrap();
                if !values.is_empty() {
                    write!(output, " ").unwrap();
                    for (i, value) in values.iter().enumerate() {
                        if i > 0 {
                            write!(output, ", ").unwrap();
                        }
                        value.to_graphviz(output);
                    }
                }
                write!(output, "\\l").unwrap();
            }
        }
    }
}
impl Statement {
    pub fn to_graphviz(&self, output: &mut dyn std::io::Write) {
        match self {
            Statement::Nop => {
                write!(output, "nop\\l").unwrap();
            }
            Statement::Drop(expr) => {
                write!(output, "drop ").unwrap();
                expr.to_graphviz(output);
                write!(output, "\\l").unwrap();
            }
            Statement::LocalSet(stmt) => {
                stmt.to_graphviz(output);
            }
            Statement::LocalSetN(stmt) => {
                stmt.to_graphviz(output);
            }
            Statement::GlobalSet(stmt) => {
                stmt.to_graphviz(output);
            }
            Statement::MemoryStore(stmt) => {
                stmt.to_graphviz(output);
            }
            Statement::If(stmt) => {
                stmt.to_graphviz(output);
            }
            Statement::Call(expr) => {
                expr.to_graphviz(output);
                write!(output, "\\l").unwrap();
            }
            Statement::CallIndirect(expr) => {
                expr.to_graphviz(output);
                write!(output, "\\l").unwrap();
            }
        }
    }
}
impl LocalSetStatement {
    pub fn to_graphviz(&self, output: &mut dyn std::io::Write) {
        write!(output, "local_{} = ", self.index).unwrap();
        self.value.to_graphviz(output);
        write!(output, "\\l").unwrap();
    }
}
impl LocalSetNStatement {
    pub fn to_graphviz(&self, output: &mut dyn std::io::Write) {
        write!(output, "local_[").unwrap();
        for (i, index) in self.index.iter().enumerate() {
            if i > 0 {
                write!(output, ", ").unwrap();
            }
            write!(output, "{}", index).unwrap();
        }
        write!(output, "] = ").unwrap();
        self.value.to_graphviz(output);
        write!(output, "\\l").unwrap();
    }
}
impl GlobalSetStatement {
    pub fn to_graphviz(&self, output: &mut dyn std::io::Write) {
        write!(output, "global_{} = ", self.index).unwrap();
        self.value.to_graphviz(output);
        write!(output, "\\l").unwrap();
    }
}
impl MemoryStoreStatement {
    pub fn to_graphviz(&self, output: &mut dyn std::io::Write) {
        write!(output, "store(").unwrap();
        self.index.to_graphviz(output);
        write!(output, ", ").unwrap();
        self.value.to_graphviz(output);
        write!(output, ")\\l").unwrap();
    }
}
impl IfStatement {
    pub fn to_graphviz(&self, output: &mut dyn std::io::Write) {
        write!(output, "if ").unwrap();
        self.condition.to_graphviz(output);
        write!(output, "\\l").unwrap();

        // Print true block
        if !self.true_statements.is_empty() {
            write!(output, "  then:\\l").unwrap();
            for stmt in &self.true_statements {
                write!(output, "    ").unwrap();
                stmt.to_graphviz(output);
            }
        } else {
            write!(output, "  then: (empty)\\l").unwrap();
        }

        // Print false block
        if !self.false_statements.is_empty() {
            write!(output, "  else:\\l").unwrap();
            for stmt in &self.false_statements {
                write!(output, "    ").unwrap();
                stmt.to_graphviz(output);
            }
        } else {
            write!(output, "  else: (empty)\\l").unwrap();
        }
    }
}
impl Expression {
    pub fn to_graphviz(&self, output: &mut dyn std::io::Write) {
        match self {
            Expression::I32Const { value } => {
                write!(output, "{}", value).unwrap();
            }
            Expression::I64Const { value } => {
                write!(output, "{}L", value).unwrap();
            }
            Expression::F32Const { value } => {
                write!(output, "{}f", value.bits()).unwrap();
            }
            Expression::F64Const { value } => {
                write!(output, "{}d", value.bits()).unwrap();
            }
            Expression::BlockParam(index) => {
                write!(output, "param_{}", index).unwrap();
            }
            Expression::Unary(op, expr) => {
                op.to_graphviz(output);
                write!(output, "(").unwrap();
                expr.to_graphviz(output);
                write!(output, ")").unwrap();
            }
            Expression::Binary(op, left, right) => {
                op.to_graphviz(output);
                write!(output, "(").unwrap();
                left.to_graphviz(output);
                write!(output, ", ").unwrap();
                right.to_graphviz(output);
                write!(output, ")").unwrap();
            }
            Expression::Call(call) => {
                call.to_graphviz(output);
            }
            Expression::CallIndirect(call) => {
                call.to_graphviz(output);
            }
            Expression::GetLocal(get) => {
                get.to_graphviz(output);
            }
            Expression::GetLocalN(get) => {
                get.to_graphviz(output);
            }
            Expression::GetGlobal(get) => {
                get.to_graphviz(output);
            }
            Expression::Select(select) => {
                select.to_graphviz(output);
            }
            Expression::MemoryLoad(load) => {
                load.to_graphviz(output);
            }
            Expression::MemorySize => {
                write!(output, "memory.size").unwrap();
            }
            Expression::MemoryGrow(grow) => {
                grow.to_graphviz(output);
            }
            Expression::Bottom => {
                write!(output, "⊥").unwrap();
            }
        }
    }
}
impl UnaryExpression {
    pub fn to_graphviz(&self, output: &mut dyn std::io::Write) {
        write!(output, "{}", self.to_string()).unwrap();
    }
}
impl BinaryExpression {
    pub fn to_graphviz(&self, output: &mut dyn std::io::Write) {
        let (op_str, _is_infix) = self.to_string_and_infix();
        write!(output, "{}", op_str).unwrap();
    }
}
impl CallExpression {
    pub fn to_graphviz(&self, output: &mut dyn std::io::Write) {
        write!(output, "call func_{}", self.func_index).unwrap();
        if !self.params.is_empty() {
            write!(output, "(").unwrap();
            for (i, param) in self.params.iter().enumerate() {
                if i > 0 {
                    write!(output, ", ").unwrap();
                }
                param.to_graphviz(output);
            }
            write!(output, ")").unwrap();
        }
    }
}
impl CallIndirectExpression {
    pub fn to_graphviz(&self, output: &mut dyn std::io::Write) {
        write!(output, "call_indirect type_{}", self.func_type_index).unwrap();
        write!(output, "(").unwrap();
        self.callee_index.to_graphviz(output);
        for param in &self.params {
            write!(output, ", ").unwrap();
            param.to_graphviz(output);
        }
        write!(output, ")").unwrap();
    }
}
impl GetLocalExpression {
    pub fn to_graphviz(&self, output: &mut dyn std::io::Write) {
        write!(output, "local_{}", self.local_index).unwrap();
    }
}
impl GetLocalNExpression {
    pub fn to_graphviz(&self, output: &mut dyn std::io::Write) {
        write!(output, "local_[").unwrap();
        for (i, index) in self.local_indices.iter().enumerate() {
            if i > 0 {
                write!(output, ", ").unwrap();
            }
            write!(output, "{}", index).unwrap();
        }
        write!(output, "]").unwrap();
    }
}
impl GetGlobalExpression {
    pub fn to_graphviz(&self, output: &mut dyn std::io::Write) {
        write!(output, "global_{}", self.global_index).unwrap();
    }
}
impl SelectExpression {
    pub fn to_graphviz(&self, output: &mut dyn std::io::Write) {
        write!(output, "select(").unwrap();
        self.condition.to_graphviz(output);
        write!(output, " ? ").unwrap();
        self.on_true.to_graphviz(output);
        write!(output, " : ").unwrap();
        self.on_false.to_graphviz(output);
        write!(output, ")").unwrap();
    }
}
impl MemoryLoadExpression {
    pub fn to_graphviz(&self, output: &mut dyn std::io::Write) {
        write!(output, "load(").unwrap();
        self.index.to_graphviz(output);
        write!(output, ")").unwrap();
    }
}
impl MemoryGrowExpression {
    pub fn to_graphviz(&self, output: &mut dyn std::io::Write) {
        write!(output, "memory.grow(").unwrap();
        self.value.to_graphviz(output);
        write!(output, ")").unwrap();
    }
}
