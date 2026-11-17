# Multi-bot world journeys

Run on a node-boot runner shard with `NODE_BOOT_WORLD=1` and
`NODE_BOOT_E2E_ONLY=mp`. Acceptance is the indexed per-row verdict
`SEAM_EVIDENCE e2e_all scenario=MP01.full status=PASS|FAIL`, together with
`SEAM_EVIDENCE MP_CHECK MP01 <step> PASS|FAIL <detail>` observations.
Compilation is not runtime qualification. Node boot needs Docker and the pinned
Platform checkout; it is run by the owner on the GitHub runner for this packet.

Add one file per mechanic family, named `mp02_<family>.rs`, then MP03, etc.
Add its module declaration in `mount.rs` in name order and its one indexed
`verdict::indexed("MP02.full", ...)` call in ID order. Keep scenario-specific
assertions in that file; add a helper to `bots.rs` only when a scenario uses it.
Every scenario must emit a terminal PASS or FAIL, even after a refusal or a
cleanup error. Unsupported commands fail with their actual status/disposition
and source location. Dependent checks fail explicitly; none silently skip.

MP01 runs on two Worlds, one after the other. On the no-PvP World its roles own four
distinct Accounts, Characters and sessions:

| Bot | Pair/index | Setup | Role |
| --- | --- | --- | --- |
| A | core/0 | knight | party leader, refused attacker |
| B | packet/0 | druid | invited party member |
| C | packet/1 | paladin | invited party member |
| D | core/1 | **never provisioned** | solo refused target, D150 no-vocation path |

On the Open PvP World (`world_type=2`) two more Accounts own one Character each:

| Bot | Pair/index | Setup | Role |
| --- | --- | --- | --- |
| A | pvp/0 | knight | PvP attacker |
| B | pvp/1 | sorcerer | PvP victim |

Both PvP Characters are provisioned at level 100, above `PVP_MIN_LEVEL` (8,
`legality.rs:7`); an unprovisioned starter is below it and is refused
(`legality.rs:96`). The PvP pair has no party: the party journey stays on the
no-PvP World. Only the existing root-only `provision.py --profile` command sets up
A/B/C and both PvP bots, each through its own node's control socket (`bots.rs`,
`control_socket`).
Profiles are explicit, so `NODE_BOOT_PROFILE` cannot change these roles. D's
vocation is checked through its own General Stats command and PostgreSQL;
the relog checks its no-vocation observation again. Equipment, party, movement,
fight modes, attacks, history queries and logout all use each bot's own
`protocol-oteryn` session. Combat tiles come from the pinned world's PZ flags;
the victim's temple comes from its durable hometown and the pinned catalogue.

Assert each participant's own client view and the corresponding durable state.
MP01 queries party membership separately as A, B and C, confirms D stays solo,
and checks the same party/leader/member names through bounded PostgreSQL reads.
Its read-only observers follow the existing qualification `death.rs`
`latest_receipt`/`PgConnection` pattern. They never acquire a fabricated runtime
write authority, update SQL, inject a combat event or synthesize a receipt.
Production fenced party reads require the serving process's current runtime
authority, which this remote protocol harness does not own.

### Two Worlds, two nodes

The no-PvP World is unchanged: `tools/qualification/node_boot/run.sh:1021` seeds
`game_spell_field_world_policies` with `world_type=1`. MP01 checks its rules
there: `nopvp-expert` requests Red Fist and requires the served expert mode to be
Dove (`pvp_expert.rs:83-92` set, `:121-131` observe); `nopvp-attack-refused`
requires A's attack on player D to come back `Rejected` (`pvp_live.rs:184`,
`legality.rs:90`) and D to take no damage in a 5 s window.

After the no-PvP bots are released, MP01 runs the PvP rows on a second World,
`world_type=2` (Open PvP, `run.sh:1291`): `pvp-world` (the served policy is
Open PvP, the fixture Characters and scope generation exist), `pvp-admissions`,
`pvp-profiles`, `pvp-combat` (Red Fist served, attack, attributed hit and damage,
the kill and temple respawn, the new durable death receipt, A's kill and B's
death history views, the frag points row, A's white skull in both clients and
the points/skull state against PostgreSQL), `pvp-stop-combat`, `pvp-relog`
(sessions, positions, histories and PvP state persist) and `pvp-release`. Every
row is PASS or FAIL; a failed `pvp-world` or `pvp-admissions` fails the rows it
blocks.

One node process serves one World/Channel scope (`node/config.rs:81-83`, the
single `[scope]`), and one database holds one live node's S2 custody
(`crates/gameplay/durability/src/durability/native_admission_source.rs:376-388`:
the single `registration_id=1` row, and a live prior holder keeps it). The
existing stages also assert one node registration, one content activation and
one runtime guard in their database. So the Open PvP World has its own database
(`oteryn_node_boot_pvp`, in the same PostgreSQL container) and its own `serve`
process on the next port, with its own operator state, fence, control socket and
launch/S2 authorizations; Platform, PKI, secrets, binaries, the compiled bundle
and gameplay inputs are shared. `run.sh` stages `pvp_world_ready` (migrate,
operator setup, grants, the `world_type=2` policy row, content activation,
assignment, World epoch), `pvp_character_bootstrap` (two Platform intents with
`--target-world-id` the PvP World, bootstrapped through that node's socket) and
`pvp_world_shutdown` (before the existing lifecycle probes). The seam harness gets
the `NODE_BOOT_PVP_*` variables; MP01 builds the PvP pair from them and shares
the core pair's Platform descriptor, grant signer and gameplay certificate.

Relog always uses checked `logout_retrying_busy`; combat blocks expire on real
server time. All logout futures are serviced together, so one killer's natural
15-minute block does not starve the other sessions. New sessions must differ,
positions and death/kill histories must persist, and every participant checks
party/skull state again. Terminal logout removes party membership by design
(`world_party.rs:792`); the expected post-relog party is empty in both client
queries and PostgreSQL. White skull deadlines may expire naturally while
relogging, so compare against the database clock instead of freezing an icon.

Every polling loop has a condition and deadline. Always release partial setup
and failed scenarios, drop remaining sockets, and call `wait_character_released`
for both indexes of **both** pairs. Cleanup failure fails the row. Add no fixed
sleeps, direct SQL mutations, fixture shortcuts, server changes or ignored tests.

A failed logout names its cause (`diagnostics.rs`, `logout_failure_cause`). The
harness never closes a live transport before logout, so a transport error is the
server's close: `server-closed-before-logout-sent` when the pre-logout liveness
drain already fails, `server-closed-during-logout` otherwise. Its reason is the
server's `serve_end` record with `server_sequence` equal to the bot's
`observed_server_sequence` and `ts` matching the `MP_BOT ... outcome=fail` line's
`at_unix_ms`. The row stays FAIL; no reconnect or retry hides that close.

Local packet checks (one Cargo job at a time, source `env.sh` in each shell):

```sh
cargo fmt --all --check
rustfmt --edition 2024 --check apps/game-server/src/qualification/client_phases/mp/mount.rs
cargo clippy -p oteryn-game-server --all-targets -- -D warnings
cargo clippy -p oteryn-game-server --all-targets --features qualification-clock -- -D warnings
cargo test -p oteryn-game-server --lib --no-run
cargo test -p oteryn-game-server --lib --no-run --features qualification-clock
```

The explicit rustfmt check follows the block-mounted scenario files, which
workspace rustfmt does not discover through the function-local mount.

## Provisioning refusal trace (follow-up to run 43982858636)

The root-only control socket dispatches `qualify` at
`apps/game-server/src/node/serve.rs:2943`. Its refusal paths are:

| Boundary | Refusal paths |
| --- | --- |
| `serve.rs:2927,2982` | Non-root peer is closed before reading; request/processing deadline closes without a qualification reply. Malformed control requests use the fallback `rejected` branch at 2964. |
| `node/serve/qualification_fixture.rs:7,25` | Invalid UTF-8/request/session UUID/profile, disabled fixture mode, or unavailable native gameplay. |
| `qualification_fixture.rs:122` | Invalid session type, unavailable/stale canonical admission, or a current scope different from the configured World/Channel. `fresh_admission.rs:1741` checks the activity origin, stored admission receipt and current reconciliation. |
| `qualification_fixture.rs:136` | Invalid Character, missing/unreadable interpretation, unavailable progression source, incompatible served policy, or failed occurrence allocation. |
| `qualification_spell_character.rs:133` | Unsupported vocation/level, invalid occurrences/Character, failed build/progression read, non-NO_VOCATION build **or any existing progression row** (156–163), mismatched policy context, or absent level threshold. |
| `qualification_spell_character.rs:184` | Any ordinary fenced progression initialization, XP award, revision read, training-formula/vocation conversion or build commit failure; final loaded build must match the requested level/vocation (249–254). |
| `qualification_fixture.rs:189` | Invalid Character/item fence, inventory read failure, no starter backpack, missing/incompatible item catalogue/facts, failed fenced reward reservation/commit, or a non-Committed mint outcome (261–262). |

There is no qualified-session-count cap and no per-bot profile allowlist beyond
the named profiles and the fresh-character predicate. MP01 A is core/0; the
preliminary world entry uses core/1 (`world.rs:3004`). With only group `mp`, no
earlier scenario provisions A. A fresh session does not itself prove that its
Character still meets the fixture's build/progression preconditions.

### Comparison with the working core path

| Input / ordering | Core provisioning | MP01 provisioning |
| --- | --- | --- |
| Character / Account | core/0 (`world.rs:2937–2938`) | A = core/0; B = packet/0; C = packet/1; D = core/1 (`mp01_party_pvp.rs:35–40`). Each has its own Account. D is never provisioned. |
| Profile | `NODE_BOOT_PROFILE`, default sorcerer; explorer maps to knight (`provision.rs:11–16`) | Explicit knight, druid, paladin; no ambient profile substitution. All are accepted names (`qualification_fixture.rs:11–15`). |
| Session ID | `uuid_text(client.join_snapshot().game_session_id.as_bytes())` (`provision.rs:25`) | The same expression on that bot's connected session (`bots.rs`, `provision_as`). Neither uses an Account ID, Character ID, stale saved ID or pre-admission guess. |
| Socket / native state / scope | `NODE_BOOT_CONTROL_SOCKET`; same node-side prepare function | Same socket and handler. Both pairs inherit the same World/Channel/scope generation (`qualification.rs:1538–1542`). Native state and fixture-mode flag are node configuration, not selected by the bot. |
| Before qualification | First admission of core/0, checked logout, E2EF1 movement on core/1, fresh reconnection of core/0, then provision (`full_e2ef1.rs:24–40`) | Previously qualified immediately on the first joined session. Now performs checked logout and reconnect of the same Character before building the qualification request. It does not run unrelated E2EF1 rows or touch D. |
| After qualification | Checked logout and reconnection; assert build/items and equip axe (`provision.rs:48–109`) | Checked logout and reconnection; MP01 separately checks build/profile and equips A's axe. |

The concrete lifecycle difference is the pre-qualification logout/reconnection;
MP01 now follows the working core Character's sequence. This is not evidence
that the lifecycle difference caused either historical refusal. The runner must
establish that with the new warning if qualification still fails.

### Corrected progression diagnosis and refusal evidence

The earlier claim that admission necessarily initializes progression, making
qualification impossible, was too strong. Run 43985564575's accepted core
qualification contradicts that claim. The progression binding has conditional
outcomes: absent content (`composed_admission/progression.rs:21–26`), failed fence
or interpretation reads, incompatible policy/context (`character_progression_binding.rs:217–264`),
or missing progression at a non-initial root revision (`character_progression_binding.rs:294–296`)
do not imply a newly initialized row. The actual Character state must be observed;
merely locating the initialization branch does not prove it ran.

Runs 43982858636 and 43987830115 expose only `rejected`, which cannot identify the
inner error: `qualification_fixture::prepare` previously discarded it. It now
emits one `level=warn event=qualification_fixture_refused` line on that error
branch, with reason, validated session ID and profile. Reason text is bounded
to 200 characters before the structured logger applies its token limit; control
characters are sanitized, and potentially sensitive diagnostic text is replaced
with `redacted_sensitive_error`. The handler's reply and every refusal check are
unchanged. The Python helper retains the escaped, at-most-64-source-byte answer;
MP01 also retains its bounded stderr diagnostic in its FAIL detail.

### Run 43994145091: framing proof and missing log evidence

The three pre-build refusals now each emit `qualification_fixture_refused` with
one static reason: `fixture_disabled`, `native_unavailable`, or
`malformed_request`. They log no request bytes, unvalidated session/profile or
node proof. Their original evaluation order, accepted request grammar and
`rejected` reply are unchanged. Each class has a test through the same preflight
used by `prepare`.

`python_provision_requests_reach_the_real_control_parser` runs the actual
`provision.py` subprocess against the node's Unix-stream `handle_control` reader
and qualification parser. It exercises all six named profiles, including MP01's
knight/druid/paladin and core's default sorcerer/explorer. The session uses the
node's UUID-v7 generator and formatter, and the test checks the complete received
frame and parsed session/profile. It tests framing, not gameplay provisioning.
MP01 and core use the same formatter (`qualification.rs:544`), helper path,
`--socket` environment variable, `--session` and `--profile` arguments. The
36-character UUID and profiles produce 50–54 byte frames, below
`serve.rs:105`'s 64-byte control-reader ceiling. There is no source evidence for
a separate malformed MP encoding. A cause-specific request rewrite would be
speculation until a concrete failing frame or refusal class is observed.

**Cleanup-log caveat:** zero warnings in the final `node.log` does not establish
that the early returns fired. `run.sh:615–618` opens that log with truncation on
node start; `run.sh:1178–1179` explicitly truncates it for the later superseding
replacement. Those stages run after the seam (`run.sh:1230–1234`). For a seam
that exits 0 despite a FAIL row, `run.sh:884–887` prints only selected session
close events, excluding qualification warnings. The old node log is preserved
only on a nonzero stage exit (`run.sh:891–894`). This matches a `NODE_BOOT_PASS`
run with an MP01 FAIL row, and means the cleanup artifact can belong to the
replacement incarnation. Inspect or retain the serving node's log at the seam
stage, before replacement, to identify the actual refusal. This packet does not
change `run.sh` or assert an unavailable historical inner error.
