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
