# Account-scoped progress and #707 disposition decision

- Decision: `ACCOUNT-SCOPED-PROGRESS-AND-QUEST-707-DISPOSITION-V1`
- Status: **CANDIDATE with owner decisions D44-D49 taken (§2)**. Acceptance requires exact-head
  validation, independent review and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.1)
- Source: Issue #707 (`CONTENT-QUEST-01` candidate, r1-r10) and its owner requirement for
  account-scoped quests
- Owner decisions posted: #707 comment 5867311210, #162 comment 5867312726
- Admission baseline: `main@800e3eb`
- Related: #162, #220, #709; `OTERYN_QUEST_AUTHORING_FORMAT_V1.md` (D32-D38);
  `OTERYN_REWARD_CHEST_PLAYABLE_SLICE_DECISIONS_V1.md` (D39-D42); #1033
- Profile scope: **both profiles**. In `Oteryn Reference` this is a declared difference (§4.2)
- Runtime, migration, protocol and production authority: **NONE**
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

#707 proposed a full quest architecture (`CONTENT-QUEST-01`) and an owner requirement that quests
can be character-scoped or account-scoped. It was never accepted. Since then the owner accepted a
smaller quest model (D32-D38) and the reward-chest slice (D39-D42).

The owner stated the purpose of the account requirement (2026-09-28): today a new character on the
same account must redo every quest, which is "dramatically stupid"; but not everything can or
should be account-wide, and some things must stay per character.

Two questions follow:

1. What happens to the #707 candidate?
2. What exactly is shared across the characters of an account, and what stays per character?

## 2. Owner decisions

| # | Decision | Owner statement (2026-09-28) |
|---|---|---|
| D44 | #707 disposition: accept its durable principles now (§4.1), treat its quest graph and shared account progress as superseded, and defer the rest until a real quest needs it. The #707 candidate text and #709 evidence stay as evidence. | "biore a" |
| D45 | The account remembers quest completion; the character collects rewards (§4.2). This applies in the default game profile (`Oteryn Reference`) as a deliberate, declared difference, and in `Oteryn Evolved`. | "zrobimy to jako defoltowy profil gry czyli lamiemy tutaj reference ale swiadomie" |
| D46 | A quest grants its completion to the account by default. A quest may opt out. A quest whose unlock depends on an exclusive choice is opted out automatically. | "3. A" |
| D47 | Outfits, outfit addons, mounts and Tibia Store purchases belong to the account, not the character (§4.3, §4.5). | "przypisane do konta nie postaci, tak samo rzeczy zakupione w tibia store" |
| D48 | Achievements belong to the account; progress counters stay per character (§4.4). | Chosen option "Konto, liczniki per postać" |
| D49 | Portability. Cosmetic unlocks, achievements and Store unlocks apply on every world of the account, in both profiles. Quest completions apply on every world of the same profile family. A Store item can be claimed on any world of the profile family it was bought for, subject to item compatibility (§4.2-§4.5). | Chosen options "Wszystko na wszystkich", then "Kosmetyki wszędzie" and "Dowolny świat profilu" |

## 3. Facts

**PROVEN**

- `MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md` puts quest progress under Character persistence with
  Character scope. Its scope vocabulary already defines `Account`: shared by all characters of one
  account where product rules require it.
- D35: only the quest domain writes quest progress; progress is character state shared across
  channels. D42: `RewardClaim` is unique per (character, claim).
- #707 Q03 proposed `ACCOUNT_WITHIN_WORLD` progress with one serial account writer; Q07 proposed
  `ACCOUNT_ONCE`, `CHARACTER_ONCE` and `EACH_CHARACTER_ON_ACCOUNT_ONCE` claims. #707 is
  `PROPOSED_NONCANONICAL` (coordinator readback, #707 comment 5772322825).
- D34 chose staged missions over a typed quest graph; a graph is added only for a quest that staged
  missions cannot express.
- `game_durability_admission_account_guards` has `account_id` as its primary key and one
  `presence_character_id` (migration `0002_fresh_admission_authority.sql:70`): one character of an
  account is present at a time, across worlds.
- ADR-0010 §6: identity may be shared; gameplay value is world-scoped; "account-wide entitlements,
  cosmetics and achievements are not assumed portable" unless a later dedicated contract permits it.
- Product profile scope baseline §3.2: a player-observable difference may enter Reference when it
  is an explicitly accepted and disclosed Reference difference.
- DUR-03 §5.2: a new item location family needs its own typed owner and scope.
- `PROD-ENTITLEMENTS-01` (accepted, lifecycle-closed): Platform owns commercial entitlement
  lifecycle; Game owns gameplay delivery and enforcement. Its acceptance does not authorize
  entitlement runtime implementation, Premium/VIP activation or product benefits (architecture
  README).
- Store catalog owner decision (2026-09-28) §1 and §3: the catalogue is Game content, and coins and
  the purchase ledger are Platform's. Ownership of purchase delivery across the Game/Platform
  boundary, entitlement lifecycle and cross-boundary idempotency stay open under gap register §32.
- Reference evidence (TibiaWiki "Achievements", read 2026-09-28): 572 achievements, 204 secret,
  grades 1-4 worth 1-10 points. Achievements are character-based, but any character's page can
  display achievements unlocked by any character of the same account; points count per character.
  This is secondary evidence, not a target-cut proof.

**DERIVED**

- Sharing a completion fact is enough for the owner's purpose. Sharing in-progress quest state is
  not needed and would require the #707 serial account writer.
- A write-once fact keyed uniquely per account needs no cross-character ordering: two inserts of the
  same fact produce one row.

**UNKNOWN**

- Which Reference-target outfits, addons and mounts carry gameplay effects; exact Reference Store
  catalogue. Neither is decided here.

## 4. Decision

### 4.1 #707 disposition (D44)

**Accepted now as principles.** No code follows from them alone; each binds the first
implementation it applies to.

| # | Principle | #707 source |
|---|---|---|
| P1 | One source occurrence advances a quest objective at most once; a replay returns the first disposition. | Q04 |
| P2 | An active quest is pinned to the content revision it started under; a changed revision needs explicit migration under DUR-04. | Q17 |
| P3 | Disconnect, reconnect or restart never completes, fails or resets a quest; a death reacts only as the quest declares. | Q12 |
| P4 | Abandoning a quest never revokes an earned completion, claim or choice. | Q14 |
| P5 | A reward is settled through DUR-03 as one transaction; Quest never mints. | Q09; D40-D42; #1033 |

**Superseded.**

- Q02 typed quest graph: superseded by D34 (staged missions).
- Q03 `ACCOUNT_WITHIN_WORLD` shared progress and Q07/Q08 account claim presets: superseded by the
  completion-fact model (§4.2). Per-character claims stay under D42.

**Deferred until the first quest that needs them.**

- Party, guild and world quest scope.
- Schedules and resets beyond the D42 cooldown.
- The Encounter contract (D27 already limits encounters to emitting outcomes).
- The Studio quest editor.
- The #707 test catalogue as an up-front gate. Each case enters with the code it covers.

**Retained as evidence.** The #707 r1-r10 texts and the #709 Annihilator evidence.

### 4.2 Account quest completion (D45, D46, D49)

- **Fact.** When a character completes a quest that grants account completion, the same DUR
  transaction also inserts `AccountQuestCompletion(account_id, profile_family, quest_key)` if it is
  absent. It records the completing character and time. It is never updated or deleted.
- **Readers.** A condition that asks "has the character completed quest Q" is satisfied when the
  character completed Q or the account has the fact for Q in the world's profile family. Such
  conditions include doors, teleports, travel routes, NPC services, boss entry and quest
  prerequisites.
- **Current policy gates the fact.** A reader accepts the account fact only while both of these
  hold at evaluation time:
  - the world's active ruleset enables account quest completion;
  - the applicable revision of Q declares `account_completion: grant`.
  The applicable revision is the acting character's pinned revision while Q is active for that
  character (P2), and otherwise Q's active revision in the world's content generation. A policy
  change reaches a character's active quest only through the explicit DUR-04 migration that P2
  requires. Otherwise only the character's own completion counts. When a quest switches from
  `grant` to `none`, or a world disables the policy, existing facts stop satisfying conditions.
  They stay as history and apply again if the policy is re-enabled. A fact is never rewritten or
  deleted to change eligibility.
- **In-progress state stays per character** (D35 unchanged). Nothing is shared before completion.
- **Limits.**
  1. The fact replaces only the "completed the quest" condition. Level, vocation, premium and item
     requirements are still checked for the acting character.
  2. It does not skip an encounter. A reward behind an encounter still requires the encounter.
  3. A quest whose unlock depends on an exclusive choice is character-only (D46). The content
     validator rejects `account_completion: grant` for it.
  4. It does not create items. A door that opens with a key item still needs the key.
  5. It covers characters created later and survives deletion of the completing character.
  6. Rewards stay per character: XP only for doing the quest; items through the character's own
     `RewardClaim` (D42); repeatable-quest cooldowns per character.
- **Content.** Each quest declares `account_completion: grant | none`, default `grant` (D46).
- **Profile.** In `Oteryn Reference` this is a declared difference (`DECLARED_DIFFERENCE`, parity
  manifest contract), with this decision as its accepted reference. It is a versioned ruleset
  policy, not a process switch.
- **Portability.** The fact applies on every world of the same profile family. A quest that
  behaves differently in another profile must use a different `quest_key`.

### 4.3 Account cosmetic unlocks (D47, D49)

- Outfits, outfit addons and mounts earned in gameplay are `AccountUnlock(account_id, unlock_key)`
  facts: write-once, never revoked by character deletion.
- Store cosmetic unlocks share this account scope and portability (D47, D49), but not this fact
  model. Their delivery, usability, refund, revocation and expiry follow the Platform-owned
  entitlement lifecycle (`PROD-ENTITLEMENTS-01` consumer contract §2.1) and the open §32 decision
  (§4.5).
- They apply on every world of the account, in both profiles. This decision is the dedicated
  permission ADR-0010 §6 requires for cosmetics.
- Portability covers the appearance only. Any gameplay effect of a cosmetic (for example a mount
  speed bonus) belongs to the world's profile ruleset and is evaluated per world.
- Where the Reference target binds an unlock to one character, this is a declared difference.

### 4.4 Achievements (D48, D49)

- An achievement is an `AccountAchievement(account_id, achievement_key)` fact: write-once. It
  records the character and earning time of the grant request it was derived from. Because
  requests may be consumed out of order, that is not guaranteed to be the account's earliest
  earner, and no ranking or reward may depend on it.
- The character event that earns it records a durable achievement grant request in its own fenced
  transaction. This keeps the handoff that the reward chest slice already uses
  (`OTERYN_REWARD_CHEST_PLAYABLE_SLICE_DECISIONS_V1.md` §6). The Achievement owner turns a committed
  request into the fact, idempotently per `(account_id, achievement_key)`, and may do so after the
  session ends. Whether it consumes requests in the same transaction or later is for its owner
  contract.
- Achievement points count once per account; rankings by achievement points rank accounts and are
  shown on their characters.
- Exclusive-choice achievements (for example Marid and Efreet ally) can each be earned by a
  different character of the account.
- Progress counters (for example fish caught) stay per character. The achievement is granted when
  any character reaches the threshold.
- The vocation promotion achievement (GAME-CHAR-01) is character state, not an achievement in this
  sense.
- An achievement or its points carry no gameplay value. Giving them one needs a separate decision.
- They apply on every world of the account, in both profiles (ADR-0010 §6 permission). This is a
  declared difference from the Reference target, which counts achievements per character.

### 4.5 Store purchases (D47, D49)

This section fixes scope and portability only (D47, D49). It does **not** decide who owns purchase
delivery across the Game/Platform boundary. That ownership, entitlement identity and lifecycle, and
idempotent delivery across a boundary failure stay open under gap register §32, as
`OTERYN_STORE_CATALOG_OWNER_DECISION_2026-09-28.md` §1 and §3 record.

- A Store cosmetic unlock is account-scoped and portable like §4.3 while its entitlement is
  usable. Whether Game keeps a delivery record, and how refund, revocation and expiry gate it, is
  for the §32 lifecycle decision. A Game record is evidence about Platform authority, never a second
  commercial authority (`PROD-ENTITLEMENTS-01` consumer contract §2.1).
- A purchased Store item or consumable waits in one account inbox, bound to the profile family it
  was bought for. The inbox holds no `ItemInstance`, so no new DUR-03 location family is
  introduced. Whether the inbox line is a Game record or a Platform entitlement line that Game
  reads is for the §32 delivery decision.
- A character of the account may claim a line on any world of that profile family whose content
  is compatible with it. Whoever owns the line, the item enters the world only as a DUR-03 MINT
  into that character's `CharacterInventory`, with the line as source cause: idempotent per line
  and unit, fenced like the reward claim (#1033). After the claim the item is an ordinary
  world-scoped item.
- The line records the item-definition provenance it was sold under (item key and content
  revision). The claim validates it against the target world's active content under DUR-03 §46.
  If the definition is absent or incompatible there, the claim fails closed with nothing minted.
  The line stays unclaimed and claimable on a compatible world or after an explicit migration. The
  item is never silently reinterpreted.
- Refund, revocation or expiry of an unclaimed line, and any correction after a claim, follow the
  §32 lifecycle decision.
- Activation waits for an explicit product decision that authorizes entitlement delivery under
  `PROD-ENTITLEMENTS-01`; accepting that contract alone does not.

### 4.6 Fencing and concurrency

- Every account fact earned by character gameplay (§4.2-§4.4) comes from a fenced character event,
  under that character's full session-generation fence:
  - quest completions and gameplay cosmetic unlocks are inserted inside the event's own DUR
    transaction;
  - an achievement is derived from the durable grant request committed in that transaction (§4.4).
  No such fact is written without a committed character event. Store delivery is outside this rule
  and waits for §32 (§4.5).
- The facts are append-only sets with a unique key. A duplicate insert changes nothing, so a replay
  or two concurrent inserts leave one row, and the first committed insert keeps its provenance
  (for achievements, the request it was derived from, §4.4).
- In the current topology the account guard allows one present character per account
  (migration 0002), so cross-character races do not occur; the unique key makes them harmless if
  topology changes.

### 4.7 Scope matrix

`MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md` gains rows for account quest completion, account cosmetic
unlocks earned in gameplay, account achievements and Store delivery (this pull request). The quest
progress row stays Character.

## 5. Rejected options

- **Accept the whole #707 package.** It contradicts D34 and front-loads a graph, an Encounter
  contract, a Studio editor and 80 gate tests that no current quest needs.
- **Leave #707 open.** Workers keep reading a proposal without a decision.
- **Shared in-progress account progress (#707 Q03).** It needs one serial writer across characters
  and mid-quest conflict rules, for no benefit over a completion fact.
- **Achievements per character.** It contradicts D45: the quest completion would transfer while
  the achievement for that quest would not.
- **Quest completions across profiles.** ADR-0010 §6 forbids moving gameplay value between
  profiles, and access to content is gameplay value.

## 6. Decision test

- **Must decide now:** YES. Quest content and the first quest-state store are being authored
  (D32-D38, reward chest). Adding account completion after character-only data is entrenched is
  costly.
- **Minimum sufficient:** append-only account facts plus one condition rule. No shared progress
  writer, no new DUR-03 location family.
- **Reversibility:** the ruleset policy can be disabled for a future pure-Reference world, and a
  quest can switch to `none`. Readers then ignore existing facts (§4.2), which stay as history.
- **Superseding evidence:** a quest that needs shared in-progress state; a cosmetic whose gameplay
  effect cannot be separated from its appearance; a Platform entitlement contract that forbids a
  Store inbox.
- **Deliberately not decided:** physical schema; the Achievement domain owner and catalogue; quest
  reset schedules; party or guild scope; Store catalogue and prices; protocol and client.

## 7. Handback

```yaml
result: RESOLVED_WITH_OWNER_DECISIONS
source_escalation: "#707 CONTENT-QUEST-01 candidate and its account-scope requirement"
owner_decisions: [D44, D45, D46, D47, D48, D49]
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_ACCOUNT_PROGRESS_AND_QUEST_707_DISPOSITION_DECISION_2026-09-28.md
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: false   # until this decision is protected-integrated
required_fresh_allocation: true
required_independent_review: "exact-head independent review (persistence scope, profile boundary)"
follow_up_owners:
  - "Quest authoring format: account_completion declaration, D46 default and validator rule"
  - "Achievement owner contract: AccountAchievement catalogue and grant path"
  - "Character appearance owner: AccountUnlock for outfits, addons, mounts"
  - "Gap register §32 delivery decision: Game/Platform ownership of Store inbox lines and cosmetic unlocks, their refund, revocation and expiry, and cross-boundary idempotency, then an explicit product activation decision under PROD-ENTITLEMENTS-01"
  - "Reference parity manifest: record the D45, D47, D48 declared differences"
required_revalidation:
  - "the first account-fact migration proves: a fact is inserted only inside a fenced character event, or for achievements derived only from a grant request committed in one; duplicate insert leaves one row and keeps the first committed insert's provenance; a stale character fence writes no fact; a condition reads character-or-account completion; level and item requirements are still checked per character; an exclusive-choice quest cannot grant account completion; an existing fact does not satisfy a condition when the quest now declares none or the world disables the policy; a character with the quest active is evaluated against its pinned revision until migrated"
remaining_unknowns:
  - Reference-target cosmetic gameplay effects
  - Achievement domain owner
closeout: "#162 may close #707 as resolved by this decision once it is protected-integrated"
next_action: "#162 validates this exact head, routes the independent review and integrates it through the governed Merge Queue."
```
