# Task Board programme entries

The retained chosen recipes in `content/quests/definitions/` specify Falconer's
first step as selecting a weekly hunting assignment (minimum level 100), and
Feral Trappers' first step as accepting a hunting programme (no minimum level).
Both target the Task Board. The native `StartWeekly` action in
`crates/gameplay/quests/src/quest/task_board_full.rs` is the selected programme
acceptance for these entry checkpoints. No new map object or client command is
invented. Queries, failed starts and arbitrary commands are not acceptance.

The fenced quest writer reads the native Task Board owner's immutable receipt
for the same character, world, session and command. Only a successful committed
`StartWeekly` action qualifies. Its existing actor admission independently retains
Falconer's level 100 requirement. Dispatch observes the receipt after the native
board commit, including replay recovery. Neither entry completes a quest or
unlocks an outfit. Later hunting/shop stages remain separate, unqualified work.

The focused PostgreSQL case supplies a committed-calendar baseline and bounded
creature catalogue as fixtures. Board starts, queries/refusals and route writes
use the production owners and revision slot. It proves admission and replay,
not the production weekly catalogue's balance or random selection.

Both imported records also retain `requirement_unknown/min_level` from their
original donor requirements. The selected `chosen_source_completion_v1` payloads
explicitly define their Oteryn levels with `CHOSEN_OTERYN_APPROXIMATION` basis.
Catalogue evidence therefore treats only that original unknown level as resolved
by the complete, bounded chosen value; the binding must still match it exactly.
Other unknown fields and native authored data holds remain blocking, and no
original-source metadata is deleted.
