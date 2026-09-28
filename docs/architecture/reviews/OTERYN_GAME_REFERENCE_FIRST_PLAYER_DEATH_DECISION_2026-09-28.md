# Reference first player death decision

- Decision: `REFERENCE-FIRST-PLAYER-DEATH-V1`
- Status: **CANDIDATE with owner decisions D58-D60 and D62-D68 taken (§2)**. Acceptance requires
  exact-head validation, independent review and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.1)
- Profile: `Oteryn Reference` (target Global Tibia at 2026-09-27, D33)
- Amends: `OTERYN_REFERENCE_DEATH_XP_SPAN_OWNER_BASELINE_2026-09-09.md` difference 1 (D58). Its
  difference 2 (no skill or magic-level loss) stays.
- Retires on implementation: D54 (player HP floor at 1) from the GAME-AI first creature slice
- Owner decisions posted: #162 5871032954 (D58-D65), 5871151324 (D66-D67); D68 in §2
- Admission baseline: `main@0a3d795992a68f57c595baf3f5ced87d1224a60c`
- Runtime, migration, protocol and production authority: **NONE**
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

Once creatures can damage players, a player must be able to die. What happens, exactly, in the
Reference profile's first player death, and how is it made durable and restart-safe?

## 2. Owner decisions

| # | Decision | Owner choice (2026-09-28) |
|---|---|---|
| D58 | XP loss follows Global: `((L+50)/100) × 50 × (L² − 5L + 8)` before reductions, which equals `((L+50)/100) × (XPThreshold(L) − XPThreshold(L−1))`. Reductions: promotion −30% and −8% per regular blessing, additive. Skills and magic level never drop. | "Dokładnie jak Global" |
| D59 | Levels 1-23 use the D58 formula instead of Global's 10% of total experience. | "Ten sam wzór od lvl 1" |
| D60 | PvE only: Twist of Fate, the unfair-fight reduction, skulls, Adventurer's Blessing and Retro rules are deferred to PvP. | "Odłożyć" |
| D62 | Blessings protect containers and equipment on one ladder: with 0/1/2/3/4/5+ blessings a backpack drops with 100/70/45/25/10/0% and each equipped item with 10/7/4.5/2.5/1/0%. | "Blessy chronią plecak" |
| D63 | Respawn at the home temple with full HP and mana; no protection window. | "Pełne HP i mana" |
| D64 | Death Redemption is deferred to the Store (gap register §32). | "Później" |
| D65 | No item loss up to level 8 or without a vocation; the Bless Charm reduction is deferred. | "Wyjątki tak, charm później" |
| D66 | Promotion keeps its Premium requirement: the −30% applies only if the character is promoted and Premium is current at the death (Premium activation decision §4.2, D76). | "Czeka na Premium" |
| D67 | The 3 Premium blessings keep their Premium requirement: buying them needs current Premium (Premium activation decision §4.4); held blessings count until death. | "Czekają na Premium" |
| D68 | The XP loss rounds down (in the player's favour) until Global evidence proves otherwise. | "W dół, na korzyść gracza" |

## 3. Facts

**PROVEN** (main `0a3d795992a68f57c595baf3f5ced87d1224a60c`, and the research sources in #162 5871032954)

- The domain calculator already has `ApplyDeathExperienceLoss`
  (`apps/game-server/src/domain/progression.rs:52-56`, `:133-158`). It computes the 2026-09-09
  basis (the L→L+1 span times one fixed ratio) and rejects experience underflow. P03 commits
  experience changes under the Character fence (`durability/character_progression.rs`).
- Global (tibia.com manual; TibiaWiki Death, Blessings, Amulet of Loss, Temple):
  - the level ≥ 24 loss above; −30% for promotion and −8% per regular blessing;
  - item loss: equipment 10% per item, containers with their contents, the blessing ladder
    30/55/75/90/100% protection; no item loss up to level 8 or without a vocation;
  - Amulet of Loss prevents all item loss for a character with fewer than 5 blessings, unless
    red- or black-skulled, and is consumed on death;
  - lost items stay with the body and any player can loot them;
  - all regular blessings are consumed on death (PvE);
  - respawn in the home temple.
- The owner's tibia.com manual snapshot (`imports/official/tibia-com/2026-09-28/facts.json`,
  `characters.5-1-characters.6` and `.11`, #1125): promotion reduces the loss when dying, and only
  Premium players can promote.
- The first Reference cut stays reproducible: later Global changes need a new decision (D33).

**CONFLICT resolved by owner decision:** containers (D62), rounding (D68).

**UNKNOWN**

- The Global low-level no-loss exemption after the 2025-10-21 Newhaven update (level 6 on the
  mainland): to be re-read at the target date before implementation; a verified exemption applies.
- Home-town and temple positions: content inputs.

## 4. Decision

### 4.1 Trigger

A player dies when a committed effect takes their runtime HP to 0 (SPELL-D2 vitals). When this
decision is implemented, D54's floor at 1 is removed. Only PvE deaths are in scope (D60). A death
with any PvP contribution is out of scope and keeps the D54 floor until the PvP death decision.

### 4.2 Death occurrence

- At the lethal commit, the Channel owner mints one server UUIDv7 `PlayerDeathOccurrence` and
  records it in the lethal effect's result. It is the idempotency key of every consequence.
- The player actor leaves playable control at once and takes no further input until respawn.

### 4.3 Durable consequences: one Character transaction

Off the owner lane (the owner never waits on the database), one DUR transaction under the
character's full session-generation fence, keyed by the death occurrence, commits all of:

1. **XP loss** (D58, D59, D66, D67, D68):
   `loss = floor( ((L+50)/100) × 50 × (L² − 5L + 8) × (1 − 0.08 × blessings − 0.30 × promoted) )`,
   where `promoted` is 1 only if the character is promoted and Premium is current at the death
   transaction (Premium activation decision §4.2, D76),
   capped so experience never drops below 0. Level follows the new experience (Global
   delevelling). Skills and magic level are unchanged.
2. **Blessings:** every regular blessing is consumed.
3. **Amulet of Loss:** if worn and the character has fewer than 5 blessings, it is consumed and
   no item is lost.
4. **Lost-item set:** otherwise, and unless D65 exempts the character, each equipped item and the
   backpack are drawn against the D62 ladder with an RNG stream bound to the death occurrence.
   A lost container is lost with its contents. The set is recorded in the transaction; it is not
   yet moved.
5. **Respawn position:** the home temple of the character's home town.

A retry with the same occurrence returns the first result. The calculator's
`ApplyDeathExperienceLoss` changes to this formula: the L−1→L span, the level factor, the
reductions and floor rounding. The skill and magic-level families stay untouched.

### 4.4 Item drops

- Each lost item moves in its own DUR-03 TRANSFER from `CharacterInventory` to Ground at the death
  cell, associated with the player's corpse. Its source cause is (death occurrence, item instance).
  Each transfer is idempotent per cause.
- A container moves as one item with its contents, as DUR-03 container semantics allow; until
  container expansion is admitted (DUR03-RL-05 = 0), a lost backpack with contents waits for that
  admission, and the implementation must not split it.
- **Restart rule** (as D52 for creatures): a lost item whose transfer has not committed when the
  runtime scope generation ends stays in the character's inventory. It is never duplicated and
  never lost twice.
- The corpse is a runtime projection with a lootable association. Any player may pick items up
  through the ordinary pickup path.

### 4.5 Respawn

After the Character transaction commits, the player respawns at the recorded temple with full HP
and mana (D63), as a new runtime actor. No protection window applies. If the transaction's outcome
is ambiguous, the same occurrence is reconciled before respawn. If the generation ends before it
commits, no durable death happened: the character recovers normally, and HP starts at the maximum
for a new actor (SPELL-D2).

### 4.6 Blessings

- Blessings are durable Character state: the set of held regular blessings, with provenance.
- The 4 free regular blessings are open to everyone; the 3 Premium blessings need current Premium to
  buy (D67). Buying them (NPC service, level-based price) is a
  separate child that consumes this state; prices come from the target-date sources.

### 4.7 Content and policy inputs

- The XP threshold table, the death policy revision (D58-D68) and the declared-difference revision
  are bound into the progression policy, as the calculator already requires.
- Temple positions per home town, blessing definitions and the Amulet of Loss item are content.

## 5. Delivery (each child needs its own #162 allocation)

| Child | Scope | Depends on |
|---|---|---|
| DEATH-1 | Calculator change (§4.3.1) and the durable Character death transaction | progression readiness; P03 |
| DEATH-2 | Death trigger, occurrence and respawn in the Channel owner; removal of the D54 floor | spell P3b-2 vitals; AI-4 |
| DEATH-3 | Item drops through DUR-03 TRANSFER with corpse association | DUR-03 TRANSFER (B3); container admission for backpacks |
| DEATH-4 | Blessing state and the NPC blessing service | NPC service owner |
| E (Combat) | Client observation of death and respawn | protocol lane |

## 6. Rejected options

- **XP and items in one distributed transaction.** DUR-03 keeps one-item transactions; the
  occurrence key makes the parts idempotent instead.
- **Keep the old L→L+1 basis.** The owner chose Global exactly (D58).
- **Lose items on a restart before their transfer.** It could lose value twice or not at all
  depending on timing; staying in the inventory is safe and never duplicates.

## 7. Decision test

- **Must decide now:** YES. The creature slice (#1110) makes player damage real, and D54 is a
  stopgap.
- **Minimum sufficient:** PvE only, blessings and promotion as the Premium decision allows, no
  redemption, no charms; the
  existing calculator and DUR-03 transfer paths.
- **Superseding evidence:** Premium activation; the PvP death decision; container expansion; a
  Global change after 2026-09-27.
- **Deliberately not decided:** PvP death, skulls, Twist of Fate, Adventurer's Blessing, Retro,
  Death Redemption, charms, blessing prices, the Evolved death.

## 8. Handback

```yaml
result: RESOLVED_WITH_OWNER_DECISIONS
owner_decisions: [D58, D59, D60, D62, D63, D64, D65, D66, D67, D68]
amends: docs/architecture/OTERYN_REFERENCE_DEATH_XP_SPAN_OWNER_BASELINE_2026-09-09.md   # difference 1 only
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_REFERENCE_FIRST_PLAYER_DEATH_DECISION_2026-09-28.md
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: false
required_fresh_allocation: true
required_independent_review: "exact-head independent review (Character durability, DUR-03 transfers, restart safety)"
implementation_lanes: [DEATH-1, DEATH-2, DEATH-3, DEATH-4]
required_revalidation:
  - "DEATH-1: the loss matches the D58 formula at levels 1, 23, 24, 100 and 500 with 0 to 4 blessings; floor rounding; experience never below 0; skills and magic level unchanged; a retry returns the first result; a stale fence writes nothing"
  - "DEATH-2: HP 0 from a creature starts one death; no input until respawn; respawn at the temple with full HP and mana; a PvP-contributed death is out of scope"
  - "DEATH-3: the drop ladder per blessing count; Amulet of Loss consumed and no loss with fewer than 5 blessings; exemptions up to level 8 and without a vocation; a pending transfer at a generation change leaves the item in the inventory; no duplication"
remaining_unknowns:
  - the Newhaven low-level exemption at the target date
  - temple positions (content)
next_action: "#162 validates this exact head, routes the independent review, integrates it, then allocates DEATH-1 after progression readiness."
```
