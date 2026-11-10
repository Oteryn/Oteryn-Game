# Independent verification of DUR-003

Base: `e2e5ced306040113dc9c395251a802f50eb5776a`. Verifier: gameplay auditor (`coordination_inventory`).

## Verdict

**CONFIRMED by static executable-path evidence; P1.** An ordinary supported nested inventory can leave a committed character death unable to finish its required item-drop work. Both fresh admission and reconnect then refuse to make that character playable. No native Rust/PostgreSQL reproduction was executed; exact SQL transaction behavior still needs the focused integration case described below.

## Concrete reachable state

Use a living character above level 8, with a vocation, no Adventurer protection, no regular blessings and no Amulet of Loss. Its ordinary main backpack contains a bag, and that bag contains an item. The nested bag need not exceed any depth or capacity limit. Kill the character through the ordinary source death path.

The nested inventory is supported by the current writer, not merely an arbitrary corrupt fixture:

- `crates/gameplay/durability/src/durability/item_loop_move.rs:737-816` resolves a character-owned non-main container, requires a live container definition, and returns `Target::Container { kind: KIND_NESTED, ... }`.
- The depth guard at `826-831` permits bounded nesting; it does not prohibit an item in a directly carried bag.
- The writer at `1517-1532` inserts that custody into `game_item_nested_entries`.

## Production failure chain

1. `crates/gameplay/dispatch/src/gameplay_transport/pvp_death.rs:11-43` is the common source death adapter. It calls `settle_runtime_player_death_with_items(..., Some(root))`, awaits the committed death, then applies item operations. It is not a fixture-only entry point despite its filename.
2. `crates/gameplay/durability/src/durability/character_death/items.rs:88-115` includes the main backpack as equipment slot 9 without inspecting its nested contents. `119-147` applies valid exemptions and the selected item-loss sampler. None excludes supported nested bags.
3. `crates/gameplay/domain/src/domain/death.rs:90,352-370` gives the nonexempt unprotected main container a 1000/1000 loss chance at zero blessings. This case needs no favorable random roll.
4. `character_death/commit.rs:295-305` freezes this selection; `405-433` inserts the pending respawn and commits the death receipt, loss and selection **before** the adapter starts item movement.
5. `crates/gameplay/durability/src/durability/player_death_items.rs:365-370` checks both direct `game_item_nested_entries` children and nested entries under a direct main-backpack child. In the concrete valid tree, the second `EXISTS` is true, so it returns `PlayerDeathItemError::InvalidInput`. This occurs before the terminal item record at `460-461` and before custody changes at `462-472`.
6. `crates/gameplay/dispatch/src/gameplay_transport/player_death_items.rs:59-103` repeats the immutable selected operation and converts this refusal to Unavailable. The comment explicitly says terminal rejection is never skipped. A retry does not change the tree or the selection.
7. `composed_admission/death.rs:521-527` refuses reconnect's respawn continuation when the same item work fails. `composed_admission/first_entry.rs:422-427` also returns `FirstEntryOutcome::RefusedUnavailable` before placing the character.

Primary defect location: [nested refusal at the exact baseline](https://github.com/Oteryn/Oteryn-Game/blob/e2e5ced306040113dc9c395251a802f50eb5776a/crates/gameplay/durability/src/durability/player_death_items.rs#L365-L370).

## Counterguards checked

- The early refusal protects item consistency, but it occurs after a different transaction has durably created the obligation. That fail-closed check is the cause of the playable-state blockage.
- `apps/game-server/migrations/0536_player_death_item_selection.sql:20-39` requires a terminal record for every selected lost/consumed item and blocks deleting the pending respawn until completion. A search of all current migrations found uses of this completion function, and no replacement removing the obligation or changing its deep-container behavior.
- `player_death_items/recovery.rs:50-62,123-127` processes the same immutable selection and invokes the same `write` function for departed activity recovery; it has no tree-aware fallback.
- `player_death_items.rs:289-319` can replay an existing exact terminal item record, but the first failing attempt creates none.
- Exempt level/vocation/Adventurer cases, a protecting Amulet of Loss, enough blessings to prevent the container being selected, or a flat/empty bag avoid the trigger. They do not protect the explicitly valid unprotected case.
- Existing flat death inventory fixtures do not demonstrate nested death completion. Tests were read as evidence of intended structure, not reported as passing.

## Impact and limits

The confirmed wrong result is a persistent pending death whose normal admission/respawn paths refuse. This verification does not establish loss/duplication of the contents, a server-wide outage, or a way to affect unrelated accounts. Earlier independent selected item operations can have their own committed receipts before the unsupported root is encountered; the failing root itself returns before writing its receipt. Operator intervention or a future corrected writer could recover the obligation; this report does not claim irrevocable data loss.

## Minimal focused reproduction still needed

In the existing PostgreSQL death fixture, create the valid main-backpack → bag → item tree through the real item movement owner, choose the nonexempt zero-blessing/no-AoL state above, and commit a source death. Assert that the immutable selection includes the main backpack, item-drop processing returns InvalidInput/Unavailable, no terminal record exists for that backpack, and pending respawn remains. Repeat both reconnect and fresh admission and observe refusal. Then use a flat inventory control. A future repair should make this case pass through the actual tree custody writer without weakening the completion guard or silently dropping obligations.
