# FND-04B-AMEND-1 — Game fast-reconnect proof delivery

- Decision: `FND-04B-AMEND-1`
- Status: **ACCEPTED** (owner, 2026-11-05)
- Owner: FND-04B, with protocol and native-client owners
- Scope: Game-issued fast reconnect and its acceptance-message delivery only
- `IMPLEMENTATION_AUTHORITY: GAME-SIDE` (owner, 2026-11-11; see Owner decisions);
  `PRODUCTION_AUTHORITY: NONE`

## Precedence and gap

On owner acceptance, this amends only proof representation/delivery in
[FND-04B §§9, 11, 15](../architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md)
and the two acceptance messages in [foundation.proto](protocol-oteryn/v1/foundation.proto).
All other eligibility, fencing, grace, protection and recovery rules remain binding.
RECOVERY-PROOF-1 found no admission delivery field despite §9 requiring issuance.
`ClientResume.reconnect_material = 2` already supplies the return path.
The signed-token verification in `crates/gameplay/dispatch/src/gameplay_transport/resume.rs`
is Platform recovery verification, not evidence of Game fast-proof support.
Platform retains sole issuance of `oteryn-reauth-recovery-v1` (FND-04B §17);
this proposal neither delegates that issuance to Game nor reuses its trust keys.

## Proposed proof profile

`oteryn-game-reconnect-v1` is an opaque-to-client bearer credential issued by the
authoritative Game owner after admission/control commit. Use JWS Compact JWT,
UTF-8 bytes, protected header exactly `alg=Ed25519`,
`typ=oteryn-game-reconnect+jwt`, and bounded `kid` selecting only configured
Game reconnect keys. Reject duplicate JSON members, unsupported headers,
algorithms, key types and token-selected key URLs; no algorithm negotiation.
Private signing keys remain Game-only; replacement owners obtain verification
trust through authenticated Game configuration, never through client input.

Claims are exactly `iss=urn:oteryn:game:reconnect`,
`aud=urn:oteryn:game:reconnect`, `profile=oteryn-game-reconnect-v1`,
`purpose=same_session_reconnect`, canonical UUID strings `game_session_id`,
`character_id`, `world_id`, positive uint64 decimal-string `connection_generation`,
`jti` (32 cryptographically random bytes encoded base64url without padding),
and integer UTC NumericDates `iat`, `nbf`, `exp`. No placement or account authority
is conferred. Proposed hard limits: 2,048 encoded bytes; `nbf=iat`,
`exp=iat+86400` seconds, no positive expiry leeway. A clock whose uncertainty
cannot prove `nbf <= now < exp` fails closed. No in-place renewal: expiry during
long play leaves current control intact and requires Platform recovery on loss.

Game verifies signature, fixed profile/purpose/issuer/audience, time, exact
session/character/world/generation binding and current proof identifier at both
PREPARE and COMMIT. A valid signature alone grants no authority. Mutable key
revocation, runtime-owner fence, session status, healthy binding and original
grace must still pass at COMMIT. Unknown/revoked keys fail closed; key retirement
must preserve verification for unexpired current proofs or explicitly revoke
them. Restart must reconstruct current/candidate proof and attempt disposition
from fenced evidence before accepting any proof. Never try Platform recovery
verification after a failed Game-profile verification, or vice versa.

## Proposed wire delta (illustrative only; not a schema edit)

```proto
// In ServerAccepted:
optional bytes reconnect_material = 11;
reserved 12 to 20; // replaces reserved 11 to 20

// In ServerResumeAccepted:
optional bytes reconnect_material = 7;
reserved 8 to 16; // replaces reserved 7 to 16
```

Both fields carry only the committed current Game proof and match the message's
GameSessionId and connection_generation. Emit only after the authoritative
boundary, over its exact authenticated TLS transport, before gameplay delivery.
Fresh admission and post-grace new-session acceptance use `ServerAccepted`;
same-session fast reconnect or Platform recovery uses `ServerResumeAccepted`.
Present-empty or over-limit fields are invalid; absence means no usable fast
proof, not refusal of otherwise valid admission. Old clients ignore the unknown
field and retain Platform recovery; new clients accept old-server omission and
take that same recovery path. No major-version bump, capability requirement or
schema-revision equality check is introduced. These tags are proposed releases
from reserved ranges, requiring protocol-owner confirmation before schema work.

## Rotation, lost responses and retention

Acceptance delivery does not replace FND-04B §11's safe successor delivery.
PREPARE may pre-sign one inactive successor for the exact attempt, successor
generation and authenticated candidate transport. Its `iat`/expiry are fixed at
preparation and never extended by retries. COMMIT atomically activates that
proof, fences its predecessor and records the attempt result; the acceptance
field repeats the identical successor bytes. Abort/expiry/supersession makes
the candidate permanently unusable. No proof is activated by delivery alone.

PREPARE response loss returns the same candidate only to the proven candidate
transport. COMMIT response loss reconciles the same attempt on that transport
or an equivalently proven current-generation channel (§15); predecessor proof
never authenticates a new channel or retrieves a successor after commit.
If transport and successor are both lost, require Platform recovery. Initial
acceptance loss likewise requires recovery if the client has no current proof.
Acceptance fields alone cannot establish this PREPARE exchange: fast reconnect
implementation remains blocked until its concrete wire mapping and candidate
transport proof satisfy §11/§15 under independent review.

Native clients keep at most one current and one inactive candidate in process
memory, associated with session/generation/attempt. Promote on proven commit;
erase predecessor and aborted candidates. Clear all on logout, terminality,
expiry or process exit. Never persist to disk, browser storage, keychain,
telemetry, logs or crash reports; redact credential-bearing buffers.

Proposed retry ceiling: one in-flight candidate per session/client, at most
three transmissions per phase including the first, with 250ms then 500ms
backoff. Stop at the earliest proof expiry, prepared deadline or original grace
deadline; retries never reset them. Ambiguous COMMIT permits only same-attempt
reconciliation, never a fresh competing attempt. Exhaustion ends automatic
retries and offers reauthenticated recovery. Server-side candidate budgets and
numeric prepared/grace deadlines still require the existing registry evidence.

## Refusals and required evidence

Use existing FND-04B §25 codes; this proposal allocates no numeric wire codes.

| Condition | Refusal / effect |
|---|---|
| Malformed, wrong profile/signature/key/binding, expired or predecessor proof | `RECONNECT_PROOF_INVALID`; no authority mutation; recover |
| Healthy current controller | `RECONNECT_HEALTHY_BINDING_PRESENT`; incumbent unchanged |
| Ineligible/terminal session; grace elapsed | `RECONNECT_NOT_ELIGIBLE` / `RECONNECT_GRACE_EXPIRED`; no revival |
| Candidate deadline elapsed; candidate superseded | `RECONNECT_PREPARED_EXPIRED` / `RECONNECT_PREPARED_STALE`; no activation |
| Currentness or commit outcome cannot be proven | `RECONNECT_RECONCILIATION_UNAVAILABLE`; bounded same-attempt reconciliation only |

Public diagnostics remain §24/§25 redacted classes, without cryptographic or
session-existence oracles. Unavailable trust evidence is unavailable, never an
allow. Existing FND-02 transport/size refusals apply before credential parsing.

| Test matrix (future implementation acceptance) | Required result |
|---|---|
| Fresh admission, Platform recovery, post-grace new session | Correct acceptance field, current proof and authoritative boundary |
| Old client/new server; new client/old server; absent/empty/oversize | Unknown ignored; absence recovers; malformed field fails safely |
| Valid crypto; tamper; none/EdDSA; duplicate/unknown members; Platform substitution | Only exact Game profile verifies; no fallback |
| `nbf`, `exp-1`, `exp`; uncertain clock; unknown/revoked/retired key | Exact time boundaries and fail-closed trust; active control unaffected |
| Wrong session/character/world/generation; stale owner; healthy incumbent | Refusal without fencing incumbent or changing loss/protection |
| Two contenders; PREPARE retry/loss; abort/expiry | One winner; identical candidate; inactive/aborted proof unusable |
| COMMIT response loss; transport+proof loss; predecessor replay | Same-attempt result on proven channel; otherwise recovery; no revival |
| Key revoke, grace expiry or owner change between phases; restart ambiguity | COMMIT revalidation rejects or fails closed without guessed state |
| Retry ceiling/backoff/deadlines; memory/logout/crash/log inspection | Bounded work, unchanged deadlines, no persisted/exported secrets |
| Platform account ban, kick or password change while a proof is current | Game session terminal; current and candidate proofs refused (`RECONNECT_NOT_ELIGIBLE`) |

## Owner decisions (2026-11-11)

The owner accepted decisions 1-4 as proposed and added decision 5:

1. Accepted: Game signing ownership, the profile and claims above, Game trust
   distribution and revocation through authenticated Game configuration, the
   2,048-byte bound and the 24-hour non-renewing lifetime.
2. Accepted: release of reserved tags 11 (`ServerAccepted`) and 7
   (`ServerResumeAccepted`) as optional fields, subject to protocol-owner
   confirmation at schema work. The concrete PREPARE/successor delivery mapping
   stays a separate implementation gate under independent review (§11/§15).
3. Accepted: memory-only client custody and the three-transmission retry
   ceiling with 250 ms then 500 ms backoff. Remaining server resource and
   deadline values are registered before the code that uses them.
4. Accepted: the existing FND-04B §25 refusal mapping and the test matrix above,
   with independent security, protocol and native-client review before
   implementation acceptance.
5. Added: a Platform account ban, kick or password change must make the Game
   session terminal, so that no current or candidate proof can be used after
   it. Implementation must show that path end to end (matrix row above); a
   session-status check alone is not enough if the Platform event never
   reaches Game.

Implementation authority covers Game-side work only. Production authority stays
`NONE` and needs a separate owner decision. Platform responsibilities are
unchanged.
