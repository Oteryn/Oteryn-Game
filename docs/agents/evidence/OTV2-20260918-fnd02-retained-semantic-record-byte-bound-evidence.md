# FND-02 retained semantic-record charged-byte bound evidence

Status: NON-PRODUCTION EVIDENCE / BOUND ESTABLISHED FOR ARCHITECT CONSIDERATION

FACT: protected #663 evidence fixes one retained terminal record per GameSession, original
binding identity as exact active Content-generation identity plus TransitionKey, and protected
512-byte ceilings for ProductionKey and ProductionAtom. TransitionKey wraps ProductionKey.

DERIVED: this child uses one immutable owned canonical byte buffer. It contains normalized
intent (placement, incarnation, family, expected revision), the original exact-generation plus
TransitionKey binding reference, the original terminal semantic result (outcome, state,
revision), and command_id retention metadata. It retains no complete TransitionBinding or
policy_guard_refs and never re-resolves the binding against current/new Content on replay.

FIRST_PLAYABLE_RETAINED_SEMANTIC_RECORD_CHARGED_BYTE_BOUND = 4*8 + 6*2 + 6*512 = 3116 charged bytes.

With the already accepted count 1/GameSession, the candidate aggregate for the exact
first-playable slice is also 3116 bytes/GameSession.

Boundary evidence:
- exact maximum encoded candidate: 3116 bytes
- exact maximum SHA-256: 91bed9f74e044a483a469c6fc78f2f98019cbeebec106b639731b98e50d9008f
- deterministic repeat: True
- decode round trip: True
- 513-byte key: rejected
- 513-byte generation candidate: rejected
- checked u64 overflow: rejected
- over-envelope retention: rejected before modeled gameplay mutation

Replay/recovery evidence:
- same normalized intent plus exact-generation/TransitionKey replays original outcome
- changed normalized intent, generation, or TransitionKey conflicts
- singleton terminal eviction makes the old outcome expired, never fresh/reservable
- later terminalization cannot pass an earlier pending CommandId
- A/1 open -> B/1 close -> replay A/1 retains one record in each GameSession
- recovery reconstructs the same record or the old GameSession is NON_RESUMABLE

The generation field reuses only the protected 512-byte ProductionAtom ceiling as an evidence
candidate. This does not freeze a production ContentGenerationRef, Rust ABI, allocator/RSS
accounting, wire/persistence/Content format, CW4 store, deployment topology, TTL, or future
general replay window.

No Foundation runtime, resource-registry, CW3/CW4, protocol, Cargo/workspace, persistence,
external-repository, or production surface is changed.

Handoff: route #663 to Oteryn: sol supervising architect to decide/freeze any production
resource value and authority. Do not resume CW4 from this evidence child alone.
