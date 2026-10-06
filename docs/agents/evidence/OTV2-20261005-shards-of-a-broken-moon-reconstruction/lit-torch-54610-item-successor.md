# Lit Torch (SU26) 54610 — A12 / ITEM-ADD successor admission

Status: **EVIDENCE + ALLOCATION-READY OWNER PACKET / NO ITEM MUTATION**

Quest use: Shards of a Broken Moon post-Magnolia prison escape.

## Resolved identity facts

`54610` is not an unknown donor identity.

The admitted current client-15.30 appearance manifest contains id `54610`:

```text
entries[43414] =
[54610,
 438e64d0f37ee71c3db866a31f20a75df01d7f1bc6dde3df2b9b957070f62c9f,
 2feacd4cca91233fc8ecc9c336b92f206898ba33df9d68b3e57a889dbcd1671c]
```

A12 therefore determines the canonical key:

```text
oteryn:item.tibia.i54610@definition-r1
```

Pinned Crystal Summer donor census independently has:

```text
source id 54610
name: lit torch
family_profile: light_source
outcome: resolved
```

Cross-source public Item-ID evidence distinguishes:

- Lit Torch (Quest) = 34017;
- Lit Torch (SU26) = 54610.

Therefore the historical B1b `PROBABLE_MATCH 54610 -> 34017` conclusion is superseded for current admission. Historical B1b evidence remains immutable.

## Why current content is missing it

Current main has no active `oteryn:item.tibia.i54610` record.

The reason is structural:

1. frozen donor epoch-2 imports only 404 donor rows admitted by its historical crosswalk;
2. 54610 is one of the historical `HELD_DONOR_IDS`;
3. ITEM-ADD-1's `appearance_only_items()` only admits the explicit current CipSoft IDs that were outside every source corpus;
4. its validator intentionally requires:
   - id in `APPEARANCE_ONLY_ITEM_IDS`;
   - id current in client-15.30;
   - id absent from active source-id allocation.

54610 is current CipSoft, but it is not semantically “appearance-only”: a pinned donor source row exists.

Therefore simply appending 54610 to `APPEARANCE_ONLY_ITEM_IDS` would make the implementation pass under a false evidence category.

## Rejected fixes

Do not:

- remove 54610 from the historical `HELD_DONOR_IDS` array;
- rewrite the historical B1b crosswalk in place;
- mint another opaque registry key;
- create an alias to i34017;
- pretend 54610 is appearance-only;
- hand-edit generated compact `content/items/**`.

## Smallest successor shape

Add a proof-backed **current CipSoft source-held addition** alongside ITEM-ADD-1.

Conceptually:

```text
PROOF_BACKED_CURRENT_ITEM_ADDITIONS = [54610]
```

For each row, require:

1. current membership in the newest admitted CipSoft appearance manifest;
2. exact canonical key by A12: `oteryn:item.tibia.i<id>`;
3. explicit successor evidence proving why the historical donor hold is resolved;
4. donor source row exists at the pinned donor revision;
5. no active canonical Item record already exists;
6. no accepted alias targets another Item;
7. source-id collision with the historical *active* source allocation is absent.

The initial record may be identity-only:

```text
family: Item
key: oteryn:item.tibia.i54610
revision: definition-r1
client_projection: ClientSafe
materializable: false
stack_class: Unknown
semantics: Unknown
```

Then the normal Item semantic/timed/admission owners can qualify materialization and Lit-Torch lifecycle separately.

For Shards execution, materialization/use remains required after identity admission; identity admission alone must not claim the prison interaction is runnable.

## Where to hook it

The correct composition point is the A12 key-switch successor path that currently accepts proof-backed current appearance Items.

Do **not** introduce a pre-A12 temporary key.

A bounded implementation can extend the current post-historical addition validation to distinguish two evidence categories:

```text
AppearanceOnlyCurrent
ResolvedHistoricalHoldCurrent
```

Both yield direct canonical A12 keys; their proof requirements differ.

## Suggested owned paths

Exact lease should be issued by coordinator/Item owner, but the smallest code surface is expected to be:

```text
apps/game-server/src/content/item_identity.rs
apps/game-server/examples/materialize_content_world_project_v2.rs
apps/game-server/tests/content_item_identity.rs
docs/agents/evidence/<new 54610 successor packet>.json
docs/agents/tasks/{active,archive}/<item successor task>.md
```

Generated Item tree/index/manifest/provenance outputs should only be included if the existing materializer deterministically changes them.

Historical evidence files stay read-only:

```text
docs/agents/evidence/OTV2-20260928-item-donor-identity-b1b-alias-crosswalk.json
docs/agents/tasks/archive/OTV2-20260928-item-donor-identity-b1b.md
```

## Tests

- 54610 is current in the pinned client-15.30 manifest;
- 54610 creates exactly one canonical `oteryn:item.tibia.i54610`;
- 34017 and 54610 remain distinct records;
- historical B1b bytes/digests are unchanged;
- a non-current ID is rejected;
- an ID without explicit resolved-hold evidence is rejected;
- a duplicate active canonical record is rejected;
- adding an arbitrary historical held donor is rejected;
- no opaque successor key is allocated;
- canonical tree regeneration is deterministic.

## Shards follow-up

After identity admission, the Item owner still must qualify:

- materializable=true;
- non-stackable physical Item shape;
- light/timed behavior consistent with the accepted Timed Item owner;
- grant from exact prison skeleton placement;
- use on the north wall;
- persistence/consumption behavior.

The quest runtime must consume the canonical `oteryn:item.tibia.i54610`, never raw donor id 54610 and never i34017.


## Fresh donor-binding correction

Current `main` readback of `imports/crystalserver/bindings/items.json` changes the exact implementation seam:

- donor source id `54610` has **no binding row**;
- this is consistent with the historical B1b `PROBABLE_MATCH` hold against 34017;
- therefore 54610 cannot enter the active Item tree merely by toggling materializability or by reusing an existing exact binding.

The correct successor sequence is:

```text
1. admit one exact Crystal source binding:
   source = zimbadev/crystalserver@00ce02a5
   namespace = ots/item_server_id
   external_id = 54610
   target = oteryn:item.tibia.i54610@definition-r1

2. add/admit the canonical Item record under that A12 key

3. separately qualify materializable/light-source/timed semantics required by Shards
```

Do not route 54610 through `apply_tibia_id_key_rule_with_appearance_items()`: that helper explicitly accepts only current CipSoft ids **absent from the source allocation**. 54610 is a donor Item identity and needs a donor-binding successor, not the appearance-only path.

This is distinct from Skewered Fish 54638, whose exact Crystal binding already exists on main.
