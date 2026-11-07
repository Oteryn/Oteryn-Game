# Admitted CipSoft appearance membership manifests (A12 §4.1)

Decision `A12-ITEM-IDENTITY-TIBIA-ID-V1` makes the canonical Item key `oteryn:item.tibia.i<id>`
for every id in the *admitted CipSoft id set*. This folder holds one digest-bound membership
manifest per admitted CipSoft `appearances.dat` and the index that combines them.

| Order | Label | `appearances.dat` sha256 | Pinned by |
|---|---|---|---|
| 1 | `crystal-ff7ede5` | `6adb790d…` | `zimbadev/crystalserver@ff7ede59` |
| 2 | `canary-47dfd51f` | `aa44a154…` | `opentibiabr/canary@47dfd51f` |
| 3 | `crystal-donor-00ce02a5` | `17a72b30…` | `zimbadev/crystalserver@00ce02a5` |
| 4 | `client-15.30` | `2dfa943b…` | client 15.30 (B3 pin; the file is in `content/assets/files/`) |

- `appearances-<sha256>.json` (`OTERYN_CIPSOFT_APPEARANCE_MEMBERSHIP_MANIFEST/v1`): every
  appearance object id of that exact file as `[id, identity_projection_sha256, record_sha256]`.
  - The identity projection is canonical JSON `{"class":"object","name":<CipSoft name or null>}`.
  - The record digest covers the object's exact protobuf bytes and only notes evolution. Sprite
    ids are repacked between client builds, so almost every record differs across files.
  - `entries_sha256` binds the entry array; the file's own digest is recorded in `admitted.json`.
- `admitted.json` (`OTERYN_CIPSOFT_ADMITTED_APPEARANCE_SET/v1`): the manifests in admission order
  with their digests, the union (43,517 ids), the current set (the newest file, 43,516 ids) and
  the retired ids (`[53161]`).

Only ids and digests are stored, never names, flags or sprites. The set only grows: a newly
admitted client adds a manifest as the last entry.

Regenerate (the three engine files come from a checkout of each pinned revision; the tool
verifies size and sha256):

```
cd tools/content-schema/item-authoring
python appearance_membership.py \
  --source crystal-ff7ede5=<crystal ff7ede59>/data/items/appearances.dat \
  --source canary-47dfd51f=<canary 47dfd51f>/data/items/appearances.dat \
  --source crystal-donor-00ce02a5=<crystal 00ce02a5>/data/items/appearances.dat
python appearance_membership.py --check   # regenerates 15.30, verifies all digests and the index
```

The alias table built on these manifests is `content/items/aliases.json`
(`tools/content-census/item_id_alias_table.py`).
