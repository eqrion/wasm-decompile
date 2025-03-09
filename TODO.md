1. Dead code elimination
1. Block re-numbering to make it RPO
1. Block fusion
1. Critical edge splitting
1. Branch param elimination?
  - Create temps and hoist them to dominating point?
1. Local definition settling

Verification
  * No critical edges

No global code motion! That will take decompilation too far away from original source.

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
