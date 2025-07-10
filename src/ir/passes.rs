use std::collections::HashSet;

use pretty::block;

use crate::ir::*;

impl Func {
    pub fn jump_threading(&mut self) {
        let mut trivial_blocks = HashMap::new();

        for (block_index, block) in self.blocks.iter() {
            if let Some(target_block) = block.is_trivial_block() {
                trivial_blocks.insert(*block_index, target_block);
            } else {
                trivial_blocks.insert(*block_index, *block_index);
            }
        }

        for block in self.blocks.values_mut() {
            block.terminator.remap_block_indices(&trivial_blocks);
        }
    }

    fn get_all_predecessors(&mut self) -> HashMap<BlockIndex, Vec<BlockIndex>> {
        let mut predecessors = HashMap::new();
        for (block_index, block) in self.blocks.iter() {
            for successor in block.successors() {
                let successor_preds = predecessors.entry(successor).or_insert(Vec::new());
                successor_preds.push(*block_index);
            }
        }
        predecessors
    }

    // A -> B, A has only one successor and B has only one predecessor. No branch parameters
    fn merge_trivial_branch_blocks(&mut self) -> bool {
        let mut changed = false;
        for (block_index, predecessors) in self.get_all_predecessors() {
            if predecessors.len() != 1 {
                continue;
            }

            let predecessor = self.blocks.get_mut(&predecessors[0]).unwrap();
            let predecessor_successors = predecessor.successors();
            if predecessor_successors.len() != 1 {
                continue;
            }
            assert_eq!(predecessor_successors[0], block_index);

            let block = self.blocks.get_mut(&block_index).unwrap();
            if block.params.len() != 0 {
                // TODO: don't handle params yet
                continue;
            }

            // Merge all of block into predecessor
            let block_statements = std::mem::take(&mut block.statements);
            let block_terminator = std::mem::replace(&mut block.terminator, Terminator::Unknown);
            let predecessor = self.blocks.get_mut(&predecessors[0]).unwrap();
            predecessor.statements.extend(block_statements);
            assert!(matches!(predecessor.terminator, Terminator::Br(..)));
            predecessor.terminator = block_terminator;
            changed = true;
        }
        changed
    }

    //   A
    //  / \
    // B   C
    //  \ /
    //   D
    //
    // A has br_if to two sucessors
    // B and C have one predecessor that is A
    // B and C have one or zero successor D
    // D has only B or C as predecessors
    // Merge B and C into an if statement in A
    // A jumps to D
    fn merge_if_blocks(&mut self) -> bool {
        let mut changed = false;
        let predecessor_map = self.get_all_predecessors();
        let keys: Vec<BlockIndex> = self.blocks.keys().cloned().collect();
        for index_a in keys {
            let block_a = self.blocks.get(&index_a).unwrap();

            match &block_a.terminator {
                Terminator::BrIf(condition, index_b, index_c, params) => {
                    if params.len() != 0 {
                        continue;
                    }

                    let block_b = self.blocks.get(index_b).unwrap();
                    let block_c = self.blocks.get(index_c).unwrap();

                    if predecessor_map[index_b].len() != 1 || predecessor_map[index_c].len() != 1 {
                        continue;
                    }
                    assert_eq!(predecessor_map[index_b][0], index_a);
                    assert_eq!(predecessor_map[index_c][0], index_a);

                    let successors_b = block_b.successors();
                    let successors_c = block_c.successors();

                    if successors_b.len() > 1 || successors_c.len() > 1 {
                        continue;
                    }

                    let successor_b = successors_b.get(0);
                    let successor_c = successors_c.get(0);

                    let index_d = match (successor_b, successor_c) {
                        (Some(x), Some(y)) if *x == *y => Some(*x),
                        (Some(x), Some(y)) if *x != *y => continue,
                        (Some(x), None) => Some(*x),
                        (None, Some(y)) => Some(*y),
                        _ => None,
                    };

                    if let Some(index_d) = index_d {
                        let block_d = &self.blocks[&index_d];
                        if block_d.params.len() != 0 {
                            continue;
                        }
                        let predecessors_d = &predecessor_map[&index_d];
                        for predecessor in predecessors_d {
                            if *predecessor != *index_b || *predecessor != *index_c {
                                continue;
                            }
                        }
                    }

                    // Do it!
                    changed = true;

                    let statements_b = block_b.statements.clone();
                    let terminator_b = block_b.terminator.clone();
                    // TODO: add some terminators as statements
                    // match terminator_b {
                    //     Terminator::Return()
                    // }
                    let statements_c = block_c.statements.clone();
                    let terminator_c = block_c.terminator.clone();

                    let if_statement = IfStatement {
                        condition: Box::new(condition.clone()),
                        true_statements: statements_b,
                        false_statements: statements_c,
                    };

                    let block_a = self.blocks.get_mut(&index_a).unwrap();
                    block_a.terminator = index_d
                        .map(|x| Terminator::Br(x, vec![]))
                        .unwrap_or(Terminator::Unreachable);
                    block_a.statements.push(Statement::If(if_statement));
                }
                _ => continue,
            }
        }
        changed
    }

    pub fn reconstruct_control_flow(&mut self) {
        self.eliminate_dead_code();

        while self.merge_trivial_branch_blocks() || self.merge_if_blocks() {
            self.eliminate_dead_code();
        }
    }

    pub fn eliminate_dead_code(&mut self) {
        let mut stack: Vec<BlockIndex> = Vec::new();
        let mut alive: HashSet<BlockIndex> = HashSet::new();

        stack.push(self.entry_block);
        alive.insert(self.entry_block);

        while let Some(current) = stack.pop() {
            let successors = self.blocks.get(&current).unwrap().successors();
            for successor in successors {
                if !alive.contains(&successor) {
                    alive.insert(successor);
                    stack.push(successor);
                }
            }
        }

        self.blocks.retain(|node, _block| alive.contains(node));
    }

    pub fn renumber(&mut self) {
        let rpo = self.rpo();

        let mut mapping = HashMap::new();
        for (rpo_index, old_index) in rpo.iter().enumerate() {
            mapping.insert(*old_index, BlockIndex(rpo_index as u32));
        }

        self.remap_block_indices(&mapping);
    }

    fn rpo(&self) -> Vec<BlockIndex> {
        let mut visited = HashSet::new();
        let mut po = Vec::new();
        self.po_recursive(self.entry_block, &mut visited, &mut po);
        po.reverse();
        po
    }

    // Naive recursive implementation, replace with iterative algorithm eventually.
    fn po_recursive(
        &self,
        current: BlockIndex,
        visited: &mut HashSet<BlockIndex>,
        po: &mut Vec<BlockIndex>,
    ) {
        if visited.contains(&current) {
            return;
        }
        visited.insert(current);

        let successors = self.blocks.get(&current).unwrap().successors();
        for successor in successors {
            self.po_recursive(successor, visited, po);
        }

        po.push(current);
    }
}
