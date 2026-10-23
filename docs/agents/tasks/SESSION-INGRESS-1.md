# SESSION-INGRESS-1 — common session-owned command ingress

```yaml
task_id: SESSION-INGRESS-1
title: "SESSION-INGRESS-1: common session-owned command ingress"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-davinci-ijz3wy-ingress
base_sha: c938f50b4f727cb0dc1adf60ca62f7d63f4f6541
pr: null
owner: codex-session-ingress
created_at: 2026-10-23
updated_at: 2026-10-23
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/dispatch/src/gameplay_transport/
  - crates/protocol-oteryn/src/command_replay.rs
  - crates/protocol-oteryn/src/lib.rs
  - docs/contracts/resource-limits/entries/0690-SESSION-INGRESS-RESULT-BYTES.json
  - docs/agents/tasks/SESSION-INGRESS-1.md
public_contracts:
  - docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md
depends_on: [ADMIT-CAP-1]
blocks: [TRADE-PLAYER-1, HOUSE-RUNTIME-1b, MAIL-1]
merge_authority: CP_ONLY
needs_migration: false
needs_registry: true
needs_content_regeneration: false
```

## Outcome and scope

One GameSession command owner consumes Foundation `CommandIngress` for all production
command families. Existing family rules, atomic durable writers and capability selection
remain their owning implementations. Client UI and D1377 physical exclusion are excluded.
This worktree is authoring only: no commit, publication, review trigger or merge.

## Server wiring gap list

| Layer | Evidence on starting HEAD | Missing server wiring |
| --- | --- | --- |
| L3 rules | `foundation/src/foundation/mod.rs`: bounded `CommandIngress`, reserve, terminalize, duplicate classification, recovery | Production consumer; currently only tests consume it |
| L4 durability/storage | Each command family has its own fenced atomic writer and receipts; fresh admission/reconnect journals preserve session authority | Session command state must survive connection replacement; no new gameplay storage schema needed |
| Migrations | Existing admission, appearance, Bank and item writers own durable rows | None: do not add a second command ledger or change family transactions |
| Dispatch | `gameplay_transport/connection.rs`: local `next_command` counter and unconditional lower-ID expiry | Session-owned reservation, bounded latest-result retention, replay and recovery checks |
| Wire/registry | Existing `ClientCommand`, `CommandResult`, `DUPLICATE_REPLAY`, expiry/gap errors and bounded codecs | Replay encoder preserving the retained result under the new generation/sequence; no new IDs |
| Capability offer | Foundation command stream is mandatory; existing family capabilities are already negotiated | No content gate or optional ingress capability; shipped worlds use the common path |
| Runtime caller | Listener calls `serve_admitted` with `ComposedFreshAdmission`; `ChannelSessionStates` already owns per-session state | Install the ingress book there; forget it at terminal retirement and check it before same-session recovery |

## Authority and recovery

PROVEN: FND-02 §§13–14 requires one session command namespace, no duplicate execution,
bounded retention and preserved ordering across eligible same-session reconnect. Family
receipts remain durable truth. Owner process restart cannot invent missing command truth:
the existing lost-session directory is process-local; a new owner must refuse same-session
continuation without that truth. Pending work whose terminal outcome is unknown must fail
closed rather than be executed a second time by the common ingress.

## Reference parity

| Visible behavior | Reference evidence | Oteryn now |
| --- | --- | --- |
| One ordered command namespace per logged-in session | FND-02 §13; existing native command families | `ChannelSessionStates.command_ingress` owns one Foundation stream. `serve_admitted` reserves before dispatch and terminalizes before result publication. All existing families use the common owner. |
| Duplicate never repeats a durable effect; latest terminal outcome can replay | FND-02 §§13.2–13.4 | Latest result retained within 65,664 bytes; lower retained ID replays without interpreting replacement bytes or calling a family. An older evicted ID expires; a future ID reports the expected ID. |
| Same-session recovery preserves ordering and retained truth | FND-02 §13.3; FND-04B §16 | Foundation recovery validates the session stream when a replacement connection opens. Recovery checks retained truth and the result sequence floor before the committed resume; unknown pending work refuses resume. |
| A durable result survives storage restart | Existing native appearance transaction/receipt, APP-3 | PostgreSQL E2E sends appearance command bytes through production dispatch, proves exactly one receipt across accepted/replayed/refused commands, restarts the root, and reloads selection at revision 2. |
| Logout still delivers its final result | Existing logout retirement order | The active stream retains its state after directory retirement, allowing terminal result publication without leaving a resumable session. |

## Implemented server path

PROVEN: `serving.rs` creates one composed authority for the listener lifetime, whose
session directory now holds the ingress book. Fresh admission registers the session;
terminal retirement forgets it. Each active connection holds a bounded reference to its
stream, so retirement during logout does not remove its pending final result. No database
lock or session mutex is held across a storage await. Existing family transaction fences
and receipts are unchanged; the common stream never retries an uncertain mutation.

The existing `ClientCommand`/`CommandResult` codecs and IDs are reused. The replay module
preserves typed payload, error, revisions and unknown result fields while changing status,
connection generation and sequence. Independent byte and boundary tests cover this rewrite.
There is no new request/result schema, optional ingress capability or missing content gate.
The appearance E2E uses the same content-dependent offer augmentation as `admit_frame`.

The live stream is session-owned runtime state, not a new durable command ledger. A whole
owner restart without that state refuses the old logical session; existing durable family
results remain readable. No claim of cross-process same-session reconstruction is made.

## Validation evidence

Final Cargo qualification uses the supplied queued wrapper (non-login shell), locked dependencies
and the pinned toolchain. PostgreSQL scope is this mechanic only; no full PG suite was run.

| Check | Evidence |
| --- | --- |
| `cargo test --locked -p oteryn-gameplay-dispatch --lib --quiet -- session_ingress_postgres_real_dispatch_replay_and_recovery --nocapture` | PASS, 1 actual PostgreSQL case, including durable reload after root restart |
| `cargo test --locked -p oteryn-gameplay-dispatch --lib --quiet -- gameplay_transport::session_ingress::tests` | PASS, 3 lifecycle tests: reserve/pending/recovery/eviction, uncertain-work refusal, retirement before final result |
| `cargo test --locked -p oteryn-gameplay-dispatch --lib --quiet -- gameplay_transport::connection::` | PASS, 73 existing wire/connection tests |
| `cargo test --locked -p oteryn-gameplay-dispatch --lib --quiet -- gameplay_transport::capabilities::tests` | PASS, 44 existing capability/command tests |
| `cargo test --locked -p oteryn-protocol-oteryn --lib --quiet` | PASS, 389 tests including the two replay byte/boundary tests |
| `cargo clippy --locked -p oteryn-gameplay-dispatch -p oteryn-protocol-oteryn --all-targets --quiet -- -D warnings` | PASS, touched crates and all targets compile and lint; cached vendor deprecation warnings are outside the diff |
| `cargo fmt --all --check -- crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/session_ingress.rs` | PASS, explicitly includes the auto-mounted case, which Cargo fmt does not discover via generated mounts |
| `python3 tools/agents/validate_governance.py` | PASS |
| `python3 tools/repository/validate_repository_policy.py` | PASS |
| `python3 -m unittest discover -s tools/agents/tests` | PASS, 129 tests |
| `git diff --check` | PASS |

D1462/D1456: reread the complete tracked delta and every added file against the owner
checklist. Workspace searches under `apps/`, `crates/`, `tools/` covered all added enum,
struct-constructor and authority-implementation sites. `ChannelSessionStates` uses its
derived default; the composed production authority supplies the new getter; injected unit
authorities keep their existing non-resumable test seam. Production refuses a missing book.
The new public replay encoder is called by the common production consumer and exercised
through the real-dispatch PostgreSQL E2E. No existing test was weakened or removed.

## Unfinished

- Publication, independent review and full repository CI are CP work on the later candidate;
  this user-authorized worktree remains uncommitted. The card stays `waiting`, with no PR.
- Additional optional `python3 -m unittest discover -s tools/contracts/tests`: 4 PASS, 1 FAIL
  in `test_split_of_the_aggregate_is_the_committed_tree`. Its first mismatch renumbers the
  untouched `0609-PARTYPVP0-RL-04.json` to `0598-PARTYPVP0-RL-04.json`. The registry tree
  validity test passes; this pre-existing round-trip/ordinal issue is outside this task.
- D1377 physical exclusion and downstream Trade/House/Mail activation remain separate cards.
