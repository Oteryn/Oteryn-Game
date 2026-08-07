# Oteryn v2 Foundation Programme — Current Status

- Status: Canonical current execution-status overlay
- Date: 2026-08-07
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: current foundation gate progression and next-action interpretation
- Does not supersede: accepted ADR semantics, dedicated accepted contract semantics, product decisions or historical evidence except where this file explicitly identifies a stale progress/coordination view

## Purpose

Keep the live foundation programme state unambiguous when long-lived backlog, global-register, gap-register, baseline or coordinator documents contain progress sentences written before later accepted closeouts and contracts.

This file is authoritative for **current execution status and gate readiness**. Accepted architecture semantics remain authoritative in ADRs and dedicated contracts. Stable gate definitions remain in `FOUNDATION_DECISION_BACKLOG.md` and the wider decision horizon remains in `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`.

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
- effect: `blakinio/otclient/oteryn-client/**` is explicitly `HISTORICAL / NON-CANONICAL`, new Oteryn v2 Rust-client work is redirected to `blakinio/Oteryn-v2`, and source history/provenance remains preserved.

### Source-marker lifecycle archive — complete

- lifecycle PR: `blakinio/otclient#275`;
- lifecycle exact head: `1e888ba073742c26bf9a1cae5786a059a270fa00`;
- repository `CI` run `31156414051`: `PASS`;
- lifecycle squash merge: `26f7646ea26b27c9ac4bf617b8cb0d63c89bdfda`;
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
- lifecycle/status closeout PR #87 squash merge: `648aa10bb5b36d8826d82ed0f1ed94a47ca53a24`;
- result: `FND-ID-01` is complete at the architecture-contract level; no protocol/runtime implementation was authorized.

### FND-ID coordination-register reconciliation — complete with this change

Issue #86 identified two long-lived coordination views that still carried the older pre-minimum-scope FND-ID candidate catalogue:

- `FOUNDATION_DECISION_BACKLOG.md`;
- `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`.

This reconciliation updates both views to the merged FND-ID contract without reopening or changing its semantics:

- `CommandId` / command sequencing are owned by `FND-02`;
- runtime-local entity/worker/task/generational handles are owned by `FND-03`;
- admission/session/lease/reconnect/takeover mechanics are owned by `FND-04`;
- event/operation/transaction/correlation/causation/pseudonymous analytics identity catalogues are owned by `ANL-*`/durability contracts as appropriate;
- the foundation catalogue remains the accepted minimum plus conditional `HandoffId`.

When this reconciliation PR is merged and validated, issue #86 is terminal and the long-lived coordination views are synchronized with the accepted semantic source.

## Current ordered foundation state

| Gate / programme step | Current status | Consequence |
|---|---|---|
| `FND-01` | `ACCEPTED AND APPLIED` | Workspace/dependency/migration contract is complete. |
| `VSL-02` destination/source closeout | `COMPLETE` | Canonical migrated client/workspace exists and old source is historical/non-canonical. |
| `FND-ID-01` | `ACCEPTED AND MERGED` | Foundation identifier semantics are frozen by `FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`. |
| coordination reconciliation #86 | `COMPLETED BY THIS RECONCILIATION` | Backlog and global register now consume the merged minimum FND-ID scope. |
| `FND-02` | `NEXT ORDERED GATE` | Draft/audit/accept the canonical `protocol-oteryn` architecture contract; no runtime implementation is authorized yet. |
| `FND-03` | `BLOCKED ON OWN CONTRACT` | No authoritative runtime implementation claim yet. |
| `FND-04` | `BLOCKED ON OWN CONTRACT` | No production admission/lease implementation claim yet. |
| `DUR-01`…`DUR-04`, `ANL-01`… | `LATER GATES` | Existing ordering and architecture requirements remain unchanged. |

## `FND-ID-01` completion boundary

`FND-ID-01` is complete at the architecture-contract level.

The merged contract freezes the minimum foundation catalogue and boundaries required by downstream foundation work:

- `AccountId` remains owned by Oteryn Platform Identity and is consumed losslessly without silent re-keying;
- `CharacterId` is a global strongly typed UUIDv7 identity;
- `WorldId` is a global logical-world UUIDv7 target identity owned/issued by Platform World Registry/topology authority;
- `ChannelId` is a UUIDv7 scoped as `WorldId + ChannelId`, owned/issued by Platform World Registry/topology authority;
- `NodeId` identifies exactly one GameNode process incarnation and is freshly generated by authenticated process bootstrap before trusted registration; registration/identity possession does not confer channel mutation authority;
- `InstanceId` is a UUIDv7 scoped as `WorldId + InstanceId`, issued by the game-domain Instance/Activity allocator;
- `PartyId` is a UUIDv7 scoped as `WorldId + PartyId`, issued by the world-level Party/Social authority;
- `GameSessionId` is a global UUIDv7 logical gameplay-session identity owned/issued by the game-domain Game Session / Admission authority only after successful authoritative admission;
- `HandoffId` is the additional conditional foundation UUIDv7 identity, scoped as `WorldId + HandoffId`, for a bounded authoritative runtime-ownership transition already required by accepted handoff architecture.

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

`docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json` retains the immutable external revision while classifying it as `RECONCILIATION_INPUT_ONLY` with `accepted_for_fnd02 = false`.

A complete cross-repository resolution later requires a separate explicitly authorized `blakinio/Oteryn-Platform` task/branch/PR. No Platform write is authorized by this Oteryn-v2 status update.

## Historical progress wording

Historical ADRs, task archives, evidence snapshots and some older coordination/prompts may retain progress-only sentences from before migration/FND-ID closeout. Do not mass-rewrite historical evidence merely to make old timestamps read like current status.

For current execution status, this file and live GitHub state are authoritative. For semantic FND-ID meaning, `FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md` is authoritative.

## Current next action

Create one bounded **architecture-only `FND-02` contract task** in `blakinio/Oteryn-v2` after this reconciliation is merged/closed.

The task must consume at minimum:

- `FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`;
- `FND-02_PLATFORM_PROTOCOL_RECONCILIATION_OWNER_BASELINE.md`;
- `docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json`;
- `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`;
- `docs/contracts/FOUNDATION_ERROR_VOCABULARY.md`;
- `docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md`;
- applicable accepted disconnect/reconnect, privacy and Game Session baselines.

`FND-02` must decide the canonical native protocol contract rather than silently copying the older Platform tuple. It must preserve ADR-0008 downgrade/fallback prohibitions and the merged FND-ID semantics.

`GAME-VISION-01` analysis may continue in parallel when it does not redefine accepted foundation identity, repository, protocol, Platform or persistence boundaries.

## Non-authorization

This status update does not authorize implementation of `protocol-oteryn`, authoritative runtime, Game Session admission, character leases, persistence schemas, durable gameplay, Platform changes, production deployment or live operations.
