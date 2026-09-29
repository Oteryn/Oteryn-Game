# D39 chest USE GAME-INTERACTION amendment decision

- Decision: `D39-CHEST-USE-GAME-INTERACTION-V1`
- Status: **CANDIDATE, no new owner decision (§2)**. Acceptance requires exact-head validation,
  independent review and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.1)
- Profile: `Oteryn Reference` (Global Tibia at 2026-09-27, D33)
- Answers: the chest USE wiring question, #162 comment 5884603451, by the ruling in #162
  comment 5884689001 (item 2)
- Amends: GAME-INTERACTION-01 successor candidate (§5.1, §5.5, §17, §19.1 only, for one slice)
- Owner decisions already taken: D39 and D40 (`OTERYN_REWARD_CHEST_PLAYABLE_SLICE_DECISIONS_V1.md`
  §4), D92 (§5.1 of the same file)
- Admission baseline: `main@005550da`
- Runtime, registry, migration, protocol and production authority: **NONE**
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

D39 accepts the GAME-INTERACTION-01 successor for the `USE` edge on a placed object. The successor
is still `PROPOSED / NONCANONICAL` and carries open blockers. CHEST-1 (#1190) is merged and
mints the reward into a main-backpack entry (DUR-03 §39.3). Which parts of the successor may an
implementer rely on to wire a player `USE` on a placed chest?

## 2. Owner decisions

No new owner decision. D39 already accepts the successor's trigger identity, child identity and
retry rules for the chest interaction and leaves its open dependencies open. D40 already fixes the
MINT cause. This record names the exact sections that become accepted for that one slice.

## 3. Facts

**PROVEN** (main `main@005550da`)

- Reward chest decisions §4: D39 as above. D40: a second MINT source cause beside the creature
  death, a GAME-INTERACTION child occurrence of a `USE` on a reward-claim placement, idempotent per
  (claim, character); a repeat returns the first outcome.
- Reward chest §5.1: the first slice covers `once` claims; container rewards, cooldown claims and
  weight are later children. Order: B3-1, CHEST-1, chest `USE` wiring (D39), cooldown claims,
  container rewards, weight.
- Reward chest §6: teleports and map objects (D37, D38), summons, NPC dialogue and quest missions
  are not in the slice.
- DUR-03 §39.3 (reward chest amendment): the source cause is the D40 `USE` child occurrence, the
  location is a new entry of the equipped main backpack, there is no Ground custody and no TRANSFER.
  The `RewardClaim` row commits in the same transaction.
- Successor §5.1: a child occurrence is identified by parent source occurrence, interaction
  definition, authoritative target, typed edge, optional ordinal and revision context. §5.5: the
  typed edge distinguishes `USE` from other edges on the same target. §17.1: the same `CommandRef`
  recovers the same terminal result and never becomes a fresh attempt. §17.2: a new attempt needs a
  new `CommandRef` only after the prior one is provably terminal, and a `PENDING` one forbids it.
  §19.1: DUR-03 owns durable value commit, abort and ambiguity; GAME-INTERACTION keeps only
  trigger and child correlation.
- Successor §19.2 to §19.5 name blockers: GAME-ABILITY effects, movement handoff, durable writable
  text, and client protocol registration (FND-02).
- Live allocations: the "Interaction Use orchestration" row is `WAITING_ARCHITECTURE`.

**UNKNOWN**

- The client command wire for `USE` (a separate lane, see §4.3). No wire is decided here.
- The FND-02 representation of the child reference and the outcome codes (successor §19.5).

## 4. Decision

### 4.1 What is accepted, for the chest USE slice only

The following successor sections are accepted into the GAME-INTERACTION-01 contract, and only as
they apply to a player `USE` on a placed reward chest:

- **§5.1 child identity.** The chest `USE` is a first-level child of the `USE` command's root source
  occurrence. The interaction definition is the reward chest definition. The target is the placed
  chest instance, resolved by the server. There is no cascade and no ordinal for a `once` chest.
- **§5.5 typed edge.** The discriminator is `USE`. The chest occurrence key is the placed chest
  instance plus the claim identity, as D40 defines it. A different edge on the same chest is a
  different occurrence.
- **§17 CommandRef retry.** The same `CommandRef` returns the first outcome. A changed intent
  under the same `CommandRef` conflicts and is rejected. After a terminal rejection, a new attempt
  uses a new `CommandRef`. While the first is `PENDING`, a new `CommandRef` for the same intent is
  forbidden. A new GameSession reconciles the old occurrence first (§17.3).
- **§19.1 DUR-03.** DUR-03 owns durable value and ambiguity. The chest `USE` reaches it through
  the existing freeze, commit and reconcile path of the reward-claim MINT. The cause is the
  `USE` `CommandRef` plus the claim (D40). GAME-INTERACTION keeps correlation only, and an
  ambiguous result stays pending on the same DUR-03 transaction.

No successor text outside these four sections is accepted.

### 4.2 What stays PROPOSED / NONCANONICAL

Everything else in the successor: GAME-ABILITY effects (§19.2), movement and relocation (§19.3),
writable text (§19.4), client protocol registration (§19.5) as an accepted contract, nested
cascades, and the D37 and D38 world-object owners. Their blockers are unchanged.

### 4.3 Scope of the slice

- Plain `once` chests only. No keys, no cooldowns, no containers, no weight, no protocol change.
- The slice is the server-side `USE` trigger on a placed chest in the interaction runtime and its
  wiring to the CHEST-1 mint. A client command for `USE` is not part of it and stays in the
  control-wire lane ("Control-wire commands beyond step").
- The reward and refusal rules are unchanged: no room means nothing written and the player may try
  again (D41, D92); a second claim of a `once` row is refused with nothing written.

### 4.4 Live allocations

The row "Interaction Use orchestration" in
`docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md` becomes `READY` for the chest
USE slice only. The rest of the lane stays `WAITING_ARCHITECTURE`. The exact current and proposed
lines are in the companion file `live-allocations-row.txt`. `Oteryn: impl interaction` stays
read-only outside the slice. Each child still needs its own #162 allocation.

### 4.5 Pointer

A short pointer paragraph is added near the top of the successor document. It names this decision
and the four accepted sections. The text and its anchor are in `game-interaction-pointer.txt`.

## 5. Delivery (each child needs its own #162 allocation)

| Child | Scope | Depends on |
|---|---|---|
| D39 chest USE wiring | Server-side `USE` on a placed `once` chest, child identity per §5.1 and §5.5, CommandRef retry per §17, call into the CHEST-1 mint | this decision merged; CHEST-1 (#1190, merged) |

The CHEST-1 worker takes this child after the merge.

## 6. Rejected options

- **Accept the whole successor.** GAME-ABILITY, movement and writable text have no accepted owner.
  Accepting them would give implementation authority the successor cannot back.
- **Wait for D37, D38 and the successor to be accepted whole.** The chest needs none of them and
  the slice would stay blocked for unrelated reasons.
- **A new chest-only identity scheme.** D39 already chose the successor's identity and retry
  rules; a second scheme would fork the contract.
- **Cooldown, key or container chests in this child.** They need their own decisions (§5.1 of the
  reward chest file).

## 7. Decision test

- **Must decide now:** YES. The wiring child cannot start while the lane is `WAITING_ARCHITECTURE`.
- **Minimum sufficient:** four successor sections for one edge on one object kind.
- **Superseding evidence:** acceptance of the whole successor; the D37 and D38 owners; the FND-02
  gameplay payload registration.
- **Deliberately not decided:** the client `USE` wire, cooldown identity, keys, container
  rewards, weight, and every other interaction edge.

## 8. Handback

```yaml
result: RESOLVED_WITHOUT_NEW_OWNER_DECISION
source_escalation: "#162 comment 5884603451; ruling 5884689001 (item 2)"
owner_decisions: [D39, D40, D92]   # already taken
amends: docs/architecture/GAME-INTERACTION-01_SUCCESSOR_CHILD_IDENTITY_RETRY_CONTRACT_CANDIDATE.md   # §5.1, §5.5, §17, §19.1, chest USE slice only
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_D39_CHEST_USE_GAME_INTERACTION_AMENDMENT_DECISION_2026-09-29.md
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: true   # the D39 chest USE wiring child may be allocated
required_fresh_allocation: true
required_independent_review: "exact-head independent review (accepted sections, scope boundary, live allocation row)"
implementation_lanes: [D39-chest-use-wiring]
required_revalidation:
  - "same CommandRef returns the first outcome; a changed intent under the same CommandRef conflicts"
  - "a second USE of a once chest by the same character mints nothing and writes nothing"
  - "a USE edge and a different edge on the same target have different occurrence keys"
  - "an ambiguous DUR-03 result stays pending on the same transaction and never mints twice"
remaining_unknowns:
  - client USE command wire (control-wire lane)
  - FND-02 child reference and outcome code representation
next_action: "#162 validates this exact head, routes the independent review, integrates it, then allocates the D39 chest USE wiring child to the CHEST-1 worker."
```
