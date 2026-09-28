# Client Asset Version Owner Decision — 2026-09-27

- Status: Owner-accepted decision record
- Date: 2026-09-27
- Decision owner: Oteryn project owner
- Issue: #162, Jira `KAN-16`
- Applies to: target Tibia client graphics version and the checksum manifest of the
  owner's local official client `assets` folder
- Evidence: `imports/official/client-assets/15.30/manifest.json`
  (schema `OTERYN_CLIENT_ASSET_MANIFEST/v1`, 6,249 files, `archive_sha256`
  `e48e478dc9071b8ccbb16dfe729c9389be74cd7826ee9ec350d51cb0ff8c65d9`)
- Does not authorize: runtime, protocol, persistence, atlas-compiler or client
  implementation

## 1. Target client version

Oteryn's client graphics are pinned to Tibia client **15.30** (Summer Update 2026),
recorded as a checksum-only manifest of the owner's local official client `assets`
folder. Nothing proprietary is committed to this repository: no sprite, `.dat`,
`.spr`, `.bin` or other client resource file, only file names, byte counts and
SHA-256 digests. A future local compiler/loader (the Content/World client role)
reads the owner's/operator's own local assets folder and must verify it against
this manifest before building any atlas or other derived resource. What the owner
packages with the Oteryn client from that verified input is outside this
repository's authority and scope.

## 2. Server-side Item data stays pinned at 15.25

Superseded 2026-09-28: the game version is 15.30 for server-side data too, and a family may re-pin to a
15.30-capable Canary or Crystal revision on a branch without waiting for upstream `main`. See
`docs/agents/programs/OTERYN_GAME_VERSION_1530_AND_OTS_BRANCHES_DECISION_20260928.md`. The text below is kept
as recorded.

Server-side Item data remains on the currently pinned engines — Canary
`47dfd51f45280a59a1d3e50ba7edd573d7234446` and Crystal
`ff7ede593c69d4c658b382c97443e8155926924a`, both `CLIENT_VERSION` 1525 (client
15.25) — until upstream moves to 15.30. Upstream `main` for both engines is still
1525 as of this decision. This is a graphics-target pin only; it does not move,
supersede or re-pin the Item authoring/identity-binding engine sources accepted
elsewhere (`OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md`,
`OTERYN_G4_MULTI_SOURCE_IDENTITY_BINDING_DECISION.md`).

## 3. Measured 15.30/15.25 appearance gap

The 15.30 `appearances.dat` in the manifest
(`appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat`)
has 43,516 objects (max id 55117) versus Crystal 15.25's 42,108 (max id 54266):
**1,409 new object ids (52977..55117) and 1 removed id**. None of the 1,409 new ids
have engine server data on the pinned 15.25 engines. Filling them from TibiaWiki
(BR/Fandom, already-admitted sources) is separate, explicitly deferred future work.

## Non-claims

- This decision claims no intellectual-property or distribution rights over
  CipSoft/Tibia client assets.
- No proprietary asset file, sprite, appearance binary or derived atlas is
  distributed, committed or otherwise stored in this repository.
- No runtime, protocol, persistence, atlas-compiler, client-loader or production
  implementation is authorized or performed by this decision.
- No server-side Item, Presentation or identity-binding record is changed by this
  decision.

## Follow-ups

1. Build the Content/World client-role local asset compiler/loader that reads the
   owner's/operator's local `assets` folder, verifies it byte-for-byte against
   `imports/official/client-assets/15.30/manifest.json`, and fails closed on any
   digest/byte-count mismatch before compiling an atlas.
2. Populate the 1,409 new 15.30 object ids (52977..55117) from TibiaWiki (BR/Fandom)
   as a separate, explicitly scoped future task; no engine server data exists for
   them today.
3. Re-pin the server-side engines (Canary/Crystal) to a 15.30-capable upstream
   revision once upstream itself moves past `CLIENT_VERSION` 1525, and reconcile
   the appearance gap above against that new revision at that time.

See `imports/official/client-assets/15.30/README.md` for the manifest's exact
production method and consumer contract.
