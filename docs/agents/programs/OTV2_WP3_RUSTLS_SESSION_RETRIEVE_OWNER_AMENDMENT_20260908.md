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
prospective_scope_result: SYMBOL_ONLY_CANDIDATE_PENDING_INDEPENDENT_REVIEW
```

This is an allocation-only amendment. It grants no present rustls, SQLx, PostgreSQL, Game, runtime or production mutation authority and creates no replacement worker. The canonical #351/#356 worker remains stopped at its published `SHARED_LEASE_REQUIRED` boundary until this amendment is independently reviewed, canonical exact-head checks pass, it integrates through the normal FULL Merge Queue, protected `main` is read back, Work refreshes custody/overlap, and Work explicitly applies the protected amendment to that SAME worker.

## Current canonical blocker

The sole WP3 writer published exact head `125923901883e8de90e5b9898a533b56704f72c4` / tree `eea23a7068b1b8cc0ba5ce3b91127f159ade21d8` with the truthful state:

```text
BLOCKED_PENDING_SHARED_LEASE

SHARED_LEASE_REQUIRED = vendor/rustls-0.23.43/src/client/hs.rs :: ClientSessionValue::retrieve :: retained session-cache custody executes before the protected ClientHello emit/decode continuation
```

The four preceding review findings on operation-owner propagation are recorded fixed at that head. Complete TLS, TLS-positive evidence, configured PostgreSQL 17.6 qualification, final whole-diff review and WP3 integration remain open.

## Protected authority already available

This amendment is intentionally additive to, not a replacement for, the existing protected rustls/SQLx allocations.

### #425 decoded/retained-state owner

Protected #425 already grants retained-session construction/clone/final-drop custody in:

```text
vendor/rustls-0.23.43/src/msgs/persist.rs
```

including `ClientSessionCommon::new`, `Tls12ClientSessionValue::{new,clone}` and `Tls13ClientSessionValue::{new,clone}`. It also explicitly records that `client/handy.rs` and `limited_cache.rs` may retain an already charged opaque session value without learning its internals.

This amendment therefore grants no new `msgs/persist.rs`, `client/handy.rs` or `limited_cache.rs` authority.

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

### `msgs/persist.rs`

Exact blob:

```text
vendor/rustls-0.23.43/src/msgs/persist.rs
blob 9b8f19e6a1947bf24db827b856c3a2da472e2889
```

PROVEN current-source facts:

- `Tls12ClientSessionValue` is cloned through its session-value clone path; the deep retained-session clone semantics are already an authored surface of protected #425.
- `Retrieved::new`, `Retrieved::has_expired`, `compatible_config` and age calculations do not themselves create a new retained content backing.
- `Tls13ClientSessionValue::quic_params()` does clone its `Vec<u8>` backing. That QUIC-only branch is explicitly excluded from this SQLx/TCP amendment.

### `client/handy.rs`

Exact blob:

```text
vendor/rustls-0.23.43/src/client/handy.rs
blob 3ad3073bbd7d93623755080a5792d85dc7b0e3cd
```

PROVEN current-source facts for `ClientSessionMemoryCache`:

- TLS 1.3 `take_tls13_ticket` uses `VecDeque::pop_back()`: it removes/moves an existing session value; it does not deep-copy the session value at retrieval.
- TLS 1.2 `tls12_session` uses `sd.tls12.as_ref().cloned()`: the new session-value backing is created by the `Tls12ClientSessionValue` clone operation whose ownership semantics are already allocated by #425.
- the store remains an opaque holder of already-charged values; making it learn the operation owner is not justified by this source evidence.

## Scope

After later protected Work application, the SAME #351/#356 writer receives exactly one new semantic surface:

```text
vendor/rustls-0.23.43/src/client/hs.rs :: ClientSessionValue::retrieve
```

Authority is limited to the minimum private owner-aware retrieval sibling/helper or signature/wiring change inside this same symbol neighborhood necessary to:

- accept the same owner identity already propagated by #430/#429 before session retrieval;
- preserve ordinary `ClientSessionValue::retrieve` behavior for existing owner-free callers;
- treat TLS 1.3 store removal as transfer/move of already-charged opaque session custody, not a newly allocated session;
- route TLS 1.2 retrieval through the already-protected #425 owner-aware session clone/custody semantics, denying before the deep clone when the accepted owner cannot fund it;
- carry the returned charged session custody into `ClientHelloInput` and the already-protected ClientHello/decoded-state continuation;
- release rejected/incompatible/expired retrieved custody only after the corresponding backing is actually destroyed;
- preserve the exact TLS 1.2/TLS 1.3 resumption decision, expiry, verifier/client-credential compatibility and session-store behavior;
- keep the SQLx owner-aware path terminal on owner denial, with no fallback to ordinary unowned retrieval.

A minimum adjacent private helper in `client/hs.rs` may be added only when inseparable from `ClientSessionValue::retrieve` and only for the semantics above. This is not blanket authority for `client/hs.rs`.

## Explicitly excluded

No new authority is granted for:

- `vendor/rustls-0.23.43/src/client/handy.rs`;
- `vendor/rustls-0.23.43/src/limited_cache.rs`;
- any new `vendor/rustls-0.23.43/src/msgs/persist.rs` symbol beyond the already-protected #425 lease;
- QUIC session-parameter cloning or QUIC cache semantics;
- `tls13::initial_key_share`, `SupportedKxGroup::start`, key-exchange provider allocation or any crypto-provider path;
- ECH state/configuration or ECH provider allocation;
- client/server certificate verifier internals or configuration builders;
- server handshake paths;
- ordinary rustls cache semantics or public cache APIs;
- any new SQLx/PostgreSQL path, PostgreSQL decoder work, Game/Foundation/Durability runtime, migration, registry, workflow, Cargo/workspace or external repository path;
- numeric-resource policy changes, TLS downgrade, cache disabling, session-resumption disabling, semantic shrink or a new magic whole-handshake reservation;
- production/live data, deployment, credentials, ruleset/protection or Merge Queue changes.

If implementation proves that correct reservation/custody requires `client/handy.rs`, `limited_cache.rs`, QUIC `quic_params`, another store implementation, another rustls file/symbol or any other unlisted path, STOP before mutation and return:

```text
SHARED_LEASE_REQUIRED = <exact path> :: <exact symbol> :: <reason>
```

Do not infer that this amendment grants it.

## Required material semantics after protected application

The sole worker must use the SAME accepted operation owner/ledger throughout. For this exact boundary it must prove:

1. TLS 1.3 retrieval removes/moves an already-charged session without double charge, early release or a second session allocation.
2. TLS 1.2 retrieval reserves before the deep session clone through the already-authorized #425 clone path; source and destination charges coexist while both backings exist.
3. An unfunded owner denies before any TLS 1.2 deep-copy allocation and cannot fall back to the ordinary unowned path.
4. Incompatible or expired retrieved sessions unwind custody exactly once after backing destruction.
5. Successful retrieval transfers custody into `ClientHelloInput` and later protected state without a logical-handoff release while backing survives.
6. Owner-free ordinary constructors/retrieval/cache behavior remain source/API/semantic compatible.
7. The PostgreSQL/SQLx owner-aware path is TCP; this amendment must not claim or exercise the QUIC `quic_params()` allocation. If the owner-aware test reaches QUIC, stop for a separate exact lease.
8. No new public API, global/thread-local owner, hidden registry, copied private allocation constant, post-allocation catch-up charge, TLS downgrade or cache disablement.

## Focused RED/GREEN qualification

Before claiming this session/cache cell closed, the SAME worker must add focused proof only within already-authorized rustls test surfaces:

- RED then GREEN: owner-aware TLS 1.3 cached ticket retrieval preserves the same charged custody across store removal and `ClientHelloInput` transfer, then releases exactly once on final drop;
- RED then GREEN: owner-aware TLS 1.2 cached session retrieval denies before the #425 deep clone when unfunded;
- successful TLS 1.2 clone retains source/destination overlap and releases correctly on later destruction;
- incompatible verifier/client-credential configuration does not leak or double-release retrieved custody;
- expired session does not leak or double-release custody;
- ordinary owner-free TLS 1.2/TLS 1.3 cache retrieval tests remain unchanged and pass;
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

1. whole-file/diff coordinator self-review on the exact candidate;
2. independent exact-head high-risk allocation review, including explicit review of whether symbol-only scope is sufficient and whether `handy.rs`/`limited_cache.rs` can remain read-only;
3. canonical exact-head repository checks selected for agent/governance documentation;
4. normal FULL Merge Queue integration;
5. protected-main readback;
6. fresh Work branch/PR/path custody check;
7. explicit application to the SAME #351/#356 worker.

If independent review finds `MULTI_PATH_REQUIRED`, repair or replace this prospective allocation before integration. Do not apply a known-insufficient lease merely because the PR checks are green.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`.
