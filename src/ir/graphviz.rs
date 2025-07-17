use crate::ir::print::Ctx;
use crate::ir::*;

impl Func {
    pub fn to_graphviz(&self, output: &mut dyn std::io::Write) -> anyhow::Result<()> {
        writeln!(output, "digraph func_{} {{", self.index)?;
        writeln!(output, "  rankdir=TB;")?;
        writeln!(
            output,
            "  node [shape=box, style=filled, fillcolor=lightblue, labeljust=l];"
        )?;
        writeln!(output, "")?;

        let ctx = Ctx { func: self };

        // Write all blocks
        let block_order = self.visual_block_order();
        for block_index in &block_order {
            if let Some(block) = self.blocks.get(block_index) {
                write!(output, "  block_{} [label=\"", block_index.0)?;
                let mut body = Vec::new();
                block
                    .pretty::<_, ()>(self, *block_index, false, ctx, &pretty::BoxAllocator)
                    .render(80, &mut body)?;
                let body_text = String::from_utf8(body)?.replace("\n", "\\l");
                write!(output, "{}\\l", body_text)?;
                writeln!(output, "\"];")?;
            }
        }

        writeln!(output, "")?;

        // Write edges between blocks
        for block_index in &block_order {
            if let Some(block) = self.blocks.get(block_index) {
                let successors = block.successors();
                for successor in successors {
                    writeln!(
                        output,
                        "  block_{} -> block_{};",
                        block_index.0, successor.0
                    )?;
                }
            }
        }

        // Mark entry block differently
        writeln!(
            output,
            "  block_{} [fillcolor=lightgreen];",
            self.entry_block.0
        )?;

        writeln!(output, "}}")?;
        Ok(())
    }
}
