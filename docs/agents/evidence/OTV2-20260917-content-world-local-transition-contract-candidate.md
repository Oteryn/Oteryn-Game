# Content / World local-transition contract candidate

```yaml
status: PROPOSED_NONCANONICAL
revision: 2
date: 2026-09-17
repository: Oteryn/Oteryn-Game
pr: 641
branch: agent/content-world-design-dossier-20260917
inspected_main: b44fefe08f6aaf1b2c1c23dedd92bab0de87146e
predecessor_head: b1fc73ee8c6dc4cfae427564dc5fdc6efaf01316
reviewed_predecessor_head: 70ac6659341a6d204ef6dbbfe990ae70950da141
implementation_authority: NONE
format_acceptance: NONE
resource_registry_mutation_authority: NONE
production_authority: NONE
```

This continuation supplies the concrete recommendation left open in the [execution design](OTV2-20260917-content-world-execution-design.md). It narrows the first executable object case to one ordinary, non-value-bearing, scope-local state transition. It is an architecture candidate for owning review, not an accepted contract, implementation allocation, new global gate or production feature.

## 1. Evidence and correction of the previous ownership question

The following sources were read on the exact main above. `PROVEN` means inspected source, not runtime qualification.

| Source | Exact inspected symbols / sections | Consequence |
|---|---|---|
| [Multichannel scope matrix](../../architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md), blob `4f7adfc8f3a11cdb0dc68a87e6b05534ce7df6c4` | Public map runtime overlay; one writer per Channel | **PROVEN:** ChannelRuntime already owns the public map's mutable overlay; it is not process-global or implicitly shared between channels. |
| [VSL Movement](../../architecture/VSL-MOVE-01_MINIMAL_MOVEMENT_VISIBILITY_CONTRACT_CANDIDATE.md), blob `f7e0cae39b46424ddad26ae4fc8201dd71fcfc18`, and [its owner acceptance](../../architecture/OTERYN_V2_STAGE_C_VSL_OWNER_ACCEPTANCE_20260816.md) | Sections 3, 7, 9, 10, 13–14 | **PROVEN:** current ChannelRuntime/InstanceRuntime owns position/occupancy; Movement composes current overlay facts; stateful interaction is not a hidden collision callback; FND-02 owns reconciliation. |
| [Foundation](../../../apps/game-server/src/foundation/mod.rs), blob `fac64fb048cd7b53af4400aca572f500ca205aef` | `CommandIngress::{reserve,mark_terminal,classify_duplicate}`, `DuplicateDisposition`, `FoundationProtocolError::CommandOutcomeExpired` | **PROVEN:** ingress already distinguishes pending-original, retained-result replay and expired outcome; terminal handling preserves order. Outcome expiration maps to resync, not new execution permission. |
| [Foundation protocol](../../../apps/game-server/src/foundation/protocol.rs), blob `6642eef72c5bbb202ebe71075dc712f92bcf89aa` | `CommandRef`, `MessageType::{CommandResult,StateDelta}`, `Sequencing` | **PROVEN:** command identity includes GameSessionId; both result and delta are server-sequenced, but they are distinct message families. |
| [Actor carrier](../../../apps/game-server/src/foundation/runtime_actor_carrier.rs), blob `4040c307d963ae5eef1b6243e73aadafccc599b2` | module preamble; private `ActorRef`, `PreProductionContinuityGrant`, `ChannelActorCarrier` | **PROVEN:** private, preproduction-only, uncomposed carrier; no grant constructor/issuer here. Do not describe it as a public production object resolver. |
| [FND-02 contract](../../architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md), blob `29dc83ed934c4931791ae10478c54344b7117c23` | Sections 7, 13, 14, 16 | **PROVEN:** semantic command identity is not protobuf byte identity; ordered admission also requires ordered authoritative commit; snapshot transfer has a server-side sequencing barrier. |
| [Protocol registry](../../contracts/PROTOCOL_OTERYN_V1_REGISTRY.json), blob `6d7f65e9247a5eace5019fdf8703259ba9536b26` | `command_types`, `state_domains`, message types | **PROVEN:** both gameplay registries are empty on inspected main. Generic command/result/delta/snapshot envelopes exist; a registered spatial/world domain does not yet exist. |
| [Snapshot facade](../../../apps/game-server/src/foundation/snapshot_facade.rs), blob `faa9625ae7b04de931c02a74d2d29aa8cdf51a6d` | `SnapshotBarrier::{begin,commit,may_emit_sequenced,discard_for_generation_change}` | **PROVEN:** a reusable snapshot barrier boundary exists. Its presence alone does not prove that a future object egress producer invokes it. |

**Correction to CW-04 and execution-design section 3:** the current runtime owner is already selected. The remaining gap is the typed world-object operation and its composition with that owner, not an unanswered choice of a new service/domain authority. Historical wording is narrowed by this section, not used to reopen the one-writer decision.

**Correction to execution-design section 7:** bounded replay does not need another global receipt framework. Use existing FND ingress/outcome semantics for client commands and existing Interaction child/reconciliation semantics where a workflow needs them. The durable reward horizon remains DUR-owned and is outside this plain-object slice.

The predecessor Merge gate `35231780745` was freshly read as SUCCESS. This says nothing about checks for the new candidate. Protected main and PR branch had not advanced at this continuation's admission. The raw corridor, Global observations and production call graph were not requalified.

## 2. Selected candidate and rejected alternatives

**RECOMMENDATION:** one local world-object transition operation executed by the existing current runtime writer. The operation changes only that object's ephemeral state and its derived spatial/presentation contributions. Movement remains the position writer. Content remains immutable input. Interaction remains the workflow/occurrence owner when required; a direct single-object operation does not require a new generic workflow engine.

Two realistic alternatives are rejected for this slice: Content callbacks mutating arbitrary world state would duplicate authority; a separate world-object process/service would add coordination without an accepted need. Merely storing `blocking: bool` on a cell is also insufficient because it cannot identify which object's contribution is being removed.

| In the first proposed child | Explicitly outside it |
|---|---|
| One plain two-state object, OPEN/CLOSE intent, exact immutable definition and placement; declared scope-ephemeral lifetime | Key consumption, quest progress, reward grants, valuable objects, durable door state, house ACL, scripts and arbitrary state dictionaries |
| Current-owner validation; movement consumes resulting spatial view | Auto-walk through doors, automatic displacement, local/cross-scope teleport, multi-owner footprints |
| Existing command/result/delta/reconciliation semantics | Numeric protocol allocation in this evidence PR, global occurrence IDs, a second result journal, authoritative client collision. A later real wire child requires its owning payload/domain registration as specified in section 6. |

Closed/open and reject-occupied-close are synthetic witness choices, not inferred Global mechanics. Executable Reference promotion still needs evidence for the exact selected object's rules; evidence work is not blocked by unrelated catalogue gaps.

## 3. Minimum typed source binding

The names below describe semantic fields, not a frozen file format, wire payload or existing Rust API.

| Record | Required meaning |
|---|---|
| Definition reference | Expected family = local world object; stable Content key; exact compatible definition revision; selected Content generation |
| Object definition | Finite named states; one initial state; per-state spatial/presentation contributions; finite supported transitions; explicit lifetime and supported operation capability |
| Placement | Stable authored PlacementKey; reference to that definition; world-project position/anchor; complete logical footprint |
| Transition binding | Stable binding/edge identity; trigger family; admissible source state; target state; supported current-owner operation; typed guard/policy references |
| Runtime target | Resolved current RuntimeScopeRef, placement and incarnation under current ownership; compact handles are revision-scoped and never trusted just because the client supplied them |
| Client projection | Only the allowed presentation/state/interaction hint; no internal policy data or server-only rule references leaked through a blanket object export |

The compiler rejects wrong reference families, duplicate placement keys, missing states/presentations, undeclared transitions, unsupported owner capabilities and ambiguous bindings for the same target/trigger/state. Explicit OPEN on an already-open object may produce a memoized no-change success; it is not a hidden state transition. Initial state appears only at authorized scope initialization, never on ordinary streaming/cache reload.

Authored records cannot select a new runtime owner or authorize a capability simply by naming it. The selected runtime profile must already support that operation. A permanently blocking wall can remain immutable spatial data; it needs neither a mutable item instance nor a state-transition allocation.

Source shards and bundle chunks do not define mutation ownership. A footprint may cross either partition while remaining one same-owner placement. A genuinely cross-owner footprint is excluded from this child. Missing footprint cells are unresolved, not empty. Geometry outside the selected executable subset must be complete or fail closed.

## 4. Owner operation and commit boundary

For client-originated work, resolve FND `CommandRef` and current authenticated authority before consulting or returning a prior outcome. Connection-generation changes do not create a new semantic command when a legitimate same-session reconnect preserves it. Loss of the session/owner's necessary continuity must follow the accepted reconnect/replacement contract, not reconstruct fresh ingress under the old identity.

```text
current scope + ownership + session/connection authority
 -> existing ingress duplicate/order disposition
 -> original selected binding, or one newly admitted binding
 -> exact target incarnation + current state and policy validation
 -> prepare complete state/spatial/receipt/observation outcome
 -> one non-interleaved current-owner commit
 -> result and post-commit observation through existing projection owners
```

Before commit, validate the selected definition, trigger/state, manipulation permission, actor range/visibility and affected footprint under the applicable owner rules. A client's expected revision is only a precondition against server truth, never permission to set the current revision. Closing and crossing are independent operations and may have different policies.

**Session commit order:** before the first authoritative effect or new terminal result, the current writer must establish that the command has its turn in the existing GameSession ingress order, across all targets and command families. Serial object-local execution alone does not satisfy FND-02 section 13.1. `CommandIngress::reserve` orders admission, while `mark_terminal` rejects out-of-order retirement; neither licenses a consumer to mutate first and discover an ordering failure afterward. A command waiting behind an earlier pending command remains the same admitted operation. Do not introduce per-door command streams or consume a later terminal rejection ahead of an earlier reserved command.

No asynchronous gap is allowed between final dynamic-occupancy validation and local commit. Worker/precomputed results must be fenced and revalidated by the owner. Reserve the necessary bounded outcome/projection bookkeeping before mutation, or use an already accepted loss-and-resync contract. Never return COMMITTED merely because an operation was queued.

The commit makes logical state, all affected spatial contributions and the reconciliation outcome coherent before any observer can use them. If an essential fallible step remains after publication, its failure behavior must already be defined; an implementation may not rely on the Python witness's assumed indivisible assignment. A rejected operation changes no gameplay state, although required ingress/result/accounting records may change.

**Spatial rule:** effective occupancy blocking is composed from base terrain plus every currently contributing object/actor/policy. Opening one door removes only that door's contribution. It does not write `cell.blocked = false`, clear an overlapping wall, erase a second object's contribution or bypass an independent crossing restriction. Cached combined flags remain derived and must rebuild to the same result.

For the fixture's reject-occupied-close policy, both serialized orders are safe: movement first makes closing reject; closing first makes movement reject. A multi-cell object cannot publish a half-closed footprint. This is a required owner-ordering property, not a claim that concurrency has been tested here.

## 5. Replay, expired outcomes and capacity

| Existing disposition | Required local-object behavior |
|---|---|
| New admissible command | Resolve one binding; commit or record one terminal rejection after bounded admission |
| Pending original | Reconcile original work; do not run the transition again |
| Retained outcome | Return original result if the binding/payload matches; do not recheck it as a fresh mutation |
| Same identity, changed payload | Conflict; never overwrite the first binding or choose a new Content revision |
| Outcome expired | Existing `CommandOutcomeExpired`/resync path; never treat a missing response-cache entry as unused command identity |
| Sequence gap, stale authority, capacity exhaustion | Preserve existing FND semantics; no partial gameplay mutation or invented success |

Do not add a world-object receipt table. FND owns client-command admission/result lifetime; an Interaction child uses its existing lifecycle and delegated owner's operation. Production consumers must derive retained-outcome evidence from their actual store: a caller-chosen boolean is not proof that an outcome exists.

Where retained input is compared to classify a duplicate/conflict, compare normalized typed intent and its originally selected binding, never raw protobuf bytes. Different legal encodings of the same typed input are not a different semantic command. Pending/expired duplicate classification still follows the authoritative CommandRef/high-water mark and must not reinterpret replacement bytes as fresh work or resolve them against newly selected Content.

The model's finite no-eviction receipt list is only a conservative stand-in for owner/result composition. Its separate ingress witness shows why the real implementation can expire response data without re-executing old commands. The model does not select production retention, infer a durable horizon or replace pending-operation recovery.

A failure before owner commit is not synonymous with an unreserved network command. FND may already have reserved the command. Preserve its pending lineage and either complete/reject it through the owner contract or reconcile the same operation; do not skip its ID or claim a new reservation. The witness's `NOT_ADMITTED` means no world-object effect was published, not a new protocol error or an instruction to reset ingress.

## 6. Observation and resynchronization

Use existing FND sequencing and state-domain reconciliation; do not create a parallel object transport. Exact local-object payload registration remains an owning implementation dependency, not a new ID selected here.

A result acknowledgement does not update world state. A late replay result may describe an earlier success while the door is currently closed. It participates in server sequencing but does not restore the old presentation. A delta is applied only under the current connection/scope/content/incarnation binding and the matching state-domain base revision. Old frames are not applied twice; gaps suspend affected state until the accepted bounded resync completes.

A complete, validated replacement snapshot is installed as one baseline. Incomplete snapshots cannot become partly visible. A stale snapshot cannot lower the known revision. Equal revision with a conflicting semantic state is a consistency failure, not last-writer-wins. Higher numeric sequence alone does not legitimize mismatched scope, generation or content.

**Registration prerequisite for real wire execution:** the inspected `PROTOCOL_OTERYN_V1_REGISTRY.json` has `command_types: []` and `state_domains: []`. There is no existing registered spatial/world state domain to consume. Reuse the foundation envelopes, but the owning gameplay/protocol child must register the typed command/result and state delta/snapshot semantics, including the required stable numeric identities, before any real client-server object exchange. Prefer one appropriately scoped shared spatial/world domain, not one domain per door. This evidence PR allocates no IDs. Do not bypass the missing registration with an opaque payload, a borrowed unrelated ID or relaxed decoder checks. A synthetic in-process test does not satisfy this prerequisite. Hidden or no-longer-visible targets still use the normal server-side access policy and current observation filtering.

**Snapshot egress barrier:** during the accepted SnapshotBegin-to-SnapshotCommit interval, later server-sequenced output must remain in the existing bounded server egress/replay path. This includes both object StateDelta and CommandResult, even though a result does not change the client's world state. Compose the existing `SnapshotBarrier::may_emit_sequenced` boundary at transmission eligibility; constructing a barrier without consulting it is insufficient. Rebind aborts the old generation's partial snapshot/barrier and establishes fresh reconciliation. Retention exhaustion follows the existing FND slow-client/recovery policy, not early transmission or an unbounded client buffer. This makes the inherited FND-02 section 16 requirement explicit; it is not a new transport or queue design.

## 7. Activation and lifetime: choose the bounded first shape

For this first child, **recommend rejecting different Content generation activation while its runtime scope is live**. Staging/validation can continue independently. Start a replacement scope only through its existing authorized lifecycle after the old scope is quiesced/retired, sessions are safely fenced or reconciled, and the selected reset policy is allowed. Do not mint scope ownership or reuse the old generation locally.

This is intentionally narrower than a generalized hot-reload design. It does not modify existing Content loader code or claim existing loaders enforce this added stateful composition rule. A later qualified compatible-rebind design may replace this restriction; the invariant is no reinterpretation or reset of admitted work.

Within a live scope, cache eviction, unload/reload, file reimport and observer disconnect do not reset the object. A new scope's ephemeral initial state is legal only under that mechanic's accepted lifetime policy. Durable state, rewards and valuable objects cannot be smuggled into the child under an ephemeral label. Reconnect/owner loss with unresolved durable work remains outside the witness and governed by existing owning contracts.

## 8. Decision timing and implementation exit criteria

| Mandatory question | Candidate answer |
|---|---|
| Must decide now? | YES for executable stateful-object composition; NO for retaining/importing source candidates. |
| What work is blocked? | The first Content-to-current-owner OPEN/CLOSE operation, Movement consumption of its committed spatial contribution, and client observation under #504/#139's exact allocated child. |
| What becomes costly later? | Binding mutable state to filenames/sprite IDs, clearing shared collision flags, a second owner/receipt system, or a public payload without generation/revision semantics. |
| What justifies supersession? | An accepted mechanic requiring durable state, multi-owner effects or live compatible activation, supported by composition/failure evidence; not hypothetical future scale. |
| What is not decided? | Permanent codec/chunk size, numeric production maxima, new public API/IDs, full quests, economics, scripting, Studio or Global occupied-door behavior. |

The smallest implementation-shaped package is source binding validation plus one current-owner transition and its spatial/client projections. It consumes existing Foundation/Interaction interfaces where applicable; it cannot expose the private #573 actor carrier as a production resolver by convenience. Fresh #162 allocation must name actual writable symbols, accepted target fields, finite resource decisions and current dependency readiness.

Choose the evidence claim before allocation. A first **synthetic in-process Rust composition** may verify typed source binding and current-owner operation without claiming Reference parity, wire compatibility or a playable client. A **real client-server child** additionally requires the owning registration above, the qualified current-owner lookup and the selected Content path; a **Reference claim** further requires evidence for each exercised target-sensitive rule. These are proof boundaries, not three mandatory new programmes or permission to release workers. Do not inherit raw Global capture as a blocker for an otherwise authorized synthetic technical child.

Required real qualification is: source/linker negatives; current-session/owner/incarnation fencing; duplicate/expired-outcome behavior and semantically equivalent wire encodings; whole-session commit order across different targets, including an earlier pending command; both movement/close orders; overlapping spatial contributors; injected failures around owner publication; registered typed payload negatives; actual client delta/result/snapshot sequence behavior with both results and deltas held behind the snapshot barrier; and exact-candidate repository checks. Extend the test scope only for capabilities actually admitted. Runtime absence, missing target evidence or a not-yet-qualified resolver remains a named dependency, not proof of a production defect.

## 9. Validation in this continuation

The embedded model was executed on Python 3.13.5: **21 tests passed**, including enumeration of **4096 length-four serialized event traces** over the eight named fixture events. Four targeted broken variants were rejected by those tests: clearing other spatial contributions; bypassing current authority; applying a result as world state; and ignoring a delta's base revision. These are model mutation checks, not repository mutation coverage or an independent review.

Witness SHA-256: `e1e6cb4713b84586df2293205daa9b7667b52f26c3f8adfff80c1d93e49aadaa`.

The predecessor execution document's embedded witness was also extracted and rerun: **26 tests passed**, source SHA-256 `9809f28dd0b34e4c2b7c42dadcdeda856d35f8587b487240de9c889d94f5034d`. The much earlier reported 63-test prototype was not recovered or rerun. Do not aggregate these into a production-test claim.

Model assumptions/omissions are explicit: trusted Context/snapshot assignment is not authorization issuance; inputs are already typed fixtures; calls are serialized; publication is indivisible by construction; the client sketch is not the FND codec; lifetime and capacities are synthetic; no database, network, fault-tolerant storage, real corpus, parallel execution or Global parity is tested. An exhaustive bounded toy trace set is not exhaustive production verification.

Direct container Git access failed with `Could not resolve host: github.com`. Read/write publication uses the working GitHub connector as API-native documentation editing, without selecting/reconstructing a local Git commit. Full local repository governance/build is not claimed. Hosted checks for the published head and independent review where applicable remain separate.

### Revision 2 — bounded author review

Review target: `70ac6659341a6d204ef6dbbfe990ae70950da141`, reconciled against unchanged inspected main. This is an author self-review, not an independent approval or owner acceptance. The only false current-state premise found in this bounded pass was the phrase "existing spatial/world state domain"; section 6 now records the empty registry and the exact wire-stage dependency. Sections 4–6 additionally make already accepted FND command-order, semantic-equality and snapshot-barrier obligations explicit rather than adding new global gates.

The embedded models are unchanged and were rerun: 21 and 26 tests passed, with the same 4096 bounded traces and four rejected deliberate mutations. In particular, the separate Owner and IngressWitness sketches do not prove their ordered composition, and the Client sketch does not prove server egress gating. A scratch boundary check deliberately invoked command 2 before pending command 1: Owner returned COMMITTED, while subsequent ingress retirement returned false. This demonstrates the missing composition proof, not a defect in the accepted FND contract or production runtime. The new acceptance obligations require real owning integration tests; they are not marked passed by the old models.

Disposition: preserve this corrected proposal for owning decision, stop broad design expansion, and hand the smallest explicitly classified implementation child to #162. No code, registry, accepted architecture or worker allocation is changed by this revision. The earlier DNS observation is historical; this revision does not claim a new transport probe or a full local repository build.

## Appendix — reproducible model

Extract the Python block between the markers and run it with Python 3.11 or newer. It requires only the standard library. The full model is retained here so the evidence does not depend on a chat attachment.

<!-- LOCAL_TRANSITION_WITNESS_BEGIN -->
```python
"""Synthetic local-transition contract model; not Oteryn runtime or a wire codec.

Trusted inputs, serialized calls and atomic tuple publication are MODEL assumptions.
Limits are injected fixture values, not production maxima. No persistence is modeled.
"""
from dataclasses import dataclass, replace
from itertools import product
import unittest


@dataclass(frozen=True)
class Binding:
    key: str = "fixture:door"
    revision: str = "definition-1"
    owner: str = "current-runtime"
    lifetime: str = "scope-ephemeral"
    footprint: tuple = (-1, 0)
    states: tuple = ("closed", "open")
    edges: tuple = (("closed", "open"), ("open", "closed"))


def link(binding):
    """Validate the deliberately narrow in-memory fixture, not a source parser."""
    if (binding.owner != "current-runtime" or binding.lifetime != "scope-ephemeral"
            or not binding.key or not binding.revision
            or not binding.footprint or len(set(binding.footprint)) != len(binding.footprint)
            or binding.states != ("closed", "open")
            or set(binding.edges) != {("closed", "open"), ("open", "closed")}
            or len(binding.edges) != 2):
        raise ValueError("UNSUPPORTED_OR_AMBIGUOUS_BINDING")
    return binding


@dataclass(frozen=True)
class Context:
    session: str = "session-a"
    connection: int = 1
    scope: str = "channel-a"
    owner_generation: int = 1


@dataclass(frozen=True)
class Command:
    session: str = "session-a"
    command: int = 1
    placement: str = "fixture:placement"
    incarnation: int = 1
    content: str = "content-1"
    definition: str = "definition-1"
    desired: str = "open"
    expected_revision: int = 0


@dataclass(frozen=True)
class State:
    opened: bool = False
    revision: int = 0
    receipts: tuple = ()


class Owner:
    def __init__(self, binding=Binding(), limit=8, revision_max=16):
        if type(limit) is not int or limit <= 0 or revision_max <= 0:
            raise ValueError("INVALID_FIXTURE_LIMIT")
        self.binding = link(binding)
        self.limit, self.revision_max = limit, revision_max
        self.current = Context()
        self.state = State()
        self.static_blockers = frozenset()
        self.occupants = frozenset()
        self.content = "content-1"

    def blocked(self):
        own = frozenset() if self.state.opened else frozenset(self.binding.footprint)
        return self.static_blockers | own

    def apply(self, context, command, fail_before_publish=False):
        # Current authorization precedes receipt lookup, even for a duplicate.
        if context != self.current or command.session != context.session:
            return "FENCED"
        if type(command.command) is not int or command.command <= 0:
            return "INVALID_COMMAND"
        ref = (command.session, command.command)
        for old, outcome in self.state.receipts:
            if (old.session, old.command) == ref:
                return outcome if old == command else "CONFLICT"
        # No eviction: capacity exhaustion cannot turn an old request into new work.
        if len(self.state.receipts) >= self.limit:
            return "NOT_ADMITTED"
        before = self.state
        if (command.placement != "fixture:placement" or command.incarnation != 1
                or command.content != self.content
                or command.definition != self.binding.revision):
            outcome = "BINDING_MISMATCH"
        elif command.expected_revision != before.revision:
            outcome = "STALE_STATE"
        elif command.desired not in self.binding.states:
            outcome = "UNSUPPORTED_STATE"
        elif command.desired == "closed" and self.occupants & set(self.binding.footprint):
            outcome = "OCCUPIED"
        elif before.opened != (command.desired == "open") and before.revision == self.revision_max:
            outcome = "REVISION_EXHAUSTED"
        else:
            outcome = "COMMITTED"
        opened = command.desired == "open" if outcome == "COMMITTED" else before.opened
        revision = before.revision + int(opened != before.opened)
        candidate = State(opened, revision, before.receipts + ((command, outcome),))
        if fail_before_publish:
            return "NOT_ADMITTED"
        self.state = candidate  # assumed single non-interleaved model publication
        return outcome

    def enter(self, cell):
        # Fixture-only Movement-owned candidate, not a client destination API.
        if cell in self.blocked():
            return False
        self.occupants = self.occupants | {cell}
        return True

    def activate_other_content(self):
        # Candidate minimum: no content switch while this scope is live.
        return "LIVE_SCOPE_REJECTED"


@dataclass(frozen=True)
class Frame:
    sequence: int
    connection: int = 1
    kind: str = "delta"
    base: int = 0
    revision: int = 1
    opened: bool = True


class Client:
    def __init__(self):
        self.connection, self.sequence, self.revision = 1, 0, 0
        self.opened, self.resync = False, False

    def apply(self, frame):
        if frame.connection != self.connection:
            return "FENCED"
        if self.resync:
            return "RESYNC_REQUIRED"
        if frame.sequence <= self.sequence:
            return "DUPLICATE"
        if frame.sequence != self.sequence + 1:
            self.resync = True
            return "SEQUENCE_GAP"
        if frame.kind == "result":
            self.sequence = frame.sequence
            return "RESULT_ONLY"
        if frame.kind != "delta" or frame.base != self.revision or frame.revision <= frame.base:
            self.resync = True
            return "REVISION_GAP"
        self.sequence, self.revision, self.opened = frame.sequence, frame.revision, frame.opened
        return "APPLIED"

    def snapshot(self, connection, sequence, revision, opened, complete):
        # Called only for a trusted, current requested snapshot in the model.
        if connection != self.connection or not complete:
            return "REJECTED"
        if sequence < self.sequence or revision < self.revision:
            return "STALE"
        if revision == self.revision and opened != self.opened:
            return "CONFLICT"
        self.sequence, self.revision, self.opened = sequence, revision, opened
        self.resync = False
        return "INSTALLED"


class IngressWitness:
    """Behavioral sketch of FND duplicate dispositions; not its implementation."""
    def __init__(self):
        self.next_id, self.pending = 1, set()

    def reserve(self, command):
        if command < self.next_id:
            return "ALREADY_RESERVED"
        if command != self.next_id:
            return "SEQUENCE_GAP"
        self.pending.add(command)
        self.next_id += 1
        return "RESERVED"

    def terminal(self, command):
        if not self.pending or command != min(self.pending):
            return False
        self.pending.remove(command)
        return True

    def duplicate(self, command, retained):
        if command >= self.next_id:
            return "NOT_DUPLICATE"
        if command in self.pending:
            return "PENDING_ORIGINAL"
        return "REPLAY_RETAINED" if retained else "OUTCOME_EXPIRED"


class ContractWitness(unittest.TestCase):
    def test_linker_rejects_unsupported_or_ambiguous_binding(self):
        for binding in (replace(Binding(), owner="content"),
                        replace(Binding(), lifetime="durable"),
                        replace(Binding(), footprint=(-1, -1)),
                        replace(Binding(), edges=(("closed", "open"),) * 2)):
            with self.subTest(binding=binding), self.assertRaises(ValueError):
                link(binding)

    def test_replay_cannot_reopen_after_later_close(self):
        owner = Owner(); first = Command()
        owner.apply(owner.current, first)
        owner.apply(owner.current, Command(command=2, desired="closed", expected_revision=1))
        before = owner.state
        self.assertEqual(owner.apply(owner.current, first), "COMMITTED")
        self.assertEqual(owner.state, before)
        self.assertFalse(owner.state.opened)

    def test_same_command_changed_payload_conflicts(self):
        owner = Owner(); owner.apply(owner.current, Command()); before = owner.state
        self.assertEqual(owner.apply(owner.current, Command(desired="closed")), "CONFLICT")
        self.assertEqual(owner.state, before)

    def test_authority_checked_before_replay(self):
        owner = Owner(); owner.apply(owner.current, Command()); before = owner.state
        for context in (replace(owner.current, connection=2), replace(owner.current, scope="channel-b"),
                        replace(owner.current, owner_generation=2), replace(owner.current, session="other")):
            self.assertEqual(owner.apply(context, Command()), "FENCED")
        self.assertEqual(owner.state, before)

    def test_valid_reconnect_keeps_original_command_identity(self):
        owner = Owner(); owner.apply(owner.current, Command()); before = owner.state
        owner.current = replace(owner.current, connection=2)  # trusted fixture rebind, not issuance
        self.assertEqual(owner.apply(owner.current, Command()), "COMMITTED")
        self.assertEqual(owner.state, before)

    def test_unknown_target_revision_and_state_never_mutate_world(self):
        for command in (Command(placement="other"), Command(incarnation=2), Command(content="other"),
                        Command(definition="other"), Command(expected_revision=9), Command(desired="locked")):
            owner = Owner(); outcome = owner.apply(owner.current, command)
            self.assertNotEqual(outcome, "COMMITTED")
            self.assertEqual((owner.state.opened, owner.state.revision), (False, 0))
            self.assertEqual(owner.apply(owner.current, command), outcome)

    def test_open_removes_only_own_spatial_contribution(self):
        owner = Owner(); owner.static_blockers = frozenset({0})
        owner.apply(owner.current, Command())
        self.assertEqual(owner.blocked(), frozenset({0}))
        self.assertFalse(owner.enter(0))

    def test_occupied_close_is_terminal_for_original_occurrence(self):
        owner = Owner(); owner.apply(owner.current, Command()); owner.enter(-1)
        close = Command(command=2, desired="closed", expected_revision=1)
        self.assertEqual(owner.apply(owner.current, close), "OCCUPIED")
        owner.occupants = frozenset()
        self.assertEqual(owner.apply(owner.current, close), "OCCUPIED")
        self.assertTrue(owner.state.opened)

    def test_both_movement_close_orders_are_safe(self):
        for movement_first in (True, False):
            owner = Owner(); owner.apply(owner.current, Command())
            close = Command(command=2, desired="closed", expected_revision=1)
            if movement_first:
                self.assertTrue(owner.enter(-1))
                self.assertEqual(owner.apply(owner.current, close), "OCCUPIED")
            else:
                self.assertEqual(owner.apply(owner.current, close), "COMMITTED")
                self.assertFalse(owner.enter(-1))
            self.assertFalse(owner.occupants & owner.blocked())

    def test_capacity_exhaustion_does_not_evict_replay_guard(self):
        owner = Owner(limit=1); owner.apply(owner.current, Command()); before = owner.state
        self.assertEqual(owner.apply(owner.current, Command(command=2)), "NOT_ADMITTED")
        self.assertEqual(owner.apply(owner.current, Command()), "COMMITTED")
        self.assertEqual(owner.state, before)

    def test_prepublication_failure_has_no_partial_commit(self):
        owner = Owner(); before = owner.state
        self.assertEqual(owner.apply(owner.current, Command(), True), "NOT_ADMITTED")
        self.assertEqual(owner.state, before)
        self.assertEqual(owner.apply(owner.current, Command()), "COMMITTED")

    def test_revision_exhaustion_cannot_wrap(self):
        owner = Owner(revision_max=1); owner.apply(owner.current, Command())
        self.assertEqual(owner.apply(owner.current, Command(command=2, desired="closed", expected_revision=1)),
                         "REVISION_EXHAUSTED")
        self.assertEqual((owner.state.opened, owner.state.revision), (True, 1))

    def test_live_activation_does_not_reset_overlay_or_receipts(self):
        owner = Owner(); owner.apply(owner.current, Command()); before = owner.state
        self.assertEqual(owner.activate_other_content(), "LIVE_SCOPE_REJECTED")
        self.assertEqual(owner.state, before)

    def test_current_result_frame_cannot_overwrite_newer_world_state(self):
        client = Client(); client.apply(Frame(1)); client.apply(Frame(2, base=1, revision=2, opened=False))
        self.assertEqual(client.apply(Frame(3, kind="result", opened=True)), "RESULT_ONLY")
        self.assertEqual((client.sequence, client.revision, client.opened), (3, 2, False))

    def test_old_connection_frame_does_not_poison_current_stream(self):
        client = Client()
        self.assertEqual(client.apply(Frame(7, connection=2)), "FENCED")
        self.assertEqual((client.sequence, client.revision, client.resync), (0, 0, False))

    def test_sequence_or_revision_gap_requires_snapshot(self):
        for frame in (Frame(2), Frame(1, base=8, revision=9)):
            client = Client(); client.apply(frame)
            self.assertTrue(client.resync)
            self.assertEqual((client.sequence, client.revision, client.opened), (0, 0, False))
            self.assertEqual(client.apply(Frame(1)), "RESYNC_REQUIRED")
            self.assertEqual(client.snapshot(1, 2, 2, True, False), "REJECTED")
            self.assertEqual(client.snapshot(1, 2, 2, True, True), "INSTALLED")

    def test_duplicate_and_stale_snapshot_do_not_regress_client(self):
        client = Client(); client.apply(Frame(1)); before = (client.sequence, client.revision, client.opened)
        self.assertEqual(client.apply(Frame(1, opened=False)), "DUPLICATE")
        self.assertEqual(client.snapshot(1, 0, 0, False, True), "STALE")
        self.assertEqual((client.sequence, client.revision, client.opened), before)

    def test_expired_outcome_is_not_new_command_authority(self):
        ingress = IngressWitness()
        self.assertEqual(ingress.reserve(1), "RESERVED")
        self.assertEqual(ingress.duplicate(1, False), "PENDING_ORIGINAL")
        self.assertTrue(ingress.terminal(1))
        self.assertEqual(ingress.duplicate(1, True), "REPLAY_RETAINED")
        self.assertEqual(ingress.duplicate(1, False), "OUTCOME_EXPIRED")
        self.assertEqual(ingress.reserve(1), "ALREADY_RESERVED")

    def test_ingress_gap_and_terminal_order(self):
        ingress = IngressWitness()
        self.assertEqual(ingress.reserve(2), "SEQUENCE_GAP")
        ingress.reserve(1); ingress.reserve(2)
        self.assertFalse(ingress.terminal(2))
        self.assertTrue(ingress.terminal(1))
        self.assertTrue(ingress.terminal(2))

    def test_equal_revision_inconsistent_snapshot_is_rejected(self):
        client = Client(); client.apply(Frame(1))
        self.assertEqual(client.snapshot(1, 2, 1, False, True), "CONFLICT")
        self.assertEqual((client.sequence, client.revision, client.opened), (1, 1, True))

    def test_all_bounded_serialized_traces(self):
        # 8^4 sequences, not a concurrency or production-state-space proof.
        events = ("open", "close", "replay", "conflict", "enter", "leave", "fail", "fenced")
        checked = 0
        for trace in product(events, repeat=4):
            owner = Owner(limit=5); saved = None
            for number, event in enumerate(trace, 1):
                before = owner.state
                if event == "enter":
                    owner.enter(-1)
                elif event == "leave":
                    owner.occupants = frozenset()
                elif event in {"replay", "conflict"}:
                    if saved is not None:
                        request = saved if event == "replay" else replace(saved, content="other")
                        owner.apply(owner.current, request)
                        self.assertEqual(owner.state, before)
                else:
                    request = Command(command=number, desired="closed" if event == "close" else "open",
                                      expected_revision=before.revision)
                    context = replace(owner.current, connection=9) if event == "fenced" else owner.current
                    outcome = owner.apply(context, request, event == "fail")
                    if event in {"fail", "fenced"}:
                        self.assertEqual(owner.state, before)
                    elif outcome != "NOT_ADMITTED" and saved is None:
                        saved = request
                self.assertLessEqual(len(owner.state.receipts), owner.limit)
                self.assertGreaterEqual(owner.state.revision, before.revision)
                self.assertFalse(owner.occupants & owner.blocked())
            checked += 1
        self.assertEqual(checked, 4096)
        print(f"BOUNDED_SERIALIZED_TRACES={checked}")


if __name__ == "__main__":
    unittest.main(verbosity=2)
```
<!-- LOCAL_TRANSITION_WITNESS_END -->
