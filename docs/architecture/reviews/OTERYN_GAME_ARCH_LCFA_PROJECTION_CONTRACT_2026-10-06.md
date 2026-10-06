# GAME-LCFA-PROJECTION-CONTRACT-1 Character projection contract and the Platform LCFA-1 packet

- Decision: `ARCH-LCFA-PROJECTION-CONTRACT-V1`
- Status: **CANDIDATE**. The owner accepts the contract revision after the independent review of
  the frozen head (owner answer **1a** to D821, 2026-10-06). The decision takes effect when this
  PR merges after that acceptance.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the control plane, D821 (#162, 2026-10-06): a Game-owned projection contract that
  lets Platform LCFA-1 (Q17a, Platform contract §5) issue tickets without mode 33a (D171), and the
  Platform implementation packet. The owner granted Platform write for that one PR once the
  contract is accepted.
- Amends: `docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md` to revision 2
  (same contract ID and `contract_version` 1; no member, path, bound or response changes).
- Runtime, migration, deployment, production and protected-World authority: NONE in this PR.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Implementation brief

1. **This PR.** The projection contract becomes acceptance-ready. Revision 2 keeps the revision 1
   wire, which the Game producer already implements (LCFA-1 #1330, LCFA-1b #1389). It adds the
   rules Platform needs to build against: multichannel (§2.2), session-generation fencing (§2.3),
   owner and authority (§2.4), and testing/preproduction rulings for U-LC1..U-LC6 (§10).
2. **PLATFORM-LCFA-1** (§2 below), in Oteryn/Oteryn-Platform after acceptance. This covers the
   ingestion routes, the read model, the D2 list read and the §5.4 issuance check that replaces
   mode 33a when the feed is configured.
3. **GAME-LCFA-ENABLE-1** (§3 below), in this repository. This wires the existing publisher to a
   projection identity in node configuration and adds an operator resync command.
4. In a stack that runs both, mode 33a is switched off (§4). Mode 33a stays the fallback only in
   stacks without a projection feed, and only until the release gate.

## 1. Rulings

| Item | Ruling |
|---|---|
| Field set | revision 1 unchanged: `character_id`, `world_id`, `name`, `availability`; no `ChannelId`, fencing, presence or progression |
| Export shape | revision 1 unchanged: per-account full snapshots plus a liveness watermark, pushed over mTLS to the two compiled Platform paths; exact member sets; `200 accepted/superseded`, empty-bodied `400/401/409/429/503` |
| Multichannel | the projection names the World only; Platform picks the Channel at issuance (§7.4) and binds it from the route record |
| Fencing | the projection reflects committed, fenced writes only, carries no fencing data, and admission revalidates |
| Authority | Game Character Authority is the only producer; Platform holds a read model that is never proof of ownership and has no write path |
| Content | the `characters` value compared canonically (#162 5910360309), now normative |
| Names | wire rule 1..64 bytes as in revision 1; Game emits the CHAR-NAME-1 subset; Platform validates the wire rule |
| U-LC1 | push for testing and preproduction; release open |
| U-LC5 | S = 30 s, clock uncertainty 1 s; watermark every 3 s, gap at most 10 s |
| U-LC6 | one projection identity per Character Authority host, from the stack's development CA, configured by certificate subject; release open (Platform U15) |
| Epoch values | Unix-millisecond scale after an epoch raise (migration 0028); compare as uint64 |

## 2. PLATFORM-LCFA-1 implementation packet (Oteryn/Oteryn-Platform)

- **task_id:** `PLATFORM-LCFA-1` (the control plane allocates it in #162 and in the Platform
  task record).
- **Repository and authority:** Oteryn/Oteryn-Platform, one PR. The owner granted write for this
  one PR once the contract is accepted (D821). Platform merge rules, Platform CI and Platform review
  apply. Testing and preproduction only; all switches default off.
- **Depends on:** acceptance of the projection contract revision 2. It does not depend on
  GAME-LCFA-ENABLE-1, because the Platform tests use contract fixtures.

### 2.1 Owned paths

- `app/GameAuth/AccountCharacters/` (new): the wire parser for both operations, settings,
  ingestion, the read model and the feed liveness check.
- `app/Http/Controllers/GameAuth/NativeAccountCharactersController.php`,
  `NativeAccountCharactersWatermarkController.php`, `NativeCharactersController.php` (new).
- `app/Http/Middleware/GameAuth/GuardNativeAccountCharactersPeer.php` (new).
- `app/GameAuth/NativeLogin/RegistryNativeAdmissionScopeResolver.php` (the §5.4 check).
- `config/game-auth.php` (a new `native_account_characters` section only), `.env.example`.
- `routes/internal.php` (the two ingestion routes), `routes/api.php` (the D2 read).
- `database/migrations/2026_10_*_add_native_account_character_read_model.php` (new, additive).
- `tests/Feature/GameAuth/AccountCharacters/` (new),
  `tests/Feature/GameAuth/NativeLogin/NativeAdmissionIssuerTest.php`,
  `tests/Feature/GameAuth/Concurrency/` (new ingestion race tests only).
- `docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md`: a status line in §5 and §17. It
  changes no rule; a rule change needs a lock update and its own review.
- The Platform task record for PLATFORM-LCFA-1.

### 2.2 Scope

1. **Ingestion** (`POST /internal/v1/game-auth/native-account-characters` and `.../watermark`).
   These follow Platform contract §5.2 and projection §3–§5:
   - The TLS 1.3 client certificate is checked with trusted-terminator provenance, as for
     native evidence.
   - Each route is per purpose: only configured projection identities are accepted, and every
     other purpose's identity is refused with `401`. Every other route refuses a projection
     identity.
   - The parser checks the exact member set, rejects unknown, duplicate and `null` members,
     enforces nesting at most 3, and requires canonical UUIDv7 values, uint64 decimal strings,
     `characters` sorted and unique (0..64), the wire name rule, and the 16384 B and 512 B
     limits enforced before parsing.
   - The response is exactly `{"contract_version":1,"result":"accepted"|"superseded"}`.
     Failures are `400/401/409/429/503` with empty bodies.
2. **Read model** (additive migration). The model has two parts:
   - One row per `AccountId` holding epoch, revision, `source_observed_at`, a canonical content
     digest, the entries and an `invalid` flag.
   - One feed row holding the highest epoch seen, `complete_through` and the watermark epoch.

   Ordering is by `(epoch, revision)` compared as uint64, under a row lock taken in a fixed order:
   feed row, then account row.
   - A lower pair is answered `superseded`.
   - An equal pair with equal content is answered `accepted` idempotently, and the first
     `source_observed_at` is kept.
   - An equal pair with different content is answered `409` and marks the account `invalid`
     until a higher pair arrives.
   - A higher epoch, whether from a snapshot or a watermark, invalidates every entry of a lower
     epoch in the same transaction.
   - A snapshot for an unknown `AccountId` is stored and authorizes nothing.
3. **Feed liveness.** The feed is `live` while both hold:
   - `now - complete_through + clock_uncertainty <= S`, with S 30 s and uncertainty 1 s, both
     configurable;
   - the watermark epoch equals the highest epoch seen.

   Otherwise the feed is `stale`.
4. **D2 read** (`GET /v1/game-auth/native-characters`, contract §5.3). It requires the OAuth
   bearer with scope `game:ticket`, with the same client and generation checks as ticket
   issuance, and does not revoke the token.
   - It returns only the caller's entries: `{"protocol_version":2,"characters":[...]}`.
   - An account with no snapshot gets an empty list.
   - It answers `503` and returns no entry when the feed is `stale`, or when the account entry is
     `invalid` or below the highest epoch seen. This is the projection contract §2 rule that stale
     state fails toward less disclosure. It narrows Platform contract §5.3, which named only
     `invalid`; the Platform PR updates §5.3 to match.
   - The response is `no-store` and rate limited per contract §10.
5. **Issuance check** (§5.4, in `RegistryNativeAdmissionScopeResolver`). The check runs inside the
   existing shared epoch lock.
   - When `native_account_characters.enabled` is on, it requires all of:
     - the feed is live;
     - the account entry exists, is in the highest epoch and is not invalid;
     - `character_id` is listed and `AVAILABLE`;
     - the selected route's World is the entry's `world_id`.
   - A failure returns `NATIVE_LOGIN_CHARACTER_CONFLICT` (not listed, not available) or
     `NATIVE_LOGIN_ROUTE_UNAVAILABLE` (stale feed, invalid or old-epoch account).
   - Precedence: when both the feed switch and mode 33a are on, the feed decides and mode 33a is
     ignored. With the feed switch off, the behaviour is today's: mode 33a, or fail closed.
   - Channel selection (§7.4) is unchanged, and the request `channel_id` only narrows the
     candidate set.
6. **Configuration** (`native_account_characters`):
   - `enabled` (default false);
   - `identities` (a JSON list of projection certificate subjects);
   - `source_authority` (expected namespace);
   - `liveness_seconds` (30), `clock_uncertainty_seconds` (1);
   - `requests_per_minute`.

   Refused outside `testing` and `preproduction`: on in any other environment answers `503` on
   ingestion and `NATIVE_LOGIN_UNAVAILABLE` on issuance.
7. **Privacy:** no log line carries `AccountId`, a name or the list (contract §13).

Out of scope:
- Platform-requested resync (U-LC3);
- progression fields;
- production enablement, deployment, credentials or PKI (U15);
- Gateway changes;
- removing mode 33a (that is the release gate, §4);
- any Game change.

### 2.3 Validation

- `composer format:check`, `composer analyse`, and `php artisan test --filter=AccountCharacters`.
- `php artisan test --testsuite=Feature --filter=NativeAdmissionIssuer`.
- The `game-auth-ticket-concurrency` workflow on MariaDB for the new race tests.
- `composer verify` on the frozen head, and the Platform required checks.
- Tests (projection §9 and Platform §16, projection rows), all mandatory:
  - Exact schema: acceptance, and rejection of an unknown member, a duplicate, `null`, a
    non-canonical UUID, a wrong version, unsorted or duplicate entries, 65 entries, and each
    oversize case.
  - Names: bidi, format, control, `"` and `\` rejected; the 64 x 64-byte snapshot fits.
  - Ordering: lower is superseded; equal and identical is idempotent and keeps the first
    `source_observed_at`; equal with different content gives `409` and an invalid account; a
    higher pair clears it.
  - Epoch: a raise by snapshot or by watermark invalidates lower entries; Unix-millisecond epochs
    are above 2^32 and must compare correctly.
  - Watermark: the feed is stale past S and stale on a lower watermark epoch; issuance refuses
    while stale.
  - Issuance: not listed, `UNAVAILABLE`, World mismatch, invalid account and old epoch all
    refused; mode 33a ignored while the feed switch is on.
  - Identity per purpose, in both directions.
  - D2: own entries only, empty for no snapshot, token not revoked; `503` with no entry when the
    account is invalid, when it is below the highest epoch after an epoch raise, and when the feed
    is stale.
  - No secret, `AccountId` or name in logs.
  - Ingestion races on MariaDB: concurrent equal pairs, and an epoch raise against a snapshot.
  - Wire fixtures byte-equal to the Game producer's encoder output at an exact Game commit.

### 2.4 Review

- An independent review on the frozen head, covering security (identity, privacy) and
  persistence (migration, lock order).
- Owner acceptance of the projection contract must come before the PR opens.

## 3. GAME-LCFA-ENABLE-1 packet (this repository)

- **task_id:** `GAME-LCFA-ENABLE-1`, allocated by the control plane.
- **Worker:** `oteryn-hard-worker`, because it touches authority configuration.
- **Owned paths:**
  - `apps/game-server/src/node/config.rs` and `apps/game-server/src/node/serve.rs`;
  - `apps/game-server/src/native_admission_source/account_characters.rs`, for wiring only;
  - `apps/game-server/src/bin/oteryn-game-ops.rs`;
  - their test files;
  - `tools/qualification/login_local/`, if RUNBOOK-1 has merged; otherwise RUNBOOK-1 takes the
    stack lines.
- **Scope:**
  1. A `[platform.account_characters]` node configuration section with:
     - `client_certificate_file` and `client_key_file`, for a projection identity distinct from
       the runtime-status identity, checked as `ProjectionDescriptor::new` already requires;
     - the Platform base URL, reused;
     - `source_authority`.
     The section is absent by default, and with it absent the publisher does not run. When the
     section is present, the node starts `Publisher` only if it is a Character Authority host,
     that is, its database role can read Character ownership.
  2. `oteryn-game-ops projection resync --raise-epoch=<true|false>` calls
     `game_character_account_projection_resync`. The explicit choice is required, and the command
     prints only the resulting epoch.
  3. The stack generates a projection certificate per run, configures the Platform identity list,
     turns mode 33a off and the feed switch on, and runs a resync before the first login.
- **Validation:**
  - `cargo test --locked -p oteryn-game-server account_characters`;
  - the node config tests;
  - the publisher against a fake Platform sink: accepted, superseded, 409 and 503 paths, the
    watermark cadence, and refusal while unreconciled;
  - `cargo clippy` with the repository flags;
  - the PostgreSQL-backed projection tests in CI.
- **No** new migration, wire change or Platform change.

## 4. Rollout and the mode 33a release gate

1. This contract revision is accepted. Then PLATFORM-LCFA-1 and GAME-LCFA-ENABLE-1 proceed in any
   order; each is inert alone.
2. In a stack that runs both, the feed switch goes on and mode 33a goes off. The joint native-login
   E2E then runs with the full §5.4 check. A stack without the feed may keep mode 33a for
   testing and preproduction only (D171).
3. Mode 33a is never enabled for a release entry. Removing its code is a later Platform change
   after the joint E2E passes with the feed.

Rollback: turn the Platform feed switch off, which falls back to mode 33a or to fail closed.
Removing the Game configuration section stops the publisher. The read model is derived state and
can be dropped and refilled with a resync.

## 5. Mandatory decision test

`docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md`:

1. **Must decide now?** YES, for `testing` and `preproduction` only.
2. **Blocked downstream work.** The joint native-login E2E with the full §5.4 issuance check
   (Platform contract §14 step 6), PLATFORM-LCFA-1, GAME-LCFA-ENABLE-1, and the removal of mode
   33a as a dependency of that E2E.
3. **What becomes harder later.** Platform builds a read model and ingestion routes on the push
   shape and the `(epoch, revision)` order. Moving the release entry to pull would add a
   Game-hosted endpoint and retire those routes.
4. **Evidence to supersede.** A measured delivery lag that does not fit S = 30 s; a lost or
   reordered update that the watermark does not catch; a security finding on the projection
   identity or the read model; a product need for progression fields (U-LC4).
5. **Deliberately not decided.** The release values of U-LC1, U-LC5 and U-LC6, the production
   restore runbook (U-LC2), a Platform-requested resync (U-LC3), progression fields (U-LC4), the
   per-account slot quota and production PKI.

## 6. Non-authorization

This decision authorizes no code, migration, configuration, deployment or Platform change until
the owner accepts the contract revision. After acceptance it authorizes only the two packets
above, each through its own #162 allocation, for `testing` and `preproduction`. Production,
release-entry rulings (U-LC1, U-LC5, U-LC6), PKI and credentials need separate authority.
