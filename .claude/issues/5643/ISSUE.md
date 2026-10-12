# SCR-D5-2026-10-11-06: Story Manager tree build is O(g^3) per sibling group and the walk's recursion depth is bounded only by node count

Labels: low,scripting,quests,bug

- **Severity**: LOW
- **Dimension**: Scripting Dim 5
- **Location**: `story_manager.rs:387-398, 440-468, 791-882`
- **Status**: NEW

**Untrusted-Input: Yes.** The tree build is O(g³) per sibling group, and `walk_siblings` recursion depth is bounded only by node count. Vanilla maxima are tiny (group 50, depth 6–7). Index members once and use an explicit stack with a depth cap

