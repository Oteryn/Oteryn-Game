# QUESTPC run8 Corruptor activation/lifecycle

HEAD `1fa04bbbddb5d96e21fecca0bc456f3c5e1109b7`, detached local AUTHORING.
No Git mutation, publication, external write, dependency download or credential read.

## Selected source and policy

Owner approved the native30 projection from `.questpc/newhaven-activation-proposal.md`.
`quest_finish_newhaven_items.py` records donor48 separately, mounts52113/52114/52115
as native30 Ground-only corpses and52116 as a Ground-only non-container. All four
retain exact current15.30 appearance bindings; canonical source shards are untouched.

Forward migration1000 extends the installed closed corpse-chain policy and the
same-XID source schedule proof. Timings are600000/600000/10000ms with terminal
retirement at1810000ms. It retains original MINT provenance, current-scope fences,
immutable schedules and existing descendant-custody retirement. No second timer,
Item store or retirement writer was added. Existing SUMMER timing branches remain.

The actual kill/take/equip cases are retained. A new real-kill lifecycle case checks
the exact schedule, immutable/bounded-stage refusal, no early transformation,
separate-process catch-up, unchanged corpse identity and child custody, no early
terminal retirement, and durable retirement of parent plus every remaining child.
The child reconstructs root/recovery seal/node/lane rather than inheriting a timer.

## Validation checkpoint

- Python regression RED: `.questpc/r8-corruptor-items-red.log`,8tests with1error
  (`CORPSE_SOURCE_CAPACITY` absent before implementation).
- Python GREEN: `python3 -m unittest discover -s tools/content-migration -p
  test_quest_finish_newhaven_items.py -v`,8passed,0failed in3.428s;
  `.questpc/r8-corruptor-items-green.log`; last repeat8/0 in3.173s.
- First attempt: `QPC_SLOT=1 /home/mole/oteryn-cache/codex-bin/cargo test --locked
  --offline -p oteryn-gameplay-dispatch --lib newhaven_corruptor -- --nocapture`;
  `.questpc/r8-corruptor-pg.log`. Compilation failed in another worker's partial
  integration: `npc_command/catalogue.rs:72`, E0425 missing `npc_rules::legacy_defaults`.
  That export is now present. No PostgreSQL case ran and no completion marker exists
  from that attempt. Initial compilation also began before the lead's loot-section
  regeneration; retry awaits a coherent native manifest.
- Added actual-generation ordinary Brass Armor witness selection with unchanged
  probabilities. Failed atomic commit plus841→840 stamina retry must retain both
  ordinary and guaranteed Brass Armor. Zero-stamina case checks gear-set-only count.
- Migration rerun is a no-op after its atomic installed-policy marker; lifecycle
  PG case executes it again before checking exact persisted schedule. Every source
  replacement needle occurs exactly once in migration0266 (static check).
- `rustfmt --check --edition 2024` on the changed dispatch case: exit0.
- Scoped `git diff --check`: exit0. Reviewed full owned sources with the external
  CODEX-CHECKLIST; no run8 Rust public API, enum, struct-field or trait additions.
  Workspace rg confirms existing Corruptor public APIs' production call sites.

Unfinished: PG qualification, clippy, final whole-diff review and exact marker
durations. Viola is assigned separately to the native-NPC transaction worker.

Authority-only filter `test --locked --offline -p oteryn-game-server --test
combat_death_reward_postgres tutorial_corruptor_preparation -- --nocapture`
was attempted during the native-content wait. Its first compilation failed on
the other worker's partial `tutorial_chest.rs:145` integration, E0599 missing
`DurabilityRoot::claim_viola_book` (`r8-corruptor-prepare-pg.log`). The owning
workers subsequently confirmed implementation/root mount; retry is in progress
(`r8-corruptor-prepare-pg-retry.log`). No PG pass is claimed from compilation.

## SOURCE_STABLE handoff to centralized validation

Lead consolidated validation into one warm slot6 build after several workers'
simultaneous resource interruptions. Per lead instruction, slot1's live duplicate
compile was interrupted with Ctrl-C; exit130 (session65486). No new Cargo job was
started afterward. `r8-corruptor-pg-stable-retry.log` contains compilation only,
no PostgreSQL completion marker. Earlier stable-content attempt instead captured
the other worker's E0308 `Ok(())`/`EquipmentAuthority` mismatch, repaired in current
source; that failure is in `r8-corruptor-pg-stable.log`.

The modifier scenario now selects a genuine base-table gold miss and requires
the actual200% event to restore gold, while checking one guaranteed set. This
proves that the event reaches the production loot owner. The Knight scenario
selects an ordinary Brass Armor hit and checks both ordinary and guaranteed
instances across the failed commit and841→840 retry. These are actual actor
occurrences; source probabilities and fixture reward rows remain unchanged.

Final source checks before centralized validation: leaf rustfmt check exit0;
scoped diff whitespace check exit0. No production/content edits after native
generation stability; last edits were the modifier qualification assertions.

Central filtered command (lead supplies slot6):
`/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p
oteryn-gameplay-dispatch --lib newhaven_corruptor -- --nocapture`.
It selects all-five vocation, low-stamina/event, lifecycle and guarantee cases.
The lifecycle process helper is selected too; its route-less parent invocation
returns normally, while the lifecycle case launches real child processes with
the recovery database locator. Only real case markers qualify PostgreSQL work.

Additional authority filter, not yet freshly executed this run:
`/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p
oteryn-game-server --test combat_death_reward_postgres
tutorial_corruptor_preparation -- --nocapture`.

Unfinished at worker handoff: centralized PostgreSQL execution proof/markers and
durations; final root clippy. No completion or green claim is made for those.

## Central PostgreSQL RED — ordinary Bone Sword admission

Read `.questpc/r8-dispatch-central-pg-repaired.log`: all-five, lifecycle and
modifier scenarios reached their strict production-row preflight and failed
`Newhaven production kill reward admission: loot_item_inadmissible`. No Corruptor
completion marker was emitted. The process helper's parent guard returned normally;
that is not lifecycle qualification.

Lead traced the exact cause to ordinary Bone Sword3338: canonical loot fact remains
materializablefalse, while the existing native profile is materializabletrue and
NonStackable. The root-owned selected-native-fact projection omitted3338; other
ordinary Corruptor facts are admitted. Lead owns the producer/test repair and will
regenerate only after the current shared PostgreSQL batch ends. Corruptor-owned
source remains unchanged; no worker Cargo job was started.

## Read-only full admission review and authorized Mushroom repair

Lead requested a full-class review after3338. Exact native-policy review found
one additional reachable defect: ordinary White Mushroom3723 was canonical/loot-
fact materializabletrue but had no native policy. Both corpse-entry take and wire
Quick Loot consult `generation_item_facts`; without that policy, the real Item
cannot enter Character inventory. Source chance47600ppm/max2 makes this reachable.

Lead authorized the bounded repair in this worker's owned helper/test. The helper
now mounts canonical3723 unchanged: StackCapable/max100, known weight40, portable,
food regeneration108s and exact definition/binding. It refuses an inactive
canonical Item and verifies the current served mushroom appearance/name/portability.
No canonical source JSON or generated content was edited.

- RED `.questpc/r8-corruptor-mushroom-red.log`:9tests,1failure in4.684s; missing
  native policy count0 rather than1.
- GREEN `.questpc/r8-corruptor-mushroom-green.log`:9passed,0failed in3.940s.
- Read-only in-memory current producer + helper audit:10/10 ordinary Items have
  exact materializable native rows, CharacterInventory/Ground destinations,
  known weights and source quantities within native stack maxima;4/4 corpse
  profiles have30/30/30/None capacities and Ground-only admission;10/10 guaranteed
  gear profiles have exact portable rows and known equipment semantics. No other
  admission mismatch found. Current on-disk3338 loot fact remains false pending
  the lead's authorized regeneration; the producer's projected fact is true.
- Knight's real actor-occurrence witness now requires both ordinary Brass Armor
  and ordinary Mushroom. The framed Quick Loot route must move that exact mushroom
  UUID and unchanged quantity from corpse custody into Character41's actual
  backpack210. Source probabilities and loot quantities remain unchanged.
- Final leaf rustfmt and scoped diff checks exit0. No Cargo or content generation
  was run for this repair. `SOURCE_STABLE` sent to lead for one consolidated
  regeneration, PostgreSQL rerun and clippy.
