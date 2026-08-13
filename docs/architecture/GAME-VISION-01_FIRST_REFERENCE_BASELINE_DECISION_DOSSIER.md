# GAME-VISION-01 — First Reference Baseline Decision Dossier

- Status: **PRE-DECISION ANALYSIS / NOT ACCEPTED**
- Date: 2026-08-11
- Gate owner: product owner
- Primary consumer: `GAME-CHAR-01` Stage B
- Coordination ID: `OTV2-GLOBAL-ARCHITECTURE`
- Trusted repository base: `blakinio/Oteryn-v2@3853127dfccf7df2421dfe0a6c63714f19e828ff`
- Purpose: select a defensible immutable first Global Tibia behavior cut for the Reference profile without guessing undocumented mechanics or turning a mutable upstream game into a moving Oteryn release target
- Does not authorize: owner acceptance, runtime/client/protocol/persistence/content implementation, proprietary-code/protocol/asset copying, production rollout or any Stage-B character rule

## 1. Decision required now

### Must decide now?

**YES — before Reference-sensitive `GAME-CHAR-01` Stage B can close.**

The owner-accepted GAME-VISION minimum and GAME-CHAR Stage-A baseline deliberately allow baseline-neutral architecture to proceed while concrete Reference behavior remains fail-closed. Stage B now needs a stable answer to a simple but far-reaching question:

> Which exact state of Global Tibia is the first Oteryn Reference profile trying to reproduce where Reference parity applies?

Without that target, naming, creation, progression, vocation, death, offline-training, quota and later item/combat/content decisions can silently choose different points in Global Tibia history.

### What this decision does not need to solve

It does not need to prove every Global mechanic today. It needs to freeze **which external behavior cut is authoritative** and **how evidence about that cut is classified**.

Unknown behavior remains `UNKNOWN`; a target date is not permission to invent missing details.

## 2. Already accepted constraints

### PROVEN — repository architecture

The following are already binding and are not reopened here:

1. first external evaluation is Reference-first;
2. each released Reference revision is immutable;
3. upstream Global changes may be observed continuously, but promotion into Oteryn requires a later explicit named Reference revision;
4. Reference mechanics take precedence over future-facing Evolved preferences inside the Reference profile;
5. security, integrity, legal/provenance and anti-duplication requirements override defect compatibility;
6. one Oteryn engine, client and `protocol-oteryn` serve Reference and Evolved profiles; parity does not create a fork;
7. Reference economy means rule/source/sink parity where in scope, not reproduction of historical market prices, total supply or player population history;
8. GAME-CHAR Stage A is binding only for baseline-neutral safety/ownership/lifecycle/revision boundaries; Reference-sensitive Stage B awaits this exact target;
9. Canary and other open-source servers may provide discovery hypotheses/reference inventory only and cannot become production authority or a source for copying proprietary behavior.

## 3. External production chronology relevant to the cut

### OBSERVED EXTERNAL — official public CipSoft/Tibia sources retrieved 2026-08-11

The public production chronology around Summer Update 2026 is material because the update was followed quickly by fixes and balancing.

### 2026-07-07 — vocation adjustments / fixes

Official Tibia news records live vocation balancing and fixes with the July 7 server save, including changes to Knight, Paladin, Sorcerer, Druid and Monk mechanics/values. The article also explicitly says further adjustments may still be needed after live evaluation.

Primary public source:

- `https://www.tibia.com/news/?id=8872&subtopic=newsarchive` — **Vocation Adjustments Changes**, 2026-07-07.

### 2026-07-13 — Summer Update 2026

Official Tibia news records the Summer Update 2026 production release, including Shards of a Broken Moon, Make Believe, Weapon Proficiency changes, Echo Raids and Discovery System changes.

Primary public source:

- `https://www.tibia.com/news/?id=8845&subtopic=newsarchive` — **Summer Update 2026**, 2026-07-13.

### 2026-07-14 — immediate fixes

The day after the Summer Update, official news records fixes affecting gameplay-visible behavior, including Spiritual Outburst, Battle Healing display/multiplier context, Blood Rage bonus correctness and custom spell behavior.

Primary public source:

- `https://www.tibia.com/news/?id=8887&subtopic=newsarchive` — **Fixes and Changes**, 2026-07-14.

### 2026-07-16 — Echo Raid production change, announced later

The official latest-news archive later stated that since the July 16 server save, Echo Raids could no longer spawn on Rookgaard, and explicitly apologised for delayed information.

This is important architecture evidence: **official change publication may lag production behavior**. Therefore an official-news-only evidence model cannot claim exhaustive completeness.

Public source family:

- `https://www.tibia.com/news/?subtopic=latestnews` / news archive entry announcing the July 16 change on July 20.

### 2026-07-21 — further fixes and changes

Official news records another production server-save change set, including Echo Raid accessibility, Swiftfoot/Momentum cooldown behavior and other map/gameplay fixes.

Primary public source:

- `https://www.tibia.com/news/?id=8899&subtopic=newsarchive` — **Fixes and Changes**, 2026-07-21.

### 2026-07-28 — balancing, fixes and changes

Official Tibia news records a further production server-save change set after the Summer Update. Publicly documented changes include boss behavior, hunting-ground density, multiple creature XP values, Echo Raid activation behavior, a mana-barrier death bug and other fixes. The July 27 maintenance notice expected game worlds back after the July 28 maintenance/server-save window.

Primary public source family:

- `https://www.tibia.com/news/?subtopic=newsarchive` — **Balancing, Fixes and Changes**, 2026-07-28;
- the adjacent official July 27 news ticker documenting the July 28 maintenance/server-save window.

### After 2026-07-28 through dossier date 2026-08-11

Search of official Tibia web results on 2026-08-11 did **not** establish a complete authoritative list proving that no behavior-changing production action occurred after July 28.

Classification:

```text
absence of discovered later patch note
!= proof of absence of later production change
```

Therefore this dossier treats the completeness of the post-July-28 public chronology as `UNKNOWN`, not `PROVEN`.

## 4. What an "exact Reference baseline" means

### RECOMMENDATION — semantic definition

The first Reference baseline should identify a **dated production behavior cut**, not a source-code snapshot, proprietary binary hash, market-state copy or continuously moving upstream target.

Conceptually:

```text
Reference target
= Global Tibia production-observable behavior
  immediately after a named server-save/change boundary
  for mechanics/content in Oteryn's declared parity scope
```

The target has two independent dimensions:

1. **target cut** — which production state we intend to reference;
2. **evidence completeness** — how strongly each claimed behavior at that cut is actually proven.

Freezing the first does not magically complete the second.

### Why a server-save behavior cut is preferable

- Global Tibia is a live service; public behavior changes can occur without a useful source revision for Oteryn.
- A server-save/change date is externally understandable and can be linked to public change chronology.
- Oteryn does not need or want proprietary server source or a proprietary client binary to define gameplay semantics.
- The same date cut can be represented by a versioned Oteryn evidence manifest whose contents grow from UNKNOWN to evidenced without changing the target itself.

## 5. Evidence classifications for the selected cut

Every material Reference parity assertion should use one of these evidence states.

### `PROVEN`

Strong primary/public evidence and/or reproducible bounded observation establishes the behavior for the target cut with no material conflict.

Examples:

- an official public change note gives an exact numeric value effective with the target/before-target server save;
- a target-period capture with clear provenance plus reproducible independent observation establishes a deterministic rule.

### `OBSERVED`

Black-box behavior was directly observed with sufficiently recorded inputs/context, but the full rule/formula is not yet proven.

Example:

- a spell produced a measured result under a known character/equipment state, but the general formula remains unresolved.

### `DERIVED`

A rule is logically derived from multiple evidenced facts. The derivation must be recorded; it is weaker than direct proof and must not conceal assumptions.

### `UNKNOWN`

Evidence is missing, insufficient, historically unavailable or cannot distinguish plausible alternatives.

`UNKNOWN` is an acceptable architecture state. It blocks a parity claim/fixture that depends on the missing behavior; it is not replaced by a convenient Canary/crystalserver implementation or developer intuition.

### `CONFLICT`

Reliable evidence sources disagree materially or appear to describe different production states/conditions.

Conflict must be resolved or explicitly scoped before `PARITY_CONFIRMED` can be claimed.

### `DECLARED_DIFFERENCE`

Oteryn intentionally differs from the selected Reference target through an explicit accepted decision, for example because of security/integrity/legal constraints or a separately accepted Reference-profile product difference.

## 6. Evidence-source hierarchy

### Tier 1 — primary public authority

Prefer first:

- official Tibia/CipSoft public news/change announcements;
- official public manuals/FAQ/rules/documentation;
- official public character/account/product surfaces where they expose behavior/state relevant to the rule;
- owner-provided primary captures with provenance when they directly record live Global behavior.

Tier 1 still does not guarantee completeness. The delayed July-16 Echo Raid notice demonstrates why.

### Tier 2 — controlled black-box gameplay observation

Use lawful, bounded observations of official production gameplay to answer questions public docs do not specify.

Required evidence record should include where applicable:

- target date/known production cut relationship;
- world type/ruleset context;
- character vocation/level/skills/relevant equipment and bonuses;
- input/action sequence;
- server/client-visible result;
- repeated sample count when randomness exists;
- timestamps/timezone;
- screenshots/video/log extracts where lawful and useful;
- uncertainty and alternative explanations.

A current observation after the target date can only prove historical target behavior if evidence establishes that the relevant mechanic did not change between target and observation. Otherwise it is current-state evidence, not historical proof.

### Tier 3 — reputable public community documentation

Community wikis, guides, calculators, forum posts and research may:

- discover candidate rules;
- provide historical clues;
- help design black-box tests;
- corroborate primary evidence.

They should not silently become the sole canonical oracle for a material Reference rule when stronger evidence is available or the claim remains uncertain.

### Tier 4 — open-source OTS/reference implementations

Canary, crystalserver and other OTS repositories may be used as:

- hypothesis generators;
- coverage inventories;
- test-case inspiration;
- migration/import discovery input where provenance allows.

They are **not proof of Global Tibia behavior** merely because they implement a similar rule. No source-code equality with Global is assumed.

ADR-0008 remains binding: Canary does not enter Oteryn production runtime/dependency/fallback/translation paths.

## 7. Provenance, legal and integrity boundary

The parity programme must not require:

- proprietary server source code;
- copying proprietary client code;
- distributing proprietary assets without rights;
- reusing private/restricted leaked material as canonical evidence;
- adopting a proprietary network protocol into `protocol-oteryn` merely to imitate gameplay;
- intentionally reproducing duplication, stale-authority, security or corruption defects.

Publicly observable behavior can be specified independently from implementation. Oteryn should document **what behavior is evidenced**, then implement it natively within accepted Oteryn authority/security/durability architecture.

Where provenance or legal status is uncertain, classify the evidence as blocked/unknown and require review rather than laundering it through another OTS repository.

## 8. Candidate A — 2026-07-13 post-Summer-Update production

### Definition

Target the production behavior immediately after the July 13 Summer Update 2026 server-save/release boundary.

### Benefits

- clean, memorable major-release landmark;
- official update announcement exists;
- initial new-system/content intent is easy to name.

### Material problems

- known gameplay-visible fixes arrived the next day on July 14;
- a production Echo Raid change occurred with the July 16 server save and was announced only later;
- July 21 brought further fixes;
- July 28 brought meaningful balancing and additional fixes, including XP values, hunting-ground density, boss behavior and a death-related mana-barrier bug.

### Assessment

**NOT RECOMMENDED for the first Oteryn Reference baseline.**

Choosing July 13 intentionally targets a short-lived launch state that CipSoft itself corrected/rebalanced within days. Oteryn would spend parity effort reproducing a transient post-update state and then immediately need a successor revision.

## 9. Candidate B — 2026-07-28 post-server-save production

### Definition

Target Global Tibia production behavior **after the 2026-07-28 server-save/maintenance change boundary**, including all earlier live changes that were already effective by that point and the July-28 balancing/fix set.

For dossier discussion only, shorthand:

```text
candidate: 2026-07-28 post-server-save production behavior
```

This shorthand is **not** the final Oteryn Reference revision naming scheme; that naming scheme remains separately deferred.

### Benefits

- postdates the Summer Update launch and its immediate correction cycle;
- postdates the documented July 14, July 16 and July 21 changes;
- includes the significant July 28 balancing pass instead of knowingly targeting pre-balance values;
- anchored to an official production server-save change event rather than an arbitrary chat date;
- early enough that the public chronology around the Summer Update remains concentrated and reviewable;
- gives GAME-CHAR Stage B a concrete behavior date without forcing continuous upstream tracking.

### Risks / limitations

- official public notes may omit some production behavior; delayed July-16 disclosure proves this can happen;
- historical black-box evidence for every mechanic may not have been captured at the time;
- current production observations after July 28 cannot automatically be projected backward if later changes are possible;
- this candidate still requires an evidence manifest and UNKNOWN classification for gaps.

### Assessment

**RECOMMENDED.**

It is the strongest currently evidenced production cut because it is a named post-update stabilization/balancing point and does not knowingly freeze the immediately superseded July-13 launch state.

## 10. Candidate C — 2026-08-11 current-production behavior cut

### Definition

Declare the first target to be the Global production state at/after the 2026-08-11 server-save boundary, i.e. "current Global" on the owner-decision date.

### Benefits

- maximally recent from today's perspective;
- allows present-day black-box observation while the target is still live;
- minimizes intentional lag behind Global at first selection.

### Material problems

- this dossier has not established an authoritative public version/change marker specifically defining an August-11 behavior set;
- web search did not prove whether any undocumented or poorly indexed post-July-28 production changes occurred;
- "whatever is live today" is much harder to reproduce later unless a broad evidence capture is frozen immediately;
- it encourages an accidental moving-target mindset unless the cut is rigorously snapshotted.

### Assessment

**NOT RECOMMENDED as the first baseline with current evidence.**

A future dossier could choose an August-11 cut if the project first creates sufficient primary/black-box evidence to identify that production state. Recency alone is weaker than reproducibility.

## 11. Candidate D — continuously latest Global behavior

### Definition

Reference always means whatever Global Tibia currently does; existing Oteryn Reference worlds silently track changes.

### Assessment

**REJECTED by existing owner-accepted GAME-VISION policy.**

The project already chose hybrid tracking:

- observe upstream continuously;
- released Reference revision remains immutable;
- promote later upstream behavior only through an explicit later named Reference revision.

Continuously mutating one Reference revision would destroy reproducibility, parity evidence, rollback reasoning and release compatibility.

## 12. Recommended first target

### RECOMMENDATION — owner decision required

Select:

> **Global Tibia production behavior after the 2026-07-28 server-save/maintenance change boundary**

as the external behavior cut for the **first Oteryn Reference baseline**.

This recommendation is about target semantics, not final revision naming syntax.

### Why this is preferred

1. It is later than the Summer Update's known immediate correction cycle.
2. It includes the July 28 post-update balancing rather than knowingly freezing obsolete values.
3. It is anchored to a public server-save production event.
4. It preserves immutable Reference releases and hybrid upstream observation.
5. It gives GAME-CHAR Stage B a deterministic date boundary now.
6. It tolerates incomplete knowledge safely because individual mechanics remain evidence-classified and can stay UNKNOWN.
7. It avoids making "latest today" the hidden architecture contract.

## 13. What acceptance would mean — and what it would not mean

If the owner accepts the recommendation, the canonical contract should state:

```text
first Reference external target
= Global Tibia production-observable behavior
  after the 2026-07-28 server-save change boundary

Reference revision mutability
= immutable once released/accepted

later Global changes
= observed as candidate evidence
= do not modify this baseline
= require explicit later Reference revision promotion
```

Acceptance would **not** mean:

- every Global rule is already known;
- every July-28 behavior can be reconstructed solely from official news;
- every current August behavior automatically belongs to the target;
- market prices/world wealth/player population are copied;
- bugs/security defects are mandatory parity;
- proprietary code/protocol/assets may be copied;
- GAME-CHAR Stage B is automatically accepted;
- runtime/content implementation is authorized.

## 14. Required evidence manifest for the accepted target

Before a Reference release may claim reproducible parity, maintain a versioned evidence manifest conceptually containing per behavior/domain:

```text
reference_target_cut
behavior_id / parity_case_id
domain
scope/profile/world assumptions
evidence_class = PROVEN | OBSERVED | DERIVED | UNKNOWN | CONFLICT | DECLARED_DIFFERENCE
source_type
source_locator / evidence artifact id
observed/effective date
retrieval/capture date
input/preconditions
expected observable behavior
confidence / uncertainty notes
conflicts
Oteryn fixture/test linkage when implemented
accepted difference reference when applicable
```

The manifest itself should eventually have a deterministic content hash/revision under the release/evidence tooling contract.

Adding evidence that changes an item from `UNKNOWN` to `PROVEN` does not necessarily change the external target cut. Changing the intended target behavior because Global changed later **does** require a later Reference revision.

## 15. Parity matrix rule

For each in-scope Reference mechanic, implementation/release claims should converge on explicit classifications such as:

- `PARITY_CONFIRMED` — evidence + Oteryn test/fixture prove the intended observable behavior;
- `PARITY_PENDING_EVIDENCE` — target is known but proof is insufficient;
- `PARITY_CONFLICT` — evidence conflict unresolved;
- `DECLARED_DIFFERENCE` — explicit accepted Oteryn Reference difference;
- `OUT_OF_SCOPE` — not part of the current bounded Reference release.

No `PARITY_CONFIRMED` claim may be inferred merely from sharing data/code with an OTS implementation.

## 16. Impact on GAME-CHAR Stage B

If the recommended target is accepted, `GAME-CHAR-01` Stage B may begin evidence-backed reconciliation against the July-28 cut for at least:

- creation choices/starter state;
- naming namespace/normalization/recycling behavior;
- character slot/quota behavior where Reference-visible;
- persistent progression categories and semantics;
- vocation/class/promotion state;
- death/respawn/progression-loss/blessing/protection behavior;
- offline training/progression where present;
- deterministic formulas/fixtures where the owning gameplay/simulation gate requires them.

Stage B still must distinguish:

```text
PROVEN Reference behavior
vs OBSERVED/DERIVED behavior
vs UNKNOWN
vs safety/integrity architecture invariant
vs explicit DECLARED_DIFFERENCE
```

The selected date alone cannot fill those categories automatically.

## 17. Impact on DUR-02

Before full GAME-CHAR Stage-B acceptance, bounded DUR-02 discovery remains limited by Stage A.

After the Reference target is accepted but before Stage B closes, DUR-02 may use the target to guide **questions and compatibility pressure**, but it still must not freeze final character fields/types from unsupported assumptions.

Only accepted Stage-B semantics may close the final character-bearing schema contract.

## 18. Impact on GAME-ITEM/content/combat/economy

The same external target cut should become the default historical reference point for later Reference-sensitive parity work, unless a dedicated owner decision explicitly scopes a different target.

This prevents:

- GAME-CHAR using July 28 while GAME-ITEM silently uses "current Global";
- content imports mixing July 13 and July 28 values;
- combat formulas being copied from an unrelated OTS revision;
- economy source/sink rules drifting between undocumented dates.

A later Reference revision can move all affected domains forward coherently through an explicit compatibility/migration/release decision.

## 19. Upstream observation after acceptance

Continue observing Global changes after the target cut, but classify them as **later-revision candidate evidence**.

For every newly observed Global change:

1. identify effective production date/server-save boundary if possible;
2. identify affected parity domains;
3. record evidence/confidence;
4. determine whether current Reference target differs;
5. do **not** mutate the released target;
6. aggregate selected changes into a future explicit Reference revision proposal.

This preserves the already accepted hybrid tracking policy.

## 20. Failure modes prevented by this decision

A dated behavior cut plus evidence manifest prevents or reduces:

- developers choosing whichever Tibia version matches available code;
- one subsystem targeting current Global while another targets an older wiki page;
- silent formula changes when CipSoft rebalances Global;
- "official patch note = exhaustive specification" overclaiming;
- community/OTS implementations being promoted from clue to authority without evidence;
- inability to reproduce why an Oteryn Reference release behaved a particular way;
- schema/content work accidentally encoding guessed behavior as permanent architecture.

## 21. Owner decision package

### RECOMMENDATION — NOT OWNER-ACCEPTED

Accept the following as one package:

1. **First Reference target cut:** Global Tibia production behavior after the **2026-07-28 server-save/maintenance change boundary**.
2. **Target is immutable:** later Global changes never silently modify this Reference baseline.
3. **Target date and evidence completeness are separate:** unknown mechanics remain `UNKNOWN` until evidenced.
4. **Evidence hierarchy:** official public sources first, controlled lawful black-box observation second, reputable community sources as corroboration/discovery, OTS repositories only as reference hypotheses/inventory.
5. **No proprietary implementation copying:** parity specifies public/observable behavior and implements it natively in Oteryn.
6. **Delayed/undocumented upstream changes are possible:** absence from patch notes/search is not proof of absence.
7. **Evidence manifest required:** every material parity claim is source-classified and ultimately linked to deterministic Oteryn fixtures/tests where implemented.
8. **Security/integrity/legal override remains binding:** defects/unsafe behavior are not copied for parity.
9. **Cross-domain consistency:** GAME-CHAR, GAME-ITEM, combat/content/economy Reference work uses this same first target unless explicitly superseded/scoped.
10. **Stage B remains separate:** accepting the target only unblocks evidence-backed GAME-CHAR Stage-B work; it does not accept Stage B or authorize implementation.

## 22. What rejection/modification would mean

The owner may instead choose:

- July 13 Summer Update launch state;
- an August-11/current-production cut after requiring stronger capture evidence;
- another explicitly dated production boundary supported by named evidence.

If the owner chooses a different cut, the evidence-classification/provenance/immutability rules above can still remain valid unless explicitly changed.

The one option already incompatible with accepted policy is silently continuously mutating the same Reference revision.

## 23. Deliberately not decided here

- final Oteryn Reference revision naming syntax/identifier;
- exact contents of every parity matrix row;
- exact GAME-CHAR Stage-B rules/formulas;
- final schema or runtime/content implementation;
- exact automated evidence-capture tooling;
- public marketing/version labels;
- future Reference revision cadence;
- Evolved gameplay changes.

Until the product owner explicitly accepts or replaces section 21, this document remains **PRE-DECISION ANALYSIS / NOT ACCEPTED** and creates no new Reference target authority.
