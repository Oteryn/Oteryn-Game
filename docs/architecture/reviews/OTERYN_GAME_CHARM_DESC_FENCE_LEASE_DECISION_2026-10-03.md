# CHARM-DESC-FENCE-LEASE Lease lifecycle mechanics of the attacker write fence

- Decision: `CHARM-DESC-FENCE-LEASE-V1` (control-plane allocation D324, #1622)
- Status: **CANDIDATE**. Acceptance needs exact-head validation and independent review (authority
  and fencing).
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Review round 1 on `af0a8553` (P1 4173613154 and 4173613160) is fixed in items 3 and 7.
- Answers: the open item of `CHARM-DESC-FENCE-V1` (D295) item 4, "lease lifecycle mechanics are not
  designed here". This decision picks one of the two ways D295 allows, and sets the lock order and
  the invalidation operation.
- Covers: the player actor slot of `foundation/runtime_actor_carrier.rs` and every transition that
  ends a GameSession's hold on a Character lease.
- Runtime, migration and production authority: NONE. A2 implements it.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

D295 requires every Ability damage write to be fenced by the current owning lease authority. A2
can meet that in one of two ways:
- (1) consult the current lease authority at the write; or
- (2) have every terminal transition fence the slot synchronously, before the durable holder is
  cleared.

This decision picks the way, sets the lock order between the durable transition and the carrier's
runtime lock, and defines the invalidation operation.

## 2. Facts (verified on `ac6fdca8`)

- **The authority is durable.**
  - The Character holder lives in the admission guard rows. The GameSession row carries
    `character_lease_generation`.
  - `FreshAdmissionStore::release_expired_loss` (FND-04B §6 grace expiry) and
    `release_abandoned_session` clear the holder through the fenced `release`, as a compare-and-set
    on the guard rows (`durability/fresh_admission.rs` from line 1879).
- **Today's order is durable first, carrier second.** Both grace-expiry callers commit the durable
  release, then take the runtime lock and call `remove_terminal_session`. See
  `gameplay_transport/mod.rs`, lines 541–558 and 1036–1050.
  - Between the commit and the lock, the slot still holds the old session as current.
  - If the removal fails, the result is `Unknown` and the slot stays.
- **The slot has no write fence.**
  - An `Occupied` slot holds `game_session_id`, `committed`, and a lifecycle with an optional
    `ControlLossMark { epoch, grace_deadline }`.
  - `restore_player_control` clears the mark only for the exact epoch.
  - The damage write path (`commit_creature_damage_inner`, through `DamageContributors::admission`)
    runs under the runtime lock. It never reads the durable store.
- **The runtime lock is a tokio mutex held for world mutation.** No path holds it across a
  durable transaction.

## 3. Decision

1. **Way (2): fence first, then commit.**
   - Every transition that ends or replaces a GameSession's hold on the Character lease follows
     three steps:
     - (a) under the runtime lock, with no I/O, fence the slot's writes for that session;
     - (b) release the lock and commit the durable transition;
     - (c) under the runtime lock again, settle the slot.
   - This covers grace expiry, abandoned-session release, explicit release and logout, revocation,
     and any separately accepted takeover.
   - The durable holder is never cleared while the slot can still admit a write for that session.
     The fence step is the "same critical section" of D295 for writes: every write is ordered by the
     runtime lock and sees the fence.
2. **The invalidation operation (carrier, A2).**
   - Fencing sets a write fence on the exact slot, bound to the session and to a fence token.
     - The token is the control-loss epoch for grace expiry. Otherwise it is a transition id
       minted by the caller.
   - The fence is set only when the slot holds that session. For grace expiry the slot must also
     still carry the control-loss mark of that epoch. Otherwise fencing refuses, and the transition
     does not start.
   - While a fence is set, every damage write whose attacker is that slot's session is refused as
     `SupersededAttackerSession`, before `DamageContributors::admission`. The write path checks
     three things, in this order:
     - the slot holds the command's session;
     - the slot is not fenced;
     - the lease binding is present.
     A missing binding refuses.
3. **Settling (step c), by durable outcome.**

   | Durable outcome | Step (c) |
   |---|---|
   | `Released` or `Terminal` | Retire the actor with `remove_terminal_session`, or rebind in place (D295 item 4). The old session's fence is never lifted for that session, which is terminal. |
   | `NotApplicable` or `NotExpired`, or a refusal that is definitely uncommitted | Lift the fence with the exact token. A lift with any other token is a no-op. |
   | Unknown (lost acknowledgement, store error) | Keep the fence and retry. The outcome is reconciled from the durable row and never decided again. |

   - **Rebind retires the old fence atomically.**
     - The post-grace in-place rebind (D295 item 4) is one carrier operation under the runtime
       lock. It detaches the terminal session, drops that session's fence, and binds the newly
       authorized GameSessionId and its lease generation.
     - A rebound slot carries no fence, so the new session's writes are admitted.
     - The old session's writes stay refused, because the slot no longer holds that session.
     - A rebind that would leave the fence on the slot, or that binds without dropping it, is a
       defect.
   - A resume of the same session lifts the fence together with the control-loss mark of the same
     epoch. That resume is `restore_player_control` after the durable resume commit.
   - A resume of a terminal session fails durably, so it never lifts a fence.
4. **Lock order (binding).**
   - The runtime lock is never held across a durable transaction.
   - The admission-domain lock (`lock_admission_domain`) and the guard-row compare-and-set are never
     acquired while the runtime lock is held.
   - Step (a) happens before the durable transaction, and step (c) after it.
5. **Stored equality stays a pre-check** (D295). The lease generation copied into the slot never
   replaces the fence. The per-creature high-water mark stays a monotonic guard.
6. **Grace-expiry timing.**
   - The worker fences only once the deadline it holds has passed, so `NotExpired` comes only from
     clock skew and its lift is short.
   - Damage writes during grace keep today's behaviour. Whether an uncontrolled actor keeps
     attacking is not decided here.
7. **Recovery after process replacement.**
   - The fence is runtime state only, and a restart discards it.
   - FND-04B §22 allows a same-session continuation after process replacement, under the same
     GameSession and lease generation. So the old tuple can come back.
   - Before a continuation binds a reconstructed slot, it reconciles every terminal transition of
     that session from the durable rows, in three cases:
     - **Terminal on the durable row** (a release that committed, including one whose
       acknowledgement was lost): no same-session continuation. Only the accepted post-grace path
       may attach control, and it rebinds as in item 3.
     - **Non-terminal, with the §22 evidence proven:** the transition never committed. The
       continuation binds the slot without a fence. A later transition starts again at step (a)
       against the original deadline, which is never restarted (§22).
     - **Not provable:** fail closed. There is no continuation, and the slot is not reconstructed.
   - No damage write is admitted for the session until that binding exists. So the reconstructed
     slot is the first point at which a write can run, and it runs after reconciliation.
   - A new session after a restart uses a newer generation and is fenced by the session check.

## 4. Rejected options

- **Way (1), consult the durable authority at each write.**
  - It needs either durable I/O under the runtime lock, which stalls the world on every hit, or a
    read before the lock, which a release can overtake.
  - An in-memory mirror of the authority would need the same invalidation as way (2), so it adds a
    copy without removing the problem.
- **Fence and durable commit in one critical section** (hold the runtime lock across the
  transaction). This inverts today's lock order and holds world mutation across database I/O.
- **Keep the order durable first, carrier second.** A queued write in the window between the
  commit and the removal sees the old tuple as current. That is the case D295 rejects for B.
- **Refuse all damage writes while control is lost.** It would close the window for grace expiry
  only, not for revocation or abandoned release. It would also decide the uncontrolled-actor
  combat behaviour without Reference evidence.

## 5. Decision test

- **Must decide now?** YES. A2 depends on it (D295 item 4), and CHARM-DESC-FENCE-1 is unblocked.
- **Harder later:** nothing irreversible. The fence is runtime-only and touches no schema or wire.
- **Supersede if:**
  - the admission contract moves the Character holder into runtime authority; or
  - an accepted takeover flow needs a different atomic boundary. That flow's contract then names
    its own fence step.
- **Deliberately not decided:**
  - the fence field's name and type, and the token's representation;
  - uncontrolled-actor combat behaviour;
  - second-session semantics (FND-04A §8, FND-04B §10 and §19);
  - the wire behaviour toward a displaced client.

## 6. Before-freeze checklist

1. Owned paths only: this file, its archive record and the D309 bundle task record.
2. No code change. A2 implements item 2 and the reordered callers of item 1.
3. A2's tests prove each of the following:
   - a write between fence and commit is refused;
   - `NotApplicable` lifts the fence by its exact token, and a wrong token is a no-op;
   - an unknown outcome keeps the fence;
   - a resume lifts the fence with its epoch;
   - a terminal session is never unfenced;
   - a rebind drops the old fence and leaves the rebound slot unfenced;
   - a same-session continuation after process replacement reconciles first: terminal refuses,
     non-terminal binds unfenced, unprovable fails closed;
   - no path takes the admission lock under the runtime lock.
