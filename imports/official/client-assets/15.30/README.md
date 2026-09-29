# Official client assets — 15.30 (Summer Update 2026)

`manifest.json` (schema `OTERYN_CLIENT_ASSET_MANIFEST/v1`) is a **checksum-only**
inventory of the repository owner's local official Tibia client `assets` folder for
target client version 15.30. It records, for every file in that folder: `name`,
`bytes` and `sha256`, plus a whole-archive `archive_sha256`, `file_count` and
`total_bytes`. It contains **no image data and no other proprietary file content** —
only cryptographic digests and plain file metadata.

The manifest itself remains checksum-only. As of 2026-09-29, the project owner confirmed redistribution rights for the currently present local asset set and directed its 6,248 files to be committed under `content/assets/files/`. Those 6,248 files match their manifest SHA-256 entries. The manifest-only 122,882,530-byte `minimap-32-0996-0984-02-dce27ae4b4d345201c9cc7f9d4f7576fc9144583779e8716b29032d6c1731073.bmp.zip` is absent from the current local source and is not committed.

## How it is produced

Every file directly inside the owner's/operator's local Tibia client `assets`
folder is hashed with SHA-256; `name`, `bytes` and `sha256` are recorded per file,
sorted by `name`. The whole manifest is also digested (`archive_sha256`) against the
exact byte-for-byte snapshot the owner packaged when generating it. Regenerating
this file from an unmodified copy of the same assets folder must reproduce it
byte-for-byte (`sort_keys`, compact separators, trailing newline).

## What consumes it

A future Content/World client-role asset compiler/loader reads the owner's or an
operator's local `assets` folder and must verify every file against this manifest
(matching `name`, `bytes` and `sha256`, and the aggregate `archive_sha256`) **before**
building any atlas or other derived client resource from it. A checksum mismatch
must fail closed rather than silently building from unverified or substituted
input. What the owner packages with the Oteryn client from that verified input is
outside this repository.

## Known gap versus the pinned server engines

Server-side Item data currently stays on Canary `47dfd51f` / Crystal `ff7ede5`
(`CLIENT_VERSION` 1525 = 15.25). The 15.30 `appearances.dat` in this manifest
(`appearances-2dfa943b….dat`) has 43,516 objects (max id 55117) versus Crystal
15.25's 42,108 (max id 54266): **1,409 new object ids (52977..55117) and 1 removed**,
none of which have engine server data yet. Filling those from TibiaWiki (BR/Fandom)
is separate future work; see
`docs/architecture/OTERYN_CLIENT_ASSET_VERSION_OWNER_DECISION_2026-09-27.md`.

## Appearance id membership manifest

`appearance-ids.json` (schema `OTERYN_CLIENT_APPEARANCE_MEMBERSHIP/v1`) in this folder
is allowed: it holds **object ids only** (plus the pinned file name, size, sha256 and
an `ids_sha256` of the id array), never names, flags or sprites. The owner emits it
locally, from the repo root in Git Bash; the tool fails closed on a size or sha256
mismatch:

```
python tools/content-schema/item-authoring/client_appearance_census.py \
  --appearances "<PATH_TO_CLIENT>/assets/appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat" \
  --membership-out imports/official/client-assets/15.30/appearance-ids.json
```
