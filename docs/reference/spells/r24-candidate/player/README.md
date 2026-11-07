# Player spell import completion

The source catalog contains 246 definitions. `route-status.json` binds each
definition to its source provenance and existing Channel dispatcher, or names the
remaining executor/owner gap. It grants no gameplay admission or production
authority. Import completeness does not mean that every definition cast successfully.

| Static route classification | Records |
|---|---:|
| Existing route, current owner facts and scenario qualification required | 227 |
| Missing runtime composition | 10 |
| Aleta route exists, House interior owner unavailable | 4 |
| Unselected duplicate-incantation alternatives retained for evidence | 5 |

The ten missing compositions are Creature Illusion, Find Fiend, Divine
Empowerment, Death Echo, Divine Grenade, Spiritual Outburst, Flurry of Blows,
Shield Bash, Shield Slam and Sweeping Takedown. Their qualified planners are
not a replacement for an initial live cast dispatcher. Aleta's four routes
remain dependent on the real House owner. Premium-required definitions also
depend on the live entitlement composition; Node currently constructs no
Premium coordinator.

Five alternatives are retained and unselected: four `Summon ... Familiar`
records and Practise Magic Missile. The last shares an equivalent complete
body with Lightest Magic Missile; the Rust compiler selects the first identity.
No source identity was deleted or given a fabricated mechanic.

The importer now refuses census/bundle mismatches in name, carrier, key or
revision and rejects unknown readiness status values. Five altered-source
cases reproduce the missing guards against the preceding implementation and
are refused after the repair. Exact valid 246-record output remains unchanged.

The 2026-10-02 fresh public wiki observations for Divine Caldera, Energy Strike
and Light Healing were read through Remote Desktop's existing Chrome/CDP after
ordinary HTTP failed. All 25 compared header fields agree with the catalog.
Divine Caldera's current power 150 agrees with Canary and both fresh wikis;
Crystal's older power 160 stays attributed as an alternative, without combining
its formula with Canary's power. These targeted reads do not verify all 246
definitions, damage formulas or live mechanics. See
[`../research/source-access-and-findings.json`](../research/source-access-and-findings.json)
for source URLs, modes of access and uncertainties.

Reproduce the inventory from the repository root:

```sh
python tools/content-schema/spell-authoring/build_player_route_report.py \
  docs/reference/spells/r24-candidate/player/route-status.json
```

Run the full Python suite using the accepted pinned Canary/Crystal checkouts.
The six external Lua test fixtures are retrieved locally using the existing
manifest and exact byte/SHA-256/Git-blob verification; they are not new packaged
assets and must not be staged for publication.

```sh
python tools/content-schema/spell-authoring/retrieve_completion_fixtures.py \
  --manifest docs/reference/spells/r21-local-candidate/external-fixture-inputs.json \
  --checkout-root . --sources /workspace/spell-sources
cd tools/content-schema/spell-authoring
OTERYN_SPELL_SOURCE_ROOT=/workspace/spell-sources \
  python -m unittest discover -p 'test_*.py' -q
```

Validation: all 224 Python tests pass, no skips; deterministic report
regeneration and `git diff --check` pass. `validation-receipt.json` retains
the bounded evidence and log hashes. Existing physical qualification remains
in r23 and is not relabelled as a new r24 gameplay run.
