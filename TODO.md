1. Function params are not locals
    - Add the funcType::params to locals.
    - Tweak printing to discern between locals and params.
1. Dead code
    - Ending the block must ignore value stack.
1. Handle return br, br_if, br_table instructions
    - Terminate block and transition into dead code.

After all of this, test this out on real modules!

BACKLOG:
  1. Skip block label and jump if there are no params and it is just fallthrough
    - This is probably a graph optimization?
  1. Add a block re-numbering step? Make it RPO
  1. Handle function calls with multi-results

Structured control flow reconstruction:
    1. Build the region tree for all blocks
    - Dominators and postdominators? Single exit, single entry
    1. Bottom up construct structured control flow
