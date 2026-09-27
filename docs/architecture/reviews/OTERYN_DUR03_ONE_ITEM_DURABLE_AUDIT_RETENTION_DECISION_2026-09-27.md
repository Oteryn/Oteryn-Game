# DUR-03 One-Item Durable-Audit P90D Retention Decision

- Status: `OWNER_SELECTED; CANONICAL ACCEPTANCE PENDING PROTECTED INTEGRATION`
- Date: 2026-09-27
- Repository: `Oteryn/Oteryn-Game`
- Product workstream: DUR-03 / [Issue #513](https://github.com/Oteryn/Oteryn-Game/issues/513)
- Owner selection: [#513 comment 5856056298](https://github.com/Oteryn/Oteryn-Game/issues/513#issuecomment-5856056298)
- Architect options/common terms: [#513 comment 5855850428](https://github.com/Oteryn/Oteryn-Game/issues/513#issuecomment-5855850428)
- Documentation allocation: [#162 comment 5856061315](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5856061315)
- Implementation, production collection and deployment authority: `NONE_BY_THIS_DECISION`

## Authority and problem

`PROVEN`: the owner selected the separate item-specific P90D option and common
bounded terms recorded on #513. [DUR-03 §39.1](../DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md)
accepts the closed one-item MINT/TRANSFER audit semantics but explicitly leaves
item retention unaccepted. [ANL-01 §§15–16](../ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md)
requires a finite accepted purpose/privacy/retention/access profile before
production collection. This record canonicalizes that owner selection; it does
not retrospectively turn the earlier semantic resolution into retention approval.

The [Character P90D decision](OTERYN_CHARACTER_DURABLE_AUDIT_RETENTION_DECISION_2026-09-22.md)
is a process precedent only. Its Character owner/world/lifecycle purpose and
profile are not item authority. The duration here comes from the item-specific
owner selection, independently of that precedent.

## Mandatory decision test and alternatives

1. **Must decide now? YES.** The first item-audit family needs its own accepted
   retention policy before a separately allocated schema/registry admission can
   close the DUR03-RL-07 audit gap or collect production evidence.
2. **Concrete downstream gate:** #513's first closed one-item mandatory-audit
   profile binding. Schema/encoding measurement, numeric resource acceptance and
   physical atomicity remain separate requirements; this decision closes none of
   those gates by itself.
3. **What becomes harder later:** admitted events bind immutable profile revisions.
   Increasing lookback later cannot recover deleted evidence; changing purpose
   or duration retrospectively creates privacy, compatibility and migration cost.
4. **Supersession evidence:** explicit changed owner/legal requirements, verified
   support or incident investigation needs, privacy findings, or measured storage
   and cleanup cost may justify a separately reviewed successor.
5. **Deliberately not decided:** replay horizon, schema/identifiers, registries,
   resource maxima, SQL/runtime mechanisms, production admission and broader
   economy/analytics retention.

The realistic owner options were P30D, P90D and P180D. P30D reduces retained
player-linked exposure and investigation lookback; P180D increases both lookback
and exposure/storage/cleanup work. The selected P90D provides the finite middle
option recommended by the architect. This is an owner choice, not a measured
claim that 90 days is optimal or legally required.

## Selected logical profile and scope

Logical profile: `DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1`.

`ordinary_retention_ceiling=P90D` (90 days), rolling separately for each retained
event from its authoritative event occurrence/commit timestamp under the owning
implementation contract. Admission, delivery, retry, query, export, legal-hold
release or a later TRANSFER must not restart or extend that original clock.
MINT and TRANSFER each keep their own timestamp and ordinary expiry. This is not
a global periodic retention cycle or a manual renewal policy.

This is a separate logical item profile, not a serialized registry entry or an
allocation of a registry identifier. A later authorized task must bind the
accepted logical profile to the actual registered event family/revision before
production collection.

Purpose is to prove and reconcile the first closed one-item transactions and
support bounded DUR-03 operations, support and security investigation:

- MINT establishes one fresh ItemInstance lifecycle into typed Ground custody,
  including applicable corpse association/provenance.
- A separate TRANSFER moves that same existing live ItemInstance from Ground to
  direct-root CharacterInventory, preserving identity, type and quantity and
  exactly one authoritative immediate location.

They remain distinct logical transactions with separate TransactionIds, event
candidates and atomic boundaries. The profile covers their complete required
ANL envelope and typed aggregate payload evidence. WorldId and ChannelId remain
distinct; Ground has its concrete runtime scope and Character-held value its
owning Character/World scope. Evidence references never confer current mutation
authority.

Multiple items, existing-stack mint, redistribution, burn, transform, non-item
accounts, nested containers and additional custody families remain outside this
first profile. No general economy, market, trade, public history, balancing,
detector or AI-use purpose is accepted.

## Privacy, readers and export

Privacy is at least `RESTRICTED_PLAYER_LINKED`, including Character/session/item
and provenance linkage. `SECURITY_SENSITIVE` applies only where a later accepted
owning security purpose requires that stricter floor and controls. This record
does not silently classify every item transaction under a new security purpose.

Ordinary readers: none. Permitted readers are explicitly authorized DUR-03
operations/support/security roles, limited to their accepted purpose and scope,
with least privilege and audited access. Role membership alone does not grant
unrestricted browsing, bulk export or gameplay mutation.

Authorized export is case-scoped and audited, with unrelated player-linked
envelope/payload fields redacted. Export does not extend retention or create an
indefinite investigative copy. Exported retained player-linked data remains
subject to the applicable original expiry/explicit hold. This profile permits
no public projection, ordinary metric labels containing restricted identities,
unrestricted debugging, indefinite pseudonymous successor or separately derived
analytics aggregate; a new purpose requires its own accepted profile.

## Expiry and bounded cleanup

At ordinary expiry, delete the retained player-linked event envelope and payload,
subject only to an active explicit audited case/legal hold. Do not replace
deletion with silent indefinite anonymized or pseudonymous retention.

A later implementation must provide bounded automatic cleanup without gameplay
or server downtime, while preserving transaction integrity, mandatory complete
retained evidence sets and active holds. Cleanup must neither partially corrupt
a still-required transaction evidence set nor remove in-flight mandatory evidence
needed for atomic commit/publication/reconciliation. Those obligations require
safe coordination with the owning durability contract; they are not permission
to retain ordinary player-linked audit indefinitely. Exact scheduler, batch size,
physical storage and atomic cleanup mechanism remain implementation decisions
under separate allocation and validation.

Expiry removes audit evidence under this purpose. It cannot delete or weaken
separately required receipt/source-cause replay protection or ItemInstance
non-reuse evidence merely because those records correlate with the expired audit.

## Explicit case/legal hold

Hold is exceptional. Each hold requires explicit case/legal authorization with
reason, authorizing actor, start time and exact affected records/scope. Placement,
access, scope changes and release must be audited; hold scope must be bounded to
the authorized case. No generic operational need silently becomes a hold.

Cleanup must protect records under an active authorized hold. On release, return
to the **original** ordinary expiry without resetting the clock: if it has passed,
the records are eligible for the bounded deletion process immediately; otherwise
only the original remaining lifetime applies. Hold never changes event semantics,
custody, identity, transaction authority or gameplay state.

## Immutable revision and rollout boundary

Use positive, forward-only immutable logical profile revisions for future
admissions. A changed purpose/duration/access policy requires explicit reviewed
supersession and a new profile revision; newly admitted events bind the accepted
revision. Already admitted events retain their original binding and original
expiry. Payload bytes, EventId meaning and existing profile bindings are not
rewritten to fit a successor.

A future rollout or rollback must gate new admissions on the applicable accepted
profile and must not erase existing bindings, lower privacy, refresh old expiry
or silently re-admit events. Missing, conflicting or unresolved profile/access
acceptance keeps production collection/projection closed under ANL-01.

## Replay and identity separation

Audit retention is independent of receipt/source-cause replay/idempotency
protection and ItemInstance identity non-reuse. Their owning horizon, storage and
compaction rules remain unselected. A later implementation must prove that audit
expiry and cleanup preserve those obligations; this decision does not require
retaining this player-linked audit forever as the replay mechanism.

Expiry, export, hold release, profile evolution and audit replay must never:

- reopen mint eligibility or remint a consumed source/cause;
- restore committed source Ground custody after TRANSFER or lost acknowledgement;
- reassign an output ItemInstance identity or reuse it for another lifecycle;
- turn duplicate delivery/retry into a fresh transaction or second consumer effect;
- execute MINT, TRANSFER or any gameplay mutation from historical evidence.

An ambiguous commit still requires reconciliation of the same immutable candidate;
retention cannot turn uncertainty into permission for a fresh transaction.

## Decision conformance tests for later implementation

These are required scenarios, not executed runtime proof supplied by this document:

1. Each MINT/TRANSFER expires from its own authoritative occurrence/commit at the
   P90D boundary; retries, later transfer, reads and exports do not refresh it.
2. Authorized purpose/scoped readers succeed with audited access; unauthorized,
   unrelated-purpose and raw unrestricted export requests reject without downgrade.
3. Case export redacts unrelated linkage and retains original expiry/hold rules.
4. Ordinary expiry deletes player-linked envelope/payload; active hold prevents
   deletion; hold release preserves original expiry, including already-expired cases.
5. Repeated bounded cleanup preserves transaction integrity, required evidence
   completeness, in-flight durability and active holds without server downtime.
6. Missing/conflicting profile, privacy downgrade or mutable admitted binding
   rejects collection; a new revision affects future admissions only.
7. Expiry before duplicate, ambiguous retry, lost TRANSFER acknowledgement and
   source replay preserves receipt/cause protection and identity non-reuse; none
   can remint, restore Ground, reassign identity or make a duplicate fresh.

## Non-decisions and remaining gates

`UNKNOWN / SEPARATE_ACCEPTANCE_REQUIRED`: event type ID/schema and EventId
representation decisions; actual encoding/count/bytes; registered profile binding;
resource maxima; receipt/source-cause replay horizon and physical retention;
ItemInstance tombstone/storage mechanism; PostgreSQL atomicity, outbox, restart
and cleanup implementation; connected loot/pickup/runtime and playable evidence.

No registry mutation, accepted-contract edit, resource-limit selection, SQL,
runtime, deployment, production collection/admission, broad economy/market/trade/
history/balancing/AI use or external repository write is authorized here.
Normal independent review, exact-head CI, Merge Queue and protected-main readback
remain required before canonical integration. The owner selection is established;
this candidate document's protected integration is not asserted.
