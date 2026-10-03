# FAMILIARS-0 Familiars

- Decision: `FAMILIARS0-FAMILIARS-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence
  and combat) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers:
  - control-plane allocation D299 (#1622), under owner extension D296;
  - the Familiars system of owner decision 1a's Q1a list (CYCLOPEDIA-0 header);
  - the open questions Q1-Q6 of `OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md` §C.1;
  - the "familiars" CREATURE-AI-0 §8.4 left to the spell lane.
- Builds on:
  - `OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md` §C.1 (spells 194-197 and 282, the
    `familiar_summon` native behaviour and its parameters);
  - CREATURE-AI-0 §8 (player summons: SUMMON-1 runtime, cap `RL-19`, behaviour, removal,
    attribution, SUMMON-WIRE-1);
  - TRAVEL-0 R3 (familiars follow a travel; other summons are removed);
  - PARTY-PVP-0 §5.1 and §7 (a summon's share first; a summon's actions are its owner's);
  - BOSS-RAID-0 §6 (boss rooms admitted by lever);
  - the Premium activation decision (the spell Premium check);
  - SPELL-PRESENT-0 §10 (cooldown display; durable cooldowns not decided);
  - owner rule 5905825574.
- Amends: none. CREATURE-AI-0 §8.4 already defers familiars here.
- Amended by the D309 P2 bundle (clarifying, review finding 4173381895): the recovery
  qualification sweep of §5.1.
- Runtime, migration and production authority: NONE. Each child needs its own #1622 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| FAMILIAR-CONTENT-1 | content lane | the five familiar creatures and the five spells with `familiar_summon` (§3) | the spell and creature authoring schemas |
| FAMILIAR-1 | hard (persistence, combat), persistence and combat review | the familiar state row and its writer (§5), the cast checks and acquisition through SUMMON-1 (§4), lifetime, warnings, follow, speed, removal and return at login (§6, §7) | SUMMON-1; FAMILIAR-CONTENT-1 |

Later, each with its own decision:
- the other three familiar looks per vocation and the Cyclopedia chooser (their unlock source is
  unknown, §2);
- familiars in the Drome (no Drome system yet);
- a general durable-cooldown carrier; it may absorb §5.

## 1. Question

How does a character summon, keep and lose a familiar, and what survives a logout or a crash?

## 2. Facts

**PROVEN**

- §C.1 lists the five spells (level 200, Premium, cooldown 1800 s, duration 900 s; mana: knight
  1000, paladin 2000, sorcerer 3000, druid 3000, monk 1500) and proposes `familiar_summon` with
  `duration_ms`, `refuse_if_owner_summons_at_least`, `timer_paused_while`, `expiry_warning_ms`,
  `return_to_owner` and `lever_boss_refusal`.
- CREATURE-AI-0 §8: player summons are created through SUMMON-1 in one all-or-nothing
  acquisition with the cap of 2 (`RL-19`); a summon that dies yields no corpse, loot, experience
  or credit; its damage share goes to its owner, halved; summons are removed at logout, death,
  channel transfer, beyond 30 tiles or 2 floors, and at `WorldReset`; they are never saved.
- TRAVEL-0 R3: familiars follow their owner through a travel.
- SPELL-D2 keeps spell cooldowns runtime-only (SPELL-PRESENT-0 §17).

**TIBIAWIKI_STRUCTURED** (F1108992, F1177634, F1182913-F1182917)

- A familiar moves at its master's speed. Its timer does not run while the owner is offline or
  swimming. The cooldown keeps running while swimming. Familiars cannot be used in Lever Boss
  fights. Each vocation has four looks that differ only by name and sprite.

**OTS_HYPOTHESIS_ONLY** (Canary)

- The cast is refused while the owner has any summon. Warnings at 60 s and 10 s left. The
  familiar teleports to its owner beyond 15 tiles or on another floor. The familiar is recreated
  at login with the time left by wall clock. 10,000 hit points; attackable; not convinceable.

**UNKNOWN**

- Whether the cooldown runs while offline; how the other looks are unlocked; soul cost.

## 3. Content (FAMILIAR-CONTENT-1)

- Five creatures, one per vocation, with the default look, Canary's 10,000 hit points
  (`PARITY_PENDING`), attackable, not convinceable, not summonable by Summon Creature, 0
  experience.
- Five spells as §C.1's table, each with `familiar_summon` and its parameters; one source for the
  summon parameters (the spell), as §C.1 says.

## 4. Cast (FAMILIAR-1 through SUMMON-1)

Checks, in order; any failure spends no mana and starts no cooldown:

1. the spell's level, vocation and Premium checks;
2. the standard spell-core checks, unchanged: enough mana for the spell (§C.1's table) and the
   2 s `support` group cooldown not active;
3. the owner is not inside a boss room admitted by lever (BOSS-RAID-0 §6);
4. the owner has no summon of any kind (R1);
5. the familiar cooldown has run out (§5);
6. SUMMON-1's placement finds a tile.

A success commits, in one acquisition: mana, the familiar's admission, the 2 s `support` group
cooldown and the state row of §5 (remaining time 900 s, cooldown 1800 s, `open = true`).

While the familiar exists, it counts toward the cap of 2 (`RL-19`), so the owner may still add
one ordinary summon (R1).

## 5. Durable state (FAMILIAR-1)

- `game_character_familiar_state`: `character_id` (primary key), `familiar_remaining_ms`
  (0 = none), `cooldown_remaining_ms`, `open` (the clean-end discriminator), `revision`, and the
  writing `session_generation`.
- **`open`.** The cast write and the return write (§7) set `open = true`: a live familiar exists
  for that generation. A clean session-end write and a removal write set `open = false`. So a row that is still
  `open = true` from an older generation at the next login was never closed: its session did not
  end cleanly.
- **Remaining, not wall clock.** Both values are stored as time left, because both freeze while
  offline (R2). The runtime counts them down while the owner is online.
- **Writes** are session-generation fenced Character writes: at the cast; at a return (§7); at a
  clean session end
  (logout, channel transfer, the end of the in-fight deadline); and at removal (§7). A write from
  a stale generation is refused.
- **Crash.** At login, the new session's fenced load reads the row (a same-session continuation
  after process replacement: §5.1). If `open = true` and its
  `session_generation` is older than the new one, the earlier session crashed: the load writes
  `familiar_remaining_ms = 0` and `open = false` under the new generation (the familiar is lost),
  and keeps `cooldown_remaining_ms` as last written, so a crash never shortens it. If
  `open = false`, the row is a clean save and a positive `familiar_remaining_ms` returns (§7);
  until that return is admitted the row stays `open = false`, and a crash before it loses nothing.
  A return refused because the owner is in a lever boss room ends the familiar (§7): the row is
  written `familiar_remaining_ms = 0`, `open = false`, cooldown kept.
- **Fencing and replay.** Every write is a compare-and-set on `revision` under the writer's
  session generation, and it passes the current FND-04 authority checks (DUR-02 §5), including the
  current RuntimeScopeAuthority ownership generation of the writing runtime owner. A write from an
  older session generation, or from a runtime owner that has been replaced (the same session
  generation after process replacement, FND-04B §22), is refused and changes nothing. A
  compare-and-set loser re-applies only after it passes both checks again. The cast
  write is part of the cast acquisition, so a retried cast command replays its first outcome and
  never writes the row twice.
- The row is gameplay state, not a DUR-03 value. A missing row means no familiar and no cooldown.
- The familiar spell's in-memory cooldown is the row's value; SPELL-D2's runtime cooldown is not
  used for these five spells.

### 5.1 Recovery qualification (negative cases, FAMILIAR-1 proves each)

Each case names one invariant. FAMILIAR-1 has one test per case, on PostgreSQL where it says so.

| Case | Path | Invariant |
|---|---|---|
| Write for another character (the row's `character_id` is not the writer session's character) | direct | A row is written only inside the fenced Character transaction of its own character; the write is refused and changes nothing. |
| Same cast or return occurrence replayed with a different binding (spell, caster, placement) | direct | One occurrence has one outcome: the replay conflicts and writes nothing. |
| Client-supplied remaining time, cooldown, generation or familiar creature | direct | Durable values come only from the row, the content definition and the session's own generation; nothing from the client is stored. The returning familiar is the caster's vocation familiar from content, never a named creature. |
| Write from an older `session_generation` (late clean-end save, removal after a takeover) | direct | Refused by the generation fence; no column changes. |
| Two writes of one generation race (timer removal and logout save) | direct, concurrent | The compare-and-set on `revision` admits one; the other re-reads, passes the session-generation and runtime-owner checks again, and re-applies to the new row or becomes a no-op when the row is already closed; never two writes for one revision. |
| Late write from a replaced runtime owner (a clean-end save from the old process, after process replacement and the same-session recovery write) | direct, concurrent, PostgreSQL | The session generation is the same, so the RuntimeScopeAuthority ownership generation fences it: the write is refused, including on a compare-and-set retry, and the familiar never returns from it. |
| Takeover: the new session's fenced load races the old session's clean-end save | reconciled vs direct, concurrent | The load raises the generation first; the old save is then stale and refused. If the old save commits first, the load sees `open = false` and returns the familiar (§7). |
| Row `open = true` with the same generation, and the familiar is still in the running process (a reconnect inside one GameSession) | reconciled | Not a crash: no reconciliation write; the familiar continues. |
| Row `open = true` with an older generation | reconciled | Crash: one write under the new generation sets remaining 0 and `open = false`; the cooldown is kept. |
| Server restart, then a new session (PostgreSQL reload) | reconciled, PostgreSQL | The new session has a newer generation, so an open row loads as a crash (the row above). A clean row returns from its stored remaining time only. |
| Process replacement with proven same-session continuation (FND-04B §22; PostgreSQL reload) | reconciled, PostgreSQL | The generation is unchanged, but the familiar is not part of the reconstructed state, and the stored remaining time predates the loss. An `open = true` row is therefore a crash under the same generation: one fenced write sets remaining 0 and `open = false` and keeps the cooldown. The familiar never comes back with stale time. A clean row (`open = false`) returns as usual (§7). |
| Missing row | reconciled | No familiar and no cooldown; a load never inserts a row. |
| Any recovery path | reconciled | `cooldown_remaining_ms` is never lowered by a recovery write. |
| Return refused in a lever boss room, immediate or delayed | direct | One fenced write ends the familiar (remaining 0, `open = false`, cooldown kept); no creature is placed. |

## 6. Lifetime and behaviour (FAMILIAR-1)

- **Timer.** The remaining time runs down while the owner is online and not swimming (swimming:
  when that state exists). Warnings as server log messages at 60 s and 10 s left (Canary text,
  `PARITY_PENDING`). At 0 the familiar is removed.
- **Cooldown.** Runs while the owner is online, swimming included; frozen offline (R2). Death or
  removal of the familiar does not reset it.
- **Speed.** The owner's current speed, updated on every change of the owner's speed (R4).
- **Follow.** CREATURE-AI-0 §8.3 targeting. When the owner is more than 15 tiles away in x or y,
  or on another floor, the familiar is placed next to the owner (Canary, `PARITY_PENDING`). The
  30-tile and 2-floor removal of §8.4 does not apply to it. It follows a travel (TRAVEL-0 R3).
- **Attribution.** As any player summon (CREATURE-AI-0 §8.5): its damage share goes to its owner,
  halved; party experience takes a summon's share first (PARTY-PVP-0 §5.1); its actions are its
  owner's for PvP.

## 7. Removal and return

| Event | Familiar | Remaining time |
|---|---|---|
| timer reaches 0 | removed | 0 |
| killed | dies (no corpse, loot or credit) | 0 |
| owner dies | removed | 0 |
| owner enters a lever boss room | removed | 0 (`PARITY_PENDING`) |
| owner logs out, transfers channel | removed | kept; it returns next to the owner at the next login or arrival |
| `WorldReset` | removed | kept |
| crash | lost | 0 (§5) |

A returning familiar, immediate or delayed, first passes cast check 3 (§4): when the owner is inside
a boss room admitted by lever (BOSS-RAID-0 §6), the return is refused and the familiar ends as on
entering such a room (the table above): one fenced write sets `familiar_remaining_ms = 0` and
`open = false` under the owner's generation and keeps `cooldown_remaining_ms`. Otherwise it is
admitted by SUMMON-1's placement; with no free tile it stays stored and returns on the owner's next
step that frees one (`PARITY_PENDING`), and that later admission checks the lever boss room again. Every admission of a return,
immediate or delayed, writes the state row in the same SUMMON-1 acquisition: `open = true` and the
new session generation, by compare-and-set on `revision` (§5). The familiar and the reopened row
commit together or not at all, so a crash after a return is seen as a crash at the next login,
never as a clean save.

## 8. Wire

Nothing new. The familiar is a creature entry with `summon = OWN` or `OTHER` (SUMMON-WIRE-1), the
cooldown shows through `ACTOR_COOLDOWNS` (SPELL-PRESENT-0 §10), and the warnings are server log
messages.

## 9. Rows (registered by FAMILIAR-1)

| Row | Value |
|---|---|
| `FAMILIARS0-RL-01` familiar duration | 900 s |
| `FAMILIARS0-RL-02` cooldown | 1,800 s |
| `FAMILIARS0-RL-03` return distance | 15 tiles, or another floor (`PARITY_PENDING`) |
| `FAMILIARS0-RL-04` warnings | 60 s and 10 s left (`PARITY_PENDING`) |

## 10. Rejected options

- **Familiars removed for good at logout.** TibiaWiki says the timer pauses while offline, which
  only makes sense if the familiar returns.
- **Wall-clock storage (Canary).** It counts offline time, against TibiaWiki.
- **SPELL-D2's runtime cooldown.** A relog or restart would reset a 30-minute cooldown on a strong
  summon.
- **A general durable-cooldown table now.** No accepted requirement beyond these five spells.

## 11. Architect rulings (owner rule 5905825574)

- **R1, other summons: a) refuse the cast with any summon; a familiar counts toward the cap of 2**
  (Canary); b) independent of summons. Recommendation and ruling: a), `PARITY_PENDING`.
- **R2, offline: a) timer and cooldown both frozen** (TibiaWiki; the manual freezes cooldowns
  offline); b) cooldown runs offline. Recommendation and ruling: a).
- **R3, logout: a) the familiar returns at login with its time left**; b) it is lost.
  Recommendation and ruling: a).
- **R4, speed: a) always the owner's current speed** (TibiaWiki); b) refreshed at casts only
  (Canary). Recommendation and ruling: a).
- **R5, looks: a) the default look per vocation now**, the other looks later; b) all four now.
  Recommendation and ruling: a), the unlock source is unknown.

## 12. Owner questions

None. Every choice above is a reversible architect ruling under owner rule 5905825574.

## 13. Decision test

- **Must decide now:** YES. Control-plane allocation D299 (owner D296); §C.1 Q1-Q6 block the five
  spells.
- **Blocked without it:** the five familiar spells and their creatures.
- **Harder later:** the durable row and its write points sit in the session-end path.
- **Supersede if:** a general durable-cooldown decision; official evidence on offline cooldowns,
  summon counting or the return distance.
- **Deliberately not decided:** the other looks and the chooser; familiars in the Drome; soul
  cost (none authored).

## 14. Before-freeze checklist

1. **Contracts:** none amended.
2. **Serialization:** the row is written in the cast acquisition and at session end, fenced by
   session generation.
3. **Restart:** a clean end (`open = false`) keeps the familiar's time; a return reopens the row
   (`open = true`) in its admission; a return refused in a lever boss room ends it
   (`familiar_remaining_ms = 0`, `open = false`, cooldown kept); a crash (`open = true` from an older generation, or after a proven
   same-session continuation, §5.1) loses the familiar and keeps the cooldown.
4. **Typed references:** CharacterId, spell id, creature key.
5. **Wire:** none new (§8).
6. **Split work:** one row per character; one familiar per owner.
