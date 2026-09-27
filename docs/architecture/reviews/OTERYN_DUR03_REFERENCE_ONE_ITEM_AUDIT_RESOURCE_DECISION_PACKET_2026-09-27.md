# DUR-03 one-item audit and resource decision packet

Date: 2026-09-27. Status: **PROPOSED / NONBINDING**. Repository:
`Oteryn/Oteryn-Game`. Allocation: [#162 comment 5854665056](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5854665056).
Requirement: [#513](https://github.com/Oteryn/Oteryn-Game/issues/513).
Consumed protected base: `76c2da68bde1928ab35e4e0f7828c675133cd297`.
This packet supplies a reviewable decision and a proposed next evidence allocation;
it does not accept a schema, register an event, set production limits or enable runtime behavior.

## Problem and authority

The integrated one-item resource evidence distinguishes MINT to Ground/corpse
custody from a later TRANSFER to direct-root CharacterInventory, but its mandatory
audit event contribution count and aggregate encoded bytes remain `EVIDENCE_GAP`.
A synthetic size probe demonstrates placement of a prototype budget check; it
cannot determine the item-audit payload, its event count or its serialized size.
The next proof needs an explicit typed evidence shape before it can measure those resources.

The consumed-base authorities are:

- [DUR-03](../DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md), especially
  §§4–5, 14, 17, 20–28, 39–43 and 46–49: identity, immediate location,
  conservation, ambiguity, atomic mandatory evidence and resource bounds.
- [ANL-01](../ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md), especially
  §§6–10, 12–18 and 20–22: the small common envelope, typed payload,
  transaction membership, immutable bytes, privacy and bounded evidence.
- [Game event registry](../../contracts/GAME_EVENT_FOUNDATION_REGISTRY.json)
  and [resource registry](../../contracts/RESOURCE_LIMITS_REGISTRY.json):
  registered types, retention bindings and actual accepted resource ceilings.
- [Decision discipline](../../agents/ARCHITECTURE_DECISION_DISCIPLINE.md)
  and [multichannel scope matrix](../MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md):
  minimum decisions for the next proof and distinct World/Character/runtime scopes.

Historical candidate/status text inside domain documents is not a fresh lifecycle
claim. The protected consumed revision supplies semantic constraints; live GitHub
governs acceptance, ownership, PR status and integration.

## Evidence classification

| Classification | Finding and implication |
|---|---|
| `PROVEN` | The retained [prototype report](../../agents/evidence/OTV2-20260917-dur03-reference-one-item-resource-evidence.json) has null actual mandatory audit count/bytes for both operations, explicitly labelled `EVIDENCE_GAP`. |
| `PROVEN` | The current event registry registers Character bootstrap evidence, not a DUR-03 item-transaction payload. Its Character retention purpose does not establish item-audit coverage. |
| `PROVEN` | The resource registry has shared ANL envelope/payload/string/nesting limits but no entries named `DUR03-RL-01` through `DUR03-RL-08`. These labels route child obligations; they are not accepted numeric registry entries. |
| `PROVEN` | The prototype reports one touched item in each operation, one MINT custody effect and two TRANSFER custody effects. Its explicit private encoding and retained carrier measurements are prototype evidence, not ANL protobuf measurements. |
| `DERIVED` | A typed transaction aggregate can capture all relevant effects for this narrow item without treating each custody line as a separately required event. DUR-03 §39 permits aggregation; the exact registered shape still needs owning review. |
| `UNKNOWN` | Item-audit event type/schema, actual contribution count/bytes, applicable retention profile, production maxima, PostgreSQL atomicity/restart behavior and executable Content definition binding. |
| `CONFLICT` | Treating the synthetic probe, participant/effect encoding, Character bootstrap payload or its retention profile as complete item-audit evidence would contradict the registered family/purpose and retained gap. No such substitution is accepted. |

The report's Gold Coin x1 uses a private fixture definition key/revision. Natural
loot probability and Reference parity remain `UNKNOWN/NOT_ASSERTED`. Linux target
carrier sizes are not universal memory use; build compiler identity remains
`UNKNOWN` in the retained report. No fresh runtime measurement is claimed here.

## Constraints and mandatory decision test

Every live durable item has one immediate semantic location. WorldId and ChannelId
remain distinct. Ground simulation ownership is runtime scoped; Character-held
value is Character/World scoped. Custody, corpse association and identifiers are
not current mutation authority. Fresh session/lease/runtime fences belong to their
owning contracts. Required audit must commit with mutation and cannot become
best-effort because a dependency, byte budget or privacy policy is unavailable.

1. **Must decide now? YES, for the next schema/measurement proof only.** Choose
   the minimum typed semantic shape and its missing acceptance inputs before
   registering or measuring an item-audit family. Production limits, storage,
   rollout and full loot design are not decisions this packet can accept.
2. **Blocked downstream work:** closing the actual audit count/encoded-byte gap
   in `DUR03-RL-07` for #513; a conforming mandatory-audit implementation also
   remains gated on schema, retention, numeric ceilings and physical atomicity.
3. **What becomes harder later:** consumers depend on event meaning and required
   fields; unsafe semantic changes require a new family/major decision. Reusing
   a type or mutating admitted bytes/policy bindings would break interpretation
   and replay. Keeping payload variants narrow limits that commitment.
4. **Supersession evidence:** a reviewed owner requirement for additional effects,
   independently verified missing reconciliation information, malformed/replay
   findings, measured payload/amplification failure or an accepted privacy change
   justifies a newer decision stating changed and preserved semantics.
5. **Deliberately undecided:** event type identifier, protobuf field numbering,
   privacy/retention approval, numeric hard maxima, physical DB/outbox design,
   production runtime APIs, connected loot/pickup, container traversal, multi-item
   trade and Reference formulas/probability/parity.

## Options, trade-offs and recommendation

Two realistic options fit §39: a typed transaction aggregate covering this item's
complete effect set, or separately typed lifecycle/custody effect events with
complete TransactionEventRef membership. Separate events can help a future
multi-effect consumer, but here they add complete-set reconciliation, repeated
context and missing/duplicate ordinal handling without a demonstrated requirement.

**Recommendation (`DERIVED`, awaiting owner acceptance):** a small ANL envelope
with one typed aggregate payload for each logical transaction. Use a closed
MINT/TRANSFER alternative, rather than a nullable mega-event or arbitrary map.
For this proposed narrow aggregate each transaction would have one membership
ordinal covering its entire payload. This is a design cardinality, not an emitted
count, accepted hard maximum or measurement. MINT and TRANSFER remain distinct
transactions; aggregation must not silently turn their sequence into one commit.

The payload recommendation is a transaction-summary header (interpretation
revisions and conservation basis) plus a typed operation alternative. Each
alternative contains one item identity with typed before/after state and typed
provenance/fence references. MINT has one establishment; TRANSFER has one removal
and one establishment, or an equivalent typed before/after custody pair. No
unbounded repeated effects, metadata bag, embedded inventory tree or entire
definition record is proposed. Unknown applicability is explicit absence under
variant rules, not zero/nil values interpreted as valid facts.

This lowers producer/consumer work for the next safe proof. It does not remove
the physical atomicity obligation or solve the privacy gap. A richer family can
be proposed when a real additional operation requires it; this child must reject
unsupported shapes instead of accepting them under an unclassified generic delta.

## Field-by-field semantic disposition

The table describes the proposed payload/envelope division, not IDL or an activated
API. Fields are required when the owning transaction's semantics make them applicable.

| Semantic field | MINT to Ground/corpse fixture | Later TRANSFER to direct-root CharacterInventory | Placement and guard |
|---|---|---|---|
| Event identity/type/schema, durability, immutable payload bytes/hash | Fixed before possible ambiguity | Independently fixed before possible ambiguity | Existing ANL envelope; no identifier allocated here; future registered type/revision required. DURABLE_AUDIT. |
| TransactionId and complete membership | MINT logical transaction; fresh output belongs to it | Distinct TRANSFER logical transaction; same item | Envelope TransactionEventRef, all-or-none and immutable; cardinality cannot be inferred from custody lines. |
| OperationId | Only if a genuinely durable multi-step workflow needs it | Same workflow identity only when that workflow exists | Optional envelope context; never invented for every simple transaction. |
| CommandRef / immediate cause | Server-originated mint need not invent a player command | Player pickup requires its actual CommandRef when player-originated | Existing envelope CommandRef/CausationRef; domain occurrence details in typed payload; grouping is not authority. |
| WorldId | Item and Ground value scope agree | Item, source Ground and Character destination agree | Envelope context and typed location consistency; cross-world mismatch rejects. |
| Runtime scope and order | Concrete ChannelRef or InstanceRef; applicable owner generation | Source Ground uses that same concrete scope; destination is not channel-owned | Envelope scope/RuntimeOrderRef when applicable; payload fence references are evidence only, never current authority. |
| Interpretation revisions | Explicit ruleset/content/item-definition meaning | Compatible meaning for the existing item | Shared envelope revisions where available; typed payload definition reference/revision, no definition blob or silent reinterpretation. |
| Touched ItemInstanceId | One fresh transaction-scoped output identity | Same existing item identity | Typed payload; UUID/lifecycle constraints remain DUR-01/GAME-ITEM owned. |
| Lifecycle before/after | Explicit not-existing before, live after | Live before and live after | Typed variant; MINT is not a zero-quantity live item and TRANSFER does not create a replacement identity. |
| Type and quantity before/after | New fixture item type, quantity one; absence before | Same type and quantity one before/after | Typed item state; quantity arithmetic exact. Fixture mapping is not Content resolution. |
| Immediate location/custody before/after | Absent before, Ground after with World/runtime/spatial identity | Ground before, CharacterInventory after with CharacterId and typed direct-root position | Closed typed alternatives; corpse occurrence/revision is association/provenance, not a new generic location family. No Container parent or traversal. |
| Mutation class | MINT | TRANSFER | Closed typed payload alternative; no generic signed delta. |
| Source/sink/rule/cause | Stable typed authorized mint source/occurrence, no sink | Stable pickup/transfer cause and source/destination, no mint/burn | Domain payload source shape awaits owning definition. Existing fixture integer cause is not a production source contract. |
| Conservation summary | Exact authorized units added, complete mint lineage | Exact conserved units unchanged; no duplicate source/destination truth | Typed summary consistent with effect/state fields; coin-as-item is not a non-item currency-account line. |
| Safe fence references | Applicable runtime ownership/corpse revision and other owning fence references | Applicable current Character session/lease and source runtime fences | Typed evidence references without credential/proof material; independent current facts must authorize any future mutation. |
| Privacy/retention binding | Item/provenance linkage assessed under owning purpose | Raw Character/session linkage makes restricted access necessary | Envelope policy floor/binding; item-specific acceptance remains a gap, never copied automatically from Character bootstrap. |

Known noncommit may rematerialize effects from independently current before-state
under the same intent and planned output identities. Any newly admitted event
must obey immutable admission rules. Once commit becomes ambiguous, exact event
IDs, semantic envelope values and payload bytes are preserved while reconciling
the frozen candidate; they cannot be rebuilt by serializing mutable state.
Duplicate delivery has no second consumer effect; replay never remints or moves
gameplay value. Lost TRANSFER acknowledgement cannot restore Ground custody.

## Privacy, retention, player and operational risks

`UNKNOWN / EVIDENCE_GAP`: no accepted item-transaction purpose/profile binding is
established by this packet. Recommend review of `RESTRICTED_PLAYER_LINKED` as the
minimum floor for this Character-linked audit, with a higher floor when the owning
security purpose requires it. An accepted profile must define purpose, finite
ordinary retention, roles, export/redaction, expiry deletion/anonymization,
explicit audited legal hold and immutable policy revision/activation rules.
Do not reuse the Character bootstrap duration/purpose by similarity.

Audit retention and receipt/source-cause replay horizon are distinct decisions:
expiry must not accidentally reopen mint eligibility or erase required non-reuse
evidence. The next owner review must establish how those horizons remain safe;
this packet selects no duration or storage mechanism. Restricted identities must
not become ordinary metric labels, unrestricted debug data or public projections.
If a pseudonymous projection is later proposed, ANL epoch/mapping rules still apply.

For players, missing atomic evidence risks duplicate/lost progress; unnecessary
event fan-out can increase work before durable acknowledgement. For implementers,
the aggregate reduces initial schema/test cost but still needs complete malformed,
idempotency and compatibility tests. Overflow must reject or safely stage through
an accepted path before partial mutation; unknown outcomes hold/reconcile, not
retry as fresh loot. This packet does not add a staging runtime.

## DUR03-RL-01 through DUR03-RL-08 disposition

Shared ANL ceilings remain conjunctive with any future item-family ceiling.
Neither a broad envelope ceiling nor a one-item fixture substitutes for accepted
DUR-03 amplification limits. Numbers below are deliberately not selected here.

| Obligation | Consumed evidence | Disposition and next required evidence |
|---|---|---|
| RL-01 touched ItemInstances | One item; multiple-item request rejected before planning | `PROVEN` one-item fixture only. Preserve rejection; broader accepted ceiling remains `UNKNOWN`. |
| RL-02 location/custody effects | MINT establishment; TRANSFER removal plus establishment; one final custody tuple | `PROVEN` prototype representation; `DERIVED` payload needs complete typed pair. Measure registered encoding; reject dual location/partial transfer. |
| RL-03 value/account lines | Unsupported capability rejects; none represented | `NOT_EXERCISED`. Coin item quantity is not an account balance. No account schema or general limit accepted. |
| RL-04 transform inputs/outputs | Transform capability rejects | `NOT_EXERCISED`. No transform lineage or maximum claimed; MINT/TRANSFER labels stay distinct. |
| RL-05 container expansion | Direct-root destination; Container rejects | `DIRECT_ROOT_ONLY`. No recursive parent graph, container pickup proof or general expansion bound. |
| RL-06 atomic participants/effects/work | Concrete prototype participant/effect records and abstract work accounting | `PROVEN` within prototype only. Future payload changes require recounting concrete records/work, not source/destination prose or CPU claims. |
| RL-07 mandatory audit contributions/bytes | Actual fields null; synthetic size-only check | `EVIDENCE_GAP`. Owner-approved typed schema/registry plus actual deterministic payload/envelope encoding, aggregate checked accounting and boundary tests required. |
| RL-08 retry/reconcile retained state/work | Stable identities and bounded in-memory ambiguous/duplicate/known-abort cases | `PROVEN` prototype state machine only. Preserve exact byte/event candidate on retries; DB/restart/retention conformance remains `UNKNOWN`. |

## Read-only Reference ordering evidence

The control plane inspected these immutable external revisions:

- `opentibiabr/canary@47dfd51f45280a59a1d3e50ba7edd573d7234446`:
  `src/creatures/monsters/monster.cpp` 3305–3319 and 3415–3435;
  `src/game/game.cpp` 2276–2293.
- `zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a`:
  `src/creatures/monsters/monster.cpp` 2407–2421 and 2521–2567;
  `src/game/game.cpp` 2107–2124.
- For both, the read-only source set also includes
  `src/items/containers/container.cpp`, `src/creatures/players/player.cpp`,
  `src/io/functions/iologindata_save_player.cpp` and
  `src/io/functions/iologindata_load_player.cpp`.

`DERIVED`: this narrow source inspection supports coarse corpse association,
loot insertion and a later player move, with separate save/load serialization.
It is control-plane supplied evidence, not a fresh writer-side external audit.
It does not establish complete standard-death caller chronology, Oteryn event
meaning, conservation/fence authority, atomic audit/SQL durability, natural
probabilities or parity. Oteryn contracts alone determine the proposed semantics.

## Proposed next schema/registry and measurement allocation

After owning DUR/ANL/privacy review accepts the narrow semantics, the active control
plane may allocate **one** successor task, proposed name
`OTV2-20260927-dur03-one-item-audit-schema-measurement-513`, on an exclusively owned
`codex/dur03-one-item-audit-schema-measurement-513` branch from fresh protected main.
This is an exact path proposal, not a current writer release or schema authority:

1. New `docs/contracts/game-events/v1/item_transaction.proto`: narrow typed
   MINT/TRANSFER payload; actual IDL fields and compatibility rules reviewed there.
2. `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json`: owning type/revision,
   durability, accepted privacy floor/profile and atomic-evidence binding. The
   owner selects an unused identifier only in that approved task.
3. `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`: only owner-accepted measured
   scope limits; each entry states allocation impact/failure category/tests.
   Do not fill missing ceilings with prototype injected budgets.
4. `apps/game-server/examples/dur03_reference_one_item_resource_prototype.rs`:
   extend only the standalone evidence route to materialize and encode the
   accepted registered payload/envelope, preserve bytes and account real counts.
5. `docs/agents/evidence/OTV2-20260917-dur03-reference-one-item-resource-evidence.json`:
   actual normalized report, separating payload, full envelope, aggregate event
   bytes and retained carriers; synthetic probes remain explicitly separate.
6. `docs/agents/evidence/OTV2-20260917-dur03-reference-one-item-resource-evidence.md`:
   reproducible measurement method, exact revisions/target/toolchain evidence,
   independent golden oracle and remaining limitations.
7. New `docs/agents/tasks/active/OTV2-20260927-dur03-one-item-audit-schema-measurement-513.md`:
   truthful lifecycle, exact owned paths, exclusions and acceptance evidence.

The existing foundation envelope IDL remains unchanged. If accepted retention
semantics, encoder dependency/public API or owning contract work cannot fit those
paths, the control plane must explicitly revise the allocation before release;
the worker may not silently add Cargo, runtime, migration or contract edits.
If privacy or numeric-limit approval is pending, keep registry acceptance gated;
an explicitly allocated offline candidate measurement may still be useful but
must not claim a registered production event or accepted maxima.

Required adversarial proof for that successor:

- Independent byte-level golden fixtures and round-trip/cross-version validation,
  with malformed/truncated/unsupported payloads and closed-variant required fields.
  Do not use the same encoder as the only golden oracle.
- Valid lower-bound identity/scope fields; individually missing, nil, stale,
  wrong-world, mismatched source/corpse/Character and incompatible revision cases.
  Each negative alters one applicable invariant while unrelated facts stay valid;
  current facts come from an independent input, not the candidate record.
- Duplicate/same-identity conflicting content, inconsistent transaction counts,
  duplicate/gapped ordinals and out-of-order complete-set handling; optional
  OperationId/CommandRef absence accepted only when semantically inapplicable.
- Exact candidate bytes/hash/envelope values unchanged across ambiguous/lost-response
  retries; known abort retains intent/output identity; unknown reconciliation holds.
  Replay cannot invoke mint/transfer; partial source removal never becomes final.
- Separate actual payload/envelope/aggregate count-byte checks at the accepted
  maximum and its successor, nested string/depth checks and checked arithmetic
  overflow before input-sized allocation or prototype custody publication.
- Missing/unaccepted profile, privacy downgrade, secret-bearing data and unauthorized
  raw-identity export reject; retention binding cannot change an admitted event.
- Repeated normalized runs and reversed presentation order agree. Report actual
  compiler/target attestation only when established, and preserve `UNKNOWN` otherwise.

Focused schema/governance/encoding checks prove the successor's evidence surface;
repository-selected checks and review apply after freeze. No successor result
alone proves PostgreSQL atomicity, production fences/restart, connected Combat
or accepted Reference execution. Those require a separate runtime allocation.
