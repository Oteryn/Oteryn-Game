# OTV2-20261003-charm-desc-a2

```yaml
task_id: OTV2-20261003-charm-desc-a2
title: "CHARM-DESC-A2: bound attacker slot and the D324 write-fence lifecycle"
mode: HARD
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/charm-desc-a2-20261003
pr: PR_NUMBER
base_sha: 98a2f95
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01JoBaEpeTJeftBZYRdCgVK2 (oteryn-hard-worker, CP session_013KJX6mv8LQveCKKXYgAX94)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
packet: "docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_D327_PACKETS_AND_HELD_P1S_2026-10-03.md §1.2"
decisions: [D295, D324, D390, D395]
owned_paths:
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/actor_spell.rs
  - apps/game-server/src/ability/commit.rs
  - apps/game-server/src/ability/mod.rs
  - apps/game-server/src/ability/charm_desc_fence_gate_tests.rs
  - apps/game-server/src/durability/fresh_admission.rs   # call sites only
  - docs/agents/tasks/archive/OTV2-20261003-charm-desc-a2.md
  # Extended by the control plane (D395): bridge call sites and fixtures only.
  - apps/game-server/src/foundation/channel_owner_ability_commit_tests.rs
  - apps/game-server/src/foundation/channel_owner_combat_death_tests.rs
leases: none (no wire, no migration, no capability, command or domain)
public_contracts: []
depends_on: ["#1652", "#1651", "ACH-NOTIFY-2 (#1656)"]
closes: "#1635"
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

D295 item 4 (option A2) with the CHARM-DESC-FENCE-LEASE lifecycle (D324, #1651).

- **Bound slot.** The carrier keeps, beside each committed player slot, the admitted session's
  `CharacterLease` and an optional write fence (`PlayerAttackerAuthority`). The side table is
  keyed by slot index and generation, bounded by the slot count, and pruned by `remove`. The fixed
  slot footprint is unchanged.
- **Admission.** `initialize_first_entry` binds the lease in the same owner step and from the same
  current durable read that proves the session current. No damage write is admitted before that.
- **Write path.** `CurrentOwnerExactActorCommit::commit_damage_for_bound_attacker` reads the
  attacker from its bound slot in the same critical section as the write. It checks, in order,
  that the slot holds the command's session, that the slot is not fenced, and that a lease is
  bound. Anything else is `SupersededAttackerSession`, before `DamageContributors::admission`.
- **Bridges.** `commit_exact_owner_damage`, `_primary_damage` and `_charm_damage` take the
  attacker's `ExactActorRef` and read the lease from the bound slot. A caller never supplies the
  Character or the lease. The raw `commit_damage_for_attacker` is now `#[cfg(test)]`.
- **Fence lifecycle (D324).**
  - Step (a): `fence_player_writes` runs under the runtime lock with no I/O. The token is the lost
    epoch for grace expiry, which needs the slot's mark of that epoch, or a transition id minted
    by the Channel owner. Another token is `WriteFenceBusy`, and the same token joins.
  - Step (b): the durable release runs with the runtime lock released.
  - Step (c): `Released` or `Terminal` retires the actor. `NotApplicable` or `NotExpired` lifts
    the fence with its exact token, but only after a durable read shows the session non-terminal.
    TERMINAL is absorbing, so a later non-terminal read proves the session held the lease at the
    attempt too. A terminal row retires the actor with the session still fenced. An unknown
    outcome keeps the fence and retries.
  - `release_after_grace` and `release_abandoned` follow these steps. An overlapping transition
    waits with backoff and re-evaluates. Grace expiry fences only once the held deadline passed.
  - `restore_player_control` (resume) lifts the fence of its own epoch together with the mark.
- **Rebind (D295 item 4).** `rebind_player_session` detaches the terminal session, drops its fence
  and mark, and binds the successor session and its newer lease of the same Character, in one
  carrier operation. No transport caller is composed yet: the post-grace takeover is not in the
  transport.
- **Continuation (FND-04B §22).** `bind_continuation` binds only a `Proven` reconcile, unfenced.
  `Terminal` and `Unprovable` refuse and leave the slot unbound. No process-replacement caller is
  composed yet.
- **Lock order.** The runtime lock is never held across durable I/O. The admission-domain lock and
  the guard-row compare-and-set run only inside the store calls, never under the runtime lock.
- **Gate (#1652), relaxed, not deleted.** `charm_desc_fence_gate_tests.rs` now forbids production
  references to the unbound entry `commit_damage_for_attacker`, wherever they are, the carrier
  included; only its canonical test-only definition is exempt. Production may call the bridges,
  because their signature goes through the bound slot.
- **D390 seam.** #1534 had not merged before freeze, so its spell damage seam stays for a
  follow-up through `commit_exact_owner_damage` with the caster's `ExactActorRef`.

## Tests

- `runtime_actor_carrier::attacker_fence_tests`:
  - a write between fence and commit is refused, and only the exact token lifts the fence;
  - one transition per session: a retry joins, another token waits, and after a lift the second
    transition fences; after a terminal settle it finds nothing to do;
  - grace expiry fences only its marked epoch, and a resume lifts that fence but no other;
  - a terminal session is never unfenced, and a recycled slot inherits neither fence nor binding;
  - a rebind drops the old fence and leaves the rebound slot unfenced;
  - continuation: terminal and unprovable refuse, proven binds unfenced;
  - a rebind during an in-flight charm or descriptor commit settles exactly once;
  - a fenced slot refuses every bridge before the owner write.
- `charm_desc_fence_gate_tests`: the real tree passes, and `gate_fails_a_synthetic_unbound_caller`
  shows a synthetic unbound caller is flagged while a bound caller is not.
- The carrier test children (`channel_owner_ability_commit_tests.rs`,
  `channel_owner_combat_death_tests.rs`) bind their attackers in the fixtures. The Charm lease
  tests now present a slot bound to another Character or to a superseding lease.

## Validation

VALIDATION_RESULTS

## Review

Concurrency, persistence and combat review (Codex) on the final frozen head. The control plane
requests it.
