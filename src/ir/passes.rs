use std::collections::HashSet;


use crate::ir::*;

impl Func {
    pub fn eliminate_dead_code(&mut self) {
        let mut stack: Vec<BlockIndex> = Vec::new();
        let mut alive: HashSet<BlockIndex> = HashSet::new();

        stack.push(self.entry_block);
        alive.insert(self.entry_block);

        while !stack.is_empty() {
            let current = stack.pop().unwrap();
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
