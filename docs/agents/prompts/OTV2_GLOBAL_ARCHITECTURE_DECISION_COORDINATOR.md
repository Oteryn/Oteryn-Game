# Oteryn v2 Global Architecture Decision Coordinator

Use this prompt to continue the Oteryn-v2 architecture programme from durable repository state without relying on prior chat.

## 1. Authority and mode

Repository routinely writable in this programme:

- `blakinio/Oteryn-v2`.

All other repositories are read-only unless the owner explicitly authorizes a write task for that exact repository.

### `ANALYZE_ONLY`

When the owner asks to analyze, review, compare, assess, discuss, recommend or think through architecture **without** also asking to save, apply, execute or otherwise mutate the repository:

- do not create or modify tasks, branches, PRs, files, issues or repository settings;
- inspect live sources and return findings, risks, conflicts, missing decisions and recommendations;
- distinguish accepted repository truth from proposals;
- do not infer write authority merely because this prompt was referenced;
- leave the repository unchanged.

### Architecture execution

Architecture/coordination execution is allowed when the owner explicitly asks to continue, save, apply or execute architecture work. Complete the bounded architecture package through task/branch/PR/validation/merge/archive when safe rather than stopping at a draft.

**Rust server/runtime implementation, PostgreSQL DDL/migrations, production deployment and live data/session/account changes require separate explicit owner authority.** A generic request to continue architecture does not grant implementation authority.

If no such implementation authority exists in the current owner instruction, remain paper-only and execute the current architecture `next_action` from the canonical programme checkpoint.

## 2. Mandatory startup

Before mutation:

1. read root `AGENTS.md` and `AGENTS.override.md`;
2. read `docs/agents/AGENTS.md`, `DELIVERY_COMPLETENESS_AND_CLOSEOUT.md`, `BUILD_TEST_MATRIX.md`, and handoff/continuation policies applicable to the task;
3. read `docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md`;
4. read `docs/agents/reports/OTV2-20260812-foundation-handover.md`;
5. read `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`;
6. read only the accepted ADRs/contracts/owner baselines needed by the selected gate;
7. inspect live default-branch head, all open PRs, active tasks/owned paths, reviews and CI;
8. classify any drift as expected, conflicting or stale before writing.

Live merged repository state and trusted-base governance override this prompt.

## 3. Current accepted baseline

Verify against live `main`, but do not rediscover or reopen these merely because older documents contain historical progress prose:

- canonical Rust workspace/client migration and cutover are complete;
- `FND-ID-01` is accepted/lifecycle-closed;
- `FND-02` `protocol-oteryn` architecture is accepted/lifecycle-closed;
- `FND-03` authoritative runtime execution architecture is accepted/lifecycle-closed;
- `FND-04` admission/GameSession/CharacterLease/reconnect architecture is accepted/lifecycle-closed;
- `DUR-01` durable identifier representation is accepted/lifecycle-closed;
- `ANL-01` event/audit foundation is accepted/lifecycle-closed;
- ADR-0014..0016 preserve TCP-default/future-QUIC architecture direction while gameplay transport runtime remains unavailable until separately implemented/proven;
- `GAME-VISION-01` minimum product architecture and first immutable Reference target are accepted/lifecycle-closed;
- `GAME-CHAR-01` Stage A + Stage B semantics are accepted/lifecycle-closed;
- profile-neutral Character persistence partial baseline is accepted/lifecycle-closed;
- whole `DUR-02 — Persistence v1` is accepted/lifecycle-closed.

At the 2026-08-12 successor handoff, trusted main was:

```text
22b64e1b20cf2220828f5a3d47b30df29f9a60b6
```

This SHA is a handoff anchor, not permission to ignore newer legitimate main commits.

## 4. Whole DUR-02 — consume, do not redesign

Canonical source:

- `docs/architecture/DUR-02_PERSISTENCE_V1_OWNER_BASELINE.md`.

Binding common rules:

1. one ordered game-owned migration history for the current native game database boundary, immutable explicit migration artifacts, dedicated least-privilege migrator, no production runtime schema auto-sync;
2. `READ COMMITTED` only with explicit anomaly-closing locks/constraints, otherwise bounded `SERIALIZABLE` or stricter accepted domain mechanism; semantic operation identity survives retry;
3. one ANL-compatible durable journal plus crash-safe mutable publication claim/checkpoint state, atomically committed when evidence is mandatory;
4. acknowledged durable success means committed durable state reconstructible across ordinary process/node restart; FND-03 runtime checkpoint/replay and disaster RPO are separate;
5. PITR-capable, restore-tested fail-closed recovery with a newer non-rollback authority/recovery fence before admission resumes;
6. `EXPAND -> MIGRATE/BACKFILL -> VALIDATE -> CUT OVER -> CONTRACT` schema evolution with writer fencing and evidence-based rollback/recovery.

Do not select exact Rust DB/migration libraries, SQL names, partitioning, RPO/RTO or backup cadence as architecture unless evidence requires it.

## 5. Ownership split after DUR-02 reconciliation

Generic persistence must not become a second semantic owner.

Preserve exactly:

- `GAME-ITEM-01 -> DUR-03` for item/currency/value semantics, transfers, conservation and anti-duplication;
- `ANL-01` for event/audit semantics;
- `EXP-ECONOMY-01` for market/economy semantics;
- `EXP-SOCIAL-01` for guild/social semantics;
- `EXP-HOUSES-01` for houses;
- `GAME-META-01` for recurring/meta rewards;
- `EXP-EVENTS-01` for encounter/event rewards;
- PERF/implementation evidence for partitioning and exact Rust database/migration tooling unless a correctness constraint emerges.

A historical disposition `MOVED` does not mean the destination gate is accepted.

## 6. Current implementation boundary

Architecture prerequisites are now sufficient for a later explicitly authorized common server/persistence implementation programme.

Accepted safe decomposition hypothesis:

```text
server bootstrap / GameNode shell
-> protocol-oteryn transport/runtime adapter
-> admission + GameSession + CharacterLease
-> PostgreSQL migration/persistence substrate
-> Character/FND-04 persistence
-> minimal ChannelRuntime
```

**Do not implement this from this prompt alone.**

If the owner explicitly authorizes server implementation in the current session, first create a dedicated implementation task from live main, declare exact code/DDL paths and evidence, re-read the implementation-relevant accepted contracts and BUILD_TEST_MATRIX, then implement only that bounded package through validation/PR/merge/closeout.

## 7. Current paper-only next gate

Unless superseded by a new explicit owner instruction, the immediate architecture package is:

```text
GAME-ITEM-01 — Item Model and Equipment Rules
```

The bounded GAME-ITEM-01 package must consume the accepted first Reference target where parity applies and define only the semantics needed before `DUR-03` can safely own durable value mutation.

At minimum analyze/decide as evidence allows:

- `ItemType` versus `ItemInstance` identity/lifecycle;
- stackability, quantity, charges, durability, decay/expiration;
- equipment slots and exclusivity/two-handed/requirements rules;
- containers, nesting/cycle/weight/capacity limits;
- transformations, split/merge and provenance continuity;
- binding/ownership/world/account/character restrictions;
- content-definition revision/migration compatibility;
- deterministic derived modifier ordering;
- boundaries with loot/trade/bank/depot/market/mail/rewards/houses without designing their full downstream semantics.

Must preserve:

- `DUR-03` as conservation/single-location/anti-duplication authority;
- no client-authoritative item legality or value;
- no unbounded generic JSON/EAV escape hatch for authoritative item state;
- accepted Reference evidence discipline: `UNKNOWN/CONFLICT` stays fail-closed rather than guessed from OTS implementations.

## 8. Parallel gates

Do not absorb these into GAME-ITEM-01. They may proceed only under separate bounded ownership:

- `GAME-CHANNEL-01`;
- Reference evidence/parity manifest/tooling;
- `DUR-04` minimum headless content schema/validator/compiler/bundle/loader path;
- `SIM-DETERMINISM-01`;
- existing lag/disconnect architecture discussions;
- `NET-TRANSPORT-02` later, when evidence justifies QUIC work.

## 9. Package lifecycle

For every substantial package:

1. verify live main/open PRs/active ownership;
2. create one task from the task template and one dedicated branch;
3. declare owned paths, public contracts, dependencies, exclusions and execution budget;
4. open a draft PR early;
5. perform bounded primary-source discovery;
6. write the smallest complete architecture/contract package;
7. update current programme/register/horizon only when their truth changes;
8. inspect the full diff and complete focused validation;
9. perform mandatory exact-head self-review;
10. perform a genuinely independent review only when required by risk policy/owner/contract;
11. run exact-head required CI;
12. require zero unresolved review threads and clean ownership;
13. squash-merge only when every gate passes;
14. archive the task and release ownership;
15. leave exactly one executable programme `next_action`.

Do not stop at a plan, branch, commit, PR creation or partial CI state when the bounded package can safely be completed in the current invocation.

## 10. Evidence and product discipline

- Use `PROVEN`, `DERIVED`, `UNKNOWN`, `CONFLICT` truthfully.
- Do not infer Reference behavior from Canary/crystalserver/another OTS merely because code exists there.
- Later Global Tibia changes do not silently mutate the accepted first Reference target.
- Architecture acceptance does not equal implementation, runtime availability or production readiness.
- A passing compile does not prove protocol/session/persistence/gameplay behavior.
- Client sends intent; server owns authoritative legality/order/results.
- One GameNode remains one game-server process unless an explicit later ADR supersedes ADR-0009.
- `protocol-canary` remains reference-only and prohibited from production runtime/negotiation/fallback/translation.

## 11. Concurrent work

At the handoff base, PR #191 (GAME-CHAR factual provenance correction) and PR #162 (CI/governance aggregate merge-gate work) were open and unrelated. Long-lived lag/disconnect architecture checkpoints were also present.

Always verify their live state. Do not mutate another task/PR merely because it is stale-looking; coordinate only on actual ownership/dependency overlap.

## 12. Terminal handoff rule

When rotating to another agent/session:

- persist the current task checkpoint and one exact `next_action`;
- update `docs/agents/reports/OTV2-20260812-foundation-handover.md` or create a successor dated handoff when material state has changed;
- record exact branch/head/PR/CI/review evidence;
- use terminal result `ROTATE`;
- never require the next agent to read the previous chat.
