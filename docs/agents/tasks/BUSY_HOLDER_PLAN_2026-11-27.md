# Combined busy-holder fix for staging

## Base and landing order

Source audit base is staging `43a8b036a0e3d9e05f130b6932b533b4cf7c7573`. The plan update starts at `1f56a658ab2028d1721221bf0363caf8951a589c` on `claude/happy-davinci-ijz3wy-holderplan`. Lane evidence comes from W13 `41926976..cf1d583d` and `381856ae`, P04 `b0b445c9..04429587`, C12 `b32ee5ed..485d7756` and its registration precursor `78d3335a`, C17b `af69ab4c..342c099b`, and P18 `19bd005d..ad644453` plus `1c1520cd`, `e79ecfcf`, and `08c043d3`.

The staging pin already defines `AdmissionRetryBudget` at `G/mod.rs:1484`. W13 supplies the shared join-deadline integration at `G/connection.rs:629` and `G/mod.rs:1561,1630` on `cf1d583d`. The staging registry has E1051 and 1201. C12 supplies `COMMAND_REFUSED` (1202) at `docs/contracts/protocol-oteryn/v1/registry/registry.json:72` on `485d7756`. Cut both implementation PRs from freshly fetched staging after W13, P04, and C12 land there. Verify the landed changes and registration on staging before cutting PR (a). Cut PR (b) from staging after (a) merges. Use C17b and P18 as implementation evidence.

Path prefixes below expand to `G=crates/gameplay/dispatch/src/gameplay_transport`, `D=crates/gameplay/durability/src/durability`, `N=apps/game-server`, `S=G/connection/serve`, and `C=G/connection/routing`. Authority anchors use the audit base unless labeled with a lane SHA.

## PR (a) · Holder, join and enrichment

- Track owner, call site, and age at every checkout and acquire of the one holder, including raw spawn, maintenance/recovery and background passes. Use the guard lifetime and one queue so `state=untracked` cannot arise. Publish wake after observed return or retirement. Wake the oldest eligible waiter, remove cancelled tickets, preserve the existing bounded background progress policy, and prevent background readiness from jumping the queue.
- Give each fresh or resumed join one absolute deadline from validated handshake through admission, repair, required snapshots and publication. Bound holder, readiness and SQL waits to the remaining time. Drop runtime, spell and revision guards before waiting. Preserve every authority predicate and session-generation fence exactly.
- On proven join or admission holder exhaustion, send the existing E1051 `SESSION_JOIN_UNAVAILABLE` (`TRANSPORT_FATAL`) once and close the transport. Use generation zero before acceptance and the advertised generation after acceptance. The coordinator decision uses the existing code and envelope with no registry or schema change. W13's `send_fresh_unavailable` currently emits E1106 at `G/connection.rs:2036` on `cf1d583d`. Route only proven pre-effect deadline exhaustion to E1051. Keep E1106 for an admission attempt that may have committed. FND-04C §4.1 explicitly covers failure after an admitted or resumed join. At pre-commit exhaustion, the grant remains unused and there is no extra controller. A fresh join that already committed its admission receipt keeps its consumed grant and uses existing unpublished-join cleanup and recovery. Resume join tests measure grant consumption relative to the already admitted baseline. [FND-04C §4.1] · [FND-02 §§11–12]
- Degrade optional appearances, speaker names, expert-mode observation and statistics by retaining the last valid view or skipping the affected batch without advancing its cursors. Defer training checkpoints and release the revision slot before readback. Session, combat and tick progress continue. Keep the real freshness and authority checks for authoritative reads. P04 qualification spawn keeps its bounded one-shot operator `holder_busy` result and publishes no actor on exhaustion. Its existing gameplay socket remains live.

Estimated **550–800 hand-written lines** including the PR (a) cases. This PR has no wire schema or registry edits.

## PR (b) · Served command refusal and proof before issue

Return `CommandResult` with `REJECTED`, `COMMAND_REFUSED` (1202), and field 5 omitted only for an eligible served command whose full budget expired after proven pre-effect holder contention. Use the C12 registration already on staging. Terminalize through `S/outbound.rs:77` and `G/session_ingress.rs:620` before writing. Duplicate replay on the same GameSession, including eligible resume in the retention window, rewrites only envelope generation, server sequence, and status (`DUPLICATE_REPLAY`). The command ID, 1202 and omitted field 5 survive. Apply the existing retained-outcome expiry behavior. The owner executes the original ID at most once. FND-02 §§8, 13.1–13.3 and 18 and the existing `CommandResult` message cover this use. [FND-02] · [CommandResult] · [Registry]

Proposed typed evidence is `PreEffectContention::HolderBusy`. Construct it at the command's checkout edge, before calling `run` or `run_with_context` and before transaction BEGIN. The underlying checkout must explicitly distinguish contention on a live or reserved holder from root unavailability. A failed checkout issues zero SQL. The adapter must also own evidence that the command has no earlier effect or unresolved attempt. Read-only receipt or fence preflight may already have issued SQL in an earlier pass. Only a successful preflight can classify that earlier state. A failed receipt lookup for a possibly issued ID cannot become 1202. Post-issue errors, including identical-looking availability errors, always retain the issued/pending path. Carry the typed evidence through retries, then check the same authority predicates again on retry before any new issue.

| Authority | Proposed raise point and proof | Lost-reply PostgreSQL control in `G/postgres_tests/` |
|---|---|---|
| USE | `D/item_use.rs:505` before the type-2 BEGIN at :508, and the corresponding C12 per-owner preflight and writer checkouts, including Stone at `G/adventurers_stone.rs:281,292` and catalyst at `G/weapon_proficiency_select/catalyst.rs:221,252` on `485d7756`. Carry `PreEffectContention::HolderBusy` only for the new command or a checked receipt miss with no pending owner. Retained rune, callback and item receipts win. | `busy_holder_use_committed_reply_lost_postgres` loses the COMMIT acknowledgement after the durable use receipt. Reconcile one item effect and return the committed result for the same ID. |
| MOVE | `D/item_transfer.rs:525` before the transfer writer and :664 before a new command's receipt read. Check the retained move lane and any issued transfer work before terminalizing. Keep the reconciliation checkout at :629 and all genuinely stale bindings on their existing paths. | `busy_holder_move_committed_reply_lost_postgres` loses the COMMIT acknowledgement and recovers the original move receipt with one source/destination change. |
| Paid Forge | `D/item_forge_live.rs:195` before the first reservation transaction at :198, after classifying any prior receipt at :118. The checkout at :287 follows committed reservation and attempt accounting at :258–276. Contention there leaves the original command pending for reconciliation with that reservation. | `busy_holder_forge_committed_reply_lost_postgres` loses the final paid COMMIT acknowledgement. Reconcile the same result, debit once, mint once, and preserve the exact item custody. Also exercise contention between the two transactions. |
| Logout | `D/fresh_admission/terminal_release.rs:23` before `run_pass` and BEGIN at :26. Use the P18 release window. Emit the variant only on proven unissued release with safe settlement of the exact local transition fence. Pending callbacks, training/familiar saves or an uncertain release are reconciled under the fence. | `busy_holder_logout_committed_reply_lost_postgres` loses acknowledgement after the release transaction at :112. Reconcile lifecycle and custody once, retire the old generation, and observe the accepted terminal release or its existing lifecycle disposition. |
| Appearance change | `D/character_appearance.rs:198` before the writer runs, after the exact request receipt preflight at `G/appearance_change.rs:299–319`. Clear only the same command's proven-unissued preparation when emitting the refusal. Keep a possibly committed preparation and resolve the immutable receipt first. | `busy_holder_appearance_committed_reply_lost_postgres` loses acknowledgement after the appearance COMMIT at `D/character_appearance.rs:274`. Reconcile the same occurrence with one appearance and revision change. |
| Expert mode | `G/pvp_expert.rs:40` before `run_with_context` and BEGIN at :45, and at the preceding authority-read checkout at `D/character_authority.rs:202` reached from :33 through `G/quest_admission.rs:183`. Carry the variant only before any mode install at :103. Keep the existing control, scope and PvP checks. | `busy_holder_expert_mode_committed_reply_lost_postgres` uses real PG context reads, then drops the outbound reply after runtime mode install and command-stream terminalization. The SQL transaction is rolled back at :63. Reconcile the retained successful result and assert one mode install. |

Use the existing `D/db.rs:2278` stalled-COMMIT-response hook where applicable. For every control, independently prove the commit or runtime install landed, reconcile the original command, and assert that its outcome never becomes 1202. Keep successful logout's normal transport close. Keep post-commit refresh failures out of refusal handling. In particular, Forge paint/inventory refresh follows a committed mutation and defers on holder contention without changing the command result.

Estimated **700–1,050 hand-written lines** including the PR (b) cases. Main review risks are false pre-effect classification, a fence lifted after an unknown COMMIT, a missed wake on cancellation, lock inversion, and an empty refusal incorrectly sent to a command-specific decoder.

### 1202 client consumer map

Line anchors in this section are on C12 `485d7756`. Refresh them against the landed SHA for implementation.

| Client surface | Required handling |
|---|---|
| Shared registry and codec | `crates/protocol-oteryn/src/tests/error_registry_tests.rs:29,68,150` checks registry parity. `crates/protocol-oteryn/src/lib.rs:218,300,338,1483,2718` owns code, operation-terminal classification, public text and result validation. Preserve the code through `crates/protocol-oteryn/src/command_replay.rs:13`. Accept the envelope-only result before any payload decoder. |
| Session sequence and duplicate retention | `crates/session/src/session_transport.rs:25` owns sequence and push framing. `crates/session/src/session_exchange.rs:24,240,263,279` owns poisoning, duplicate demultiplexing and normal refusal. C12's normal branch handles 1202, while the earlier duplicate branch currently drops error codes. Preserve 1202 in `crates/session/src/session_types.rs:59` and `crates/session/src/session_access.rs:289`, keep duplicate outcomes from consuming the current command's result or delta, and carry it through the dev client facade at `tools/dev-client/src/lib.rs:393` and its error translation at :98 and USE/MOVE at :465,474. `crates/session/src/error.rs:21,211` keeps the typed refusal nonfatal. |
| USE | `crates/session/src/session_exchange.rs:108` maps core USE to a rejected outcome without reading a delta. `crates/session/src/features/0011-world-interaction/protocol.rs:8`, `crates/client-runtime/src/features/0011-world-interaction/protocol.rs:83,93`, `apps/client/src/play/session_adapter.rs:327,377`, and `apps/client/src/play/session_stepper_methods_1.rs:308,358` reach the raw and gated USE replies. `apps/client/src/features/0011-world-interaction/protocol.rs:13,31–37,91` clears targeting/pending state and surfaces refusal before refreshing dependent views. |
| MOVE | `crates/session/src/session_commands.rs:109,130` and `crates/session/src/features/0004-item-view-move/protocol.rs:58` cover ordinary and counted MOVE, then `crates/client-runtime/src/features/0004-item-view-move/protocol.rs:28` and the adapters at `apps/client/src/play/session_adapter.rs:26` and `apps/client/src/play/session_stepper_methods_1.rs:11`. Route through `apps/client/src/features/0004-item-view-move/protocol.rs:69,86,103`, `apps/client/src/play/session_loop.rs:313` and `apps/client/src/play/session_loop_run_session_1.rs:59`. Retire pending move or batch UI and keep inventory and delta cursors unchanged. |
| Logout | `crates/session/src/session_commands.rs:281,301`, `apps/client/src/play/session_adapter.rs:176`, `apps/client/src/play/session_stepper_methods_1.rs:157` and `apps/client/src/play/session_loop.rs:448` use the nonfatal refusal path to clear the queued logout. Reuse `apps/client/src/play/mailbox/operations.rs:196`. Only an accepted logout ends the session. |
| Forge, appearance and expert modes | `crates/session/src/gated_command.rs:13,25–36` and `crates/session/src/appearance_state.rs:90` must see the typed refusal before result validation. Client gates are `crates/client-runtime/src/raw_feature.rs:17`, `apps/client/src/play/session_adapter.rs:337,377`, and `apps/client/src/play/session_stepper_methods_1.rs:318,358`. Feature and mechanic dispatch use `apps/client/src/play/session_loop.rs:230,246,253,323,337` and properties at `apps/client/src/features/0115-mail/protocol.rs:147`. Clear submitted UI and retain the last authoritative projection in `apps/client/src/features/0152-item-forge/protocol.rs:12`, `crates/client-ui/src/features/0152-item-forge/protocol.rs:73`, `apps/client/src/features/0025-appearance/protocol.rs:74` and `apps/client/src/features/0087-pvp-expert/protocol.rs:50`. |
| Shared UI and other envelope readers | `apps/client/src/play/refusal_details.rs:7` and `apps/client/src/play/session_loop.rs:288,385,616` handle C12's nonfatal feedback. Check the early command handlers at :100,143,170,191 before `apps/client/src/play/end.rs:8` can close the UI. `crates/session/src/activity_handoff.rs:203,209` validates the mandatory scope acknowledgement, and `crates/protocol-oteryn/src/daily_boost_login.rs:113` reads the one-shot login query. Neither is a producer of the six served-command results during its separate exchange. Preserve their contract and keep unsupported statuses out of their success paths. |

## Mounted PostgreSQL cases

Every case starts with `let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };`. Place new snake_case files in the mounted directories. Use a barrier on the actual holder to test release just within budget and exhaustion. For served-command cases assert exactly one refused result, no effect, retained ID replay, and a liveness probe and succeeding new ID on the same socket. Include separate stale-authority controls. The lost-reply cases above use the same dispatch mount.

| PR | Directory | Test and proof |
|---|---|---|
| (a) | `G/postgres_tests/` | `busy_holder_fifo_tracks_all_checkouts_postgres` covers raw, maintenance, scheduled and foreground checkouts, observed returns, owner/site/age, oldest eligible wake and cancelled head handoff. |
| (a) | `G/postgres_tests/` | `busy_holder_join_shared_deadline_postgres` spans fresh and resumed snapshot/repair, verifies one budget, and forces resume join exhaustion before any new grant use. Assert E1051, close, no additional consumed grant or controller, and a successful eligible reconnect after release. A separate late fresh-join subcase proves a receipt that already committed is preserved and the unpublished controller is cleaned up before recovery. |
| (a) | `G/postgres_tests/` | `busy_holder_admission_budget_postgres` holds through the fresh pre-commit deadline. Assert generation-zero E1051, close, no grant consumed, no extra controller, and successful admission on a new connection after release with the same still-valid unconsumed grant. |
| (a) | `N/tests/support/authority_cases/` | `busy_holder_spawn_training_postgres` checks the operator's holder refusal, zero spawns and live gameplay, combat and tick. After release, one spawn and one deferred checkpoint complete. |
| (a) | `G/postgres_tests/` | `busy_holder_enrichment_degrades_postgres` checks appearance, speaker, expert-mode observation and statistics, including skipped cursors and continuing combat/tick/liveness. |
| (b) | `G/postgres_tests/` | `busy_holder_use_refusal_replays_postgres` holds receipt preflight or the proven unissued writer and checks no item effect. |
| (b) | `G/postgres_tests/` | `busy_holder_move_refusal_replays_postgres` holds receipt or writer and checks unchanged source and destination. |
| (b) | `G/postgres_tests/` | `busy_holder_forge_refusal_replays_postgres` holds first paid issue and checks no reservation/attempt charge, debit, transfer or mint. |
| (b) | `G/postgres_tests/` | `busy_holder_logout_refusal_replays_postgres` proves the lease and controller remain live. A later new logout commits and closes normally. |
| (b) | `G/postgres_tests/` | `busy_holder_appearance_refusal_replays_postgres` checks no mutation, revision change or cursor advance. |
| (b) | `G/postgres_tests/` | `busy_holder_expert_mode_refusal_replays_postgres` checks no mode install on refusal and unchanged generation/control checks. |

Run real PostgreSQL on the authorized local or CI route. For changes to a SHA-pinned test, rerun its PG cases for real with duration above 0.01 s, then re-import via the owning proof tool such as `quest_reward_use_proofs.py import_log`. Boot, serve or content activation changes require the full local `tools/qualification/node_boot/run.sh` matrix, including `spell_input=full_manifest`.

## Generated site dispositions

[Site table] includes one row per source revision, file and line. `Matched pattern` gives the pinned revision, matching query and source text. `Consumer` identifies the enclosing function or method at the matched line. Shared leaves inherit the policy of the caller. For ambiguous routes, `keep` preserves the current end or reconciliation until positive pre-effect or advisory evidence reaches the consumer. The indirect producer scan includes option propagation, error erasure and owner adapters. Its broad syntactic matches include unrelated local branches. Refresh and review the table against the landed implementation base before each PR.

From the repository root with the staging and C17b objects available:

```sh
python3 docs/agents/tasks/generate_busy_holder_sites.py > docs/agents/tasks/BUSY_HOLDER_SITES_2026-11-27.tsv
```

The [generator] checks 413 C17b `ServeEnd::server` sites and includes the 71 direct `state_projection_unavailable` sites as a tagged subset of that same scan. Reproduce the subset from the root with:

```sh
git grep -nF 'ServeEnd::server("state_projection_unavailable")' 342c099bcd197f10ecb69baecaaa0c6300a1b267 -- crates/gameplay/dispatch/src/gameplay_transport/connection/serve
```

| Disposition | Rows |
|---|---:|
| keep | 2,423 |
| retry-then-refuse | 58 |
| degrade | 90 |
| unrelated | 813 |
| **Total** | **3,384** |

Overlapping query tags count 1,142 checkout lines, 506 explicit session-end lines, 65 route-producer lines, 1,446 indirect-producer lines and 413 C17b direct-close lines. The 71 projection lines are included in the C17b count.

[Site table]: BUSY_HOLDER_SITES_2026-11-27.tsv
[generator]: generate_busy_holder_sites.py
[FND-02]: ../../architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md
[FND-02 §§11–12]: ../../architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md#11-bootstrap-and-admission-boundary
[FND-04C §4.1]: ../../architecture/FND-04C_ERROR_DIAGNOSTICS_FAILURE_COMPATIBILITY_CONTRACT.md#41-fresh-admission
[CommandResult]: ../../contracts/protocol-oteryn/v1/foundation.proto#L171
[Registry]: ../../contracts/protocol-oteryn/v1/registry/registry.json#L48
