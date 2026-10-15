# SCOPE-HANDOFF-2 Cross-node house entry and the any-node fallback

- Decision: `SCOPEHANDOFF2-CROSS-NODE-HOUSE-ENTRY-V1`
- Status: **CANDIDATE** (architect-authored). Acceptance needs exact-head validation, independent review (security, concurrency, persistence and protocol) and protected integration. Owner acceptance is the owner's to give; none is recorded here.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the mechanics completion plan, Appendix 7 archive follow-up SCOPE-HANDOFF-2 ("hard (persistence): cross-node house entry, the any-node fallback"); the task record `docs/agents/tasks/SCOPE-HANDOFF-2.md`; the open D573 boundary of the SCOPE-HANDOFF-1 record (`docs/agents/tasks/archive/OTV2-20261004-scope-handoff-1.md:80`: "Entry is same-node only ... a house assigned to another node is `BUSY` before anything is written") and its *Not in scope* ("needs a connection transfer/redirect contract that no contract owns yet").
- Builds on: HOUSE-RUNTIME-0 (accepted: §3 activation "on the node that runs the entering character's Channel if it has capacity, else any node of the World"; §4.1 prepare, commit, abort, "the durable handoff row is the commit point"; §4.2 exit; §4.3 recovery; `HOUSERT0-RL-01` to `-04`); the SCOPE-HANDOFF-1 writers on main (`apps/game-server/src/durability/house_scope_handoff.rs`: `prepare_house_entry`, `commit_house_entry`, `abort_house_entry`, `reconcile_house_entries`, `recover_stale_house_entries`, `release_house_entry_tile`; migration 0074 triggers); the node registry and assignment writer (`runtime_scope_assignment.rs`: `BootstrapSecret`, `NodeIncarnationProof`, `ControlActor`, `AssignmentCommand`); ADR-0003 (route and endpoint come only from the World Registry and Gateway); the instance baseline (seamless handoff, single-use short-lived admission material, the Gateway is not a gameplay proxy, placement privacy); FND-04 and FND-04B (§18 transfer refusal and ADMIT-0 §3.2 lifting conditions, §22 locator, §23 handoff boundary); ADMIT-0 §3.2; ADR-0001 §10 and §12.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`
- Runtime, migration, wire and production authority: NONE until accepted and allocated. Each child needs its own #162 allocation.

## 1. Scope

A character on a Channel owned by node A enters a house whose scope is, or must become, owned by node B. This brief decides who places the house scope (the controller), how the two nodes agree without sharing secrets (the peer contract), how the client is moved to node B (the redirect), how the destination confirms the move, every failure outcome, and how D573 is closed. The house exit stays as HOUSE-RUNTIME-0 §4 and ADMIT-0 decide (H10).

## 2. Facts

- On main the same-node entry is one commit transaction holding both proofs: the proving node holds the origin Channel and the house scope; a house assigned to another node refuses `BUSY` at prepare with nothing written (D573). Exit into a Channel is a typed refusal (`ExitNotAdmitted`).
- Handoff rows are PREPARED (1), COMMITTED (2) or RELEASED (3). A nonterminal house session commits only as the destination of its COMMITTED or RELEASED handoff (0074 deferred trigger). Placement evidence is a destination session whose connection generation passed the admitted 1, or that is terminal.
- `BootstrapSecret` is a one-launch secret; only its SHA-256 digest is retained. `NodeIncarnationProof` is process-held: "Only this process holds the secret; the database retains only its digest. A public NodeId/revision alone never proves currentness to a fenced writer."
- Assignments are control-plane commands with an authenticated `ControlActor`; a node never assigns itself a scope.
- No protocol message today moves a client between nodes; no Game contract owns a redirect.

## 3. Rules

**H1. Two paths, one invariant.** When the house scope is held by the same node as the origin Channel, the SCOPE-HANDOFF-1 path runs unchanged. When it is held by another current node, the cross-node path (H3 to H6) runs. Both keep HOUSE-RUNTIME-0 §4.1's invariant: a character is in exactly one admitted scope at every instant, the source until the commit point and the destination after it; the commit point is the durable handoff row.

**H2. The controller places the house scope.** Activation (HOUSE-RUNTIME-0 §3) is a control-plane assignment, never a node's own choice:
- The origin node finds the house unassigned and records one activation request keyed by `HouseId` (idempotent; a second request for the same house is the same request) and refuses the entry for now with `BUSY` (the player retries, as for any busy door).
- The controller (the assignment writer, with its `ControlActor`) consumes the request and picks the node: the origin node if it is current, ready and below `HOUSERT0-RL-04` active house scopes; else, among the World's current, ready nodes below `HOUSERT0-RL-04`, the one with the fewest active house scopes, ties broken by ascending NodeId. Capacity comes from the runtime status reports (`NRS-HEARTBEAT`); a node whose report is older than two heartbeats is not a candidate. With no candidate, the request ends `NO_CAPACITY` and the next entry attempt refuses `BUSY`.
- The choice is deterministic for the same inputs and is recorded on the assignment receipt (the any-node fallback). One live assignment per house (HOUSE-RUNTIME-0 §3) still holds.

**H3. The peer contract is the database, not a node-to-node channel.** Node A and node B never exchange secrets, proofs or calls. Each writes only under its own `NodeIncarnationProof` in its own transaction, and each verifies the other only through committed rows: the assignment row and its generation, the handoff row and its state. A wake signal (a database notification carrying only the handoff id) tells B a row is waiting; it is not authority, and a missed wake is covered by a poll every `SCOPEHO2-RL-03`. `BootstrapSecret` never leaves its node: it is not stored, logged, sent to the peer, put in a handoff row, a wake signal or redirect material, or used to derive any key.

**H4. Cross-node entry, four writes.**
1. **Prepare (node A, origin proof).** The SCOPE-HANDOFF-1 prepare as today (actor command serialized with combat; guards; freeze), except that it reserves no tile. One transaction locks the origin Channel assignment and the house assignment `FOR SHARE`, checks the house holder is a current registered node other than A, and inserts the handoff row in the new state `PREPARED_REMOTE` naming the house generation it read. Source session stays live and fenced.
2. **Accept (node B, house proof).** The house runtime, single writer per `HouseId`, selects and reserves the tile under the §6.3 tile rules and `HOUSERT0-RL-03`, and moves the row to `ACCEPTED` with the tile, or to `REFUSED{NO_ROOM | HOUSE_CLOSED}`. It allocates the destination GameSessionId at this point and records it on the row; it admits nothing yet.
3. **Commit (node A, origin proof): the commit point.** One transaction, in the SCOPE-HANDOFF-1 lock order with the house assignment read `FOR SHARE` after the origin Channel assignment: the HOUSE-RUNTIME-0 §4.1 revalidation (property row, content fence, exact `acl_revision`, guild revisions), the house assignment still at the recorded generation and held by B, the row `ACCEPTED`; then the source session becomes terminal and the row becomes `COMMITTED`. Any difference aborts (HOUSE-RUNTIME-0 §4.1 step 3: delete the row and, through B, the reservation; resume the source) with `NO_ACCESS`, `HOUSE_CLOSED` or `BUSY`.
4. **Admit (node B, house proof).** B inserts the destination session (the recorded GameSessionId, origin Channel recorded) as the destination of the COMMITTED row; the 0074 deferred trigger already proves that pairing. This write is idempotent on the GameSessionId. Between steps 3 and 4 the character's placement is the destination by the committed row; nothing admits or resumes it elsewhere (HOUSE-RUNTIME-0 §4.3).

A waits for step 2 at most `SCOPEHO2-RL-01`; with no answer it aborts with `BUSY`. Only A aborts a `PREPARED_REMOTE` or `ACCEPTED` row while A is current; B never deletes a row it did not create, but releases its own reservation when it sees the row gone or `REFUSED`.

**H5. The client redirect.** After step 3, node A sends the client `SCOPE_REDIRECT{handoff_ref}` and closes the transport. The message carries no endpoint, NodeId, ChannelId, InstanceId or house generation. The client presents `handoff_ref` to the Gateway on its authenticated account session. The Gateway (ADR-0003) reads the COMMITTED row and the current house assignment through the FND-04B §22 locator, resolves B's endpoint from the World Registry, and issues single-use admission material (within `FND02-ADMISSION-MATERIAL-BYTES`, valid `SCOPEHO2-RL-02`) bound to (CharacterId, destination GameSessionId, handoff id, house generation). The client connects to B and presents it; B binds the transport to the destination session (its connection generation advances past 1, which is the placement evidence for `release_house_entry_tile`). The Gateway carries no gameplay traffic. Material that is replayed, expired, or names a stale generation is refused uniformly.

**H6. Current confirmation.** On binding, B sends `SCOPE_ADMITTED` naming the WorldId, the house scope (as an opaque reference) and the selected capability set; no snapshot or gated message precedes it. A client whose `SCOPE_ADMITTED` was lost reconnects through the Gateway, which follows current authoritative placement (FND-04B §23) back to B, and B sends `SCOPE_ADMITTED` again before anything else. This is the shape of ADMIT-0 §3.2 conditions (a) to (c) applied to the house scope; ADMIT-0 governs transfers into channel scopes, so entry into a house needs it only as this contract's own rule.

**H7. Failure behaviour.**

| Failure | Outcome |
|---|---|
| Any guard or revalidation fails before step 3 | abort; source resumes; typed reason; nothing else written |
| B does not accept within `SCOPEHO2-RL-01` | A aborts `BUSY`; B releases any late reservation |
| B's house generation changes before step 3 | step 3 refuses (`HOUSE_CLOSED`); abort |
| A crashes before step 3 | A's restart reconcile, or `recover_stale_house_entries` by any proven current incarnation once A's Channel generation is no longer current, deletes the row; B releases the reservation; source holds |
| A crashes after step 3 | the row stands; the client reconnects through the Gateway to B (H5); step 4 runs if it has not |
| B crashes after step 3 | a new house generation; `recover_stale_house_entries` releases the COMMITTED row of the stale generation; the character is placed at the `entrance` on its origin Channel at its next admission (HOUSE-RUNTIME-0 §4.3) |
| The redirect is lost or the client never arrives | the destination session is admitted but unbound; FND-04B control-loss and grace apply (`FND04B-SAME-SESSION-GRACE-S`), then the post-grace house logout of HOUSE-RUNTIME-0 §6.3 |
| The Gateway cannot resolve B | the client retries through the Gateway; no client-chosen endpoint is ever accepted |

**H8. D573 is closed by replacement, bounded by currentness.** "A house assigned to another node is `BUSY`" is replaced by the cross-node path when the holder is a current registered node. `BUSY` remains only when the holder's registration is not current or cannot be proven (fail closed), when the activation request is pending (H2), or while the cross-node capability is off. Commit still never runs with a proof the committing node does not hold: same-node commit keeps both same-node proofs; cross-node commit uses A's proof for the source and relies on B's committed `ACCEPTED` write for the destination.

**H9. Privacy.** The client learns no NodeId, endpoint before the Gateway hands it over, ChannelId or InstanceId (instance baseline, Privacy consequences). Handoff refs are random, single-use per handoff and useless without the account session.

**H10. Exit is unchanged.** The house exit into a Channel stays refused (`ExitNotAdmitted`) until its own child (SCOPE-EXIT-1) satisfies ADMIT-0 §3.2 (a) to (c) under independent protocol and security review. That child reuses H4, H5 and H6 in reverse (B prepares, A accepts the tile under CHAR-POSITION-0 §3.3, B commits); the origin-first rule and the §4.2 fallback to another Channel come from HOUSE-RUNTIME-0 §4.2 and ADR-0001 §12, with the destination node chosen by the World's channel admission, never by the client.

**H11. Rows (proposed; registered by the children before implementation).**

| Row | Value | Source |
|---|---|---|
| `SCOPEHO2-RL-01` accept wait at the origin | 2,000 ms | engineering |
| `SCOPEHO2-RL-02` redirect admission material validity | 30 s | engineering |
| `SCOPEHO2-RL-03` accept poll interval when no wake arrives | 250 ms | engineering |
| `SCOPEHO2-RL-04` open cross-node handoffs per house | counted inside `HOUSERT0-RL-03` (PREPARED entries already count) | HOUSE-RUNTIME-0 |
| `SCOPEHO2-RL-05` pending activation requests per World | 1,024, one per house | engineering ceiling |
| Cross-node door transition | `HOUSERT0-RL-02` does not apply; a separate target is measured before activation (open point) | measurement |
| Cited, unchanged | `HOUSERT0-RL-03`, `HOUSERT0-RL-04`, `NRS-HEARTBEAT`, `FND02-ADMISSION-MATERIAL-BYTES`, `FND04B-SAME-SESSION-GRACE-S` | |

## 4. Packets (proposed)

Allocated one by one by the control plane after acceptance; each its own PR.
1. **SCOPE-HANDOFF-2a** (hard, persistence and concurrency review): the migration for the new states (`PREPARED_REMOTE`, `ACCEPTED`, `REFUSED`) and the destination GameSessionId column, the four writers of H4, the H7 recovery rows, PostgreSQL crash tests at every step. Needs HOUSE-RUNTIME-1a.
2. **SCOPE-HANDOFF-2b** (hard, concurrency review): H2, the activation request and the controller's deterministic placement on the assignment writer, with restart and replay tests.
3. **SCOPE-REDIRECT-1** (protocol and security review, Reg): H5, H6 and H9: `SCOPE_REDIRECT`, Gateway material issue and validation, `SCOPE_ADMITTED`, lost-ack reconnect. Needs SCOPE-HANDOFF-2a and the Gateway owner (Platform boundary for the material issuer). Its capability is reserved on #162 at allocation.
4. **SCOPE-EXIT-1** (hard, protocol and security review): H10, lifting ADMIT-0 §3.2 for the house exit. Needs SCOPE-REDIRECT-1.

Order: 2b and 2a in parallel, then SCOPE-REDIRECT-1, then SCOPE-EXIT-1. The cross-node path is enabled only after SCOPE-REDIRECT-1 merges; until then H8's `BUSY` stands.

## 5. Open points (not decided here)

- The cross-node door transition latency target (measured before activation).
- Whether the Gateway material issuer is a Game component or a Platform endpoint (ADR-0003 names the Gateway; the issuer's owner is a Platform boundary question).
- The task record's owned paths (`apps/game-server/src/house_scope/`, `apps/game-server/tests/house_scope_*.rs`) do not include `apps/game-server/src/durability/house_scope_handoff.rs`, the migration or the assignment writer, which SCOPE-HANDOFF-2a and 2b need; the children's records must own them. The record is not edited here.

## 6. Split out, not decided here

- **Healthy-session migration between nodes** (FND-04B §10) and Channel to Channel transfer: their own contracts; H5 and H6 are reusable but not decided for them.
- **Instance entry across nodes** (boss rooms, arenas): BOSS-ROOM-1 and ARENA-INSTANCE-1 reuse this contract only after their own review.

## 7. Handback

```yaml
result: CANDIDATE
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_SCOPE_HANDOFF2_DECISION_2026-10-15.md
resource_values_changed: false
production_authority_changed: false
implementation_may_resume: false
required_fresh_allocation: true
required_independent_review: "security, concurrency, persistence, protocol"
amends_pending_acceptance: "SCOPE-HANDOFF-1 D573 (H8); HOUSE-RUNTIME-0 §3 activation placement (H2)"
```
