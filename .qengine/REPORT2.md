# QENGINE-2 local implementation and qualification

2026-11-06. AUTHORING in this worktree; no Git mutations, commits, pushes,
external publication, external review or other worktree writes. The bound META
3.1.0 policy is the immutable revision recorded in RESUME.md. The earlier
QENGINE-1 access work and its unresolved source questions are preserved.

## Implemented scope

- The promoted native catalogue has 62 non-store outfit identities, both donor
  sexes, 181 explicit compatible grant mappings, and addon bits 1/2. It includes
  Nightmare, Nobleman, Norseman, Oriental, Pirate, Rascoohan, Revenant and Rift
  Warrior. Unavailable newer appearance records are not fabricated. The existing
  Revenant base grant mapping is retained. Its content revision follows the
  Character gameplay content revision pinned by the native manifest.
- NPC reward callbacks compose carried item consumption, a native backpack MINT
  (optionally a filled container), account outfit/addon unlocks, an optional
  achievement and quest progress in one fenced QuestState transaction. The
  existing Character revision sequencer, custody guards, MINT audit and
  appearance owner remain authoritative.
- Input requirements aggregate across stacks and closed nested containers.
  Missing inputs, filled hand-in containers, unsupported stateful items and
  insufficient reward capacity refuse without partial effects. Existing account
  outfit facts retain their original receipts. Command replay cannot move value
  twice; quest predicates prevent a new immediate repeat grant.
- Native NPC greeting admission is derived from the callback registry, with
  exact NPC/default-handler greeting evidence from the committed donor corpus.
  Only greetings open here; the shared dispatcher owns every reward effect.
- Four physical Item profiles are admitted through the existing source importer:
  `i133`, `i139`, `i875`, `i3244`. The semantic comparison records exactly four
  additions and zero changes/removals among the previous 3,566 native records.
  Canonical Item definitions and existing holds are unchanged. Ordinary native
  regeneration retains these profiles and the promoted cosmetics section.

## Migration allocation

Took **0999**, `apps/game-server/migrations/0999_quest_reward_callbacks.sql`.
The repository next-free-number script was run against `origin/main` and open
PRs; its next free migration was 999, with open PRs holding through 998.
`numbers2-refresh.txt` retains the result. A separate read of live protected main
found migrations through 0990 (`live-migration-main.txt`), also below 0999.
No capability or command number was added.

The migration adds immutable same-transaction reward source evidence and extends
only the existing carried quest consumption proof arm for these command-bound
sources. Runtime/control grants and the existing use-callback proof stay intact.

## Qualification

Both shared pieces have three successful real-dispatch PostgreSQL quest
compositions. The final run (`rewards-pg-accepted.log`) passed **6/6**, zero
failures/ignored, in **402.28s**, with exactly six `POSTGRES_CASE_COMPLETED`
markers. No database-unconfigured early return occurred. This qualifies the
named reward stages, rather than claiming each complete quest journey.

| Piece / real quest composition | Real-dispatch PostgreSQL filter | Result / completion duration |
| --- | --- | --- |
| Outfit: Pirate sabre addon | `qengine_rewards_pirate_outfit` | PASS; 56.956123079s |
| Outfit: Knight sword addon | `qengine_rewards_knight_outfit` | PASS; 64.518478929s |
| Outfit: Deepling helmet addon | `qengine_rewards_deepling_outfit` | PASS; 83.170409551s |
| Exchange: Rest in Hallowed Ground holy water | `qengine_rewards_hallowed_ground_exchange` | PASS; 55.983934749s |
| Exchange: To Outfox a Fox helmet exchange | `qengine_rewards_outfox_exchange` | PASS; 62.662817588s |
| Exchange: Sam's Old Backpack hand-in / achievement | `qengine_rewards_backpack_exchange` | PASS; 78.984041962s |

Each case uses an isolated real PostgreSQL database and the production NPC
dispatcher. Successful rewards run through a persistent `serve_admitted` wire
conversation, and a literal retained-command replay proves no second receipt or
value movement. Outfit cases also require the emitted client catalogue delta
and select the resulting appearance through command 25. The male Pirate and
female Knight/Deepling assertions use exact donor look types.

The shared tests additionally cover missing inputs with unchanged progress and
receipts; multi-input refusal; partial stack consumption with seven units left;
closed nested custody; a full backpack refusal and retry; filled backpack
hand-in refusal; an existing account unlock with subsequent quest completion;
and optional achievement composition. Sam's later Kroox armor delivery is a
separate stage, not claimed here.

| Focused check | Result / evidence |
| --- | --- |
| Cargo check | PASS, seven touched crates: dispatch, durability, NPC, quests, world, entities and game-server; `rewards-check-qualified.log`. |
| Clippy | PASS, the same seven crates with `--locked --all-targets --all-features -- -D warnings`; final dispatch test-helper delta also passes (`rewards-clippy-last-delta.log`). Existing vendored dependency warnings are unchanged. |
| Rustfmt / diff | PASS, `cargo fmt --all --check`, explicit edition-2024 mounted-file check, and `git diff --check`; `rewards-*-last-delta.log`. |
| Quest reward loader | PASS, 1/1, 0.82s; `rewards-unit-qualified.log`. |
| Native NPC admission | PASS, 1/1, 27.60s; `rewards-npc-qualified.log`. |
| Session appearance reducer | PASS, 2/2, 0.11s; `rewards-session-qualified.log`. |
| Content repository | PASS, 5/5, 40.13s; `rewards-content-qualified.log`. |
| Offline reward authoring / source | PASS, 7/7, 3.399s plus source reproduction; `rewards-revision-green.log`, `rewards-revision-source.log`. |
| Existing grant mappings | PASS, 3/3, 0.130s; `rewards-mappings-continuation.log`. |
| World pins | PASS, regenerated and checked; unchanged reward-delta bundle digest `d4071ed1808cc027b4d52501d886887b5615966f5d8e58e0a38ae2032d49ea78`; `rewards-revision-pin-{refresh,check}.log`. |

Red evidence retained:

- `rewards-red.log`: original six missing callbacks fail.
- `rewards-unit.log`: wrong canonical track ownership rejected.
- `rewards-pirate-context.log`: inconsistent historical creation metadata
  rejected by the real Character authority. The fixture now binds sex correctly
  into creation root, receipt and bootstrap floor, with no disabled trigger.
- `rewards-pirate-birth.log`, `rewards-pg2.log`: native NPC greeting admission
  and the one-command helper's conversation reset exposed before repair.
- `rewards-npc-unit.log`: Navigator missing from the native talk book; its
  captured default-handler greeting now participates in ordinary admission.
- `rewards-native-items-red.log`: missing native Item custody facts rejected.
- `rewards-pg3.log`: the first two exchange compositions pass; outfit tests
  expose closing the wire before the normal appearance refresh, and the capacity
  fixture exposes coin-stack merging. The final tests wait for the catalogue
  predicate and prove all backpack slots are occupied using full stacks.
- `rewards-pg-continuation.log`: three exchanges pass, while all three outfit
  catalogue assertions fail. The fixture selected capabilities from the static
  offer list, which intentionally excludes dynamically gated appearance
  capability 25. It now uses the same admitted-cosmetics offer gate as production
  and asserts that capabilities 6 and 25 were actually selected. The persistent
  test client also answers normal liveness probes. No production guard, timeout
  or assertion was relaxed for this repair.
- `rewards-pg-qualified.log`: catalogue delivery succeeds, but all three
  appearance selection commands refuse the unrelated cosmetics revision.
  `rewards-revision-red.log` reproduces the offline producer mismatch
  (`quest-cosmetics-r1` versus `s3b-content-1`). The producer now uses the native
  gameplay revision, and the test generation helper binds both training and
  cosmetics to its `content-1` Character fixture before sealed activation.
  Regeneration hashes newly generated cosmetics bytes instead of their previous
  disk version. Seven authoring/source tests pass in
  `rewards-revision-green.log`; source reproduction passes as well.

## Exact focused commands

All Cargo commands use the absolute queued wrapper, `--locked`,
`CARGO_NET_OFFLINE=true` and `CARGO_BUILD_JOBS=5`. PostgreSQL scope is filtered to
this worker's mechanics; no full PostgreSQL suite or full nextest is run.

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked --quiet -p oteryn-gameplay-dispatch --lib qengine_rewards_ -- --nocapture --test-threads=1
/home/mole/oteryn-cache/codex-bin/cargo test --locked --quiet -p oteryn-gameplay-quests --lib reward_callbacks -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked --quiet -p oteryn-gameplay-npc --lib qengine_reward_npcs -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked --quiet -p oteryn-session --lib appearance_session_tests -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked --quiet -p oteryn-game-server --test content_world_project_repository
```

## Changed files and unfinished work

The QENGINE-1 inventory remains in CHANGED_FILES.md. QENGINE-2 adds the reward
callback/offer/greeting registries, promoted outfit authoring/native catalogue,
quest callback loader, NPC talk admission, shared NPC callback adapter, fenced
durable reward composition, migration 0999, focused PostgreSQL cases, closed JSON
schema, authoring tests, source Item profile importer and native-generation hook.
The exact inventory includes 77 product/tool/content paths across the preserved
QENGINE-1 and current QENGINE-2 work, plus local reports and evidence.

The full current product inventory, including preserved access work, is
[CHANGED_FILES2.md](CHANGED_FILES2.md). The continuation repairs test capability
negotiation and liveness, aligns cosmetics with Character content revisions, and
hashes new cosmetics bytes during same-pass generation. Earlier access work and
the reward transaction implementation are preserved.

## D1462 self-review

Re-read the complete handwritten diff and new files against both the requested
`/home/mole/oteryn-cache/CODEX-CHECKLIST.md` and the repository checklist. Reviewed
generated inputs through their producers, source pins and semantic comparisons;
the Item provider adds exactly four records with no existing record changes.
Searched new enum variants, structs, constructors and methods throughout
`apps/`, `crates/`, and `tools/` (`rewards-constructor-final.txt` and
`rewards-sites-continuation.txt`). The added fixture sex variant is handled by
creation metadata, receipt bindings and bootstrap sex; existing fixtures keep
their earlier meaning.

The new public reward loader/extension and revision-slot entry point have
production callers and are exercised by the six named real-dispatch cases.
Reviewed stable cause/Character/account/item locking, bounded carried input
reads, exact counts, immutable receipts, rollback on every refused participant,
same-transaction deferred proofs, existing fence/deadline enforcement, replay
and pending-command reconciliation. The auto-mounted dispatch library tests
need no new test target or shared mount edit. No protocol code or runtime role
was added, no existing test was deleted or weakened, and no forbidden CI-policy
file was touched. Independent review and CI remain coordinator work after local
authoring; this report makes no mergeability claim.

Unfinished: no local implementation or validation failure remains in these two
shared pieces. Coordinator work remains: independent exact-candidate review,
CI, lane integration and publication. Complete quest journeys and Sam's later
Kroox delivery remain lane work. The earlier QENGINE-1 source holds are unchanged.
No unrelated failure is being deferred as CI-only. No mergeability or protected
integration is claimed.

## CALLBACKS FOR LANES

Registry: `content/quests/shared-reward-callbacks.json`, top-level
`{"schema":1,"callbacks":[...]}`. Schema and exact field semantics are in
`tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json` and
`shared-reward-callbacks.md` beside it.

Exact current callback names:

- `oteryn:quest-reward/pirate-sabre`
- `oteryn:quest-reward/knight-sword`
- `oteryn:quest-reward/deepling-helmet`
- `oteryn:quest-reward/hallowed-holy-water`
- `oteryn:quest-reward/outfox-helmet`
- `oteryn:quest-reward/sam-backpack`

For another source-backed composition, add a unique
`oteryn:quest-reward/<name>` with fields `key`, `quest`, `npc`, `keywords`,
`topic`, `next_topic`, `storage`, `take`, `give`, `outfit`, `achievement`, `reply`,
`sources`. Each storage entry is `{track, minimum, maximum, set, preserve?}`;
at least one changed value must leave its inclusive accepted range. `take` is up
to eight distinct `{item,count}` entries with counts 1..65535. `give` is null or
`{item,count,contents?}`, one native root with count 1..100 and up to twenty
`{item,count}` children. `outfit` is null or `{outfit,addons}`: mask 0 grants base,
1 base/first addon, 2 base/second addon, 3 base/both. `achievement` is null or the
canonical achievement key. Both sexes share the same canonical outfit key.

Confirmation offers use `content/quests/shared-reward-offers.json`, top-level
`{"schema":1,"offers":[...]}`, with `npc`, `keywords`, `topic`, `next_topic`,
`storage`, `reply`; their storage entries are predicates only. Native NPC greeting
evidence and Item custody facts must be admitted for new referenced identities.
The registry feeds native greeting admission automatically; no capability,
command or per-quest Rust adapter is needed. These are shared reward-registry
callbacks, not extra fields in `world-callbacks.json`. Preserve all existing
shared records when integrating lane additions.
