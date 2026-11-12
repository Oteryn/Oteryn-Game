# QUESTPC run 7 callback validation

This record transcribes the observed tool outputs; it does not represent CI or a
frozen candidate. Work was local and uncommitted. The repository HEAD read when
writing this record was `1fa04bbbddb5d96e21fecca0bc456f3c5e1109b7`.
No code or tests were changed, or tests rerun, to produce this record.

## Successful dispatch commands

### Seven literal travel uses

```bash
QPC_SLOT=1 /home/mole/oteryn-cache/codex-bin/cargo --locked --offline test -p oteryn-gameplay-dispatch --lib quest_world_callbacks_literal_use_accepts_clicked_nonzero_ordinal -- --nocapture
```

- Exec session: `20403`; terminal output chunk: `186b2d`.
- Normal Cargo termination: **exit 0**.
- Observed build duration: **1m 38s**.
- Observed test duration: **207.66s**.
- Result: **1 passed; 0 failed; 0 ignored; 2492 filtered out**.
- Seven completion markers. Source item/ordinal pairs were Blue Valley
  `49948/3`, `49948/3`, `49948/1`, `50310/2`; Cheesy `1760/1`;
  Stag return `6391/2`; crypt exit after donor startup `52656/1`.

### Eight guarded Stag cases and the actual static crypt wall

```bash
QPC_SLOT=1 /home/mole/oteryn-cache/codex-bin/cargo --locked --offline test -p oteryn-gameplay-dispatch --lib quest_world_callbacks_read_only_stag_uses_gate_without_quest_writes -- --nocapture
```

- Exec session: `11190`; output chunks `d43ef5` and terminal `356cb3`.
- Normal Cargo termination: **exit 0**.
- Observed build duration: **1m 06s**.
- Observed test duration: **242.24s**.
- Result: **1 passed; 0 failed; 0 ignored; 2494 filtered out**.
- Nine completion markers: boat `6391/4` at absent/2/3/13 progress;
  marked wall `52735/1` at absent/10/11/13 progress; static crypt wall `1507/1`.
- Positive cases assert the exact destination and committed USE result;
  negative cases assert rejection and the unchanged origin. Source feedback,
  durable quest snapshots, receipt counts and CharacterRevision are checked.
- No temporary diagnostic output appeared in this successful run.

The dispatch executable reported by both commands was:
`/home/mole/oteryn-cache/target-codex/lanes/questpc-s1/debug/deps/oteryn_gameplay_dispatch-c5edc9fe5e94b27c`.

## The 16 successful completion markers

The first seven lines are from session `20403`; the last nine are from `11190`.
The duplicate crypt key represents two distinct appearances: `52656`, then
the actual pre-startup map appearance `1507`.

```text
QUEST_CALLBACK_LITERAL_USE_POSTGRES_CASE_COMPLETED oteryn:quest-callback/batch3/the_way_of_the_monk/blue-valley-2 stage=None allowed=true tracks_unchanged receipts_unchanged revision_unchanged
QUEST_CALLBACK_LITERAL_USE_POSTGRES_CASE_COMPLETED oteryn:quest-callback/batch3/the_way_of_the_monk/blue-valley-3 stage=None allowed=true tracks_unchanged receipts_unchanged revision_unchanged
QUEST_CALLBACK_LITERAL_USE_POSTGRES_CASE_COMPLETED oteryn:quest-callback/batch3/the_way_of_the_monk/blue-valley-4 stage=None allowed=true tracks_unchanged receipts_unchanged revision_unchanged
QUEST_CALLBACK_LITERAL_USE_POSTGRES_CASE_COMPLETED oteryn:quest-callback/batch3/the_way_of_the_monk/blue-valley-5 stage=None allowed=true tracks_unchanged receipts_unchanged revision_unchanged
QUEST_CALLBACK_LITERAL_USE_POSTGRES_CASE_COMPLETED oteryn:quest-callback/pirates-tail-cheesy-exit stage=None allowed=true tracks_unchanged receipts_unchanged revision_unchanged
QUEST_CALLBACK_LITERAL_USE_POSTGRES_CASE_COMPLETED oteryn:quest-callback/order-of-the-stag-boat-return stage=None allowed=true tracks_unchanged receipts_unchanged revision_unchanged
QUEST_CALLBACK_LITERAL_USE_POSTGRES_CASE_COMPLETED oteryn:quest-callback/order-of-the-stag-crypt-exit stage=None allowed=true tracks_unchanged receipts_unchanged revision_unchanged
QUEST_CALLBACK_LITERAL_USE_POSTGRES_CASE_COMPLETED oteryn:quest-callback/order-of-the-stag-boat-to-ada stage=None allowed=false tracks_unchanged receipts_unchanged revision_unchanged
QUEST_CALLBACK_LITERAL_USE_POSTGRES_CASE_COMPLETED oteryn:quest-callback/order-of-the-stag-boat-to-ada stage=Some(2) allowed=false tracks_unchanged receipts_unchanged revision_unchanged
QUEST_CALLBACK_LITERAL_USE_POSTGRES_CASE_COMPLETED oteryn:quest-callback/order-of-the-stag-boat-to-ada stage=Some(3) allowed=true tracks_unchanged receipts_unchanged revision_unchanged
QUEST_CALLBACK_LITERAL_USE_POSTGRES_CASE_COMPLETED oteryn:quest-callback/order-of-the-stag-boat-to-ada stage=Some(13) allowed=true tracks_unchanged receipts_unchanged revision_unchanged
QUEST_CALLBACK_LITERAL_USE_POSTGRES_CASE_COMPLETED oteryn:quest-callback/order-of-the-stag-mark-wall stage=None allowed=false tracks_unchanged receipts_unchanged revision_unchanged
QUEST_CALLBACK_LITERAL_USE_POSTGRES_CASE_COMPLETED oteryn:quest-callback/order-of-the-stag-mark-wall stage=Some(10) allowed=false tracks_unchanged receipts_unchanged revision_unchanged
QUEST_CALLBACK_LITERAL_USE_POSTGRES_CASE_COMPLETED oteryn:quest-callback/order-of-the-stag-mark-wall stage=Some(11) allowed=true tracks_unchanged receipts_unchanged revision_unchanged
QUEST_CALLBACK_LITERAL_USE_POSTGRES_CASE_COMPLETED oteryn:quest-callback/order-of-the-stag-mark-wall stage=Some(13) allowed=true tracks_unchanged receipts_unchanged revision_unchanged
QUEST_CALLBACK_LITERAL_USE_POSTGRES_CASE_COMPLETED oteryn:quest-callback/order-of-the-stag-crypt-exit stage=None allowed=true tracks_unchanged receipts_unchanged revision_unchanged
```

## Three focused unit tests

```bash
QPC_SLOT=1 /home/mole/oteryn-cache/codex-bin/cargo --locked --offline test -p oteryn-gameplay-quests --lib read_only_world_callback -- --nocapture
```

- Exec session `64629`; terminal chunk `041a46`; normal **exit 0**.
- Build **4.50s**; tests **0.00s**.
- **2 passed; 0 failed; 0 ignored; 257 filtered out**:
  - `quest::world_callbacks::tests::read_only_world_callback_has_no_quest_write_transition`
  - `quest::world_callbacks::tests::read_only_world_callbacks_reject_mixed_mutations`

```bash
QPC_SLOT=1 /home/mole/oteryn-cache/codex-bin/cargo --locked --offline test -p oteryn-gameplay-durability --lib read_only_callback_keeps_access_on_an_unchanged_historical_content_pin -- --nocapture
```

- Exec session `22125`; terminal chunk `8fef53`; normal **exit 0**.
- Build **41.13s**; test **0.00s**.
- **1 passed; 0 failed; 0 ignored; 680 filtered out**:
  `durability::quest_world_callback::read_only_pin_tests::read_only_callback_keeps_access_on_an_unchanged_historical_content_pin`.
- The test uses a `QuestStateRecord` with an older content revision and an equal
  definition hash, then checks unfinished hash mismatch rejection and completed
  quest acceptance. Its module was subsequently moved to EOF to answer the
  root combined-Clippy `items_after_test_module` finding; its behavior was unchanged.

## Observed regression failures and repairs

These were controlled authoring checks, not frozen candidate validation.

| Session | Exact filter / package | Observed termination and duration | Failure proved |
| --- | --- | --- | --- |
| `32456` | `quest_world_callbacks_literal_use_accepts_clicked_nonzero_ordinal` / dispatch | exit `101`; build `3m 14s`; test `49.64s`; 1 failed | Restoring the original position-only ordinal restriction yielded `NothingToUse` instead of `Committed` for the real Blue Valley ordinal-3 target. The corrected matcher was restored before successful `20403`. |
| `91553` | `read_only_world_callback_has_no_quest_write_transition` / quests | exit `101`; build `2.09s`; test `0.00s`; 1 failed | Temporarily restoring transition lowering failed with `read-only USE acquired a write transition`. The no-transition branch was restored; `8824` then passed (build `1.51s`, test `0.00s`), and final `64629` passed both quest units. |
| `51246` | `read_only_callback_keeps_access_on_an_unchanged_historical_content_pin` / durability | exit `101`; build `1m 36s`; test `0.00s`; 1 failed | Restoring content-revision equality rejected the older content pin despite an equal definition hash. The standard hash/completion semantics were restored before successful `22125`. |
| `25297` | `quest_world_callbacks_read_only_stag_uses_gate_without_quest_writes` / dispatch | exit `101`; build `1m 26s`; test `102.55s`; 1 failed | The positive stage-3 case returned `Rejected`. Inspection found the pin SELECT accidentally lacked `FROM game_character_quest_states`. |
| `40597` | same guarded dispatch filter | exit `101`; build `1m 06s`; test `76.47s`; 1 failed | The already-compiling diagnostic snapshot reproduced the pre-repair unavailable result at stage 3. The corrected SELECT and removal of all diagnostics were validated by successful `11190`. |

For each single-unit regression command above, the exact command form was the
same wrapper invocation shown in the successful unit sections, with the table's
single test filter. Dispatch regressions used the exact dispatch commands above.

## Other unsuccessful authoring attempts

These failures did not qualify callback behavior and are excluded from the 16
successful markers. A dash means the tool output did not report a completed
build/test duration; no duration has been inferred.

| Session | Filter | Exit | Observed build / test | Reported blocker |
| --- | --- | --- | --- | --- |
| `7041`, `76132`, `97590` | literal dispatch filter | `101` | — / not run | Missing `Storage.read_only` constructor field in `quests/src/quest/rotten_blood.rs:38`. |
| `7497` | literal dispatch filter | `101` | — / not run | Falcon `intent_binding` error conversion in `durability/reward_claim_mint/falcon.rs:41`. |
| `53926` | literal dispatch filter | `101` | — / not run | NPC test module mount plus Falcon `map_item_spatial_position` argument types. |
| `77975` | guarded dispatch filter | `101` | — / not run | Newhaven callers still referenced `claim_tutorial_corruptor` during its API repair. |
| `90325` | historical-pin durability unit filter | `101` | — / not run | Missing `RewardClaimMintContent.charges_remaining` in a shared test constructor. |
| `90285` | guarded dispatch filter | `101` | `4m 12s` / `9.08s` | `Content(InvalidArtifact("Reference Item Equipment vocation order"))`, before callback execution. |
| `28139` | guarded dispatch filter | `101` | — / not run | Protocol/domain `CharacterId` mismatch in `tutorial_corruptor.rs:15`. |

All listed sessions have observed terminal exit codes. One initial output-only
duplicate invocation did not retain its exec session ID or terminal result; it
is not used as validation evidence. No command was counted as passed merely
because it was launched or produced an early completion marker.

## Final local hygiene and boundary

- Final owned-path `git diff --check`: **exit 0**, output chunk `c2e3d6`.
- Search for diagnostic markers, temporary `false && callback.is_read_only`,
  and historical-pin revision equality: **exit 1 with no matches**, output
  chunk `d0e03e`.
- Combined Clippy, final formatting, generated content, constructor integration
  and publication belong to the root. This worker did not run its own Clippy,
  create commits, push, publish, trigger external review or claim whole-quest
  certification.
