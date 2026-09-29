# WP3 rustls session-retrieval owner amendment

Coordinator: #162. Programme: #364. Existing sole material worker: #351 / #356. Prior protected rustls ownership allocations: #424, #425, #427, #429. SQLx owner propagation: #430.

## State

```yaml
allocation_id: OTV2-WP3-RUSTLS-SESSION-RETRIEVE-OWNER-20260908
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: 78397d42d082da8abdc47f378e16b05949ec66c1
allocation_state: NOT_ACTIVE
preparation_branch: coord/wp3-rustls-session-retrieve-351
worker_branch: agent/sqlx-driver-budget-351
source_wp3_head: 125923901883e8de90e5b9898a533b56704f72c4
source_wp3_tree: eea23a7068b1b8cc0ba5ce3b91127f159ade21d8
risk: HIGH
scope_result: MULTI_PATH_REQUIRED_REPAIRED_PENDING_REREVIEW
```

This is an allocation-only amendment. It grants no present rustls, SQLx, PostgreSQL, Game, runtime or production mutation authority and creates no replacement worker. The canonical #351/#356 worker remains stopped at its published `SHARED_LEASE_REQUIRED` boundary until this amendment is independently reviewed on its repaired exact head, canonical exact-head checks pass, it integrates through the normal FULL Merge Queue, protected `main` is read back, Work refreshes custody/overlap, and Work explicitly applies the protected amendment to that SAME worker.

## Current canonical blocker

The sole WP3 writer published exact head `125923901883e8de90e5b9898a533b56704f72c4` / tree `eea23a7068b1b8cc0ba5ce3b91127f159ade21d8` with the truthful state:

```text
BLOCKED_PENDING_SHARED_LEASE

SHARED_LEASE_REQUIRED = vendor/rustls-0.23.43/src/client/hs.rs :: ClientSessionValue::retrieve :: retained session-cache custody executes before the protected ClientHello emit/decode continuation
```

The four preceding review findings on operation-owner propagation are recorded fixed at that head. Complete TLS, TLS-positive evidence, configured PostgreSQL 17.6 qualification, final whole-diff review and WP3 integration remain open.

## Independent review correction

The first prospective candidate `b540647281773e41df5e128aa3fa78a366592817` deliberately asked independent high-risk review to decide whether a symbol-only lease was sufficient. Review returned:

```text
CHANGES_REQUIRED
P0=0 / P1=1 / P2=0
SCOPE_RESULT = MULTI_PATH_REQUIRED
```

The finding is correct. The public `ClientSessionStore::tls12_session` dispatch reaches `ClientSessionMemoryCache::tls12_session`, whose `.cloned()` executes inside `client/handy.rs`. Therefore the destination `Tls12ClientSessionValue` backing is allocated before `ClientSessionValue::retrieve` receives the returned value. A helper confined to `client/hs.rs` cannot pass the caller-supplied operation owner to that clone or deny before allocation.

This repaired amendment adds only the two exact pre-clone dispatch surfaces required by that finding. It does not convert the session store or cache into a general resource-accounting owner.

## Protected authority already available

This amendment is intentionally additive to, not a replacement for, the existing protected rustls/SQLx allocations.

### #425 decoded/retained-state owner

Protected #425 already grants retained-session construction/clone/final-drop custody in:

```text
vendor/rustls-0.23.43/src/msgs/persist.rs
```

including `ClientSessionCommon::new`, `Tls12ClientSessionValue::{new,clone}` and `Tls13ClientSessionValue::{new,clone}`. It also records that session stores may retain an already charged opaque session value without learning its private allocation internals.

This amendment grants no new `msgs/persist.rs` semantic surface beyond that already-protected #425 lease. The repaired TLS1.2 retrieval path must call/reuse the #425 owner-aware retained-session clone/custody primitive; it must not copy private retained-allocation formulas or constants into the cache or dispatch layer.

### #429 ClientHello preconstruction owner

Protected #429 installs the same accepted owner before owner-aware ClientHello/ALPN construction. It explicitly left `ClientSessionValue::retrieve`, key exchange and ECH/configuration as separate open cells. The canonical worker correctly stopped instead of inferring blanket pre-handshake authority.

### #430 SQLx operation-owner propagation

Protected #430 carries one caller-supplied operation `ResourceBudget` identity through the dedicated PostgreSQL/SQLx owner-aware connection path into rustls. Ordinary connect/pool/TLS paths remain owner-free. This amendment must use that same identity and must not mint another owner model.

## Exact current-source evidence

Evidence is bound to the canonical WP3 source head `125923901883e8de90e5b9898a533b56704f72c4`.

### `client/hs.rs`

Exact blob:

```text
vendor/rustls-0.23.43/src/client/hs.rs
blob 1e0b544661b4a5aaf3bdaae509b06757dbd8f38b
```

`ClientHelloInput::new` calls `ClientSessionValue::retrieve(...)` before the owner-aware ALPN/ClientHello continuation. `retrieve` performs, in order:

1. TLS 1.3 `ClientSessionStore::take_tls13_ticket(server_name)`;
2. otherwise TLS 1.2 `ClientSessionStore::tls12_session(server_name)` when enabled;
3. configuration-compatibility filtering;
4. `current_time` / `persist::Retrieved::new` / expiry filtering;
5. only for QUIC, a `quic_params()` copy into `cx.common.quic.params`.

The current `ClientHelloInput::new_with_resource_owner` reaches ordinary `Self::new(...)` after its ALPN reservation. It therefore does not make the accepted owner available to this earlier retrieval boundary.

### `client/client_conn.rs`

Exact blob:

```text
vendor/rustls-0.23.43/src/client/client_conn.rs
blob 207efd69d457a2105509417295f5f4a3920a1b7b
```

The public `ClientSessionStore` trait dispatches TLS1.2 retrieval through:

```text
ClientSessionStore::tls12_session(&ServerName) -> Option<Tls12ClientSessionValue>
```

The existing method has no operation-owner argument and must remain source/API/semantic compatible for ordinary callers. The owner-aware SQLx path needs a distinct additive dispatch surface, with a fail-closed default for stores that cannot perform an owner-aware pre-clone operation. The repaired amendment does not authorize changing ordinary `tls12_session` semantics or making operation ownership ambient/global.

### `client/handy.rs`

Exact blob:

```text
vendor/rustls-0.23.43/src/client/handy.rs
blob 3ad3073bbd7d93623755080a5792d85dc7b0e3cd
```

PROVEN current-source facts for `ClientSessionMemoryCache`:

- TLS 1.3 `take_tls13_ticket` uses `VecDeque::pop_back()`: it removes/moves an existing session value; it does not deep-copy the session value at retrieval.
- TLS 1.2 `tls12_session` uses `sd.tls12.as_ref().cloned()`: the destination session value is allocated before control returns to `ClientSessionValue::retrieve`.
- therefore an owner-aware TLS1.2 dispatch must enter this exact cache method neighborhood before `.cloned()` and invoke the already-protected #425 owner-aware clone/custody primitive.
- no evidence justifies general `LimitedCache`, insertion, eviction, TLS1.3 ticket insertion/removal, kx-hint or server-cache mutation authority.

### `msgs/persist.rs`

Exact blob:

```text
vendor/rustls-0.23.43/src/msgs/persist.rs
blob 9b8f19e6a1947bf24db827b856c3a2da472e2889
```

PROVEN current-source facts:

- the TLS1.2 destination backing is created by the retained-session clone operation whose allocation/overlap/final-drop mechanics are already an authored surface of protected #425;
- `Retrieved::new`, `Retrieved::has_expired`, `compatible_config` and age calculations do not themselves create a new retained content backing;
- `Tls13ClientSessionValue::quic_params()` clones its `Vec<u8>` backing. That QUIC-only branch remains explicitly excluded from this SQLx/TCP amendment.

## Scope

After later protected Work application, the SAME #351/#356 worker receives only these exact semantic surfaces:

```text
vendor/rustls-0.23.43/src/client/hs.rs
  :: ClientSessionValue::retrieve
  :: minimum inseparable owner-aware retrieval sibling/wiring

vendor/rustls-0.23.43/src/client/client_conn.rs
  :: ClientSessionStore TLS1.2 retrieval dispatch only
  :: minimum additive owner-aware dispatch sibling/default needed before clone

vendor/rustls-0.23.43/src/client/handy.rs
  :: ClientSessionMemoryCache TLS1.2 retrieval only
  :: minimum owner-aware pre-clone sibling/dispatch needed before `.cloned()`
```

Authority is limited to the minimum implementation necessary to:

- pass the same accepted operation owner already propagated by #430/#429 from owner-aware `ClientHelloInput` construction into session retrieval before any TLS1.2 destination clone;
- preserve the existing public/ordinary `ClientSessionStore::tls12_session` method and all owner-free callers unchanged;
- add only a source-compatible owner-aware TLS1.2 dispatch surface as required for the dedicated SQLx path; any default implementation for stores that cannot honor pre-clone ownership must fail closed rather than silently call the ordinary allocating method;
- make `ClientSessionMemoryCache` perform the owner-aware TLS1.2 clone through the existing #425 retained-session clone/custody mechanism before `.cloned()`-equivalent destination backing appears;
- treat TLS1.3 `take_tls13_ticket` store removal as transfer/move of already-charged opaque session custody, not a newly allocated session; no `handy.rs` TLS1.3 method change is granted unless inseparable dispatch typing requires a non-semantic companion and independent review accepts it;
- carry returned charged session custody into `ClientHelloInput` and the already-protected ClientHello/decoded-state continuation;
- release rejected/incompatible/expired retrieved custody only after corresponding backing is actually destroyed;
- preserve exact TLS1.2/TLS1.3 resumption decisions, expiry, verifier/client-credential compatibility and session-store behavior;
- keep the SQLx owner-aware path terminal on owner denial, with no fallback to ordinary unowned retrieval.

This is not blanket authority for `client/hs.rs`, `client/client_conn.rs` or `client/handy.rs`.

## Explicitly excluded

No new authority is granted for:

- `vendor/rustls-0.23.43/src/limited_cache.rs`;
- `ClientSessionMemoryCache::{set_tls12_session,remove_tls12_session,insert_tls13_ticket,take_tls13_ticket,set_kx_hint,kx_hint}` semantic changes;
- general `ClientSessionStore` insertion/removal/kx/TLS1.3 semantics;
- any new `vendor/rustls-0.23.43/src/msgs/persist.rs` surface beyond the already-protected #425 retained-session construction/clone/final-drop lease;
- QUIC session-parameter cloning or QUIC cache semantics;
- `tls13::initial_key_share`, `SupportedKxGroup::start`, key-exchange provider allocation or any crypto-provider path;
- ECH state/configuration or ECH provider allocation;
- client/server certificate verifier internals or configuration builders;
- server handshake paths;
- breaking ordinary public cache/store API changes;
- any new SQLx/PostgreSQL path, PostgreSQL decoder work, Game/Foundation/Durability runtime, migration, registry, workflow, Cargo/workspace or external repository path;
- numeric-resource policy changes, TLS downgrade, cache disabling, session-resumption disabling, semantic shrink or a new magic whole-handshake reservation;
- production/live data, deployment, credentials, ruleset/protection or Merge Queue changes.

If implementation proves that correct reservation/custody requires `limited_cache.rs`, a different cache/store implementation path, QUIC `quic_params`, another rustls file/symbol or any other unlisted path, STOP before mutation and return:

```text
SHARED_LEASE_REQUIRED = <exact path> :: <exact symbol> :: <reason>
```

Do not infer that this amendment grants it.

## Required material semantics after protected application

The sole worker must use the SAME accepted operation owner/ledger throughout. For this exact boundary it must prove:

1. TLS1.3 retrieval removes/moves an already-charged session without double charge, early release or a second session allocation.
2. TLS1.2 owner-aware retrieval reaches the cache before destination clone allocation and reserves/charges through the already-authorized #425 clone path before that allocation.
3. Source and destination retained-session charges overlap truthfully while both backings exist; no logical handoff releases source custody early.
4. An unfunded owner denies before any TLS1.2 deep-copy allocation and cannot fall back to ordinary unowned `tls12_session`.
5. A custom/external store that lacks owner-aware pre-clone support fails closed for the dedicated owner-aware route without changing ordinary store behavior.
6. Incompatible or expired retrieved sessions unwind custody exactly once after backing destruction.
7. Successful retrieval transfers custody into `ClientHelloInput` and later protected state without a logical-handoff release while backing survives.
8. Owner-free ordinary constructors/retrieval/cache behavior and existing public `tls12_session` semantics remain source/API compatible.
9. The PostgreSQL/SQLx owner-aware path is TCP; this amendment must not claim or exercise the QUIC `quic_params()` allocation. If the owner-aware test reaches QUIC, stop for a separate exact lease.
10. No global/thread-local owner, hidden registry, copied private allocation constant, post-allocation catch-up charge, TLS downgrade, cache disablement or session-resumption disablement.

## Focused RED/GREEN qualification

Before claiming this session/cache cell closed, the SAME worker must add focused proof only within already-authorized or newly exact-scoped rustls test surfaces:

- RED then GREEN: owner-aware TLS1.3 cached ticket retrieval preserves the same charged custody across store removal and `ClientHelloInput` transfer, then releases exactly once on final drop;
- RED then GREEN: owner-aware TLS1.2 cached session retrieval reaches the owner-aware dispatch and denies before the #425 destination clone when unfunded;
- successful TLS1.2 clone retains source/destination overlap and releases correctly on later destruction;
- an owner-aware request through a store without owner-aware clone support fails closed before ordinary allocating retrieval;
- incompatible verifier/client-credential configuration does not leak or double-release retrieved custody;
- expired session does not leak or double-release custody;
- ordinary owner-free TLS1.2/TLS1.3 cache retrieval tests remain unchanged and pass;
- SQLx owner-aware TCP construction has no ordinary unowned fallback on session-owner denial;
- exact owner identity continues into the #429 ClientHello/ALPN and #425 decoded-state paths;
- explicit control proves this WP3 path is non-QUIC and this amendment does not claim QUIC resource accounting.

Run affected rustls tests, existing SQLx owner-propagation/resource tests, strict Rust 1.94 checks, provenance/delta verification and applicable repository governance before another readiness claim.

## Remaining open cells after this amendment

Even if this session-retrieval cell becomes GREEN, WP3 is not complete. The canonical worker must continue only under existing or later exact authority and stop again at the next unallocated allocation boundary. In particular these remain separate until proven/allocated:

- `tls13::initial_key_share -> SupportedKxGroup::start` and active key-exchange allocation;
- ECH/configuration/crypto allocation where reachable;
- complete TLS phase/lifetime composition;
- actual TLS-positive qualification;
- configured PostgreSQL 17.6 driver qualification;
- final adversarial whole-diff review;
- canonical exact-head CI, normal FULL Merge Queue and protected-main readback.

WP4 #335 remains frozen until terminal protected WP3 delivery and fresh Work custody release.

## Validation

This allocation PR itself changes one documentation path only. Runtime gameplay/TLS/PostgreSQL E2E is `NOT_APPLICABLE` because this document does not mutate executable/runtime code or activate the material lease.

Required before material application:

1. whole-file/diff coordinator self-review on the repaired exact candidate;
2. fresh independent exact-head high-risk re-review proving the repaired three-surface lease is sufficient and no broader cache/store/QUIC authority is required;
3. canonical exact-head repository checks selected for agent/governance documentation;
4. zero unresolved review threads on the final head;
5. normal FULL Merge Queue integration;
6. protected-main readback;
7. fresh Work branch/PR/path custody check;
8. explicit application to the SAME #351/#356 worker.

The superseded symbol-only review finding is historical evidence and must not be treated as PASS for this repaired head.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`.
