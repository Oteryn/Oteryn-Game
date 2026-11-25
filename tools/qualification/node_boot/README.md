# Node qualification with the pinned World and spell content

All reference shards use the native portal-rs producer pinned by
`update_platform_pin.py`, including basic, row batches, journey groups and spells.
They retain the full node-boot seam pipeline; only the separate `game_platform`
lane takes its identity/gateway-specific completion path. Bootstrap issuance uses
v4 with explicit Female sex, the requested qualification vocation, each bot's
name and the `newhaven-starter-r1` starter interpretation.
World runs seed all four bot accounts through the real producer CLI. To update
the producer pins, run `python3 tools/qualification/node_boot/update_platform_pin.py
/path/to/pinned/platform`; both creation v2 and bootstrap v4 fixtures are required.

The default `run.sh` first runs the native entry room's SEAM stages in a
disposable baseline pass. Then it runs the World qualification with
`content/spells.manifest.json` and three regions on every authored floor.
Regions `(126,125)` and `(126,126)` cover Thais and its outskirts, and
`(127,127)` retains the default start for ordinary home respawns.

For quest walk-in qualification that needs the other regions, set
`NODE_BOOT_WORLD_PROFILE=quests` and select the relevant scenarios with the
existing row or group selectors. The default `town` profile serves the ordinary
node-boot shards. The 20-region bundle has 29,367 compiled spawn points versus
4,063 in `town`, increasing the work in actor censuses shared by gameplay and
admission. The `quests` profile adds the following regions.

| Region | Coverage |
| --- | --- |
| `(125,121)` | Barbarian Test and Arena |
| `(126,124)` | Paradox Tower |
| `(126,127)` | Extends the Bounac, crypt and Ada boat coverage in `(126,126)` |
| `(127,123)` | Koshei assembly |
| `(127,125)` | Feaster entry |
| `(127,126)`, `(128,125)` | Dreamer's Challenge levers and carrot destination |
| `(128,122)` | Blood Brothers mirrors and boss landing |
| `(128,126)`, `(129,126)` | Drefia brain cells |
| `(128,128)` | The Many portal and room for Killing in the Name of |
| `(129,123)`, `(129,124)` | Edron quest chests with uniques 6028, 6032 and 6033 |
| `(129,126)`, `(129,127)`, `(130,126)` | Koshei coffin, boss, rewards and parts |
| `(132,125)` | Isle of Ada landing and death binding |
| `(133,123)`, `(133,124)`, `(128,126)` | Rotten Blood Blood Vestibule walk-in (landing `(34101,31679,-13)` is in `(133,123)`) |

Both profiles use the production compiler's `compile-regions` equivalence
proof and pass the generated qualification digest to the shipped `serve`
command's `[world_bundle]` loader. First entry starts at Thais temple
`(32369, 32238, -7)`. Each pass creates and cleans up its own preproduction
Platform, PostgreSQL and node topology.

W20a/W20b use an explicitly disposable `qualification-clock` build by default.
The World UTC clock and dispatch's monotonic owner clock share an atomic file
offset. W20a advances 75 game seconds per observed sample; W20b advances one
light-decay interval at a time after the cast, cooldown and relog assertions.
The original transition, color, rate, mana, decay and expiry assertions remain
in place and measure elapsed game time. Each accelerated row has a 120-second
wall-time budget, plus its existing cleanup budget (together under five minutes).
Other rows do not advance the offset. Ordinary builds omit the clock input;
even a qualification build requires the existing disposable fixture configuration.

To retain the original real-clock observations and build the ordinary binary:

```sh
OTERYN_NODE_BOOT_REAL_CLOCK=1 PLATFORM_SOURCE=/path/to/pinned/platform \
  bash tools/qualification/node_boot/run.sh
```

That mode retains the original 3,750/3,300-second row deadlines and is intended
for an on-demand extended run, outside the normal 60-minute job budget.

Before World admissions, the disposable administrator initializes epoch zero through
`game_world_reset_initialize`, binding the compiled bundle to the operator's issued
content activation. An exact replay must be idempotent. Runtime Cyclopedia and daily
boost preparation require this admitted epoch; schema installation does not create it.

The World client reads the join snapshot and drives real commands for town
navigation, browse field, daily boost, depot search, combat, loot, death and
recovery. Declare Journey for a row that tests acquiring its prerequisite, or
Seeded for a row that starts with prepared prerequisites. Seeded rows may
use the qualification-only admin fixture through the durable owners. Inventory
comes from starter grants, player commands or fenced grants, and raw game-table
writes are prohibited. Missing equipment or forge materials are reported as
explicit prerequisite skips. Store remains unavailable at the pinned Platform
revision. `world_boot_seconds` measures process start through readiness;
`world_loader` records the client's local production-loader duration. The focused
`node_boot_qualification_world_loader` library test loads the same bundle without
sudo and checks the temple protection zone and reachable depot.

Seeded rows can request `qualify <session> item-<id>-<count>` over the
root-only node control socket. The qualification-spawn build resolves the
served item definition and commits a fenced reward claim into the main backpack
for counts 1 through 20. An exact retry in the same admission replays its claim.
Rows re-admit to observe the backpack through the normal client snapshot.

C11 and C12 use dedicated World accounts. The fixture opens C11’s first
character through the historical unassigned bootstrap shape and awards level 8
through the progression owner. The town crop includes Oressa’s retained Dawnport
placement. The fixture uses the movement owner to stage the dedicated character
near Oressa, and the dialogue runs through player commands. The separate Knight buys promotion and saves his Wheel through the
normal commands. A root-only Premium preparation request submits a bounded
snapshot to Game’s normal Premium consumer for the dedicated Knight or Monk.
C12 receives item 50271 through the fenced backpack grant.

The f5 spell rows use dedicated Platform-created Sorcerers and a Knight. The
node fixture awards their starting XP through the progression owner. The rune
rows seed platinum, then walk to a served seller and buy from its live shop
window whenever an authored seller has a reachable placement in the served
bundle. The rune grant covers a crop with no reachable authored seller. A live
shop refusal still fails the row.

Challenge, the party-spell pair, and the Light caster use the fixture Premium
consumer, with separate accounts for the seeded casters. S08 prepares its own
players through the Open PvP node’s socket. P21 receives enough carried currency
for both charges and still proves the actual debits through the imbuement owner.

To qualify the preserved full spell manifest through the same topology:

```sh
PLATFORM_SOURCE=/path/to/pinned/platform \
NODE_BOOT_SPELLS=1 \
  bash tools/qualification/node_boot/run.sh
```

The Platform checkout must be the exact revision pinned in `run.sh`. The runner
needs the existing Docker, sudo, OpenSSL and Rust toolchain prerequisites. Supply
`NODE_BOOT_SPELL_MANIFEST=/absolute/path/to/manifest.json` to test another candidate.
The default is `content/spells.manifest.json`.

Spell mode first runs the existing full-manifest compile/decode/stage test. It
copies only the manifest and its declared, SHA-bound inputs into the disposable
node directory, preserving safe relative locators, and gives the
same manifest to the operator's content issuance and the service's activation.
It replaces the tiny-room SEAM stage with
`node_boot_spells_against_running_node`; assignment, admission and shutdown keep
the existing real paths. It records the selected manifest SHA and the actual live
cast results. This is not a claim that every catalog spell cast successfully:
normal vocation, level, resources, targeting and cooldown rules still apply.
Unsupported mechanics and approximate source fidelity remain separate from the
observed execution result.

Activation event waits use `activation_wait.sh`: the tiny fixture keeps 60 seconds;
world/spell full-manifest passes allow 180 seconds. On manifest `35de21c6`, the
real operator qualification took 31.917 seconds and node qualification/staging/
activation took 48.227 seconds idle. With six CPU load workers on a four-CPU
quota, node activation completed successfully in 122.451 seconds, beyond the
old 60-second wait. The bound covers that measured path; assignment's 120-second
wait, readiness's 60-second wait, and all grant/replay/stale-predecessor assertions
are unchanged. `content_full_manifest_boot` exercises those real content APIs,
checks the exact issued pins and sequence, and uses the runner's same wait profile.

## Running in Codex Cloud

From the Game checkout, run `tools/qualification/node_boot/cloud_run.sh core`
through `exec_command` with `sandbox_permissions=require_escalated`. Leave
`RUNNER_TEMP` unset. To verify the escalated environment, run these commands
with the same permissions.

| Command | Expected stdout |
| --- | --- |
| `id -u` | `1000` |
| `stat -c '%u %a' /` | `0 755` |
| `stat -c '%u %a' /tmp` | `0 1777` |

In the default sandbox, `/` and `/tmp` appear owned by UID 65534. The
unprivileged `run.sh` correctly blocks that ownership with
`unprivileged_work_ancestor_not_owned`, and `cloud_run.sh` reports
`CLOUD_RUN=BLOCKED reason=need_escalated_sandbox` during preflight.

Docker and the exact pinned Platform checkout are required. Set
`PLATFORM_SOURCE` to that checkout if it lives outside `_platform`.
Pass multiple groups as separate arguments, for example
`tools/qualification/node_boot/cloud_run.sh core f6`. The Cloud runner uses
the bundled environment, runs every group sequentially in the same checkout
and inherits `OTERYN_E2E_CREATURE_BATCH` for `creature_sweep`. It prints a
summary for each group and writes private logs under `/tmp/node-boot-cloud.*`
at the path reported on stderr. It exits nonzero if any group fails.

Use `tools/qualification/node_boot/cloud_run.sh c19a` to run the starter
backpack and relog journey by itself. The `f6` group still includes C19a.

Use `tools/qualification/node_boot/cloud_run.sh p16` to run the P16 blessing
services journey by itself. The `f6` group still includes P16.

## Parallel stacks, the single-row loop and diagnostics

Local runs can start several isolated stacks at once. Unset knobs keep the CI
values: `NODE_BOOT_SERVICE_USER` (`oteryn-node-boot[-suffix]`), `NODE_BOOT_BASE`
(under `/srv/` or `$TMPDIR`, checked before any sudo), `NODE_BOOT_PROJECT`
(Compose project and PostgreSQL container prefix), `NODE_BOOT_PG_PORT`,
`NODE_BOOT_PLATFORM_PORT`, `NODE_BOOT_GAME_PORT` and `NODE_BOOT_SKIP_BASIC=1`
(skip the basic fixture pass). `NODE_BOOT_E2E_ONLY`, `NODE_BOOT_NO_FAILFAST` and
`NODE_BOOT_NO_PROGRESS_S` are passed to the seam harness when set.

On a host without sudo, set `NODE_BOOT_UNPRIVILEGED=1` for a disposable
`qualification-unprivileged` run. The runner uses a private temporary deployment
directory and runs Game's operator and service with the caller's UID. Docker
and the pinned Platform checkout are still required. Operator/service isolation
checks are skipped and the output records `mode=unprivileged` so a successful
run cannot satisfy privileged qualification.

Set `RUNNER_TEMP` to a short path you own, such as `/workspace/nbtmp` (mode
0700). The directory and each ancestor up to `/` must be owned by you or root.
The runner checks ownership before building, including when it defaults to
`/tmp`.

Single-row loop (refused when `CI=true` or `GITHUB_ACTIONS` is set):

```sh
NODE_BOOT_KEEP_STACK=1 PLATFORM_SOURCE=... bash tools/qualification/node_boot/run.sh
NODE_BOOT_REUSE_STACK=1 NODE_BOOT_E2E_ONLY=f6 PLATFORM_SOURCE=... bash tools/qualification/node_boot/run.sh
NODE_BOOT_REUSE_STACK=teardown PLATFORM_SOURCE=... bash tools/qualification/node_boot/run.sh
```

The first run stops after the seam stage, keeps the stack and writes an
owner-only state file (`$TMPDIR/node-boot-reuse-<project>.env`, mode 0600, it
holds the disposable stack's passwords). The second reruns only the seam stage
against the same node and characters. Neither reports `NODE_BOOT_PASS`: they end
with `NODE_BOOT_PARTIAL` and exit 3. Teardown removes the stack and the state.

Every run starts PostgreSQL with lock-wait (200 ms), deadlock and 500 ms
slow-statement logging and `pg_stat_statements`. `pg_observe.py` samples blocked,
waiting and long transactions every 2 s and dumps the stage's top statements and
lock/slow log lines at each stage end; a failed stage prints a bounded excerpt.
All query text is normalized and redacted. On failure the files are copied to
`$NODE_BOOT_FAILURE_LOG_DIR/postgres-world<N>-spells<N>/`. `failure_summary.py`
prints the failed seam rows grouped by `root=` before the stage table (and to
`$GITHUB_STEP_SUMMARY`). `timeline.py --node-log node.log --pg-activity
.../activity.log --out trace.json` builds a Perfetto/chrome://tracing trace.
