# QUESTPC run 8 catalogue qualification A..M

Head: `1fa04bbbddb5d96e21fecca0bc456f3c5e1109b7`, local AUTHORING only.
Owned source: `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_catalogue_chests.rs`.

Added focused test `quest_catalogue_chests_a_m_unqualified_source_placements_use_replay_and_refusal`.
Its frozen list contains exactly the 132 claims assigned at run-8 start. Its ownership guard uses the actual coverage quest identity suffix first ASCII letter a..m, retaining leading articles; it never infers a title from a claim/storage/item key. Selection survives promotion from supported_unqualified to dispatch_pg_qualified without emptying or expanding the test batch.
132 supported-unqualified claims, 140 source placements.
The existing complete source sweep is unchanged.

Each selected scenario calls the existing source-world binder, literal production USE dispatch, durable native mint, retained duplicate replay, new-command refusal, cold-owner replay, custody/atomicity assertions, and stale-fence refusal.
Reward-only proof does not certify access callbacks.
The runner emits `QUEST_CATALOGUE_CHEST POSTGRES_CASE_COMPLETED <claim> <position> native_source=<bool> duration=<elapsed>` only after the complete scenario succeeds.

Command (first attempt):
`QPC_SLOT=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --all-features --lib quest_catalogue_chests_a_m_unqualified_source_placements_use_replay_and_refusal -- --nocapture`
Log: `.questpc/r8-catalogue-pg.log`.
Mapping: `.questpc/r8-catalogue-claims.json`.

First attempt: 0 passed, 1 failed in 0.31s; zero placement completion markers. The production generation rejected stale shared content with `RevisionMismatch("native gameplay input digest")`. Preserved first log. Root has the coordinated native generation transition; wait for stable signal before repeat.

Unfinished: qualified PG execution pending stable generation. Two coverage `unlinked` claims remain outside scope by lead routing (Rookgaard mushrooms R and Witch House W); key3620 is explicitly routed to Mintwallin M below. No source evidence/access-path qualification inferred.

Stable-content attempt: compiler E0277 in `crates/gameplay/durability/src/durability/tutorial_viola.rs:136` (`EquipmentError` cannot convert to `DurabilityError`); exit101,118.955s wall, zero tests/markers. Preserved `.questpc/r8-catalogue-pg-stable.log` and `.result.json`; lead notified for owned-lane repair. No code changed outside allocated catalogue file.

Next compile attempt: rustc terminated by SIGTERM15 compiling dispatch libtest, no source diagnostic; exit101,502.678s wall, zero tests/markers. Preserved `.questpc/r8-catalogue-pg-final.log` and result.

Lead explicitly routed unlinked key3620 to M after fresh TibiaWiki `Key 3620` identifies Mintwallin Prison Key and barracks drawer, superseding stale Black Knight donor section. Added separate focused `quest_catalogue_chests_mintwallin_prison_key3620_use_replay_and_refusal` with fixed32411,32155,-15/uid5009/appearance2434/root2969/key3620 assertions and complete shared dispatch scenario. Frozen132batch unchanged. Current attempt `.questpc/r8-catalogue-pg-qualified.log` followed by `.questpc/r8-catalogue-pg-key3620.log` on slot4 if batch passes.

Lead observed simultaneous worker rustc SIGTERM at500s and consolidated final dispatch build/tests into one warm slot6. On lead request interrupted retry session81989 with Ctrl-C; tool confirmedexit130. No further worker Cargo starts or source writes. SOURCE_STABLE: frozen132test plus separateMintwallinKey3620test complete; final consolidated root log provides remaining PG evidence.

Central repaired execution completed with132claim test FAILED at key3520/i2973 DefinitionNotFound after110 in-batch placement markers; separate Mintwallin key3620 test passed with marker6.438445793s. The extractor requires the individual132test to pass, so zero132batch claims qualify despite prior markers. Whole133/151Item dependency census found exactlyone missing profile (i2973), all other150 exact tuples valid. Read-only proposal `.questpc/r8-catalogue-native-mount-proposal.md` returned to lead; no production edits made. Await lead-owned registration and whole132 rerun.
