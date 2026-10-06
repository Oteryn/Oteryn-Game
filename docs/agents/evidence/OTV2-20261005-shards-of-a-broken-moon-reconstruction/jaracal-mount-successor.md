# Jaracal Mount successor population

Status: EVIDENCE / ALLOCATION-READY / NO CANONICAL MUTATION

## Historical boundary

The existing `OTERYN_G4_MOUNT_252_SELECTED/v1` packet is immutable and `populate_mounts()` intentionally requires exactly 252 rows. Jaracal must be admitted as a successor population, not appended to that historical packet.

## Accepted source identity

Post-release TibiaWiki:

- title: `Jaracal (Montaria)`
- page id: `67553`
- revision: `440479`
- timestamp: `2026-07-14T02:57:52Z`
- captured raw UTF-8 bytes: `653`
- raw SHA-256: `a3a5ce1a344c4848829e8551a40f72483086a27b5e5b4d931cac7f038bb96311`
- identity namespace: existing `mediawiki/page_id`

The page qualifies Premium, +10 speed, Skewered Fish taming, and Six Steps Ahead on successful taming.

Independent Crystal Summer corroboration at `00ce02a5:data/XML/mounts.xml`:

- mount id 250
- clientid 1962
- name Jaracal
- speed 10
- premium yes
- type quest

## Successor declaration

Proposed canonical record:

```text
Mount oteryn:content.mount.jaracal@definition-r1
display_name = Jaracal
speed_bonus = 10
premium = true
taming_item = oteryn:item.tibia.i54638
presentation = pending Presentation-owner resolution of client look 1962
```

Source binding:

```text
source = oteryn:source.tibiawiki
namespace = mediawiki/page_id
external_id = 67553
disposition = EXACT
```

No new donor-Mount identity namespace is needed.

## Item correction

The current taming Item is `oteryn:item.tibia.i54638`. Historical `oteryn:item.registry.i00038475` is a retired alias to this A12 key.

The i54638 Item record already exists but remains `materializable=false`, so Item owner must promote it in place.

## Smallest population change

Compose:

```text
historical_mount_population_252
+ jaracal_successor_population_1
= 253 canonical Mount records
```

Keep the historical `selected.len() == 252` invariant unchanged. The successor packet independently verifies page id/revision/hash, key collision freedom, source-binding uniqueness, speed/premium and the canonical taming Item.

Likely owned paths:

- `apps/game-server/examples/materialize_content_world_project_v2.rs`
- `apps/game-server/tests/content_world_project_v2.rs`
- `apps/game-server/tests/content_world_project_repository.rs`
- new successor evidence/task files

Generated Mount tree outputs are touched only through deterministic materialization.

Runtime taming/account unlock remains a separate task. Success probability and item failure/break probability remain UNKNOWN and must not be invented.
