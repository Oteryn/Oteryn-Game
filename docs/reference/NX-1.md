# NX-1 — ambient sounds, detailed refusals and GM tools

Delivery base: `faf8ae8700d935bca612261730aca5466526389a`, as directed by the owner.
Original base: `1fa04bbbddb5d96e21fecca0bc456f3c5e1109b7`. No commit or push.

## Scope and source decision

W24 and ADD-0066 were already implemented in the supplied base (PRs #2998 and
#2999 respectively); this task verifies their production paths rather than
introducing duplicate producers. S12 extends the existing signed GM-1 transport,
reports and analyzers with native staff tools. S17 is outside this task.

Owner decision, 2026-11-06: current-Tibia GM tooling is not publicly documented
beyond the donors. Use the Crystal/Canary action catalogue and donor group model
(GM/CM/God) because no current-Tibia source was reachable. The attempted MediaWiki
queries were:

- `https://www.tibiawiki.com.br/api.php?action=query&prop=revisions&rvprop=content&rvslots=main&titles=Gamemaster&format=json&formatversion=2` — HTTP 403.
- `https://tibia.fandom.com/api.php?action=query&prop=revisions&rvprop=content&rvslots=main&titles=Gamemaster&format=json&formatversion=2` — HTTP 402.

These failures do not block the owner-approved donor model. No Dawnport, OT
commands, autoloot, OT VIP, auras or Prey Hunting Tasks are introduced. Newhaven
starting equipment and other packages' mechanics are unchanged.

Public sources were fetched read-only at immutable revisions:
`zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8` and
`opentibiabr/canary@47dfd51f45280a59a1d3e50ba7edd573d7234446`.
Donor evidence remains an OTS hypothesis, with the above explicit owner acceptance
for this catalogue and tiers; it is not represented as independent CipSoft proof.

| Rule | Donor file:line | Production implementation / accepted difference |
| --- | --- | --- |
| City day/night ambient loops and stop outside zones | Crystal `data-global/scripts/custom/ambient_sounds.lua:1-36,41-50,66-107`; `src/creatures/creatures_definitions.hpp:862-901` | Existing `ambient_sounds.rs` producer, observer, capability 288/domain 101 and native loop owner. Pinned city/subregion membership is the existing recorded Oteryn choice. |
| Detailed step/use refusal | Canary `src/utils/tools.cpp:1392-1415,1601`; `data/scripts/actions/doors/key_door.lua:32` | Existing capability-66 reference-message decoder and native step/context-use feedback. |
| GM/CM/God permissions | Crystal `data/XML/groups.xml:20,68,116` | Signed roster roles GM/SENIOR_GM/ADMIN map to donor groups 4/5/6; mute/unmute require God. Roles never arise from capability selection. |
| Kick / inspect | Crystal `data/scripts/talkactions/gm/kick.lua:3-24`; `gm/info.lua:3-11,18-37,53` | Signed, audited actions and actual native result. Inspect exposes gameplay facts, position and staff/sanction state; donor IP/private account dossier is excluded. |
| Ban / unban | Crystal `data/scripts/talkactions/gm/ban.lua:17-34,49`; `gm/unban.lua:14-27` | Account-wide Game admission restriction, replay-safe signed operations; Platform account state is untouched. Ban duration is explicitly bounded to 1 ms–365 days, reason to 1024 bytes. |
| Exact position / goto / bring | Crystal `data/scripts/talkactions/gm/position.lua:15-26,35`; `gm/teleport_to_creature.lua:12-22`; `gm/push_creature.lua:18-33` | Live runtime movement, collision checks and deterministic nearest-free placement; same-channel world tools, refusal on blocked/offline/stale targets. |
| Broadcast / private notification | Crystal `data/scripts/talkactions/gm/broadcast.lua:3-10,29` | Existing native server-message delivery in the signed current Channel (the existing GM-1 scope boundary); private notification is the single-recipient subset. Signed notice text uses the existing ASCII control grammar, bounded to the 255-byte message ceiling and never truncated. |
| Ghost / invisible | Crystal `data/scripts/talkactions/gm/ghost.lua:8-16,23`; `src/creatures/players/player.cpp:1383-1400` | Ghost hides from ordinary observers while all access staff can see it, bypasses player occupancy and is excluded from AI and PvP targets. Ordinary invisibility retains physical occupancy and follows sense-invisible rules. Changes bump observation revisions. |
| Mute / unmute | Crystal `data/scripts/talkactions/god/mute.lua:26,32,55,61` | Existing bounded-duration audited sanctions, now God-gated on both transports. Historical signed audit validation remains compatible. |

## Authority, delivery and native use

Capability **330**, command **560** (request) and **561** (receipt) extend the
existing capability-51 signed control chain. Migrations **0988** and **0989** are
the only new migrations. Internal Activity terminal-return reasons **10** (active transport) and **11**
(recorded control-loss epoch) represent staff-forced return; neither is a new
protocol message. No other assigned numbers
are consumed.

`Ctrl+Alt+0` opens the native staff panel. It imports operator identity, exact
canonical signed authorization and detached hexadecimal signature. The help lists
the catalogue; rich results show actual inspect facts or the runtime action outcome.
Signing keys remain off-host. Imports are bounded, malformed authorizations fail
closed, pending submissions cannot be duplicated, and authorization material clears
after confirmation or session end. Headless native play tests exercise the actual
PlayLink/session driver; a Windows window and audible playback are not manually
exercised in this Linux environment.

Capability 330 requires `issuer_character_id` inside the signed request to match
the fenced current character. Initial signed self-inspection binds the trusted
operator to that character. An inspection signed for another carrier cannot grant
staff authority. Legacy capability-51 canonical requests remain byte-compatible.
Every execution checks current roster/activation, signature, scope, generation,
sequence, time and durable audit; current authorization is checked again before
pending world effects. Protected staff cannot be sanctioned through a lower-trust
player carrier.

A durable accepted intent is distinct from physical completion. The reply carries
the existing signed result plus bounded runtime information; durable Pending /
Applied / Refused state prevents replaying movement or messages. Pending targets
capture both connection and scope-ownership generation, preventing an uncertain
old action from mutating a successor session. Runtime control revisions also reject
same-session reconnects or terminal write fences that race the awaited SQL reads. Live bans/kicks trigger the existing
save/release machinery with exact fences, including Activity return and already
lost sessions polled within one second without extending original grace, while ordinary
logout combat restrictions remain intact. Unban does not resurrect old sessions.
Ghost/invisible state comes only from current trusted bindings and is refreshed
before snapshots and incoming commands; absent authority cannot retain privilege.

## Validation

Focused command output and durations are recorded in the delivery report. No full
nextest run or full PostgreSQL suite is used. Registry and registration goldens are
regenerated only with repository tools. New rules, codec/native integration and
PostgreSQL cases cover rejected signatures, replay, SQL binding tamper, protected
targets, account-ban scope/expiry, live forced release, world effects and visibility.

Ghost player visibility follows Crystal `src/creatures/players/player.cpp:1383-1396`: the observer always sees itself; any access group sees ghosts, ordinary players do not. `:1398-1400` permits access players and ghost targets to be walked through. No inferred same/lower-tier ghost restriction is added.

Two integration regressions exposed by the client crate tests are corrected:
movement-spell speech uses its own local command origin so an ordinary chat reply
cannot clear that panel's pending request, and the appearance play-test readiness
predicate waits for the actual catalogue before exercising it. The latter retains
all catalogue assertions. These are client flow fixes, not S17 casting changes.

Fixture corrections preserve the production invariants: tool actors bind the
actual current Character lease, fresh admission after kick uses the persisted
successor lease generation, and the two deliberately superseded-transport cases
require disconnection before any command result (the zero-write assertions stay).
The test-only trusted-clock override is isolated per current-thread scenario so
one test's unavailable clock cannot alter another test. Successful NX-1 fixtures
close and drop only their own temporary databases through the repository harness.

## Focused verification (2026-11-06, rebased onto faf8ae87)

All commands below exited zero on the delivered source tree. No full nextest or
full PostgreSQL suite ran. The shared `staff` filter ran 16 tests, including eight
new NX-1 PostgreSQL cases and the existing signed mute/receipt/restart case.

| New PostgreSQL filter | Executed wall time |
| --- | ---: |
| `nx1_imported_staff_signature_preserves_bytes_and_replays_once_postgres` | 4.937 s |
| `nx1_staff_tools_fenced_ack_and_signed_sql_bindings_postgres` | 7.631 s |
| `nx1_staff_ban_unban_account_scope_live_guard_replay_and_expiry_postgres` | 7.151 s |
| `nx1_staff_tools_teleport_bring_ghost_invisibility_and_notices_postgres` | 53.288 s |
| `nx1_staff_signed_kick_saves_in_fight_and_releases_exact_session_postgres` | 5.695 s |
| `nx1_staff_activity_return_requires_current_transport_and_signed_decision_postgres` | 6.339 s |
| `nx1_staff_captured_kick_rejects_lost_resume_before_generation_mutation_postgres` | 6.313 s |
| `nx1_staff_lost_activity_return_keeps_epoch_proof_and_forces_terminal_postgres` | 5.775 s |

Existing-row server checks also executed: `reaudit_ambient_sounds_postgres` in
60.553 s and `refusal_detail_bundle_world_door_too_far_with_and_without_capability_66`
in 48.48 s. The client run passed 319 library tests, two ambient play tests and two
staff driver play tests. Session passed 169 library tests; client-runtime 30 and
client-ui 262 library tests, with their integration suites also passing. Entities
passed 250 library tests plus its integration and compile-fail documentation tests.
The focused durability/rules/wire staff filters passed 2/8/5 tests respectively.

Exact commands (each Cargo/PG shell sources the prepared env first):

```sh
cargo test --locked -p oteryn-gameplay-dispatch --all-features staff -- --nocapture
cargo test --locked -p oteryn-gameplay-dispatch --all-features reaudit_ambient_sounds_postgres -- --nocapture
cargo test --locked -p oteryn-gameplay-dispatch --all-features refusal_detail_bundle_world_door_too_far_with_and_without_capability_66 -- --nocapture
cargo test --locked -p oteryn-gameplay-entities --all-features
cargo test --locked -p oteryn-gameplay-durability staff
cargo test --locked -p oteryn-client --lib --test staff_nx1 --test ambient_b24
cargo test --locked -p oteryn-client-runtime -p oteryn-client-ui -p oteryn-session
cargo test --locked -p oteryn-gameplay-social-rules staff
cargo test --locked -p oteryn-protocol-oteryn staff
cargo clippy --locked -p oteryn-gameplay-dispatch -p oteryn-gameplay-durability -p oteryn-gameplay-entities -p oteryn-gameplay-social-rules -p oteryn-protocol-oteryn -p oteryn-client -p oteryn-client-ui -p oteryn-client-runtime -p oteryn-session --all-targets --all-features -- -D warnings
cargo fmt --all --check
oteryn-architecture-check workspace .
```

Independent review found and closed the target-resume race and the missing session
FK index. The rebase/layout review found no open P0/P1 issues. Its source-manifest SHA-256 was
`bec6b67189aadb8c78224717e66961d64acfe7d6a4bbbf5e3bbb9039f5b826c5`;
subsequent edits only record validation. Historical mechanics-audit files are
left unchanged; completion is recorded here for coordinator integration.
Initial red evidence includes missing new APIs, native pending-result routing and
shortcut failures, advertised-capability ordering, and production-incomplete fixture assumptions. Assertions remain in place; no
ignored tests or broadened production authority were used to make checks pass.

A full disk interrupted validation. The task's dispatch incremental cache was
preserved in the free `/tmp` filesystem with a symlink at its original path;
compiler flags/incremental compilation were unchanged. PostgreSQL data was not
removed for this recovery. The focused run above was performed afterward on the rebased tree.

## Rebased delivery

Reapplied without commits onto `faf8ae8700d935bca612261730aca5466526389a`.
The upstream MovementSpeech implementation replaces the duplicate local adapter.
Staff mechanic ordinal is 638; 640 and capabilities 381–383 remain reserved for
NX-8. Capability 330, commands 560/561 and migrations 0988/0989 are unchanged.
New modules are split only to keep each plain-text delivery part below 15,000
characters while ending at a file boundary.

| Row | Server | Client |
| --- | --- | --- |
| W24 | DONE (existing producer verified) | DONE |
| ADD-0066 | DONE | DONE (existing native step/use flow verified) |
| S12 | DONE | DONE |
