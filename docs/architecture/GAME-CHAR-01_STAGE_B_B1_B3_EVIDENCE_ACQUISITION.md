# GAME-CHAR-01 — Stage B B1-B3 Evidence Acquisition

- Status: **EVIDENCE ACQUISITION / NONBINDING / NOT ACCEPTED**
- Date: 2026-08-11
- Gate: `GAME-CHAR-01` Stage B
- Reference target: Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance change boundary
- Scope: B1 naming, B2 deletion/quota, B3 creation/starter only
- Trusted repository base: `blakinio/Oteryn-v2@1411994c70abbf065273c0502c88413b61ca5ca0`
- Does not authorize: Stage-B acceptance, overall GAME-CHAR acceptance, runtime/client/protocol/persistence/content implementation, physical schema or intentional Reference differences

## 1. Executive result

This bounded pass makes meaningful progress on **B1 naming** and **B3 creation/starter**, but does not close either blocker. **B2 deletion/quota remains materially unresolved for the July-28 target** beyond the strong 25-active-character continuity evidence.

No owner decision is useful yet; the remaining gaps are evidence questions.

### Result by blocker

- **B1 naming — PARTIALLY REDUCED:** Tibia-wide uniqueness is explicit historical primary evidence. Six-month old-name lookup after rename has strong primary before/after continuity. Exact July-28 normalization, 29-letter continuity, complete permitted repertoire and deleted-name recycling remain unresolved.
- **B2 deletion/quota — LARGELY UNCHANGED:** current official documentation states 60-day reversible deletion, total 30 active+scheduled and at-least-six-month deleted-name hold, but this pass found no provenance-clear July-28 historical snapshot/change chronology for those exact values. They remain `UNKNOWN` for the target. Active limit 25 remains a strong continuity candidate.
- **B3 creation/starter — PARTIALLY REDUCED:** official 2025/2026 chronology establishes Newhaven as the new entry area, planned/released vocation-selection flow and the later Targuna continuation before July 28. Exact starter stats/items/home state and exact target-era skip-tutorial level remain unresolved.

## 2. Evidence rule

Accepted evidence states remain:

- `PROVEN`
- `OBSERVED`
- `DERIVED`
- `UNKNOWN`
- `CONFLICT`
- `DECLARED_DIFFERENCE`

Current official documentation is primary **current** evidence, not automatic July-28 proof. Pre-release official teasers establish planned semantics; when combined with a later production release/current official evidence and no material conflict they may support `DERIVED` continuity, but they are not relabelled as direct target `PROVEN` solely because the feature subsequently launched.

Likewise:

```text
no discovered change note
!= proof that a rule did not change
```

The available search also attempted to find provenance-clear archived official account-manual snapshots near July 28. None were resolved through the available search path. That is an evidence limitation, not evidence that current B2 values either matched or differed from the target.

## 3. B1 — Tibia-wide uniqueness

Official April 2, 2008 news explicitly described Tibia's naming policy as one in which every character across all game worlds had a unique name, contrasting this with per-server uniqueness in other games.

Primary source:

- `https://www.tibia.com/news/?id=708&subtopic=latestnews` — **Tibia Character Names - Now and in the Future**, 2008-04-02.

Current official creation/name-change surfaces still require a requested name to be available before the operation succeeds.

### Classification

**`DERIVED` strong continuity candidate:** Reference character names use a Tibia-wide/global namespace rather than independent per-world namespaces.

### Still unresolved

This does not define the implementation-grade comparison function:

- Unicode normalization;
- case folding;
- whitespace normalization;
- exact permitted repertoire;
- system/reserved namespace handling;
- historical alias/deleted-name reservation interactions.

Final Character Authority/database uniqueness mechanics remain not freeze-ready.

## 4. B1 — technical name-format restrictions

Official April 8, 2008 patch notes introduced/confirmed technical creation restrictions including:

- no names with more than three words;
- no special characters such as a hyphen.

Primary source:

- `https://www.tibia.com/news/?id=716&subtopic=newsarchive` — **Minor Patch Released - New Tibia Rules Valid Now!**, 2008-04-08.

Current official creation documentation additionally states:

- maximum 29 letters;
- no numbers;
- no special characters;
- some words/letter combinations disallowed;
- Tibia Rules apply.

### Classification

- existence of long-lived technical name-format restrictions: **strong continuity evidence**;
- exact July-28 maximum `29`: `UNKNOWN` continuity;
- exact parser/repertoire/canonicalization: `UNKNOWN`;
- exact restricted/reserved-pattern revision: `UNKNOWN`.

The historical rule family is useful evidence, but it does not reconstruct the 2026 validator byte-for-byte.

## 5. B1 — rename history/alias semantics

Official February 19, 2010 introduction of Character Name Change states that after rename:

- the old name remains listed on the character page for six months;
- the character can be found through the old name for six months.

Primary source:

- `https://www.tibia.com/news/?id=1240&subtopic=newsarchive` — **Release of New Extra Services**, 2010-02-19.

Current official Store/product documentation still states that the former name remains usable for six months in the Lost Account Interface or Characters search.

Current source:

- `https://www.tibia.com/gameguides/?section=products&subtopic=manual` — Character Name Change.

### Classification

**`DERIVED` strong continuity candidate:** six-month searchable old-name/history behavior after rename.

### Architecture consequence

Current authoritative name, old-name searchable alias/history and deleted-name reservation are separate semantics. CharacterId remains stable through rename under Stage A.

## 6. B1 — deleted-name recycling remains different and unresolved

Current account documentation says names of deleted characters cannot be selected for new characters for **at least six months**. Current official support material also warns that a deleted character's former name may remain unavailable for several months or even years depending on factors.

This is not equivalent to the fixed six-month old-name lookup period after rename.

### Classification

- current deleted-name hold `>=6 months`: current-primary fact;
- exact July-28 continuity: `UNKNOWN`;
- exact release algorithm/timing: `UNKNOWN`;
- model `retired_at + 6 months => available`: **not supported**.

## 7. B1 result

**Partially reduced, not closed.**

Strong candidates:

1. Tibia-wide/global name uniqueness;
2. long-lived technical format-restriction family;
3. six-month old-name lookup/history after rename.

Still blocked:

- exact normalization/case/space/repertoire;
- exact 29-letter target continuity;
- restricted/reserved pattern authority;
- deleted-name release timing/algorithm;
- complete rename collision/history rules.

## 8. B2 — active-character quota

Official February 24, 2025 Monk product-management material announced an increase from 20 to 25 characters per account. Later live Monk/balancing material confirmed the account limit had been increased from 20 to 25. Current manuals still state 25 active characters.

Primary sources include:

- `https://www.tibia.com/news/?id=8260&subtopic=latestnews` — **Monk: Product Management Insight Part 2**, 2025-02-24;
- `https://www.tibia.com/news/?id=8307&subtopic=newsarchive` — live changes confirming the increase.

### Classification

**`DERIVED` strong continuity candidate:** maximum 25 active characters at the July-28 target.

## 9. B2 — current deletion/undelete contract

Current official account manual states:

- deletion is reversible for 60 days / two months;
- Undelete Character cancels deletion during that period;
- final deletion is irreversible;
- scheduled-for-deletion characters do not count as active;
- active limit is 25;
- active + scheduled-for-deletion total may not exceed 30;
- undelete can be blocked by the 25-active limit;
- deleted names cannot be selected for at least six months.

Current source:

- `https://www.tibia.com/gameguides/?section=accounts&subtopic=manual`.

### Historical-target search result

The targeted official-news/manual/archive search did not resolve a dated primary July-28 snapshot or introduction/change record for:

- 60-day manual deletion grace;
- total 30;
- exact undelete/quota interaction;
- deleted-name minimum hold.

### July-28 classification

- deletion grace = 60 days: `UNKNOWN` continuity;
- scheduled deletion excluded from active quota: `UNKNOWN` continuity;
- total 30: `UNKNOWN` continuity;
- undelete interaction: `UNKNOWN` continuity;
- deleted-name hold/release: `UNKNOWN` continuity.

No contrary change note was found, but that does not promote continuity to proof.

## 10. B2 interaction with Stage A

The target-value gap does not invalidate the accepted baseline-neutral lifecycle:

```text
ACTIVE
-> DELETION_SCHEDULED
-> RETIRED
```

Stage A already safely captures reversible nonterminal intent, terminal retirement, CharacterId non-reuse and privacy-erasure separation. B2 blocks numeric/product-policy values and exact Reference workflow, not the semantic lifecycle boundary.

## 11. B2 result

**Largely unchanged.**

Strong candidate:

- 25 active characters.

Still `UNKNOWN` for July 28:

- 60-day grace;
- total 30;
- exact undelete constraints;
- exact deleted-name hold/release.

## 12. B3 — Newhaven release chronology

Official October 2025 material defines the Newhaven redesign before the selected target.

October 15, 2025 pre-release material describes:

- a redesigned tutorial;
- post-tutorial choice to Rookgaard or portal to Newhaven;
- vocation selection in Newhaven through a pop-up;
- Newhaven's early progression services/areas;
- a level-8/email-confirmation requirement on the western exit.

Primary source:

- `https://www.tibia.com/news/?id=8543&subtopic=newsarchive` — **A New Helping Hand II**, 2025-10-15.

October 8, 2025 pre-release material states that players would no longer be able to change vocation after their initial choice or return to Newhaven after leaving it.

Primary source:

- `https://www.tibia.com/news/?id=8541&subtopic=newsarchive` — **A New Helping Hand**, 2025-10-08.

October 21, 2025 production news states that Newhaven opened as Tibia's new entry point and Dawnport was retired.

Primary source:

- `https://www.tibia.com/news/?id=8553&subtopic=newsarchive`.

Current starting documentation still places skip-tutorial characters in Newhaven ready to select a vocation.

### Classification

- Newhaven live as entry point before July 28: `PROVEN` by dated production release;
- tutorial/Newhaven/vocation-selection model: **`DERIVED` strong continuity** from pre-release primary + production launch + current primary, not direct July-28 snapshot proof;
- no casual re-vocation/return-to-Newhaven rule: **`DERIVED` candidate**, not promoted to target `PROVEN` solely from the teaser.

This correction prevents planned pre-release semantics from being overstated as direct target observation.

## 13. B3 — Targuna superseded the old level-8+ route before target

Official March 17, 2026 news states that level-8+ characters would no longer travel directly from Newhaven to Thais Peninsula and that all characters, including monks, would continue to Targuna. Subsequent March/April live-fix chronology refers to the Newhaven/Targuna production flow, establishing it as live before July 28.

Primary source:

- `https://www.tibia.com/news/?id=8733&subtopic=newsarchive` — **Greater Lessons for Young Tibians**, 2026-03-17.

The currently indexed official Quick Start still says Newhaven exits to Thais Peninsula and monks to Blue Valley.

Current source:

- `https://www.tibia.com/gameguides/?subtopic=quickstart`.

### Classification

- Targuna continuation live before target: `PROVEN` by dated release/follow-up chronology;
- current Quick Start Thais/Blue Valley wording: `CONFLICT / stale official documentation` for the July-28 route.

Dated production chronology outranks the stale paragraph for target reconstruction.

## 14. B3 — current skip-tutorial level-2 state

Current starting manual says players eligible to skip the tutorial start at **level 2 in Newhaven**, ready to select a vocation.

Current source:

- `https://www.tibia.com/gameguides/?section=starting&subtopic=manual`.

The targeted historical search did not resolve a dated 2025/2026 release note explicitly fixing the same `skip tutorial -> level 2` value.

### Classification

- current level-2 skip rule: current-primary fact;
- exact July-28 continuity: `UNKNOWN`.

## 15. B3 — exact starter template remains unresolved

The historical material establishes tutorial/Newhaven architecture but not a complete July-28 starter template including:

- exact initial level for every creation path;
- initial HP/mana/capacity/skills;
- starter inventory/equipment;
- home town/citizenship/respawn state;
- exact Rookgaard/Newhaven eligibility by account state;
- web/client creation retry/error semantics;
- all account-confirmation constraints.

These remain `UNKNOWN`.

## 16. B3 — target-era corroboration

Official Character Bazaar pages indexed around the target include `Newhaven` as a completed quest line on characters whose character-state metadata predates July 28. This corroborates Newhaven's presence in durable/public target-era character data.

It does **not** establish that Newhaven quest state belongs in the core Character aggregate. Stage A remains binding: display on a character page does not transfer semantic ownership from content/quest authority.

## 17. B3 result

**Partially reduced, not closed.**

Strong anchors:

1. Newhaven was live as the new entry point before target;
2. tutorial/Newhaven/vocation-selection flow has strong continuity evidence;
3. Targuna replaced the old post-Newhaven Thais/Blue-Valley continuation before target;
4. current Quick Start is stale for that route.

Still blocked:

- exact target-era skip-tutorial level-2 continuity;
- exact creation input validation/error semantics;
- exact starter stats/items/template;
- home/citizenship/respawn details;
- precise account-state eligibility rules.

## 18. Revised B1-B3 blocker state

### B1 naming

**PARTIALLY REDUCED**

Strong candidates:

- Tibia-wide/global uniqueness;
- technical format-restriction family;
- six-month old-name lookup/history after rename.

Still blocked:

- exact normalization/case/space/repertoire;
- exact 29-letter target continuity;
- restricted/reserved patterns;
- deleted-name release algorithm/timing.

### B2 deletion/quota

**LARGELY UNCHANGED**

Strong candidate:

- 25 active characters.

Still blocked:

- 60-day deletion continuity;
- total 30 continuity;
- undelete interaction;
- deleted-name hold/release.

### B3 creation/starter

**PARTIALLY REDUCED**

Strong anchors:

- Newhaven entry point;
- tutorial/Newhaven/vocation-selection continuity;
- Targuna continuation before target.

Still blocked:

- exact starter template;
- skip-tutorial level-2 target continuity;
- exact creation validation/error semantics;
- home/citizenship/respawn details.

## 19. Architecture consequences

This nonbinding evidence pass reinforces existing accepted boundaries without creating new authority:

- Reference naming should preserve a game-wide namespace rather than silently per-world uniqueness;
- current name, searchable old-name alias/history and deleted-name reservation are distinct concepts;
- B2 numeric policies belong in versioned Reference/product policy, not in the semantic lifecycle enum itself;
- starter state remains a versioned ruleset/content template, not hard-coded Character-constructor defaults;
- early-game route/quest content remains content/quest authority even when it affects character progression;
- DUR-02 must not freeze final naming indexes, deletion timers or starter-state fields until target semantics are accepted.

## 20. Evidence-acquisition stopping rule

This bounded pass reached diminishing returns for general primary-web search on B1-B3.

Further B1-B3 progress should require a new evidence source/hypothesis, such as:

1. provenance-clear archived official manual snapshots near July 28;
2. owner-provided historical official captures;
3. newly discovered rule-specific official change chronology;
4. controlled current observation only when continuity to July 28 is independently established.

Do not repeat generic searches for the same 60/30/skip-level-2 phrases without new evidence leverage.

## 21. Owner-decision result

**No owner decision required.**

The pass does not produce a complete target-accurate B1/B2/B3 owner package. Asking the owner to choose values that should be determined by Reference evidence would be premature.

If historical evidence ultimately proves unavailable, a later owner decision may choose an explicit Reference fallback/difference policy. That point has not yet been reached.

## 22. Next programme evidence work

Proceed autonomously to the remaining high-leverage Stage-B evidence blockers:

1. **B4 progression formulas / authoritative-vs-derived mapping**;
2. **B5 promotion continuity and entitlement boundary**;
3. then B6/B7 death-edge and offline-effectiveness evidence;
4. B8 modern build-state ownership with GAME-ITEM/ability/content boundaries.

B1-B3 remain registered as open evidence gaps and should be revisited when genuinely new historical primary evidence becomes available.

Until Stage B receives an owner-accepted complete evidence-backed contract, overall `GAME-CHAR-01` remains **PROPOSED / PLANNED / NOT_STARTED** and runtime/schema authority remains **NONE**.
