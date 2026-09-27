# DUR-03 deterministic one-item resource evidence

Classification: `NON_PRODUCTION_RESOURCE_EVIDENCE_ONLY`.

Task: `OTV2-20260917-dur03-reference-one-item-resource-evidence-513`.
Source base: `fd504f5659fe900861b6556751275488c2a22dec` in `Oteryn/Oteryn-Game`.
Allocation: [#162 comment 5854116222](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5854116222).
Requirement: [#513 comment 5712622190](https://github.com/Oteryn/Oteryn-Game/issues/513#issuecomment-5712622190).

## Scope and authority

`PROVEN`: the standalone example consumes the existing public GAME-ITEM
`ItemInstanceId`, `ItemInstance`, `ItemDefinitionRef`, `ItemLocationRef` and
`ItemPlacementPolicy` types. It changes no server source, Cargo/workspace/lock file,
SQL/migration, resource registry, event family/schema, protocol or production path.
The fixture is **Gold Coin x1**, with `natural_drop_probability = UNKNOWN/NOT_ASSERTED`.
The private numeric definition key/revision `(1, 1)` labels this fixture only; it
does not resolve a real Content catalog definition or prove Reference parity.

`DERIVED`: the accepted DUR-03 semantics motivate stable transaction/output IDs,
one immediate location, no second effect on ambiguous retry, and audit-budget
preflight. Their implementation here is an in-memory evidence state machine.
Its `committed` fact represents a separately inspected committed-store fixture,
not a PostgreSQL transaction, durable receipt, restart, lease/session or isolation proof.

The private `EvidenceTransaction([u8; 16])` validates UUIDv7 version/variant bits
and accounts the accepted ANL identity width. Fixed deterministic identity bytes
are fixture data, not a unique production allocator. Nothing exports a production
`TransactionId` API. MINT fixes its output ID before the first possibly ambiguous
attempt; TRANSFER uses that same item ID and a distinct logical transaction ID.

## Representation and measurements

The retained qualified run is `PROVEN` for the Linux x86_64 executable target.
The executable reports its own target OS/architecture from `std::env::consts`.
`build_toolchain_identity = UNKNOWN`: this standalone example has no build-time
compiler attestation. It neither probes runtime rustc nor binds a compiler found
on PATH to the binary's build identity. Separately, the validation environment
observed `rustc 1.94.0 (4a4ef493e 2026-03-02)`; that observation is not build
compiler proof. Another target reports its own OS/architecture and retained
sizes, while its build toolchain remains explicitly unknown without attestation.
`size_of` includes the executable target's padding;
these are retained carrier sizes, not peak process RSS, allocator high-water marks,
SQL rows, protobuf events or production memory limits. Encoded bytes below are
the actual explicit big-endian example-only participant/effect encoding.

| Measurement | MINT | TRANSFER |
|---|---:|---:|
| Touched items / quantity | 1 / 1 | 1 / 1 |
| Participant records | 1 | 1 |
| Participant retained / encoded bytes | 44 / 44 | 44 / 44 |
| Custody/effect records | 1 | 2 |
| Effect retained bytes | 72 | 144 |
| Effect encoded bytes | 58 | 94 |
| Participant + effect encoded bytes | 102 | 138 |
| Fixed Plan bytes, including intent and spare effect slots | 344 | 344 |
| Planning / application work units | 2 / 2 | 3 / 3 |
| Container expansion work | 0 | 0 |
| Reconciliation records / retained bytes per logical operation | 1 / 360 | 1 / 360 |
| Retry/reconcile work units per call | 3 | 3 |
| Private transaction identity width | 16 | 16 |

MINT's single `ESTABLISH` line describes the fixture Ground/corpse custody and
encodes 58 bytes. TRANSFER has a 58-byte `REMOVE` Ground line and a 36-byte
`ESTABLISH` direct-root CharacterInventory line. Both lines are validated before
one authoritative custody tuple is replaced; they are never two simultaneous
authoritative locations. TRANSFER requires MINT's acknowledged/reconciled result.

Planning work counts one participant visit plus each effect visit. Application
work counts each effect plus one receipt publication. Retry/reconcile counts
receipt lookup, identity checking and outcome resolution as three abstract units.
These units exclude encoding, actual lookup implementation cost and wall time;
they do not claim production CPU cost. No recursive container traversal exists.

The final fixture holds one item at one direct-root CharacterInventory location.
The two reconciliation records occupy 720 bytes, and the fixed Model occupies
816 bytes including its custody tuple, optional slots and padding. The report's
lost-response/duplicate/reconcile sequence consumes six cumulative retry work
units for each logical operation and preserves their original IDs.

## DUR03-RL-01 through DUR03-RL-08

| Row | Disposition and bounded proof |
|---|---|
| DUR03-RL-01 | `ONE_ITEM_SEMANTIC_FIXTURE`: OneItem contains exactly one ItemInstance. MultipleItems has no item list/plan and rejects before planning. This is not a broad transaction maximum. |
| DUR03-RL-02 | `MEASURED_PROTOTYPE_ONLY`: actual 1/2 custody effect records, 72/144 retained and 58/94 encoded bytes, checked against injected evidence limits. One custody tuple prevents dual immediate truth. |
| DUR03-RL-03 | `NOT_EXERCISED/0`: ValueLine and AccountLine reject before planning. |
| DUR03-RL-04 | `NOT_EXERCISED/0`: Transform rejects before planning; MINT and TRANSFER retain their classifications. |
| DUR03-RL-05 | `DIRECT_ROOT_ONLY/0`: Container rejects before planning, even with zero budgets. Zero expansion describes only this child, never future container pickup. |
| DUR03-RL-06 | `MEASURED_PROTOTYPE_ONLY`: independent operation counts, retained/encoded bytes and deterministic work derive from the concrete records, not source/destination prose. |
| DUR03-RL-07 | `EVIDENCE_GAP`: actual mandatory audit event contribution count and aggregate encoded bytes are null. A separately labelled synthetic size-only probe tests the budget boundary; it is not an event/payload measurement. |
| DUR03-RL-08 | `MEASURED_IN_MEMORY_STATE_MACHINE_ONLY`: one frozen logical operation record with stable transaction/output IDs. Duplicate, lost response, unknown reconciliation, known abort and retry exhaustion do not mint/transfer again. |

## Injected evidence limits and boundary cases

The caller injects finite limits. The report uses participants 1, effects 2,
participant retained bytes 44, effect retained bytes 144, record encoded bytes
138, planning/application units 3 each, reconciliation records 2, reconciliation
retained bytes 720 and cumulative retry work 6. These are evidence fixture inputs,
not accepted production maxima, and the resource registry remains unchanged.

For each of MINT and TRANSFER, the focused matrix covers 11 preflight dimensions:
participants, effects, participant/effect retained bytes, record encoded bytes,
planning/application work, synthetic audit contributions/bytes, reconciliation
records and reconciliation retained bytes. Each dimension accepts usage at its
injected maximum and rejects the same physical usage at `max + 1` (limit one
smaller). The rejection compares the entire Model to the unchanged baseline.
This gives 22 accepted and 22 rejected resource-boundary cases.

Retry/reconcile work is separately tested at three units, then exhausted at the
next call without changing the original record, transaction or custody; raising
the injected budget to six allows reconciliation of the same result. Known-abort
retry rechecks its frozen audit and work budgets before publication. Checked
count addition, byte multiplication, reconciliation retained-byte and cumulative
retry arithmetic overflow fail before dynamic encoding allocation/publication.
Fixed stack carrier creation is bounded by the example's semantic shape.

One-invariant negative fixture cases change only current WorldId, definition key,
definition revision, authority generation or corpse revision, independently for
MINT and TRANSFER. Additional cases reject a mismatched Ground world and a
mismatched transfer item. The supplied CurrentFacts are independent fixture input,
not evidence reconstructed from the pending record, and are not production
current-authority proofs.

Replay cases include an ambiguous committed MINT/TRANSFER, a lost response,
unknown reconciliation that holds the same candidate, a proven noncommit followed
by retry of the exact candidate, conflicting intent/probe under one identity,
and a changed transaction/cause attempting to replace an ambiguous operation.
A stale corpse projection cannot remint; a lost TRANSFER response cannot restore
source custody. The example permits just one MINT and one TRANSFER logical slot.

## Audit gap

`UNKNOWN/EVIDENCE_GAP`: bounded code search at the consumed base finds no executable
DUR-03 event family/payload in `apps/game-server/src` or `crates/foundation/src`.
DUR-03 section 39 and ANL-01 sections 6, 8 and 9 fix identity, envelope membership,
exact payload stability and atomic evidence semantics, but do not choose the
concrete item transaction event family, contribution count or payload shape.
Thus neither count nor total encoded event bytes can be measured here.

The injected `AuditBudgetProbe { contributions: 1, encoded_bytes: 16 }` is synthetic
size-only boundary input. Its 16-byte sentinel matches identity width only; it is
not a complete ANL event size. No event is emitted, no event family or schema is
registered, and actual mandatory audit fields remain null. Exhausting either
synthetic audit budget rejects before the custody tuple or reconciliation slot
is published, including a known-abort retry. This proves the prototype's budget
check placement; it does not prove production mandatory audit atomicity.

## Reproduction and authoring validation

Use an isolated Linux checkout with LF files and the exact consumed base plus the
four-file authoring delta. The validated Linux workspace was
`/home/mole/dur03-513-p2.9unFjx/repo`, freshly cloned at local predecessor
`d561af69b4c57fd5b844b2810f9019cdf5ce7bd9` plus the bounded authoring delta;
tracked source showed `i/lf w/lf`. The earlier
`/tmp` fixture directory was cleared between WSL sessions; the retained user-directory
checkout is the completed validation source. A failed initial Windows-linked
worktree clone led only to a nonmutating formatting check on CRLF files; it is
not counted as a successful formatting check. No shared formatter write occurred.

```sh
cargo fmt --all -- --check
cargo test --offline --locked -p oteryn-game-server --example dur03_reference_one_item_resource_prototype --no-default-features
cargo build --offline --locked -p oteryn-game-server --example dur03_reference_one_item_resource_prototype --no-default-features
cargo clippy --offline --locked -p oteryn-game-server --example dur03_reference_one_item_resource_prototype --no-default-features --no-deps -- -D warnings
target/debug/examples/dur03_reference_one_item_resource_prototype > run1.json
target/debug/examples/dur03_reference_one_item_resource_prototype > run2.json
target/debug/examples/dur03_reference_one_item_resource_prototype --reverse-fixtures > reversed.json
cmp run1.json run2.json
cmp run1.json reversed.json
cmp run1.json docs/agents/evidence/OTV2-20260917-dur03-reference-one-item-resource-evidence.json
sha256sum run1.json run2.json reversed.json
python3 tools/agents/validate_governance.py
python3 tools/repository/validate_repository_policy.py
cargo run --offline --locked -p oteryn-architecture-check -- workspace .
git diff --check
```

Results: **16 focused tests passed, 0 failed**; locked build, strict example Clippy
and full-workspace rustfmt check passed. The vendored Tokio 1.53.1 dependency emits
its existing `missing_docs` warning for `sync/oneshot.rs:502`; the example has no
new warnings and no lint suppression of dependency warnings was added.
Local governance PASS: 22 required policy documents and nine project lanes.
Local repository policy PASS: 23 files, 43 workflows, including post-merge
exact-candidate routing regressions. The candidate-specific inherited-policy
and routing checks remain pending hosted PR CI after the control plane creates
the PR and freezes the remote SHA. The worker did not fabricate an event/PR or
run the pre-PR CLI's unrelated full active-task health scope.
Local `workspace-boundaries: PASS`; staged `git diff --check: PASS` and exactly
the four allocated paths are added. All four index blobs are LF, and the Rust
source and isolated Rust validation checkout are LF.

Normalized successor output: 6,795 bytes, including the final LF; SHA-256
`ea79b26bd3412f478b6ffe43ce277625e7afa513f68556d66b855fa67690920f`.
Both repeated runs and reversed **presentation** fixtures are byte equal. Only
presentation order is reversed, then normalized by mutation class; the required
execution dependency MINT before TRANSFER remains intact. The retained JSON is
the program's actual normalized output, not a hand-entered measurement summary.

Independent-review P2 thread `4114686069` disposition: **fixed**. Runtime rustc
probing/identity was removed. Dynamic target metadata remains, and build toolchain
identity is explicitly `UNKNOWN` with the absent-attestation reason. The focused
regression checks these fields and excludes the former runtime identity fields.
The 15 existing correctness
tests still pass, and every physical count/retained-byte/encoding/work measurement
is unchanged from the preceding authoring candidate. Determinism is qualified on
this environment, not claimed across different toolchains/targets.
An additional executable check with rustc absent from PATH succeeds with byte-identical
JSON: normalized evidence no longer depends on finding a runtime compiler.

These are authoring-delta checks. Exact remote candidate freeze, candidate-specific
repository CI, independent review and Merge Queue remain control-plane work;
the record does not claim a PR, qualification, merge or protected integration.

## Publication hold

The authoring worker returns a local four-path candidate, without pushing. At
bound META commit `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`, the Publication
Integrity Policy lines 58-60 require a verified exact-candidate recovery artifact
before first publication; lines 66/72 require the expected-old-value lease and
guarded transport. The current allocation forbids force options and adds no
helper/recovery path authority. Under the coordinator's stop instruction, the
worker creates no fifth authored path, helper or bundle and does not bypass the
guard with a plain push. The control plane must reconcile publication before
remote freeze; candidate-specific hosted checks remain pending.

## Remaining evidence and excluded claims

`UNKNOWN`: production transaction ID allocation, exact registered audit event
count/bytes, PostgreSQL atomic receipts/isolation/fencing, restart/crash recovery,
production maxima and replay/retention horizon, real Content definition binding,
connected Combat/loot/pickup and Reference parity. Natural Rat loot probability
remains `UNKNOWN/NOT_ASSERTED`. No production behavior or architecture acceptance
is selected by the injected limits or these component measurements.
