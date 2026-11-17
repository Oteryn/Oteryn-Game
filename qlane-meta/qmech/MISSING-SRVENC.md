# SRVENC checked M24 quest mechanics

None remains missing in the assigned native death-progress boundary. This is
code inspection of the listed steps, not a claim that each entire quest is
playable. M24 uses `quest_batch3_c::capture`, the existing `quest_kills::settle`
and the fenced `quest_state::source_deaths` writer. Other assigned lanes retain
the preceding/following NPC, object, encounter and reward mechanics.

| quest | missing mechanic (server or client) | evidence (quest file and step, plus missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| Heart of Destruction | none in M24 | `native-death-bindings/quest-batch3-c-heart_of_destruction.json`: six room-death rules registered by `quests/src/quest/source_deaths.rs`; native lethal capture freezes exact room recipients and the existing quest queue settles the source flags. | The previously missing boss-death progress boundary is implemented. |
| The Order of the Stag | none in M24 | `native-death-bindings/quest-batch3-c-the_order_of_the_stag.json`: Court Warlock room rule, source questline 11 → 12, consumed by the same capture/settlement chain. | The Court Warlock death-progress boundary is implemented. |
| Tower Defence | none in M24 | `native-death-bindings/quest-batch3-c-tower_defence_quest.json`: Goblin Leader damaging-player rule, questline 2 → 3. `kill_reward.rs` preserves actual damage shares when the corpse Item cannot be materialized. | The final kill-progress boundary is implemented. |
| The Primal Ordeal | none in M24 | `native-death-bindings/quest-batch3-c-the_primal_ordeal.json`: Magma Bubble damaging-player flag. Primal Menace stays with the existing `character_hazard/primal.rs` atomic writer and is explicitly excluded from M24. | The Magma Bubble progress boundary is implemented. |
| A Piece of Cake | none in M24 | `native-death-bindings/qmech-m24-a_piece_of_cake.json`: exact chosen s2 transition, s1 prerequisite, 20 personal Cake Golem kills; generated from `missions/quest-state-completion-candidate.json`. Guard revisions prevent pre-mission credit while allowing queued counter increments. | The personal kill boundary is implemented; owner-unavailable bridge 48227 remains unchanged. |
| Ancient Tombs | none in M24 | `native-death-bindings/qmech-m24-ancient_tombs.json`: chosen kill stages s2/s3/s5/s6/s8/s10/s12 bind Omruc/Thalas/Dipthrah/Mahrdis/Vashresamun/Morguthis/Rahemos to the actual native lethal queue. Source predecessor stages remain required. | Seven pharaoh kill-progress boundaries are implemented; puzzle/reward stages remain outside M24. |

Paths above are relative to `content/quests/` unless a Rust module is named.
Focused tests are `srvenc_source_boss_deaths`, `srvenc_m24_postgres_*` and
`test_qmech_m24_kill_bindings`. Original donor URLs/file:line references and the
accepted chosen-recipe distinctions are retained in
`docs/reference/content-prep/qmech-m24/source-notes.md`.

## Finished work transfer (B)

SRVENC PR #3034 was already merged as
`c68395ccc0029cb3c8511ec804dfe3536afd4cc6`; no old unpublished W02/W03/W17 work
needed transplantation. M24 allocates no migration or protocol number; its
coordinator-assigned range is 2180–2189. The coordinator owns CLAIMS.md updates.
