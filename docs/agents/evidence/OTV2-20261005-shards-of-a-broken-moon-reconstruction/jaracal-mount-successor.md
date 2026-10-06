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


## Exact code surface and count deltas

Fresh protected-main readback confirms this is a population successor, not a schema amendment.

### Existing typed support

`ProjectV2Declaration::Mount` already carries:

```text
presentation
speed_bonus: Option<i32>
premium: Option<bool>
taming_item: Option<ProjectV2DefinitionRef>
acquisition_interactions: Vec<ProjectV2DefinitionRef>
```

and ProjectV2 validation already checks Mount Item/Interaction references.

No new Mount field or family is needed.

### Historical population must remain frozen

Current materializer:

```text
apps/game-server/examples/materialize_content_world_project_v2.rs
```

has `populate_mounts()` hard-bound to:

```text
OTERYN_G4_MOUNT_252_SELECTED/v1
selected.len() == 252
batch_id = g4-mount-252-tibiawiki-r1
```

and intentionally emits all 252 historical records with:

```text
speed_bonus = None
premium = None
taming_item = None
acquisition_interactions = []
```

Do not edit that packet/count/source policy to smuggle Jaracal into the historical snapshot.

### Successor composition

Add one successor composition step after `populate_mounts()`, conceptually:

```text
historical MountPopulation(252)
  + Jaracal successor(1)
  -> active Mount population(253)
```

Jaracal declaration:

```text
identity.key      = oteryn:content.mount.jaracal
revision          = definition-r1
speed_bonus       = 10
premium           = true
taming_item       = oteryn:item.tibia.i54638
presentation      = client mount 1962 through Presentation owner
acquisition route = use i54638 on oteryn:creature.jaracal
```

The acquisition interaction reference can be populated only once that Interaction definition is admitted. Until then the successor may carry the exact identity/typed static fields while runtime acquisition remains separately held, if the Mount owner permits partial admission.

### Exact source identity

Primary post-release identity witness already frozen in this packet:

```text
TibiaWiki mediawiki/page_id = 67553
revision = 440479
timestamp = 2026-07-14T02:57:52Z
```

Pinned donor corroboration:

```text
Crystal Summer 00ce02a5
mount id = 250
clientid = 1962
name = Jaracal
speed = 10
premium = yes
type = quest
```

Prefer the post-release TibiaWiki page-id namespace for the canonical source binding, consistent with the existing Mount population. Preserve Crystal's id 250/clientid 1962 as corroborating source/provenance rather than inventing a new canonical Mount namespace.

### Smallest expected code lease

```text
apps/game-server/examples/materialize_content_world_project_v2.rs
apps/game-server/tests/content_world_project_repository.rs
apps/game-server/tests/content_world_project_v2.rs            # only if focused typed-reference regression is needed
docs/agents/evidence/<Jaracal successor source packet>
docs/agents/tasks/{active,archive}/<allocated task>
```

Generated canonical Mount shard/index, source bindings, editor and provenance outputs follow the normal materializer.

No Mount schema file needs widening.

### Exact count deltas

Repository tests currently hard-code:

```text
Mount declarations        252
Mount source bindings     252
Mount editor entries      252
unique Mount page ids     252
```

After successor admission:

```text
Mount declarations        253
Mount source bindings     253
Mount editor entries      253
unique Mount page ids     253
```

The original `g4-mount-252-tibiawiki-r1` import itself remains exactly 252. The +1 belongs to a separately named successor import/source.

The tree validator's printed `mounts=252 mount_editors=252 mount_bindings=252` expectations must likewise become the composed active counts without rewriting the historical source count.

### Acceptance

- historical 252 packet digest and rows byte-identical;
- one new canonical key only: `oteryn:content.mount.jaracal`;
- no collision with existing 252 Mount keys/page ids;
- active Mount count exactly 253;
- speed +10 and premium=true typed;
- canonical taming Item reference is `oteryn:item.tibia.i54638`;
- no direct Shards s16 Mount grant;
- no invented tame RNG/failure semantics;
- generated tree/repository round-trip and source-binding/editor counts pass.
