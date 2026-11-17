# SRX-3 attached creature-event branch reachability

Read-only comparison: `claude/happy-davinci-ijz3wy-srvpkg4` at
`5e60c197` and published quest branch at
`6c09e0365110db6683f938dd08d5d584744cbf37`. This does not establish the
state of unpublished owner-PC edits.

The quest branch contains exact Heart of Destruction death source in
`content/quests/heart-of-destruction-deaths.json` and
`content/quests/native-death-bindings/quest-batch3-c-heart_of_destruction.json`.
Both explicitly set `runtime_enabled: false`; their scope metadata says
`left_out`, names missing `quest_batch3_c::capture` and `::settle`, and says
no production loader or runtime consumer is registered. The first file
retains the 23483-to-23482 vortex/action mutations, but preservation is
not an execution caller. These files are not evidence of a duplicate
runtime implementation.

Reproduce the narrow published-branch caller search:

```sh
git grep -n -i -l -E 'AscendantBossesDeath|RathletonBossDeath|ForgottenKnowledgeBossDeath|14961|22761|23483' 6c09e0365110db6683f938dd08d5d584744cbf37 -- crates/gameplay/dispatch/src content/quests
```

The only matching files are quest definition shards, the inactive Heart
death source, and mission completion plans. There is no dispatch caller
hit. This result does not license adding synthetic Ground or Teleport
policies: the static object mutations still need an actual current-map
owner, source destination preservation, and a focused production test.

The separate familiar reachability evidence is recorded in
`srx3-monster-callback-reachability.json`. C13 lifecycle completion does
not prove offense. The 27 offensive slots (25 familiar and two Skeleton)
remain W01 runtime work unless an actual caller is implemented or a
concrete duplicate is demonstrated.

## Direct registration calls in captured W01 event bodies

The 27 `source_events` rows in `crates/gameplay/content/src/content/native_monster_callback_witness.json` were checked against their pinned raw SHA256 values. None contains a direct `:registerEvent(...)` or `:unregisterEvent(...)` call. This does not cover shared libraries: for example, the source BossLever library explicitly registers `BossLeverOnDeath` after a successful birth. That entry-owner behavior must be audited separately and is not inferred from the static monster event arrays.

The check can be reproduced against a local read-only donor Git object database:

```python
import hashlib, json, re, subprocess, sys
from pathlib import Path
witness = json.loads(Path("crates/gameplay/content/src/content/native_monster_callback_witness.json").read_text())
checked = 0
calls = []
for event in witness["source_events"]:
    raw = subprocess.check_output(["git", "-C", sys.argv[1], "show", event["revision"] + ":" + event["path"]])
    assert hashlib.sha256(raw).hexdigest() == event["sha256"], event["path"]
    checked += 1
    for line, text in enumerate(raw.decode().splitlines(), 1):
        if re.search(r":(?:un)?registerEvent\s*\(", text):
            calls.append((event["name"], event["path"], line, text.strip()))
print({"checked": checked, "direct_registration_calls": calls})
```

Expected result for the current bounded capture: `checked=27`, `direct_registration_calls=[]`. This is source evidence, not a claim that all captured callbacks have production consumers.

### Heart action and movement boundaries

The source `HeartBossDeath` Outburst branch does not inspect killer eligibility:
it transforms the **current** `23483` at `(32225,31285,14)` to `23482`, sets
action `14325`, and updates storage `14332` for each current Player in the
inclusive source rectangle. An empty contribution map therefore does not make
this branch a no-op. The corresponding lever prefix conditionally transforms
current `23482` to `23483` with action `14350`; it must preserve the source
nil/no-op when that current item is absent.

`BundleFacts::transform_movement_item_with_action` is the shared map commit
boundary for semantic state and action. Its primitive regression uses existing
published flame states to test revision, collision, action, and unchanged bound
attributes. That regression is not evidence that the actual `23483/23482`
teleport pair or a Heart STEP handler has executed.

A reproducible caller search is:

```sh
rg -n '14325|14350|23482|23483' crates/gameplay/dispatch/src/gameplay_transport
rg -n '\.action_at\(' crates/gameplay/dispatch/src
```

At this checkpoint the first search has no explicit Heart action consumer; the
second reaches generic object-use and Dawnport readers. This search does not
prove that a default Teleport path is absent: actual initial `23483` retains
destination `(32208,31372,14)` and has no action attribute. That existing
transport path must be traced before adding a custom STEP handler. The inactive questpc
slice is not a duplicate production STEP consumer. Real Heart death,
current-item selection, Player storage, and subsequent STEP execution must be
verified together before this source branch can be marked complete.

## Snapshot 41 execution inventory

The main callback witness contains 36 native slots: 16 spawn slots, nine cast
slots, three threshold slots (Lisa, Tyrn, Professor Maxxen), and eight Seven
slots (Brain, Vortex, Maxxen/Zamulosh teleport, two Time Guardian slots, Smelly,
Ragiaz). These are source classification counts, not completion counts.

At this checkpoint `NativeCallbacks::consume_source_death` validates the
canonical BEGIN seal and copied event but returns `false` for W01 handlers.
Thus the shared FIFO and ordered descriptors do not establish Gaz/Corrupted
Ground creation, Heart portal/storage changes, ChargingOutDeath, or the other
copied transformation/progression effects. The genuine ground targets are
`srx3_gaz_minion_actual_player_lethal_creates_nightmare_vortex_postgres` and
`srx3_corrupted_soul_actual_player_lethal_first_vortex_has_no_expiry_postgres`;
a failure before the copied-handler boundary is not an effect regression.

Other concrete remaining callers include Ragiaz's native transform leaf,
Mazoran's consequences beyond delayed Rage creation, Outburst's source pad
prefix effects and accepted-entry initialization, and the attached HP-event
families. The latter must use actual Combat/OriginSPELL boundaries; direct Lua
addHealth/OriginNONE does not authorize invoking onHealthChange. Familiar27
remains W01 offense work independently of C13 lifecycle status.

The older 22-name audit predates the context-parent extension. The generated
`native_w01_death_events_witness.json` records **Possessed Tree** attaching
`PossessedTree` at literal parent line 56, registration index 0, with the exact
onDeath body hash. It is therefore an attached additional handler, unlike the
three declared-but-unattached review families HirintrorHeal, WhitePaleHeal,
and OmrafirExplode. Those three remain inactive in production unless a separate
accepted attachment is supplied. An explicit review-engine fixture does not
constitute production attachment.

Registered HP0 source actors remain distinct from completed corpse-pinned
slots. The actual native Foam regression proved that an older retained stage
was lost after canonical lethal damage, and the actual top-stack regression
proved that a lower Player was incorrectly exposed while the source monster
was still registered. Their narrow fixes require fresh execution verification;
these regressions do not authorize relaxing ordinary new-cast or movement
liveness checks. Equal cached-loop timestamps also require scheduling order,
not an actor-ID tie-break or an invented timer clock.

The current 71-parent ordered witness has 13 distinct **onDeath** handler names:
AscendantBossesDeath (six parents), BoneCapsule (one), ChargedAnomalyDeath
(one), ChargingOutDeath (one), CorruptedSoul (one), DisgustingOozeDeath (one),
ForgottenKnowledgeBossDeath (six), HeartBossDeath (seven), HeartMinionDeath
(eight), MinionGazDeath (one), PossessedTree (one), RathletonBossDeath (one),
and ThornKnightDeath (two). The broader attached-name census also includes
other callback kinds. Reproduce this narrower count by grouping each generated
parent's `on_death` entries by `name`; do not count all attached callback kinds
as death handlers.

The actual post-death STEP is source-scripted, not merely the retained default
Teleport attribute. Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
`data-global/scripts/quests/heart_of_destruction/movements_teleport_heart.lua`
(SHA256 `059f7b6a9b503d33c66c95bbb999695f25473657aa5d6f63b0e38f9997ce132d`),
normal-vortex table maps action `14325` to `(32216,31380,14)` and action
`14350` to `(32208,31372,14)`. Its `normalVortex` branch teleports the current
Player to the selected destination. The registration loop explicitly registers
the normal-vortex table's action IDs. Therefore the initial `23483` default
destination cannot substitute for action `14325` after HeartBossDeath. A private
source room also needs a real scoped exit/return owner for the external source
destination; the sparse retained attribute does not create walkable geometry or
an authority grant. This movement script is not yet among the current main
witness's 27 captured event bodies; a source closure addition must use the
controlled producer/data window after the actual target is established.
