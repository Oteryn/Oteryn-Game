# CHARM-DESC-FENCE Live attacker authority for Ability damage commits

- Decision: `CHARM-DESC-FENCE-V1` (control-plane allocation D295, #1622)
- Status: **CANDIDATE**. Acceptance needs exact-head validation and independent review (authority
  and fencing).
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: Codex P1 4173012827 on PR #1625 (head `3f7f1700316e62221cc739b2fb943c5d52452be0`),
  "Validate the lease against live authority at the write". The packet is #1622 comment 5969245263,
  and the ruling was first posted as #1622 comment 5969262075.
- Covers: the primary and descendant (charm) damage commits of `apps/game-server/src/ability/commit.rs`.
- Runtime, migration and production authority: NONE.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

**The finding.** After P1 4172944113, e07fd3ff passes a `foundation::CharacterLease` to
`commit_exact_owner_charm_damage` and refuses a lease that does not accept the frozen generation
(`SupersededAttackerSession`). P1 4173012827 is correct that `CharacterLease::new` is public, so the
lease is still a caller-supplied snapshot and not live authority.

**What is decided here.** Where the live attacker fence belongs, and whether e07fd3ff's parameter
stays.

## 2. Facts (verified on `3f7f1700`)

- **No live lease check today.**
  - `commit_exact_owner_damage`, `commit_exact_owner_primary_damage` and
    `commit_exact_owner_charm_damage` take the attacker identity and lease from their caller.
  - The only owner-boundary check is the per-creature high-water mark (`DamageContributors::admission`,
    `foundation/runtime_actor_carrier.rs`). It never reads a live lease.
- **Dead code.** All three bridges are `#[allow(dead_code)]` (`ability/commit.rs` lines 167, 262,
  349, 473 and 522; `ability/mod.rs` lines 12, 14 and 17) and have no live gameplay caller.
- **The player slot.**
  - A player slot is bound to one GameSession. `game_session_id` is set in `reserve_player` and
    `commit_reserved_player` and is never rebound.
  - The slot holds no CharacterId and no lease generation.
- **Slot removal.**
  - `remove_terminal_session` **has production callers** in the grace-expiry path
    (`gameplay_transport/mod.rs:547` and `:1040`, outside `mod tests` at line 1801). They run after a
    durable `Released` or `Terminal` result.
  - So an old slot is removed only once its session is terminal.
  - The packet said this function had no production caller. That claim was wrong, and this
    decision corrects it.
  - `remove_terminal_session` calls `RuntimeActorCarrier::remove_terminal_player`, which calls
    `remove`: it destroys the slot, its position and its runtime state, and invalidates the actor
    reference. It is an actor-retirement path, not a session-replacement path.
- **Session continuity (FND-04 line 106; FND-04B lines 380 and 429).** A terminal GameSession never revives.
  While the same actor stays `PRESENT_UNCONTROLLED`, post-grace recovery attaches a **new**
  GameSessionId to that exact actor at one atomic boundary, without resetting actor or gameplay
  state. Within grace, `player_control_loss` and `restore_player_control` keep the same session.

## 3. Decision

1. **C: defer the live fence to composition.** P1 4173012827 is **deferred, not dismissed**.
   - The live attacker-authority fence belongs to the composition stage that wires a live
     GameSession into an Ability damage commit.
   - It covers the primary and the descendant commit together, in one change.
   - PR #1625 proceeds without a foundation change. Its review reply cites this decision.
2. **Keep e07fd3ff's `CharacterLease` parameter (a)** as the typed seam the fence will feed.
   - It is a caller-supplied snapshot and must not be documented as live authority.
   - It closes P1 4172944113 for the frozen-generation case only.
3. **Hard wiring gate (binding).** No production caller may be added for any of the three bridges
   until the follow-up fence (item 4) is merged.
   - Removing their `#[allow(dead_code)]` is the observable trigger.
   - A PR that adds such a caller without the fence fails review on this decision.
   - While the gate holds, no live path can reach a stale-tuple damage write.
4. **Follow-up task, direction A2** (admission and composition lane; high risk; it needs the owning
   admission/session contract and independent review):
   - admission binds `CharacterId` to the player slot, and at most one live slot exists per
     character;
   - a new session's admission ends the old session's control at once, through the authoritative
     terminal fact, so takeover never leaves the old session able to command the actor until grace
     expiry;
   - **session replacement is an in-place rebind, never a removal.** A2 adds one carrier operation
     that, in the same atomic boundary as the FND-04B recovery or takeover, detaches the terminal
     session from the slot and binds the new GameSessionId to the **same** actor slot. The slot,
     its actor reference and generation, its position and its runtime state are preserved, as
     FND-04B requires for a `PRESENT_UNCONTROLLED` actor. A command that still carries the old
     session then fails the slot's session check;
   - `remove_terminal_session` stays only for actor retirement, when the actor legally becomes
     `ABSENT` (FND-04B line 107). A2 must not route a session replacement through it, and it confirms
     that the grace-expiry callers (`gameplay_transport/mod.rs` lines 547 and 1040) remove only an
     actor that is leaving the world, not one that post-grace recovery may still attach to;
   - the damage write validates the attacker's live slot and its session against the command;
   - **B** (the lease generation stored on the slot) may be added under A2 as an equality check.
     On its own it is rejected, because it keeps the lifecycle gap.

## 4. Rejected options

- **A (a "live slot" check alone).** Between takeover and the terminal fact, the old slot stays
  committed under the old session. A stale command with the old actor ref would pass.
- **B alone.** It has the same lifecycle gap as A unless A2's takeover and in-place rebind are in place.
- **A2 inside #1625.** It changes session and fencing semantics in a charm-damage PR, without the
  owning contract.
- **Reverting e07fd3ff (b).** That would lose the typed seam and reopen the frozen-generation case
  of P1 4172944113.

## 5. Decision test

- **Must decide now?** YES. #1625 is on HOLD on an open P1.
- **Blocked:** #1625 and the reply to 4173012827.
- **Harder later:** nothing irreversible. The seam stays typed, and the fence attaches at the single
  wiring point that item 3 gates.
- **Supersede if:**
  - a live caller is needed before A2 lands (that caller's PR then carries the fence itself);
  - the admission contract chooses a different single-live-slot mechanism.
- **Deliberately not decided:** the exact refusal semantics for a second session, the rebind
  operation's name and signature, the slot schema, and the wire behaviour toward a displaced
  client. All three belong to the A2 task and
  its contract.

## 6. Before-freeze checklist

1. Owned paths only: this file and its task record.
2. No code change. #1625's writer replies on 4173012827 citing this decision.
3. Split work: the A2 task (control plane opens it).
