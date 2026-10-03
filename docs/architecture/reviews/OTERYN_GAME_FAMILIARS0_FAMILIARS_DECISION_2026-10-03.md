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
- Amended (2026-10-03, control plane, review finding 4173381895 on #1644): §5.2, the complete
  authority and recovery finding-family sweep, with §4 check 4, §5 writes and §7 tightened to
  match. This is a precondition for allocating FAMILIAR-1.
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
4. the owner has no summon of any kind (R1), and no familiar waits to return (§7: stored time
   with no free tile yet);
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
- **`open`.** The cast write and the return write (§7) set `open = true`: a committed acquisition
  that is not yet cleanly closed or reconciled (§5.2 F-I7). It does not claim that the familiar is
  live: after a crash the row stays open until the next fenced load reconciles it. A clean
  session-end write, a removal write and a reconciliation write set `open = false`. So a row that is still
  `open = true` from an older generation at the next login was never closed: its session did not
  end cleanly.
- **Remaining, not wall clock.** Both values are stored as time left, because both freeze while
  offline (R2). The runtime counts them down while the owner is online.
- **Writes** are session-generation fenced Character writes: at the cast; at a return (§7); at a
  clean session end
  (logout, channel transfer, the end of the in-fight deadline); and at removal (§7). A write from
  a stale generation is refused. A logout or channel transfer completes only after its clean-end
  write commits, as TIMED-ITEM-0B §6.1 orders its checkpoints: if that write fails, the session
  stays and the familiar is kept. `WorldReset` writes as a removal with the time kept
  (`open = false`).
- **Session end fences return admission.** A logout or channel transfer first closes return
  admission for its session, then waits for a return acquisition already in flight to commit or
  abort, and only then makes its clean-end write. A return that reaches admission after the close
  is refused and stays stored; it returns at the next login or arrival. The close holds until
  authority release, so the clean-end write is the session's last write to the row (F-I11). A
  return that committed before the close reopened the row, and the clean-end compare-and-set reads
  that row and closes it. Return admission opens again only when the clean-end write is proven not
  committed: a compare-and-set loss or an error before commit. An unknown commit outcome (a lost
  response) is reconciled from durable state first (DUR-02): the session re-reads the fence and the
  row, and a row it finds closed under its own generation and revision is its committed clean end,
  so the session end completes and admission stays closed. Only a row it finds unchanged lets the
  session stay and reopen admission.
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

### 5.2 Finding-family sweep (review finding 4173381895)

This section completes the authority and recovery qualification of the task template
(`AuthorityInvariant x ConsumerBoundary x MutationOperator`). FAMILIAR-1 takes it as its
qualification and proves every row of §5.1 and of this section, one invariant per negative case.

**Authority invariants.**

| Id | Invariant | Current fact source |
|---|---|---|
| F-I1 | A row is written only in the fenced Character transaction of its own `character_id`. | The session's CharacterId from the GameSession row. |
| F-I2 | A write carries the writer's current session generation; an older one is refused. | The session-generation fence row, never the familiar row's own `session_generation`. |
| F-I3 | A write from a replaced runtime owner is refused, also on a compare-and-set retry. | The RuntimeScopeAuthority ownership generation (FND-04B §22). |
| F-I4 | At most one write per `revision`. | The compare-and-set on the row. |
| F-I5 | One cast or return occurrence has one outcome and one write. | The occurrence's binding (spell, caster, placement). |
| F-I6 | Durable values come only from the row, content and the session. | Content definition, row and fence; nothing from the client. |
| F-I7 | `open = true` marks a committed cast or return acquisition that has not yet been cleanly closed (clean end or removal) or reconciled (crash recovery). It does not claim that a familiar is live: after a crash the row stays open until the next fenced load reconciles it. | The cast and return acquisitions, which set it; the clean-end, removal and reconciliation writes, which clear it. |
| F-I8 | A recovery write never lowers `cooldown_remaining_ms` and never raises `familiar_remaining_ms`. | The row before the write. |
| F-I9 | The cast commits mana, admission and row together; a return commits placement and row together. | The SUMMON-1 acquisition. |
| F-I10 | A load never inserts a row, and it never returns a familiar from an unreconciled open row. | The fenced load. |
| F-I11 | After a session's clean-end write commits, that session writes nothing more to the row until its authority is released. | The session's return-admission fence, closed by session end before the clean-end write. |

**Negative cases beyond §5.1.**

| Case | Path | Invariant |
|---|---|---|
| A load by a session whose generation is not the current one in the session-generation fence (a stale login after a takeover), whatever generation the row stores (for example row 1, loader 2, fence 3) | reconciled | F-I2: loader authority is decided first, against the fence alone. A stale load is refused before the row is classified. It neither reconciles nor returns the familiar. |
| The crash classification is computed from the row's own `session_generation` compared with itself, or with a value the row supplied | reconciled | F-I2: "older" is decided only against the fence row's current generation. A test whose row and fence disagree classifies by the fence. |
| A reconciliation write that loses its compare-and-set or ends in a database error, including an unknown commit outcome, and the session generation has since moved on | reconciled, concurrent, PostgreSQL | F-I2: the session is not admitted and writes nothing until it reconciles the outcome. It re-reads the fence first; a loader whose generation is no longer current is stale and refused, and every later write from it is refused too. |
| The same failure, and the runtime owner has since been replaced (same session generation) | reconciled, concurrent, PostgreSQL | F-I3: the fence re-read finds a newer RuntimeScopeAuthority ownership generation; the loader is refused, and every later write from it is refused too. |
| The same failure, and the loader is still current | reconciled, concurrent, PostgreSQL | F-I10: nothing is returned or written from the earlier in-memory snapshot. The loader re-reads the row and classifies it from scratch: a committed reconciliation reads as a clean row with remaining 0, and a row another authorized writer advanced is taken as read. The fresh row then follows the normal path: a clean row with time left returns its familiar (§7), and an open row is reconciled again. |
| A cast while a familiar waits to return with no free tile | direct | F-I7: refused by check 4. A stored familiar and a new one never exist together. |
| A delayed return racing a removal write for the same character | direct, concurrent | F-I4: one compare-and-set wins per `revision`; the loser's write never lands on that revision. A return that loses re-reads and, finding `familiar_remaining_ms = 0` (the removal won), writes nothing. |
| A delayed return whose own acquisition already committed is retried (a lost commit response, then a re-read) | direct, concurrent | F-I5: the occurrence's first outcome stands. The retry finds the familiar its own acquisition placed and replays that outcome; it places no second familiar and writes nothing. |
| A cast racing a delayed return for the same character | direct, concurrent | F-I7: the cast's compare-and-set re-checks check 4 against the row it replaces and commits only over a row with `familiar_remaining_ms = 0`, so it never replaces a familiar that waits to return. |
| A cast refused by that compare-and-set (it lost, or it found stored time) | direct, concurrent | F-I9: mana, admission and row commit together or not at all, so the refused cast spends no mana and starts no cooldown. |
| A delayed return in flight while logout or channel transfer makes its clean-end write | direct, concurrent | F-I11: session end closes return admission and drains the in-flight return before its clean-end write (§5). The return either committed first, and the clean-end write closes the row it reopened, or it is refused and stays stored. No return reopens the row after the clean-end write, so the next login never reads a clean exit as a crash. |
| A channel-transfer arrival that loads before the departure's clean-end write commits | reconciled, concurrent | The transfer completes only after that write (§5 writes), so the arrival never sees the departing `open = true` row. If it does, the transfer was not admitted, and the arrival is refused as a stale owner (F-I3). |
| `WorldReset` with the owner online | direct | F-I7: the removal writes `open = false` with the time kept. A later crash is then not a loss of the stored time. |
| A familiar creature created by any path other than a familiar spell or return (Summon Creature, convince, an administrative creature command) | sibling API | F-I6: it is an ordinary creature or summon, and nothing about it becomes durable familiar state; content marks the familiars as not summonable and not convinceable (§3). |
| The same creature, checked against the row | sibling API | F-I7: it never sets `open` and writes no row; only a familiar cast or return acquisition opens one. |
| The ordinary summon writer (SUMMON-1 without `familiar_summon`) | sibling API | F-I1: it never writes the familiar row. Only the familiar cast, return, clean-end, removal and recovery writers do. |
| The returning creature after a vocation change while offline | direct | F-I6: it is the current vocation's familiar from content. The stored time is kept, and no creature key is stored. |
| A familiar spell's content revision changes the duration or cooldown | direct | F-I6: the row's stored remaining times stand. New values apply from the next cast. |

**Mutation operators.** Applicable: cast insert and update; return update; clean-end update;
removal update (timer, killed, owner death, lever boss room, `WorldReset`); crash reconciliation
update. Considered not applicable: delete (a row is never deleted while the character lives;
character deletion follows the Character's own cascade) and administrative edit (none defined).

**Consumer boundaries.** These are the spell core's cast, the session-end path (logout, channel
transfer, the in-fight deadline), removal events, the fenced login or arrival load, return
admission (immediate and delayed), the runtime timer, and the in-memory cooldown read by the spell
core and `ACTOR_COOLDOWNS`.

**Families with nothing to add.** Protocol versions: none, since no wire is new (§8). Test
helpers derived from the record may build only the positive happy path. Every negative authority
or provenance case uses the independent current fact sources in the table above.

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
   same-session continuation, §5.1) loses the familiar and keeps the cooldown. The full
   authority and recovery sweep is §5.2.
4. **Typed references:** CharacterId, spell id, creature key.
5. **Wire:** none new (§8).
6. **Split work:** one row per character; one familiar per owner.
