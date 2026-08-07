# Oteryn v2 Foundation Programme — Current Status

- Status: Canonical current execution-status overlay
- Date: 2026-08-07
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: current foundation gate progression and next-action interpretation
- Does not supersede: accepted ADR semantics, dedicated accepted contract semantics, product decisions or historical evidence except where this file explicitly identifies a stale progress/coordination view

## Purpose

Keep the live foundation programme state unambiguous when long-lived backlog, global-register, gap-register, baseline or coordinator documents still contain progress or scope sentences written before later accepted closeouts and contracts.

This file is authoritative for **current execution status and gate readiness**. Accepted architecture semantics remain authoritative in ADRs and dedicated contracts. Stable gate definitions remain in `FOUNDATION_DECISION_BACKLOG.md` and the wider decision horizon remains in `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`, subject to the explicit stale-view reconciliation notes below.

Chat history is not execution authority.

## Exact migration and closeout evidence

### Destination cutover — complete

- source snapshot used for the accepted migration: `blakinio/otclient@c923ad8a1dff17b4933a6110931b0823cec2c590`;
- canonical destination: `blakinio/Oteryn-v2`;
- canonical client path: `apps/client`;
- destination PR: `blakinio/Oteryn-v2#50`;
- destination squash merge: `78988f72a80cc904aa9176ae850c50d4efa0b0f0`;
- result: accepted 19-member Rust workspace and ADR-0011 `pre-native-protocol` client state.

### Source-only historical marker — complete

- source-marker PR: `blakinio/otclient#274`;
- exact source-marker head: `0bb7f92ae420fc3e81a4ade62a9b9b994c894f0c`;
- exact-head `Rust Client` run `31155904330`: `PASS`;
- exact-head repository `CI` run `31155910869`: `PASS`;
- unresolved review threads: `0`;
- source-marker squash merge: `8c56c45c6c25147470ce3ca23e639a31d9085e47`;
- effect: `blakinio/otclient/oteryn-client/**` is explicitly `HISTORICAL / NON-CANONICAL`, new Oteryn v2 Rust-client work is redirected to `blakinio/Oteryn-v2`, and the source history/provenance remains preserved.

### Source-marker lifecycle archive — complete

- lifecycle PR: `blakinio/otclient#275`;
- lifecycle exact head: `1e888ba073742c26bf9a1cae5786a059a270fa00`;
- repository `CI` run `31156414051`: `PASS`;
- lifecycle squash merge: `26f7646ea26b27c9ac4bf617b8cb0d63c89bdfda`;
- `blakinio/otclient/main` after closeout: `26f7646ea26b27c9ac4bf617b8cb0d63c89bdfda`;
- task ownership: released and archived.

### `FND-ID-01` minimum-scope decision — complete

- minimum-scope delivery PR: `blakinio/Oteryn-v2#80`;
- delivery squash merge: `96760a99ce09bf20417a4a9d6dc1961785156b6c`;
- corrected lifecycle closeout PR: `blakinio/Oteryn-v2#84`;
- closeout exact head: `b27b5c4fab2b212bf6467b1a2e16b0b79c14614f`;
- exact-head Agent governance run `31209475571`: `PASS`;
- exact-head Dependency review run `31209474653`: `PASS`;
- exact-head CodeQL run `31209477716`: `PASS`;
- independent closeout audit review `4885857937`: `PASS`, zero material findings;
- closeout squash merge: `54f929580b980b6adc5025bd838bd426ff302436`;
- superseded stale closeout PR #81 was closed rather than merged.

### `FND-ID-01` complete foundation identifier contract — complete

- canonical contract: `docs/architecture/FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`;
- delivery PR: `blakinio/Oteryn-v2#85`;
- final exact delivery head: `6686a4b62c1e6b518d38ab3c80326b8621abe5bb`;
- final branch was synchronized with then-current `main` and was zero commits behind before validation;
- exact-head Agent governance run `31210390375`: `PASS`;
- exact-head Dependency review run `31210390471`: `PASS`;
- exact-head CodeQL run `31210390382`: `PASS`;
- independent architecture audit review `4885881365`: `PASS`, zero open material findings;
- unresolved review threads: `0`;
- squash merge: `2c584543cd1e3758958755478a6cc6ed3d39a8a9`;
- result: `FND-ID-01` is complete at the architecture-contract level; no protocol/runtime implementation was authorized by that merge.

## Current ordered foundation state

| Gate / programme step | Current status | Consequence |
|---|---|---|
| `FND-01` | `ACCEPTED AND APPLIED` | Workspace/dependency/migration contract is complete. |
| `VSL-02` destination cutover | `COMPLETE` | Canonical migrated client/workspace exists in Oteryn-v2. |
| `VSL-02` source-only closeout | `COMPLETE` | Historical/non-canonical marker and archive are merged in `blakinio/otclient`. |
| `FND-ID-01` | `ACCEPTED AND MERGED` | Foundation identifier semantics are frozen by `FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`. |
| coordination reconciliation #86 | `NEXT CLEANUP` | Reconcile stale pre-minimum-scope FND-ID wording in the long-lived backlog/global register before presenting the programme views as fully synchronized. |
| `FND-02` | `READY AFTER #86 COORDINATION CLEANUP` | The semantic identifier prerequisite is complete. The canonical protocol contract may begin after the stale coordination views are reconciled; the old Platform native contract remains reconciliation input only. |
| `FND-03` | `BLOCKED ON OWN CONTRACT` | No authoritative runtime implementation claim yet. |
| `FND-04` | `BLOCKED ON OWN CONTRACT` | No production admission/lease implementation claim yet. |
| `DUR-01`…`DUR-04`, `ANL-01`… | `LATER GATES` | Existing ordering and architecture requirements remain unchanged. |

## `FND-ID-01` completion boundary

`FND-ID-01` is now complete at the architecture-contract level.

The merged contract freezes the minimum foundation catalogue and boundaries required by downstream foundation work:

- `AccountId` remains owned by Oteryn Platform Identity and is consumed losslessly without silent re-keying;
- `CharacterId` is a global strongly typed UUIDv7 identity;
- `WorldId` is a global logical-world UUIDv7 target identity owned/issued by Platform World Registry/topology authority;
- `ChannelId` is a UUIDv7 scoped as `WorldId + ChannelId`, owned/issued by Platform World Registry/topology authority;
- `NodeId` identifies exactly one GameNode process incarnation and is freshly generated by that authenticated process bootstrap before trusted registration; registration/identity possession does not confer channel mutation authority;
- `InstanceId` is a UUIDv7 scoped as `WorldId + InstanceId`, issued by the game-domain Instance/Activity allocator;
- `PartyId` is a UUIDv7 scoped as `WorldId + PartyId`, issued by the world-level Party/Social authority;
- `GameSessionId` is a global UUIDv7 logical gameplay-session identity owned/issued by the game-domain Game Session / Admission authority only after successful authoritative admission;
- `HandoffId` is the one additional conditional foundation UUIDv7 identity, scoped as `WorldId + HandoffId`, for a bounded authoritative runtime-ownership transition already required by accepted instance/handoff architecture.

The gate explicitly does **not** invent `AdmissionId` or `CharacterLeaseId`. `FND-04` may request a narrow identity amendment only if its final design proves a separately addressable semantic entity is required.

`CommandId` and protocol sequencing remain `FND-02` work. Runtime-local handles remain `FND-03` work. Admission/session/lease state machines remain `FND-04` work. PostgreSQL representation and later durable-domain identities remain `DUR-*` work. Analytics/audit identity catalogues remain `ANL-*` work.

Connection/channel/instance generations and party revision are ordering/revision/fencing values, not entity identities.

## External Platform protocol reconciliation boundary

The repository owner accepted on 2026-08-07 that the merged `blakinio/Oteryn-Platform` native gameplay contract at `c0b8703d326a04b43ae8e06f6192b0cb91c859b7` is **bounded reconciliation input only** for Oteryn v2 and is not the final `protocol-oteryn` contract.

Canonical interpretation is recorded in `FND-02_PLATFORM_PROTOCOL_RECONCILIATION_OWNER_BASELINE.md`.

The external revision remains useful evidence for Platform Identity, one-time ticket, Game Gateway, World Registry, exact route/session binding, fail-closed handling, downgrade prevention and related security requirements where those concepts remain consistent with accepted Oteryn-v2 ADRs and the merged `FND-ID-01` contract.

It does not pre-authorize or freeze for Oteryn v2:

- production Canary protocol negotiation, adapter, listener or fallback;
- Otheryn C++ as the target gameplay runtime;
- the exact `tcp.tls13.protobuf.be32.v1` transport;
- protobuf as the final IDL solely because that external contract uses it;
- its schema revision/hash or fixed capability digest as the Oteryn-v2 `FND-02` answer;
- its exact command, sequencing, revision or pre-admission session representation before the owning Oteryn-v2 gates decide them;
- Platform issuance of canonical game-domain `GameSessionId`.

`docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json` therefore retains the immutable external revision while explicitly classifying it as `RECONCILIATION_INPUT_ONLY` with `accepted_for_fnd02 = false`.

A complete cross-repository resolution later requires a separate explicitly authorized `blakinio/Oteryn-Platform` task/branch/PR. No Platform write is authorized by this Oteryn-v2 status update.

## Stale progress and coordination-view reconciliation

Long-lived documents may retain historical progress sentences saying the source-only marker is pending or that `FND-ID-01` cannot start. Those statements remain stale for execution status and are superseded by the exact evidence in this status overlay.

Additionally, two coordination views retain an older **pre-minimum-scope** FND-ID candidate catalogue:

- `FOUNDATION_DECISION_BACKLOG.md`;
- `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`.

Those older FND-ID scope bullets list protocol, analytics and durability identities that were deliberately reassigned by the later owner-accepted minimum-scope decision and the merged `FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`.

Until issue #86 edits those two views, interpret their FND-ID scope using this precedence:

```text
FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md
    -> FND-ID-01_MINIMUM_CROSS_BOUNDARY_SCOPE_OWNER_BASELINE.md
    -> later dedicated owner baselines
    -> stale FND-ID scope bullets in backlog/global register
```

This explicit precedence is narrow: it corrects only the superseded FND-ID scope description. Unrelated gate definitions and architecture horizon entries in those registers remain in force.

Other historical documents may also retain old progress-only wording, including:

- `ARCHITECTURE_ANALYSIS_GAP_REGISTER.md`;
- `PRODUCT_DIRECTION_BASELINE.md` and ADR-0010 historical programme-effect text;
- `docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md`;
- earlier task/evidence snapshots.

Do not mass-rewrite historical ADRs or evidence merely to replace old progress wording.

## Current next action

Resolve issue #86 with one bounded architecture/documentation reconciliation task that updates the stale FND-ID scope entries in:

- `FOUNDATION_DECISION_BACKLOG.md`;
- `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`.

That reconciliation must preserve history and the gate ordering while moving:

- `CommandId` / command sequencing to `FND-02`;
- runtime-local entity/handle mechanics to `FND-03`;
- admission/session/lease mechanics to `FND-04`;
- event/operation/transaction/correlation/causation/analytics identity catalogues to their `ANL-*` / `DUR-*` owners.

After #86 is merged and verified, the next ordered foundation contract is:

```text
FND-02 — canonical protocol-oteryn contract
```

`FND-02` must consume the merged FND-ID contract and `FND-02_PLATFORM_PROTOCOL_RECONCILIATION_OWNER_BASELINE.md` and must not silently adopt the stale external Platform wire tuple as its answer.

`GAME-VISION-01` analysis may continue in parallel when it does not redefine accepted foundation identity, repository, protocol, Platform or persistence boundaries.

## Non-authorization

This status update does not authorize implementation of `protocol-oteryn`, authoritative runtime, Game Session admission, character leases, persistence schemas, durable gameplay, Platform changes, production deployment or live operations.
