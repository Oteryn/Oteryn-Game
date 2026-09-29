# A13 Character build state (vocation and magic level)

- Decision: `A13-CHARACTER-BUILD-STATE-V1`
- Status: **CANDIDATE with owner decisions D150-D151 (§2)**. Acceptance requires exact-head
  validation, independent review and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.1)
- Answers: #162 comment 5896414182 (`ARCHITECTURE_ESCALATION_REQUIRED`, SPELL-CASTER-FACTS) and
  the owner question in 5896342127
- Ruling posted: #162 comment 5896480875
- Decides: the "Magic-level training" item in
  `OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md` §10
- Migration, runtime, content and production authority: **NONE**. Each lane in §5 changes code
  under its own allocation and review.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

`CasterState` needs a Character's vocation and magic level. Without them, the SPELL-D4 cast gate
stays closed, so every production cast is rejected (#1263). Three things must be settled:

- which store owns these facts;
- whether they join the CharacterRevision receipt chain;
- how a Character acquires them in V1.

## 2. Owner decisions

The owner confirmed these directly in this session on 2026-09-29.

| # | Decision | Owner choice |
|---|---|---|
| D150 | Vocation is acquired as in Global. A Character starts without a vocation and chooses it on Dawnport before leaving at level 8. There is no choice at creation and no dev-only profile. | "Jak Global: Dawnport" |
| D151 | Magic-level training from mana spent ships in V1, as in Global. It is not deferred. | "Wszystko od razu" |

## 3. Facts

**PROVEN** (main `48de3868`, #162 5896414182)

- `0009` `game_character_progression_state` stores only `level` and `total_experience`. No
  migration from `0001` to `0015` stores a vocation, magic level or mana spent.
- The Character aggregate and game-domain mutations are Game-owned. Platform may only project
  class or vocation (`CHARACTER_AUTHORITY_PLATFORM_BOUNDARY`).
- `CasterState` (`apps/game-server/src/spell/mod.rs`) reads `vocation` and `magic_level`. The
  `Vocation` enum has no "none" value.
- The spell cast contract §10 left magic-level training undecided.
- DEATH-0 (#1264, migration `0016`), STANCE-0 and H-1 all rewrite the same `0009` guard functions.

**UNKNOWN**, to be settled from Reference evidence by the lanes in §5:

- the Dawnport vocation-choice interaction and its departure rule;
- the magic-level formula and the per-vocation mana multipliers;
- the Global death loss of magic-level progress.

## 4. Decision

### 4.1 Storage

- **Table.** `game_character_build_state` holds one row per Character with three fields:
  - `vocation`: `none` or a vocation key;
  - `magic_level`;
  - `mana_spent`: progress toward the next magic level.
- **Creation.** Character creation inserts the row with `vocation = none`, `magic_level = 0` and
  `mana_spent = 0`. A Character without a row after creation is invalid, and admission fails
  closed.
- **Admission.** Admission loads the row into the live Character, and `CasterState` reads from it.

### 4.2 Receipt chain

- **Revisions and receipts.** Every change to the row is a CharacterRevision with a receipt of a
  new kind, `game_character_build_receipts`, in the `0009` chain.
  - The receipt holds the before and after values of the three fields.
  - It is immutable, and the chain cannot be truncated.
  - Writes are session-generation fenced.
- **Shared guard.** Build state joins the guard-rewrite chain of DEATH-0, STANCE-0 and H-1.
  Whichever of these migrations merges last carries every receipt kind in the shared guards. The
  migration takes the next free number when it is allocated.
- **Deferred guard.** The row must equal the latest build receipt.

### 4.3 No vocation

- `Vocation` gains a `None` value.
- Before Dawnport, a spell is castable only if its Reference vocation list includes "none".
- A class spell stays rejected until the Character has chosen that class.

### 4.4 Vocation choice (D150)

- **The choice.** The Dawnport vocation choice is one revision that carries:
  - a build receipt for the vocation change;
  - the A11 stance-prune receipt, when the new vocation invalidates the stance (A11 combined rule).
- **Content.** The Dawnport island, the choice interaction and the departure rule come from
  Reference evidence. They belong to their own content lane.
- **Promotion.** Promotion (level 20, Premium, NPC) uses the same vocation writer in a later slice.

### 4.5 Magic-level training (D151)

- **Accumulation.** `mana_spent` accumulates in the live session with each cast's mana cost.
- **Commits.** It is committed as a build receipt:
  - on every magic-level advance, which is always durable;
  - before the death receipt, in the same revision (§4.6);
  - at logout;
  - at a checkpoint of at most 60 seconds.
- **Crash loss.** A crash may lose at most one checkpoint of mana-spent progress. It never loses a
  magic level.
- **Formula.** The advance formula and the per-vocation multipliers come from Reference evidence,
  cited in W2b. `vocation = none` uses its own Reference multiplier.

### 4.6 Death

- Death loss of magic-level progress follows Global.
- The DEATH lane writes it as a build receipt in the same revision as the death receipt.
- The loss amounts come from Reference evidence in that lane.

## 5. Delivery

| Child | Scope | Depends on |
|---|---|---|
| CHAR-BUILD-1 | Migration and writers: the table, the receipt kind, creation insert, admission load, vocation writer, training commit, guards (§4.1-§4.2). It needs a persistence review. | this decision; the guard chain order |
| W2b | `CasterState` facts from build state, `Vocation::None`, training accumulation and the Reference formula | CHAR-BUILD-1 |
| DAWNPORT-1 | Dawnport content and the vocation-choice interaction | CHAR-BUILD-1 |
| DEATH ML loss | Magic-level progress loss at death | CHAR-BUILD-1, DEATH lane |

## 6. Rejected options

- **Choice at creation (control-plane option a).** The owner chose Global parity (D150).
- **A dev-only starter profile (option c).** It is not durable and would be replaced anyway.
- **Columns on `game_character_progression_state`.** Build state changes on its own events. A
  separate row and receipt kind keep the XP receipts unchanged, following the A11 pattern.
- **A receipt per cast.** It adds a durable write to every cast, which costs too much for
  mana-spent progress. The checkpoint bound (§4.5) caps the loss instead.

## 7. Decision test

- **Must decide now:** YES. Every production cast is rejected until `CasterState` has facts.
- **Minimum sufficient:** one row, one receipt kind and the Global acquisition path.
- **Superseding evidence:** a Reference rule that contradicts §4.3-§4.6.
- **Deliberately not decided:** promotion details, skills other than magic level, and
  vitals persistence.

## 8. Handback

```yaml
result: RESOLVED_WITH_OWNER_DECISIONS
source_escalation: "#162 5896414182 (SPELL-CASTER-FACTS) and 5896342127"
owner_decisions: [D150, D151]
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_A13_CHARACTER_BUILD_STATE_DECISION_2026-09-29.md
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: true   # CHAR-BUILD-1 may be allocated; W2b and DAWNPORT-1 follow it
required_fresh_allocation: true
required_independent_review: "CHAR-BUILD-1 persistence review (receipt kind, guards, fences)"
implementation_lanes: [CHAR-BUILD-1, W2b, DAWNPORT-1, DEATH ML loss]
remaining_unknowns:
  - Dawnport choice and departure details
  - magic-level formula and multipliers
  - death loss amounts
next_action: "#162 validates this exact head, routes the independent review, integrates it, then allocates CHAR-BUILD-1."
```
