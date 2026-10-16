# GM-1 — GM tools, mute, kick, staff audit and reports (skeleton task record)

```yaml
task_id: GM-1
title: "GM-1: GM tools, mute, kick, staff audit and reports (skeleton)"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/gm-1
base_sha: 5742f30f3
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
updated_at: 2026-10-16
owner: "chat I / root lead; gm_record prepared the record-only correction"
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 6 row GM-1 (row 136); App. 7 GM-1; §5 Wave 5"
class: "hard (child graph)"
needs_migration: yes
needs_registry: yes
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "apps/game-server/src/staff/ (new: audit, sanctions, roster, reports) + tests"
  - "apps/game-server/src/bin/oteryn-game-ops.rs (existing executable, signed requests)"
  - "docs/agents/tasks/GM-1.md"
slot_only_paths:
  - "apps/game-server/src/node/serve.rs, apps/game-server/src/node/config.rs"
  - "migrations: game_character_sanctions, game_staff_roles, game_player_reports"
  - "docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (staff panel)"
  - "docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json (`staff_action`)"
  - "apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs, crates/protocol-oteryn/src/lib.rs"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in apps/game-server/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## Goal

Game side of OPS-GM-01: OPS-GM-AUDIT-1, SANCTION-2, REPORT-3, ROSTER-4, STAFF-5, PREPORT-6. No value-creating staff action.

## Dependencies (CP D1044, 2026-10-16)

| Dependency | State | Gate |
|---|---|---|
| LOGOUT-WIRE-1 | MERGED (#2127); former SANCTION-2 gate removed | cleared |
| WRITE-MOD-1 (REPORT-3 commands only) | merge required for REPORT-3 | **BLOCKING REPORT-3 ONLY** |
| OPS-GM-AUDIT-1 | first child PR; awaiting CP allocation after this record correction | allocation required |
| OPS-GM-P1 (Platform), ARCH-CONTROL-ANCHOR-0 | external gate for EXT phase only | ok |

WRITE-MOD-1 gates only REPORT-3. SANCTION-2 owns its session-generation
fences. Child implementation requires its own CP allocation; the corrected
family path inventory below does not allocate a child or authorize slot work.

## Owned paths (family inventory; child allocation required)

- apps/game-server/src/staff/ (new: audit, sanctions, roster, reports) + tests
- apps/game-server/src/bin/oteryn-game-ops.rs (existing executable, signed requests)
- docs/agents/tasks/GM-1.md

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- apps/game-server/src/node/serve.rs and apps/game-server/src/node/config.rs
- migrations: game_character_sanctions, game_staff_roles, game_player_reports
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (staff panel)
- docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json (`staff_action`)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs, crates/protocol-oteryn/src/lib.rs
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

`docs/contracts/game-events/v2/staff_action.proto` and the `STAFF_ACTION`
registration require a CP contract allocation. Shared module registration and
slot-only files require the allocated slot; this record authorizes no wiring.

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/chat/ (mute)
- logout path (LOGOUT-WIRE-1)
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- migration + writer + durability per child
- wire + registry + capability + handler + tests (STAFF-5)

## Acceptance criteria

- [ ] every staff action emits an audited signed `staff_action` event
- [ ] sanctions are staff records, not Character aggregate writes
- [ ] no value-creating staff action
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.

## Validation commands

```sh
cargo test -p oteryn-game-server staff
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_ARCH_LIVE_READINESS_2026-10-06.md §1
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- After CP allocates a child, fill only its owned_paths until CP gives the queue turn; then merge main and add allocated slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Local preparation — OPS-GM-AUDIT-1 (2026-10-15)

**Historical packet:** retained as evidence of the preparation at its stated
source SHA. The 2026-10-16 checkpoint below supersedes its UNKNOWN codec,
roster/head and clock bindings, guessed executable ownership, unresolved
OWNER-E, and LOGOUT/SANCTION gate. Its validation is historical, not validation
of this checkpoint; no child implementation was delivered by that packet.

This is a prerequisite and qualification packet, not delivery, acceptance or runtime
authority. Existing draft #2181 remains a draft. Work is local only on
`skeleton/gm-1` at `81e5a0f9263ff5cfac4628afa8e4fc54365164f0`; no commit,
publication, registration, schema, key generation or shared wiring is authorized.
The lead supplied META 3.1.0 binding
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb`: one authoring writer, independently
current authority at each consuming boundary, no new production grants or trust.

### Accepted scope and source evidence

- **PROVEN:** ARCH-LIVE-READINESS-0 §1 rulings 2 and 11 and its packet table
  define OPS-GM-AUDIT-1, including the request-signature module, as **ALPHA,
  first**. The lead independently verified the owning record on authoritative
  main. LOGOUT-WIRE-1 gates SANCTION-2 and WRITE-MOD-1 gates REPORT-3 commands;
  neither blocks this first child's preparation. No pending acceptance batch
  is asserted for ARCH-LIVE-READINESS-0.
- **PROVEN:** the accepted verifier is upstream `ssh-keygen -Y verify`, with
  personal off-host OpenSSH signing, the reviewed namespace-limited
  allowed-signers roster, restart-only environment/roster configuration, and
  independently recorded roster activations. A local key list, self-declared
  request environment or stored request alone cannot substitute for them.
- **PROVEN at the source SHA above:** `apps/game-server/src/staff/` and
  `docs/contracts/game-events/v2/staff_action.proto` are absent; the event
  registry has types 1–3 and no `STAFF_ACTION` or its retention profile.
  Bounded searches of `apps/game-server/src`, `crates`, `docs/contracts` and
  Cargo manifests found no reusable staff signing implementation or contract.
- **PROVEN:** `node/operator_files.rs` uses pretty JSON plus newline and
  ordinary JSON decoding for existing unsigned operations. It defines no staff
  signed-byte format or canonical-byte equality check. `NodeConfig` and
  `OpsConfig` in `node/config.rs` expose no staff environment, signer-roster or
  activation-history binding. `node/serve.rs::handle_control` accepts a bounded
  bootstrap operation UUID, not a signed staff request.
- **PROVEN:** ANL-01 §§2, 9 explicitly reject protobuf serialization as semantic
  canonicalization. Its exact event-byte retention rule is not a canonical
  operator-request signing format.

### Prerequisites before signature or audit implementation

1. **Canonical bytes (UNKNOWN binding):** §1 specifies the signed semantic
   fields, but does not bind a concrete encoding, canonical rejection rules,
   field representations or shared test vectors. Resolve the accepted codec
   and version for complete typed requests, per-signer tuples, chain links and
   submission receipts; bind timestamp units, digest encodings and digest
   inputs. Requests must include every parameter, environment, roster digest,
   `issued_at`, `control_seq` and previous-link digest. The receipt domain is
   `oteryn-ops-submission-receipt-v1`. Bind account-ref component encoding for
   the accepted SHA-256 domain formula without retaining AccountId. Do not
   choose an independent JSON/protobuf envelope or handwritten cryptography.
2. **Independent authority source (UNKNOWN interface):** bind the restart-only
   verifier environment and reviewed roster bytes/digest, plus an off-host
   private-deployment checkout's recorded activation sequence/digests. Define
   how the verifier obtains the current chain head independently from the
   submitted link. New submissions must name the newest activation; historical
   verification needs retained roster revisions and exact recorded links.
   No live keys or private deployment access is requested by this packet.
3. **Clock and operation outcome binding (UNKNOWN interface):** the accepted
   10-minute first-execution window is marked CANDIDATE; bind its limit and
   future-time treatment. Resolve the durable atomic operation-id/request-
   digest/outcome lookup and chain-append serialization. Identical bytes return
   the stored outcome at any age; different bytes under that operation id
   refuse. Stored signed evidence never supplies independently current facts.
4. **Audit transaction contract (slot work):** lease event/schema and ops-error
   identifiers through CP, register `STAFF_ACTION` and
   `STAFF_ACTION_AUDIT_RETENTION_V1`, then bind the existing durable event writer
   and authoritative mutation transaction. An audit failure must roll back its
   mutation; inspect/list/trace must commit their audit before exposing data.
   Retained evidence and signed `COMMITTED` receipts must outlive audit expiry
   whenever a surviving domain row relies on them. No schema is assumed here.

These are implementation-binding gaps, not a claim that the accepted security
semantics are undecided. No Rust files are created while the exact signing and
authority contracts are unbound. The accepted action/reason/result closed sets
in §1 ruling 11 must be reused in full, including economy/privacy/restore kinds;
no fallback string, value-minting command or unsigned bypass is allowed.

### Boundary qualification required when the prerequisites land

Each negative case must change one invariant while keeping unrelated facts
valid; expected refusal means no authoritative mutation, audit insertion or data
disclosure. This table is a test contract, **not executed runtime evidence**.

| Consumer boundary | Invariant / single negative operator | Required observation |
|---|---|---|
| First submit | Identity: missing signature; unknown principal; wrong namespace, independently | Each refuses before a write |
| First submit | Binding: other environment; other roster; altered typed target, reason or parameter, independently | Each refuses; execute only decoded verified bytes |
| Decode before verify/execute | Provenance: missing/unknown field; alternate noncanonical bytes, independently | Refuse without accepting an unsigned sibling field |
| First submit | Temporal: stale never-executed request; future request outside the bound policy | Refuse without applying; resolve exact future bound first |
| Retry lookup | Replay: same operation id and same bytes after expiry | Return exact stored outcome once; freshness cannot force re-execution |
| Retry lookup | Replay: same operation id and different bytes | Refuse without replacing stored outcome |
| Chain append | Current authority: stale sequence; wrong previous digest; obsolete roster after activation, independently | Refuse with chain and database unchanged |
| Concurrent submit | Replay/concurrency: two writers extend one predecessor | One ordered winner; typed loser, no fork or duplicate effect |
| Historical audit/restore | Provenance: missing middle link; duplicate sequence/fork; changed previous digest; unrecorded activation, independently | Failing verification; restore applies nothing |
| Historical audit/restore | Revocation: removed key signs old-roster bytes before/after activation | Recorded activation fixes history; flag and refuse replay |
| Audit verification | Binding: event kind/target/reason/one parameter differs from signed bytes; missing bytes, independently | Flag each failing row |
| Mutation verification | Provenance: missing receipt; valid REFUSED receipt; wrong request/outcome digest; changed receipt signer/namespace, independently | Row fails; intent never proves execution |
| Restore | Replay: valid COMMITTED receipt and absent outcome | Re-run own checks once; a current refusal applies nothing |
| Restore | Replay: valid REFUSED receipt whose original checks now pass | Preserve refusal; never execute or allow resolve-apply |
| Restore | Missing provenance: no verifying receipt | No replay; restore stops pending signed resolution |
| Read result | Audit commit fails | Inspect/list/trace expose no data |
| Mutation transaction | Audit insert fails | Domain mutation rolls back atomically |
| Retained domain row | Expired audit versus missing retained evidence/receipt/roster | Expired audit alone passes; missing verification material fails |

Session-generation mutation tests are **NOT_APPLICABLE to this no-runtime-change
packet**; future kick/sanction consumers must preserve their own current session
fences. Signature/chain/clock/receipt cases above are **BLOCKED, not passed**,
until the prerequisite interfaces and accepted fixture bytes exist.

### Cross-domain findings and next allocation

**REPORT_ONLY:** the packet's real executable integration paths are
`apps/game-server/src/bin/oteryn-game-ops.rs` and `node/serve.rs`, with restart
configuration in `node/config.rs`; they are outside this local worker's owned
paths. Do not create a guessed `tools/oteryn-game-ops` replacement. CP must route
those owners, migrations, proto, registry, error numbers and module registration
at a later authorized slot. EXT off-host-head closure remains
ARCH-CONTROL-ANCHOR-0; its absence does not cancel ALPHA first-child preparation.

Whole-diff self-review: this change only records the accepted semantics,
evidenced binding gaps and next qualification work. It introduces no new
authority, signed envelope, trust root, production behavior or readiness claim.

Local validation: `python tools/agents/validate_governance.py` PASS (22 policy
documents, 9 project lanes); `python -m unittest discover -s tools/agents/tests`
PASS (124 tests, 0 failures); `git diff --check` PASS. Owned-path inspection
confirms only this task record changed and HEAD remains the source SHA above.
No Rust/runtime tests ran, because there is no runtime change; the negative
signature/authority/restore cases remain unqualified as stated above.

## CP implementation-binding checkpoint (2026-10-16)

**PROVEN decision evidence:** [CP D1040–D1045](https://github.com/Oteryn/Oteryn-Game/pull/2181#issuecomment-6216592423)
and [OWNER-E acceptance](https://github.com/Oteryn/Oteryn-Game/pull/2181#issuecomment-6216812069).
Initial authoring head: `0be3b3f2673544916228bd6e14db6cf7b45ace7b`;
the only file in this correction is `docs/agents/tasks/GM-1.md`.
These resolve the historical packet's stated gaps as follows; accepted
architecture, closed action/reason/result sets, atomic audit, retained evidence
and restore requirements continue to apply.

- **D1040:** canonical bytes use JCS (D1029), a closed typed object containing
  every parameter, environment, roster digest, `issued_at` (uint64 milliseconds
  encoded as a decimal string), `control_seq` and previous-link digest. Use
  upstream `ssh-keygen -Y sign` / `-Y verify`; the request namespace is
  `oteryn-ops-request-v1` and receipt namespace is
  `oteryn-ops-submission-receipt-v1`. Verify before decoding, then reject any
  non-identical canonical re-encoding. This namespace binding supersedes the
  historical preparation's request-namespace assumptions.
- **D1041:** pin the restart-only roster configuration to the allowed-signers
  file SHA-256 and newest activation digest. Check the submitted previous-link
  against the DB head. ALPHA has no runtime activation; ROSTER-4 remains EXT.
  Retained revisions and recorded activations still support historical checks.
- **D1042:** use `crates/ntp-clock::read_synchronized`; `None` fails closed.
  First execution has a 10-minute window and permits at most 5 minutes future
  skew. Identical request bytes return the stored outcome; different bytes
  under the same operation id refuse. Retry never becomes a second execution.
  The clock interface is present in `crates/ntp-clock/src/lib.rs` at checkpoint
  source HEAD `0be3b3f2673544916228bd6e14db6cf7b45ace7b`.
- **D1043–D1044:** the active path inventory and dependency table above replace
  the guessed tools path and stale gates. `staff_action.proto` / registration
  remain a CP contract allocation, and SANCTION-2 carries session fences.
- **D1045 / OWNER-E:** AUDIT-1 is the first PR. SANCTION-2, REPORT-3, ROSTER-4
  and PREPORT-6 each receive a separate PR with their own migration. The owner
  accepted custody of GM signing keys and the initial roster off the host.

**Next boundary:** after this record/owned-path correction, CP allocates
AUDIT-1. No child has been allocated by this checkpoint; implementation,
fixtures, migration/event/error leases and runtime qualification remain
undelivered. The negative-case table remains a qualification contract, with
the bound clock windows and trusted DB head above replacing its open questions.
This local work changes only this record and grants no readiness, merge or
production authority.

Checkpoint validation: `python tools/agents/validate_governance.py` PASS (22
policy documents, 9 project lanes); `python -m unittest discover -s
tools/agents/tests` PASS (124 tests, 0 failures); `git diff --check` PASS.
No Rust build or runtime test was needed for this record-only correction.
