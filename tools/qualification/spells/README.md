# Shared spell testing entry point

Use the existing Oteryn test server and its **Thalom spell test map**. The server is
expected to remain test/preproduction for an extended period. Approximate source
parity and known balance gaps are allowed in test content; record them separately
from whether a spell actually executes. A runtime failure is not a successful cast.

This entry point reuses `oteryn-dev-client`, the live synthetic harness, native
spell engine fixtures, and the existing room/topology qualification. It introduces
no second server, client protocol, world issuer, database or Docker topology.

## Fast iteration

```sh
bash tools/qualification/spells/run.sh runtime
bash tools/qualification/spells/run.sh runtime spell::combat_execution::tests
bash tools/qualification/spells/run.sh client
```

Cargo reuses its build cache. Focus the runtime filter on the changed family during
iteration. Cargo filters can match zero tests: check the reported executed count;
zero tests are not qualification. The default 16 MiB test stack is only for the existing composed-content
tests; it does not change the server's production stack. Ignored tests remain
ignored in this fast tier and do not establish database/full-system qualification.

## Existing test server

```sh
bash tools/qualification/spells/run.sh live \
  --addr 127.0.0.1:7171 --server-name localhost --ca /path/to/server-ca.pem \
  --character-id CHARACTER_UUID --grant-file /path/to/admission-grant \
  --script /path/to/scenario.txt
```

The address, CA, admitted character and grant must come from your existing test
server setup. Without `--script`, the same client accepts interactive terminal
commands. Grants remain in files/environment rather than command-line literals.
The runner does not issue accounts, grant Premium, change live stats or bypass
normal admission. Spell indices are the **1-based spell book indices of the loaded
content generation**, not upstream numeric spell IDs. Verify the selected spell
book before a batch; a correct request for the wrong book is not useful evidence.

For example, after identifying a suitable self-cast spell's current book index,
a scenario may contain:

```text
# Replace 7 with the index from your loaded spell book.
cast 7 self
expect Cast
wait 2000
cast 7 self
expect Cast
quit
```

`expect` accepts all ten existing dispositions (including `CoolingDown` and
`NotEnoughMana`); a mismatch or missing prior cast terminates with a nonzero exit.
Scripts are validated before connecting, limited to 1 MiB / 4096 lines and five
minutes of total waits; each `wait` is at most 30 seconds and services liveness.
A completion summary counts responses and passed expectations. A series without
expectations does not qualify successful gameplay.

Movement/use/click commands remain available. Casts use the existing wire targets:
`none` (also accepted as `self` shorthand), `attack` (the server's current attack
target), or `position X Y FLOOR`. `self` is not a new targeting rule: the server
still decides which spells may be cast with no explicit target. The client does
not invent Character/Creature-target commands absent from the current protocol.

A series is a text script using the same commands; add explicit waits for cooldowns
and expectations after casts. Actual result dispositions and observed own-actor
vitals are printed. They do not prove target damage, all visual effects, or boss
phases. Use existing engine tests for those effects and full server journeys for
integration. No automatic retry turns a refused/failed attempt into a pass.

## Existing Thalom map and full spell input

The existing `tools/content-schema/native-gameplay/canary-thalom-world.json`
contains 2,833 tiles imported from the pinned Canary Thalom rectangle, with Item
policies and native bindings. The preserved `r21-local-candidate/active-artifact/`
manifest pins the exact map SHA. Reuse it rather than create a new arena.

```sh
bash tools/qualification/spells/run.sh map
# Default reads the committed content/test-packs/spells/r25/manifest.json package.
# Or an explicitly produced candidate manifest:
bash tools/qualification/spells/run.sh map /path/to/manifest.json
```

This runs the existing full-manifest qualification, including compilation, strict
decode and staging of server/client artifacts. It explicitly executes the normally
ignored test; it does not activate content on a live instance. Optional
`OTERYN_FULL_SPELL_TEST_OUTPUT` saves newly qualified artifacts and proof in a chosen
output directory; keep that output outside the preserved historical candidate.

The existing node selects gameplay through `OTERYN_NATIVE_GAMEPLAY_MANIFEST`.
Starting a node requires the matching content issuance and the normal existing
assignment/admission configuration; setting that variable alone does not grant
activation. This map includes negative native floors. Their compiled presence is
not a claim that every movement/visibility consumer supports those floors.

## Full spell server qualification

```sh
PLATFORM_SOURCE=/path/to/pinned/platform \
  bash tools/qualification/spells/run.sh server
# Or select an explicit candidate:
bash tools/qualification/spells/run.sh server /path/to/manifest.json
```

This delegates to the existing S3-B runner with `WP5_QUALIFICATION=spell-seam`.
It uses the real Game owners, TCP/TLS and Platform admission with the Thalom/full
spell manifest. With no explicit manifest, it reads the committed
`content/test-packs/spells/r25/manifest.json` package. It creates disposable qualification services and cleans them up;
it does not deploy to or change a long-lived test server. Default room/SEAM runs
remain available. A spell's observed rejection is recorded as a rejection, not a
successful execution or complete catalog qualification.

Prerequisites are the exact Platform checkout pinned by `wp5_s3b/run.sh`, Docker
with Compose/BuildKit, OpenSSL, Rust 1.94.0 and an isolated PostgreSQL 17.6 service
selected through `OTERYN_TEST_POSTGRES_ADMIN_URL`. Keep connection secrets in the
existing private environment/profile, not a committed scenario. The runner
requires disposable database administration and does not require host `sudo`.
Optional build CA and exact image mirror settings are documented in
[the S3-B runner README](../wp5_s3b/README.md).

## Existing room qualification

```sh
bash tools/qualification/spells/run.sh room
```

This delegates to `native_entry_room/run.sh`, which needs its exact pinned Platform
checkout and Docker. It qualifies the room input with a Platform-issued world;
it does not independently qualify every spell or start a permanent test server.

## Next content batches

Reuse these entry points while completing the common mechanics first: direct
combat/healing, conditions/fields, summons, then monster scheduling and custom
encounters. Keep source revision, chosen approximation, execution status and known
gaps per entry. Enable partial test content only when the implemented path works;
unsupported custom behavior stays explicitly disabled until implemented.

## Explicit playable test baseline

The default numerical policy remains `strict`. To qualify a separate test content
generation with documented neutral values for missing optional damage modifiers:

```sh
python tools/qualification/spells/prepare_test_manifest.py \
  docs/reference/spells/r21-local-candidate/active-artifact/manifest.json \
  /tmp/oteryn-spell-test-candidate \
  --magnitude-policy baseline_test \
  --training-revision s3b-content-1 \
  --creature-profiles /path/to/qualified-creature-profiles.json
bash tools/qualification/spells/run.sh server /tmp/oteryn-spell-test-candidate/manifest.json
```

The training revision must match the actual Character build owner in the selected
qualification environment; `s3b-content-1` belongs to the existing S3-B bootstrap.
Use the serving owner's revision for a different environment. The output directory
must not exist. The generator verifies the source pins, copies the declared inputs,
and creates a new manifest. This operation neither activates a node nor grants
Premium, items, targets, houses or quest access.

`baseline_test` preserves unknown owner observations and lists omitted numerical
modifiers. It keeps session, equipment, target, health and content fences. Current
Wheel level and promotion eligibility still apply. The expanded S3-B scenario
uses ordinary progression, movement, casts and durable training writes; casts on
an empty footprint do not prove damage to a creature. Monster melee profiles
explicitly declare fixed physical bite approximation and a nonlethal player floor;
ranged attacks, defenses, spawning and encounter execution need separate owners.

## Complete reusable scenario matrix

`docs/reference/spells/r24-candidate/scenarios/player-spell-scenarios.json` provides
positive fixture requirements and isolated refusal cases for all 246 definitions,
including source-selected aliases, costs, professions, target rules and dependency
references. It is a qualification plan, not 246 gameplay passes. Regenerate from
pinned inputs using `build_scenarios.py`; see the scenario README for the exact
command. Reuse this existing Thalom/S3-B entry point when executing the plans.

Run parser/provisioning regressions with:

```sh
python -m unittest discover -s tools/qualification/spells -p test_qualification_tools.py
```

The test manifest helper refuses missing mandatory v5 providers, malformed or
escaping paths, invalid digests/revisions and unknown fields before creating an
output directory. Reads remain bounded; output creation remains exclusive.
