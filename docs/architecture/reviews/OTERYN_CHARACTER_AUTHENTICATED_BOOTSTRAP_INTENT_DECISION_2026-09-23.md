# CHARACTER_AUTHENTICATED_BOOTSTRAP_INTENT_V1

Status: architecture decision candidate for Issue #791. No implementation authority follows until this document is reviewed, protected-integrated and read back from `main`.

## Scope

This decision closes only the missing authenticated intent needed by the already-bounded WP5 Character Authority first slice. It does not enable ordinary player-facing Character creation, naming policy, quota policy, Character transfer, World transfer, rename, retirement, Platform writes or a second Character system.

Parent control plane: #162  
Source-readiness programme: #319  
Blocked implementation: #790 / WP5 material #414  
Related accepted authority: `CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`, FND-04, DUR-02, ANL-01, protected S1/S2 and #415.

## Decision

The first G0-serving Character creation capability is an **operator/control-plane initial bootstrap**, not the general player-facing Character-create product flow.

Game Operations / the Game control plane owns issuance of one typed, single-use `CharacterBootstrapAuthorizationV1`. The authorization permits Character Authority to establish one exact initial Character binding only after all independent current prerequisites are revalidated. It does not make the control plane the Character owner: Game Character Authority still chooses and persists the canonical `CharacterId`, current `AccountId` owner binding, `WorldId`, lifecycle and positive `CharacterRevision`.

The later user-facing Platform Character-create flow remains disabled until a separately accepted intent/transport contract proves its own authentication, naming/quota/starter-policy and product semantics. G0 does not need that broader flow.

## Authority separation

A bootstrap authorization is distinct from all of the following and none of them may substitute for it:

- `NodeIncarnationProof` from #415 — proves the current Game process incarnation only;
- S1/S2 account-security evidence — proves current Platform account-security state only;
- a caller-filled `AccountId` or `WorldId`;
- an account portfolio guard row;
- a prior Character receipt, audit event, cache or projection;
- an FND-04 admission/recovery grant.

Platform remains the native `AccountId` identity/security owner. For the exact bootstrap `AccountId`, Character Authority must also consume independently authenticated current S1/S2 Platform account-security evidence under the accepted native-source non-rollback/freshness rules. An allowed security observation is a required prerequisite but never the creation authorization itself.

## Authorization contract

`CharacterBootstrapAuthorizationV1` binds exactly one semantic bootstrap operation:

- stable semantic `OperationId` / operation identity;
- Platform-owned `AccountId`;
- target `WorldId`;
- exact profile, ruleset, content and starter-template interpretation revisions already required by the #414 first slice;
- authorization class `OPERATOR_INITIAL_BOOTSTRAP_V1`;
- one opaque high-entropy authorization secret/capability, with only an authenticated digest retained by Game;
- the Game control-plane issuer identity/version expected by configuration;
- issuance provenance sufficient to distinguish exact replay from a different authorization decision.

No client-provided initial stats, name policy, quota value or product entitlement becomes authority through this object.

The Game control plane writes the authorization through a dedicated Character-bootstrap authorization operation. The record is immutable except for one terminal consumption transition. The raw secret is never persisted or logged.

## Consumption and atomicity

Before first commit, Character Authority must prove all of the following in the authoritative operation:

1. the caller presents the exact unconsumed bootstrap capability and exact bound operation;
2. the current Game process proves its #415 current-incarnation authority independently;
3. current authenticated S1/S2 Platform security evidence matches the exact `AccountId`, remains allowed and satisfies its existing source revision/freshness/non-rollback rules;
4. the Character restore/non-rollback fence selected by `CHARACTER_RESTORE_NONROLLBACK_FENCE_V1` is current and permits authoritative mutation;
5. the bound World/context revisions match the exact requested bootstrap;
6. account-portfolio serialization and the accepted Character transaction locks are held.

Authorization consumption, fresh `CharacterId` allocation, Character root/current state, positive `CharacterRevision`, immutable operation receipt, registered durable-audit event bytes and pending outbox state commit in one PostgreSQL transaction or none of them commit.

## Replay and ambiguity

The semantic operation identity is stable across retry.

- exact retry after a lost/ambiguous response reconciles the original receipt and returns the same Character, revision, EventIds, transaction identity and payload bytes;
- the same authorization/operation with any changed AccountId, WorldId or interpretation binding is a conflict;
- a consumed authorization cannot create a second Character;
- a different secret/capability cannot claim an existing operation;
- failure before commit leaves no Character and does not manufacture successful consumption;
- uncertain commit outcome is reconciled before a new bootstrap attempt is allowed.

No wall-clock duration is introduced by this decision. Currentness of Platform security evidence remains governed by the already accepted FND-04/S1/S2 timing contract; bootstrap authorization replay safety is single-use + exact-operation based. Restore rollback safety is governed separately by the Character recovery fence.

## Minimum Game implementation handoff

After protected integration, Work may amend the SAME #790 lineage only as narrowly as required to:

- add the typed bootstrap authorization/consumption state to the existing Character migration and owner module;
- bind `bootstrap_character` to a sealed bootstrap authorization rather than caller authority;
- consume independently current S1/S2 account-security evidence for the exact AccountId through the existing source/publication composition path;
- consume the current Character recovery-fence proof;
- extend the existing `character_authority_postgres` qualification.

Prefer the existing #415 one-launch authorization implementation as a mechanical pattern only. Do not reuse its types, secrets, tables or authority meaning.

If a compiler/API proof shows an additional existing shared path is necessary, Work must grant that path explicitly. This decision itself grants no path lease.

## Qualification matrix

At minimum, real PostgreSQL 17.6 and the final composition must prove independent cases for:

- valid operator bootstrap with current process proof, current allowed Platform account evidence and current recovery fence;
- missing, malformed, unknown, already-consumed or wrong bootstrap capability;
- exact replay versus changed-binding replay;
- wrong AccountId, WorldId or interpretation revision;
- Platform account-security denial, stale/lower source revision, equal-revision changed decision and unavailable source;
- current-process proof missing/revoked/replaced independently of bootstrap capability;
- recovery-fence missing/regressed/restore-blocked independently of bootstrap capability;
- two concurrent executions of one operation produce one semantic Character/result;
- account portfolio serialization under independent bootstrap operations without inventing a quota rule;
- injected Character/audit/outbox failure rolls back authorization consumption and domain mutation atomically where the commit did not become durable;
- lost commit response reconciles the same receipt/EventIds/payload;
- restart reloads the consumed authorization and receipt without re-authorizing it;
- logs/public projections do not expose the raw bootstrap secret or private AccountId-to-alternate-Character relation.

## Explicitly deferred

This decision does not enable the normal player-facing `CreateCharacter` flow. That later flow must have a separately authenticated Platform intent boundary and product-specific naming/quota/starter semantics. Deferral is intentional under the protected playable-first/minimum-sufficient policy and does not weaken the current G0 authority floor.
