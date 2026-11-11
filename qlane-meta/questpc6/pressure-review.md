# Pressure-sewer source review

Base for this continuation: c74555edf. An external WIP snapshot moved HEAD to deeb77891 during validation; this agent invoked only read-only Git commands. Source comparison remains against the start commit, not the externally updated HEAD.

Read the complete six-file product diff and the owner CODEX-CHECKLIST.md again. No enum variant, struct field, trait method, protocol number, migration, dependency or CI change. New public method remove_movement_item has the production caller quest_sewer_step -> stage_special_movement_step -> paced movement; pressure-caller-sites.txt contains the complete workspace scan. It removes exactly one callback-owned item, retains unrelated items and uses copy-on-write for pinned snapshots. Base sewer removal uses the existing revision-bearing projection owner. No durable Ground/House inventory is removed.

Source values are verified against the archived Crystal raw blob, with manifest hash and line spans in pressure-sewer-source.json. Fixture plate coordinates are explicitly synthetic; they are not map-placement evidence. Current-player state and runtime/map scope checks precede the movement hook. Framed replay does not call the movement hook again. No quest flags, receipts or Character revisions are manufactured by this physical operation.

Review repair: preserve compose_tile errors as errors; an invalid view must not authorize creation as if the sewer were absent. Final exact-tree validation reruns the focused PG cases after that repair.

All old fixture branches and assertions are retained. The three new fixture branches exercise absent sewer, preexisting base sewer, and unrelated unique ID. No existing test was skipped, deleted, loosened or ignored.

This is a mechanism slice, not full quest qualification. The created sewer's ordinary dropdown interaction, complete access path and remaining quest mechanics still need qualification/implementation. No complete quest or full five-quest batch is claimed. Check results remain pending until pressure-final-validation.json and terminal logs are read.

Final: all seven validation steps passed; PG3/3 actual82.83s,9units. Four unrelated formatting-only child modules restored to verified initial bytes. Workspace fmt--all--check and explicit skip_children rustfmt on all six owned files pass after cleanup. Source hashes of the six owned files match the final compile/PG evidence. Whole source diff reread after formatting; no unresolved finding on this bounded mechanism slice. Remaining full-quest work is explicitly UNFINISHED.
