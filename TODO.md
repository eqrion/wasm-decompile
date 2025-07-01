1. GC support

1. Improve formatting!
 - coalesce arg and locals with same type
 - omit fallthrough br instructions
   - br_if
   - br
   - do it during printing
   - somehow skip printing label if only hit by fallthrough?

1. Merge blocks where pred has only successor and successor only has pred
  - Need predecessor information that doesn't exist yet.
  - Add a function to generate it as a side table.
  
# Playground

1. Paste wat or upload wasm
1. One pane for the wat, optional extra pane for the disassembly
1. Virtualize the display of the wat
1. Click on links
1. Query param support for loading wasm

# SCF

1. Critical edge splitting - Could be introduced by building. Interferes with SCF
1. Branch param elimination?
  - Create temps and hoist them to dominating point?
1. Local definition settling
1. Verification:
  - No critical edges
Algorithm idea:
  2. Build a loop tree, convert the top level down as you go
  1. N - N => M
  1. Diamond => If
  1. Loop header with M branches to ancestor loops (or self), and 1 exit branch to block within same loop depth
  1. Switch case?

BACKLOG:
  1. Handle function calls with multi-results
  1. Wasm-GC
