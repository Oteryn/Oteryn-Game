# Mechwire package S: row -> client-to-server test (base 60e40aae)

Scope: the rows of `apps/game-server/src/qualification/SCENARIO_MATRIX.md` that start with S and
that `IMS.md` marks BOTH-WIRED. S16c/S16d (Dawnport) and S17 (livestream) are owner-excluded.
SERVER-ONLY/CLIENT-ONLY rows (S03, S07, S14a-c) are out of this package.

Path roots: `CP` = apps/game-server/src/qualification/client_phases; `PG` =
crates/gameplay/dispatch/src/gameplay_transport/postgres_tests. Every CP phase boots a real node
and drives `oteryn_session::Session` through `oteryn_dev_client::DevClientSession`, using
client_runtime/protocol-oteryn command builders and decoders. `world.rs` is CP/world.rs, which runs
the phase under `e2e_row_selected`.

"Verified how": every CP phase was read line by line (send path, decoder, assertion) and the
game-server test target type-checks (`cargo check -p oteryn-game-server --lib --tests`, exit 0).
None was run against a node here: node boot needs the disposable Platform topology with
PostgreSQL 18.6 (`tools/qualification/e2e_all/run.sh`), which this sandbox lacks. The last
recorded node-boot row run (E2E_ROWS_REMAINING.md) had 0 PASS.

| row | test (file:line) | new/existing | client send -> decode -> assertion | relog | gap |
|---|---|---|---|---|---|
| S01 | CP/client_row_s01.rs:56 (world.rs:1933) | existing | PartyCommand + runtime::send; decode_party_result; one invitation, two members, leadership passed | no (party is not durable) | - |
| S02 | CP/client_row_s02.rs:70 (world.rs:1940); CP/full_row_s02.rs:244 (world.rs:979) | existing | party + gated party_skull query; decode_party_icons; exact shield/skull in full row | full row: fresh roster and icons | - |
| S04a | CP/full_row_s04a.rs:219 (world.rs:972); CP/client_row_s04a.rs:66 | existing | VIP encode_request + gated_command; wire::decode_result; entries, groups, presence | full row: identical view after relog | client row has no presence step (full row has) |
| S04b | CP/client_row_s04b.rs:74 (world.rs:1961) | existing | Session::chat PrivateChannel; ChatChannelList, ChatLine::PrivateChannel, NotInvited | no (channels are transient) | - |
| S04c | CP/full_row_s04c.rs:94 (world.rs:965); CP/client_row_s04c.rs:52 | existing | Session::chat; chat_log; exhausted and muted lines absent | full row: mute retained | - |
| S05 | CP/full_row_s05.rs:227 (world.rs:888); CP/client_row_s05.rs:50 | existing | house query/mutation builders + gated_command; house::page/result | full row: fresh admission state | refusal accepts any disposition 4..=10; guildhall needs a guildhall row in content |
| S06 | CP/client_row_s06.rs:59 (world.rs:2059) | existing, extended | appearance::Change::encode + gated_command; ChangeResult and Snapshot decode; committed selection shown, unearned addon Rejected | **new**: logout, fresh login, the same selection and root revision read back through the client snapshot decoder | mount/taming: CP/full_row_s06.rs:223 always fails, content gap (no legitimate acquisition of taming item 16251 in the world) |
| S08 | CP/client_row_s08.rs:40 (world.rs:2066) | existing | attack_target; pvp_expert encode; decode_expert_fight_modes == Red Fist; party_skull skull 3; victim health drop | no | - |
| S09 | CP/client_row_s09.rs:18 (world.rs:2122) | existing | Session::logout; LogoutOutcome Accepted | yes: position kept, session renewed | - |
| S10 | CP/full_row_s10.rs:245 (world.rs:867); CP/client_row_s10.rs:95 | existing | npc_service encode + gated_command; npc intent/conversation/trade-window decoders | full row: position, gold, Rashid shop | needs Rashid placed at the UTC-day destination |
| S11 | CP/client_row_s11.rs:37 (world.rs:2150) | existing | highscores::command + gated_command; decode_highscores_result; identity and row cap; category 13 Rejected | n/a (read-only) | row contents not compared |
| S13a | CP/client_row_s13c.rs:150 via CP/client_row_s13a.rs:8 (world.rs:2087) | existing, tightened | Session::cast_spell solo; **now requires disposition `Rejected`** (the wire mapping of `CastRejection::NoPartyMembers`, crates/gameplay/spell/src/spell/cast/gate.rs:29) and unchanged vitals; before any non-Cast passed | no | wire carries no refusal detail for the party gate, so a PZ refusal would also read `Rejected` |
| S13b | CP/client_row_s13c.rs:150 via CP/client_row_s13b.rs:7 (world.rs:2080) | existing | in-party cast; mana drop on own vitals | no | - |
| S13c | CP/full_row_s13c.rs:200 (world.rs:958); CP/client_row_s13c.rs:150 | existing | cast + party/effects gated_command; PartyEffects decode | full row: member condition after fresh admission | - |
| S15a | CP/full_row_s15a.rs:15 (world.rs:2655); PG/s15a_login_protection.rs:12 | existing | no wire message: client asserts no monster damage until admitted+10 s, then monster damage and HP loss | no | client signal is indirect (a monster that never engages gives a vacuous quiet window); the exact window is proven server-side by the PG case |
| S15b | CP/full_row_s15b.rs:63 (world.rs:860); CP/client_row_s15b.rs:21 | existing | attack_target; InFight with retry_after; fresh-admission protection | full row: reconnect via recover | - |
| S16a | CP/client_row_s16a.rs:126 (world.rs:2143) | existing | npc_service encode + gated_command; exact disposition and conversation line | no | - |
| S16b | CP/full_row_s16b.rs:248 (world.rs:853); CP/client_row_s16b.rs:17 | existing | general_stats validate; SkillProgressSnapshot decode; level-8 values | full row: stats, skills, vitals after fresh admission | client row sends a hand-built `[1,0,0,1]` payload (no builder); needs credited rat spawns |
| S18 | CP/client_row_s18.rs:57 (world.rs:1968) | existing | Session::chat Say; ChatLine::Local mode/text, same speaker | no (speech is transient) | yell/range/mute pending per matrix |
| S19 | CP/full_row_s19.rs:95 (world.rs:2697) | existing | client_runtime::player_typing::send; SessionEvent::PlayerTyping for the peer | n/a (transient) | - |

Counts: 20 rows; 20 with an existing client-to-server test, 2 of them changed in this package
(S06 relog added, S13a refusal tightened), 0 new files; content gaps: S06 mount/taming, S05
guildhall, S10 Rashid placement, S16b rat spawns.

## Harness finding (not fixed here)

Thirteen client rows return without any evidence line unless `E2E_ALL_CAPTURE_DIR` is set:
S04c, S05, S06, S08, S09, S10, S13a, S13b, S13c, S15b, S16a, S16b, S18
(`run_packet` begins with `if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() { return; }`).
In a run without that directory they report neither PASS nor FAIL. Client rows report
`status=CAPTURED`, never PASS. The rows that also have a `.full` phase are still covered then;
S08, S09, S16a and S18 are not.
