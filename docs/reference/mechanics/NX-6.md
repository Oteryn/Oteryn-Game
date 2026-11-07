# NX-6 — achievement and title sources

Base: `81504ec792c186a3adcdc95aef8610d0862334cf` (owner-directed consolidation).
The uncommitted patch was moved from the original task base without creating commits.
Target: current Tibia; no Dawnport, OT-only features, Prey Hunting Tasks or S17 changes.
No new capability, command, protocol message or migration is needed for P11.
Reserved 355–359 / 610–619 / 0994 remain unused.

## P11 — Wayfarer

The accepted task explicitly requires Wayfarer at the Wrath of the Emperor dream finale.
The retained achievement catalogue independently identifies `oteryn:achievement/wayfarer`,
name Wayfarer, three points, secret, TibiaWiki page 62813/revision 844409.

| Rule | Evidence | Implementation |
| --- | --- | --- |
| At Questline 27, the exact case-sensitive opening word requires the used potion. Subsequent words compare the whole lowercased message. | Crystal `data-global/npc/a_sleeping_dragon.lua:67–185`; existing captured dialogue pins the blob SHA-256. | Existing `respond` gates and 32 captured steps retained. |
| Topic 32, `use mirror`, publishes seven paragraphs and resets topic to zero. | Crystal `data-global/npc/a_sleeping_dragon.lua:185–198`; Canary `data-otservbr-global/npc/a_sleeping_dragon.lua:188–201`. | Shared dialogue executor publishes after the durable commit and reconciles lost acknowledgements. |
| Grant Wayfarer, set Questline 28 and Mission09 2 together. This is not completion of the whole Wrath quest. | Same Crystal lines 195–197; same Canary lines 197–199. | One existing fenced Quest transaction, existing account achievement owner and existing quest receipt. Any grant error rolls back the quest writes. |
| Grant once per account; source occurrence bound to the completing character. | Accepted native ACHIEVEMENT-0 account ownership and existing source-event receipt semantics; official manual capture `docs/reference/tibia-manual/achievements.md`. | `record_achievement_grant`; no duplicate point balance or achievement store. |
| Client shows the name and current achievement points. | Existing cap 8/domain 13 contract and native achievement notice panel. | Existing production decoding/panel retained; Wayfarer-specific decoded-wire play test added. |

Title donor paths below abbreviate `src/creatures/players/cyclopedia/player_title.cpp`.

Crystal revision: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8`.
Canary revision: `opentibiabr/canary@47dfd51f45280a59a1d3e50ba7edd573d7234446`.
Donors are `OTS_HYPOTHESIS_ONLY`; the owner's explicit Wayfarer task and retained
structured catalogue establish this bounded accepted rule. No new third-party text or assets added.

Existing QuestState §6 definition pins remain enforced: this addition does not repin or
rewrite historical in-progress quest states. An older definition still returns
RevisionMismatch until its normal content migration is supplied. The production
completion and tests use the current definition.

The no-op Potion=1 effect validates the already-used prerequisite under the same locked
snapshot as the completion. It neither consumes another potion nor resets its flag.
The former tests of intentional deferral are replaced by strict completion assertions;
opening, incorrect words, help, stage, case and whitespace rejection remain covered.

## P12a — STOPPED, no invented title rules

No title implementation is claimed. The three points titles are not quest-point rewards:
Crystal `player_title.cpp:275–277` explicitly calls `getTaskHuntingPoints()`. The owner's
exclusion of Prey Hunting Tasks therefore excludes Aspiring Huntsman, Competent Beastslayer
and Feared Bountyhunter. Do not substitute quest counts, charm points or Grizzly task counts.

Dedicated Entrepreneur requires at least 50% of **all main map areas**, conditionally:
Crystal `src/game/game.cpp:405`, `player_title.cpp:279–283`, and
`data/scripts/discovery_system.lua:530–538,561`. The current native owner deliberately uses
an accepted basic single-area model (`crates/gameplay/domain/src/domain/cyclopedia/basic.rs`),
whose discovered boolean is 0% or 100% of that one area. Treating it as half of the Global map
would be a new rule change, not the sourced denominator. Existing Globetrotter semantics are
an accepted earlier difference and are not expanded into another unsourced title.

Crystal `player_title.cpp:289–290,306–311,325–333,337` lists comments for Proconsul of
Iksupan, Robinson Crusoe, Chompmeister, Hero of Bounac, Planegazer and Time Traveller but
returns false rather than implementing their predicates. Canary confirms the same
six unimplemented branches in `src/creatures/players/components/player_title.cpp:280–281,297–302,316–326,328`.
Its map predicate also returns false (`:271–274`) and its task predicate also reads
hunting-task points (`:267–269`). In particular, visiting the island
has no retained native visitation fact, Hot Cuisine has native cooking exchanges but its
complete-menu predicate must be matched to the title, and generic lowered quest tracks
are not sufficient evidence for the other named Global completion facts. Source comments
alone do not settle such details as a quest-context kill versus any bestiary kill, exact
menu completion, and retroactive qualification.

The requested structured cross-checks were refused by the network: Fandom API requests for
Titles, Dedicated_Entrepreneur, Wrath_of_the_Emperor_Quest and Wayfarer returned HTTP 402
Payment Required. The normal TibiaWiki BR API Titles requests at www.tibiawiki.com.br and
tibiawiki.com.br returned HTTP 403 Forbidden. No UA changes, retries around protection,
HTML scraping or CipSoft calls were attempted. Exact refused URLs:

- https://tibia.fandom.com/api.php?action=parse&page=Titles&prop=wikitext&format=json
- https://tibia.fandom.com/api.php?action=parse&page=Dedicated_Entrepreneur&prop=wikitext&format=json
- https://tibia.fandom.com/api.php?action=parse&page=Wrath_of_the_Emperor_Quest&prop=wikitext&format=json
- https://tibia.fandom.com/api.php?action=parse&page=Wayfarer&prop=wikitext&format=json
- https://www.tibiawiki.com.br/api.php?action=parse&page=Titles&prop=wikitext&format=json
- https://tibiawiki.com.br/api.php?action=parse&page=Titles&prop=wikitext&format=json

These unresolved rules remain STOPPED rather than granting titles on guessed facts.

The owner lifted the earlier path restriction and directed this patch onto consolidated
PR #3002 head `81504ec792c186a3adcdc95aef8610d0862334cf`. No STOPPED decision depends on
file ownership. That head adds the Tibiadrome title source, not these P12a sources.

### Exact unresolved donor facts

- **Exploration:** the current Global main/subarea universe and completion eligibility,
  especially underground completion. Crystal `discovery_geometry.lua:1–3` has extracted
  surface masks but no underground masks; `discovery_system.lua:32–49` explicitly uses
  approximate/hand-authored underground geometry and overrides. Canary has no map predicate.
  Treating the synthetic native “Entry environs” discovery flag as completion of an actual
  Global main area would invent a mapping. No evidence establishes that mapping or a Global
  denominator for the accepted no-Dawnport world. The 50% threshold itself is established;
  the facts it must measure are not.
- **Chompmeister:** the exact title qualification predicate (which dishes, counts and
  completion event), not Culinary Master's achievement predicate. Crystal Jean Pierre
  `:126–136` proves the latter requires two cooks for dishes 2–16; neither donor's title
  checker implements Chompmeister. Those counters cannot establish a different title's
  exact threshold by inference.
- **Proconsul of Iksupan:** the required Atab/arena event versus outfit/addon ownership;
  both title checkers leave only the combined descriptive comment.
- **Robinson Crusoe:** the title grant event and retroactive qualification predicate for
  visiting the island. Crystal's level-999 teleport script implements movement, not a
  title grant or retained title-qualification fact.
- **Hero of Bounac:** unlock at the credited stage-4 Drume kill versus the subsequent Kesar mission report. Crystal
  `creaturescripts_drume_kill.lua:8–10` advances KesarMission only from 4 to 5;
  `kesar_the_younger_day.lua:241–255` announces the title and grants Lionheart but does not
  implement a title grant. The title checker returns false.
- **Planegazer / Time Traveller:** the exact quest-context completion event and eligibility
  predicate. Both donor title checkers return false for these names.

These are missing source facts, not a request for approval or a reserved-path blocker.
No guessed rule or unreachable title catalogue entry is added.

## Overlap inspection

Initially inspected the supplied srvpkg3, clpkg6, srxpkg and platwire1 heads read-only;
none implemented P11 or the missing P12a sources. Subsequently fetched consolidated
srvpkg3 head `81504ec792c186a3adcdc95aef8610d0862334cf` and reapplied only this task's patch.
The delivered diff excludes the consolidation's own changes. qpkg3 and clpkg5 are
merged/deleted per owner; PR #2999 was already in the original base.

## Validation

No ledger edits, commit or push.

- RED before production changes: `cargo test --locked -p oteryn-gameplay-quests nx6_finale -- --nocapture`:
  one failure, `Error: "final reply"`.
- RED real PostgreSQL caller before the fix: `cargo test --locked -p oteryn-gameplay-dispatch --all-features nx6_wayfarer -- --nocapture`:
  one failure in 81.33 s, the absent finale fell through as ordinary Ok instead of fenced rejection.
- GREEN on consolidated base 81504ec7: the same `nx6_wayfarer` command passed one test in
  120.69 s, 2506 filtered out; `NX6_POSTGRES_CASE_COMPLETED elapsed=70.717246441s`.
  This executes all 32 source dialogue steps, stale connection/lease refusals, an injected
  achievement INSERT failure with quest/receipt rollback, lost-ack reconciliation, exact
  replay, duplicate-word refusal to award again, durable account/quest readback, and
  the production notice observer/encoder. Its exact bytes match the native client play test.
- `oteryn-architecture-check workspace .`: `workspace-boundaries: PASS` on the consolidated base.
- Existing missing-copy greeting regression: `cargo test --locked -p oteryn-gameplay-dispatch --all-features conditional_quest_greetings_missing_copy_refuse_in_real_npc_path -- --nocapture`:
  one real PostgreSQL case passed in 92.08 s, 2506 filtered out.
- `cargo test --locked --all-features -p oteryn-client nx6_wayfarer_server_payload_reaches_native_notice_panel -- --nocapture`:
  one matching play test passed in 0.03 s. It decodes the notice payload, opens the play panel,
  checks Wayfarer and three points in the text shared with the native HWND panel, and checks logout cleanup.
  This is a headless play test on Linux; a Windows HWND was not launched.
- Full client crate run: 312 passed, one failed, zero ignored, 23.45 s. The existing
  `play::tests::feature_local_movement_spells::movement_spell_controls_reach_session_and_decode_refusal_without_public_rooms`
  fails with `Elapsed(())`; its isolated repeat also fails (0 passed, one failed, 312 filtered,
  3.02 s). Its source and the client production code are unchanged by NX-6. No timeout,
  assertion, feature gate or skip was changed to hide the failure.
- Clippy on quests/durability/dispatch/client, all targets/all features with `-D warnings`:
  exit 0. Existing dependency warnings in vendor were left untouched.
- All changed server crates (quests/durability/dispatch), with the PostgreSQL URL unset for
  this broad route-less run: **3444 passed, zero failed, 18 pre-existing ignored** across
  library/integration/doc-test targets. Dispatch: 2489 passed / 18 ignored in 1707.40 s;
  durability: 688 passed in 72.37 s; quest library: 250 passed in 9.29 s, plus its integration
  and documentation targets. Command:
  `env -u OTERYN_TEST_POSTGRES_ADMIN_URL cargo test --locked --all-features -p oteryn-gameplay-quests -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch --no-fail-fast --quiet`.
- Final `cargo fmt --all --check` and architecture wrapper: exit 0. `git diff --check`: clean.
- The fetched Crystal dragon's SHA-256 matches the retained dialogue's source blob digest.

## Review and limits

Reviewed the ten-file task diff against 81504ec7. The shared dialogue executor retains the
current actor/session checks, revision sequencing, bounded reply validation and retained
request reconciliation; no new authority path or second achievement store is introduced.
The grant executes after the existing quest writes and before the same physical commit.
No merged migration, generated pin/golden, protocol registry or protected file was edited.
Existing tests remain strict; the deliberate deferral checks now assert the requested
completion and its locked prerequisites. No new ignored test or timeout change was added.
The known unrelated client timeout and historical quest-definition pin limitation above
remain explicit; this packet does not claim a completely green client crate or live upgrade.
