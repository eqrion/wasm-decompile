1. br_if is hard!

1. Dead code elimination
1. Block re-numbering to make it RPO
1. Block fusion for simple control flow
1. Branch param elimination?

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
