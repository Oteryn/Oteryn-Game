# FND-02 terminal-outcome retention resource evidence

Status: NON-PRODUCTION EVIDENCE / BLOCKED_ON_EVIDENCE

- Task: FND02_TERMINAL_OUTCOME_RETENTION_RESOURCE_EVIDENCE_663
- Admission main: 4b5377f4caa765321477011df859cf20de32786a
- Allocation: #162 comment 5733345229
- Architecture source: #663 comment 5733275908
- Foundation runtime mutation: NONE
- Resource registry mutation: NONE
- Production authority: NONE
- CW4 resume authority: NONE

## Result

FACT — count: the accepted CW4 witness uses two valid GameSessions. A/1 belongs to session A and B/1 belongs to session B. Therefore the per-GameSession functional lower bound demonstrated by A/1 open -> B/1 close -> replay A/1 is 1 retained terminal record, not 2.

RECOMMENDATION — count candidate: 1 retained terminal record per GameSession for the first-playable slice. This is the minimum that satisfies the exact accepted replay witness. FND-02 already permits an evicted older result to become COMMAND_OUTCOME_EXPIRED without making its CommandRef reservable again.

UNKNOWN — charged bytes: there is not enough accepted evidence to select an aggregate charged resident-byte maximum per GameSession without inventing a production resource value.

Existing 65,536-byte command/result limits are wire payload bounds. They do not prove a bound for the Foundation-owned normalized semantic intent, originally selected binding evidence, terminal semantic outcome, retained copies and metadata. Issue 663 also deliberately leaves the physical Rust layout unfrozen.

Therefore the evidence disposition is BLOCKED_ON_EVIDENCE for the byte maximum. Both registry values cannot truthfully be frozen yet.

## Smallest missing discriminator

FIRST_PLAYABLE_RETAINED_SEMANTIC_RECORD_CHARGED_BYTE_BOUND:

one accepted representation-independent charging rule or one concrete bounded Foundation API candidate proving the maximum owned bytes for server-normalized intent plus original selected binding evidence plus terminal semantic outcome plus retention metadata for the first-playable command family, including every simultaneously retained copy.

It must not be replaced by fixture 8, FND02-OUTSTANDING-COMMANDS=64, NET03 queue budgets, either 65,536-byte wire payload limit, or arbitrary headroom.

## Deterministic model

The harness is intentionally synthetic. It exercises the accepted lifecycle and resource boundary behavior without choosing a production Rust layout.

- modeled fixed store charge: 32 bytes;
- modeled fixed record charge: 40 bytes;
- record equation: record_fixed_logical_bytes + normalized_intent_bytes + binding_bytes + outcome_bytes;
- session equation: store_fixed_logical_bytes + sum(retained_record_charged_bytes).

Those byte widths are test accounting only: not RSS, Python object size, Rust ABI size, wire size or a proposed production charge rule.

### Synthetic candidate envelopes

| Shape | Count | Intent | Binding | Outcome | Record charge | Session charge |
|---|---:|---:|---:|---:|---:|---:|
| tiny | 1 | 8 | 16 | 8 | 72 | 104 |
| tiny | 2 | 8 | 16 | 8 | 72 | 176 |
| tiny | 4 | 8 | 16 | 8 | 72 | 320 |
| tiny | 8 | 8 | 16 | 8 | 72 | 608 |
| cw4_like | 1 | 16 | 128 | 32 | 216 | 248 |
| cw4_like | 2 | 16 | 128 | 32 | 216 | 464 |
| cw4_like | 4 | 16 | 128 | 32 | 216 | 896 |
| cw4_like | 8 | 16 | 128 | 32 | 216 | 1760 |
| stress | 1 | 1024 | 2048 | 1024 | 4136 | 4168 |
| stress | 2 | 1024 | 2048 | 1024 | 4136 | 8304 |
| stress | 4 | 1024 | 2048 | 1024 | 4136 | 16576 |
| stress | 8 | 1024 | 2048 | 1024 | 4136 | 33120 |

No row in that table is a production maximum. The table proves only deterministic arithmetic, boundary behavior and how different semantic-record widths change the aggregate charge.

## Required semantic and boundary checks

- zero retained-record capacity rejects replay-required terminalization before modeled gameplay mutation;
- retained same-input replay returns the original terminal outcome without another mutation;
- changed normalized intent or original binding conflicts;
- count and aggregate-byte exact-boundary and one-over behavior is deterministic;
- pressure evicts terminal entries only; pending entries are never cache-evicted;
- evicted terminal CommandRef resolves to expired/reconciliation and stays non-reservable;
- later terminalization cannot pass an earlier pending CommandId;
- checked integer overflow rejects before allocation/commit;
- repeated measurements are byte-for-byte deterministic.

## Evidence classification

- functional count lower bound: 1 / GameSession;
- count architecture candidate: 1 / GameSession for the current first-playable slice;
- aggregate charged resident bytes: BLOCKED_ON_EVIDENCE;
- accepted production authority: false.

## Handoff

Route issue 663 back to the supervising architect only after the missing first-playable retained semantic-record charged-byte bound is supplied. Until both values can be frozen and protected-integrated, Foundation runtime implementation authority remains absent and CW4 remains WAITING_ARCHITECTURE.
