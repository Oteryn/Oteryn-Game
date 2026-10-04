# Architect batch: serving the imported world content on a node

- Batch: `ARCH-WORLD-CONTENT-SERVE-1` (owner `1a`)
- Status: **ACCEPTED WHEN THIS DECISION MERGES** for the rulings, leases and packets below.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers:
  1. ARCH-QUEST-WIRING-PACKETS-1 §1.5 left serving imported chests undecided. The node serves
     one synthetic chest, so no imported chest can advance a quest on a running node.
  2. MAP-LOAD-PACKET-1 §1.1 and MAP-CUTOVER-1 left out where the World pin lives and which CI
     job builds the bundle.
  3. #1791 (owner PR, imports only) holds the CrystalServer `00ce02a5` spawn candidates. They are
     the source for spawn admission.
- Builds on:
  - ADR-0021 §4.2-§4.5 and §4.7;
  - MAP-LOAD-1 (#1783, the reader) and MAP-KIND-CLASS-1 (#1786), both on `main`;
  - SOCIAL-MAP packets §2.5-§2.8 (MAP-OVERLAY-1a to MAP-CUTOVER-1);
  - CHAR-POSITION-0 (the pinned `entry_start`);
  - CORE-LOOP packets §1.3-§1.4 (SPAWN-CONTENT-1, SPAWN-1a, SPAWN-1b);
  - ARCH-QUEST-WIRING-PACKETS-1 (#1789; QUEST-CAT-BOOT-1, CHEST-QUEST-BIND-1).
- Runtime, migration, deployment and production authority: NONE. Each packet needs its #162
  allocation. Running any World on a protected or production deployment is separate authority.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Leases and order

### 0.1 What is missing on `main`

The live path from the imported content to a quest step is cut in five places:

| Gap | Today | Closed by |
|---|---|---|
| Bundle artifact and World pin | `map::load` takes pins from its caller; nothing builds a bundle or holds a pin | WORLD-BUNDLE-CI-1 (§2.2) |
| Spawn source | Canary `47dfd51f` spawns, on a different source revision from the map | SPAWN-ADMIT-1 (§2.1) |
| Chest to map binding | A RewardClaim placement has a legacy `unique_id` and a project position, but nothing finds its map entry | CHEST-PLACE-BIND-1 (§2.3) |
| Node composition | `serve.rs` serves `with_entry_chest` over the native entry room only | WORLD-CONTENT-SERVE-1 (§2.4) |
| Quest binding | `ResolvedChest.quest_transition` is `None` | CHEST-QUEST-BIND-1 (#1789 §2.2) |

The map wire (MAP-WIRE-1, MAP-WIRE-2, MAP-CLIENT-1) is outside this batch (§1.8).

### 0.2 Shared files

| File | Owner | Others |
|---|---|---|
| `content/world/spawns/**` | SPAWN-ADMIT-1 | none |
| `.github/workflows/world-bundle.yml` | WORLD-BUNDLE-CI-1 (new) | none |
| `content/world/pins/**` | WORLD-BUNDLE-CI-1 (new), then SPAWN-ADMIT-1: the pin refresh only (§1.2) | WORLD-CONTENT-SERVE-1 reads it and does not write it |
| `apps/game-server/src/map/mod.rs` | CHEST-PLACE-BIND-1: the sparse `unique` table, with each entry's palette appearance id, and its accessor only (§2.3) | not at the same time as MAP-OVERLAY-1a or MAP-CUTOVER-1 if they touch it; the control plane serializes them |
| `apps/game-server/src/content/world_reward_claims.rs` | CHEST-PLACE-BIND-1 (new) | WORLD-CONTENT-SERVE-1 calls it and does not change it |
| `apps/game-server/src/content/mod.rs` | CHEST-PLACE-BIND-1, then WORLD-CONTENT-SERVE-1: each its `mod` line and re-export only | none at the same time |
| `apps/game-server/src/content/activation.rs`, `src/content/production.rs` | WORLD-CONTENT-SERVE-1: `activate_world_bundle`, `WorldBundleContentPin` and the World-artifact staging branch only (§1.5) | none at the same time |
| `apps/game-server/src/bin/oteryn-game-ops.rs` | WORLD-CONTENT-SERVE-1: the bundle-World mode of `content activate` only (§1.5) | none at the same time |
| `apps/game-server/src/node/serve.rs` | QUEST-CAT-BOOT-1 and CHEST-QUEST-BIND-1 first, then WORLD-CONTENT-SERVE-1 | none at the same time |
| `apps/game-server/src/interaction/chest_use.rs` | CHEST-QUEST-BIND-1 first, then WORLD-CONTENT-SERVE-1 | none at the same time |
| `apps/game-server/src/gameplay_transport/mod.rs` | ATTACK-1b, QUEST-CAT-BOOT-1 (#1789 §0.1), then WORLD-CONTENT-SERVE-1: the chest target lookup and the revision source (§1.6) | none at the same time |
| `apps/game-server/src/world_runtime.rs`, `src/map/boot.rs` | MAP-CUTOVER-1 (SOCIAL-MAP §2.8) | WORLD-CONTENT-SERVE-1: none; it consumes the booted base |

No packet here takes a migration lease (§1.7) or changes a protocol registry row.

### 0.3 Order

1. **WORLD-BUNDLE-CI-1** now. Its first non-production pin is built over the Canary spawn set
   on `main`. It is a CI and tooling pin, and no World serves it.
2. **SPAWN-ADMIT-1** after #1791 and WORLD-BUNDLE-CI-1 merge. Replacing the spawn family changes
   the bundle digest, so the same PR refreshes the imported World's pin from its own tree. The
   `world-bundle.yml` job must pass on that PR (§1.2).
3. **CHEST-PLACE-BIND-1** now. It needs MAP-LOAD-1 only.
4. **WORLD-CONTENT-SERVE-1** after MAP-CUTOVER-1, WORLD-BUNDLE-CI-1, SPAWN-ADMIT-1,
   CHEST-PLACE-BIND-1 and CHEST-QUEST-BIND-1 merge. The pin it serves is therefore the one
   SPAWN-ADMIT-1 refreshed over the admitted spawn set.
5. **SPAWN-1b** (CORE-LOOP §2.8) is packeted once SPAWN-ADMIT-1 and MAP-CUTOVER-1 have merged.
   It feeds the SPAWN-1a seam from the active bundle; this batch does not change its scope.

## 1. Rulings

### 1.1 One spawn source per World: the CrystalServer `00ce02a5` set from #1791

- The map's palette donor is CrystalServer `00ce02a5`. #1791 pins `world-monster.xml` from that
  same revision and converts it to 54,680 records and 87,815 points
  (`CANDIDATE_NOT_RUNTIME_ADMITTED`).
- SPAWN-ADMIT-1 **replaces** the Canary `47dfd51f` set in `content/world/spawns/` with that
  candidate set. The two sets are not merged: a merge would need a per-point source choice, and
  a Canary point can name a cell the CrystalServer map does not have.
- The canonical shards are regenerated with `tools/world-bundle-compiler/convert_spawns.py`
  from the pinned file in `imports/crystalserver/summer-update/`. They must equal the #1791
  candidate records, apart from the admission status and provenance fields. A difference is a
  `BLOCKER`, not something the worker reconciles.
- The 33 held groups (446 points) in `spawns/held-groups.json` stay out. Each one waits for the
  owning contract named in its reason, for example `SAME_CELL_SELECTION_REQUIRES_OWNING_CONTRACT`.
- CREATURE-AI-0 §6.1 still applies at compile time. If the set exceeds `RL-01`, `RL-02` or
  `RL-03`, or a delay falls outside `RL-13`, the worker stops with `BLOCKER`. Limits are not
  raised in this packet.
- #1791 is not edited. Its graphics links (`map-appearance-links.json`) are input for
  MAP-CLIENT-1 and are not admitted here.

### 1.2 The World pin lives in the repository and the bundle is a CI artifact

- A World's pin is one reviewed file, `content/world/pins/<world slug>.json`, with:
  - the `BundlePins` fields: `digest`, `project_format_version`, `world_schema_version`,
    `content_revision` and `production`;
  - `entry_start`, the native cell where CHAR-POSITION-0 places a first login until HOME-TOWN
    exists;
  - the World Project commit the bundle was built from.
- The workflow `world-bundle.yml` builds the bundle from `content/world/` with the compiler and
  uploads it as an artifact named by its digest. It runs on a change to any compiler input
  (`tools/world-bundle-compiler/src/main.rs` `project` and `registry`):
  - `content/world/**`: placements and their shards, terrain, objects, transitions, spawns,
    Worlds and the pins;
  - `content/houses/**`;
  - `content/creatures/definitions/**`;
  - `content/items/definitions/**`;
  - the compiler and its format crate: `tools/world-bundle-compiler/**`,
    `crates/world-bundle/**` and `Cargo.lock`.
- It fails when:
  - a rebuild from the same commit gives another digest (the compiler must be deterministic);
  - a pin names a digest that the pinned commit does not reproduce;
  - a production pin names a `non-production` build (ADR-0021 §4.2).
- **Pin refresh.** A PR that changes the imported World's digest refreshes its pin in the same
  PR. Any change to a path in the list above can do so. The job runs on every such PR and fails
  on a pin that its commit does not reproduce. SPAWN-ADMIT-1 is the first such PR (§0.3).
- A pin changes only by a reviewed PR. Activating a changed pin on a running World still needs
  the planned reset (ADR-0021 §4.7, MAP-OVERLAY-1c). A different digest at boot outside a reset
  refuses boot (MAP-CUTOVER-1).
- The node reads the bundle from a local path given by its operator. Fetching from an artifact
  store, retention and production storage are not decided here (§3, §4.5).

### 1.3 The imported content is served by its own World

- The fixture World keeps the native entry room, the entry chest and D116 unchanged.
- The imported map is served by a separate World with its own `WorldId` and channels. Its
  content revision is the `content_revision` of its pin, for example
  `oteryn:content/world-r1`.
- Characters belong to one World, and a quest track is pinned to the revision it was created
  on (#1789 §1.2). A fixture-World Character therefore never meets the new revision, and no
  revision migration is needed.
- The node selects its served content per World from the pin. A World without a pin serves
  the fixture content exactly as today.

### 1.4 A RewardClaim placement binds to exactly one bundle entry, by cell and unique id

For each placement of a served claim (§1.5), the binding takes:

- the cell: native `(x, y, floor = -z)` of `source_binding.project_position` (ADR-0021 §4.3,
  checked arithmetic);
- the id: the `unique_id` of its `crystalserver` entry in `legacy_unique_ids`.

The placement is **bound** when exactly one top-level base entry on that cell has the `unique`
attribute equal to the id, and the entry's palette appearance id equals `appearance_tibia_id`.
It then gets the entry's compiler `placement_key` and cell.

- **The palette appearance id** comes from the entry's palette key, not from an Item:
  - `<id>` of `oteryn:item.tibia.i<id>` or `oteryn:terrain.tibia.i<id>`;
  - `source_item_id` of a provisional `donor:crystalserver@<rev>:item/<id>` key, which is the
    client appearance id in the 15.x CrystalServer source;
  - none for any other key.
- `appearance_tibia_id` stays source evidence, and some chest appearances are not Items
  (`tools/content-schema/reward-claim-authoring/README.md`). The rule compares two appearance
  ids, so a chest whose appearance is not an Item still binds, once the bundle keeps its entry.
- **Provisional chest entries are not in the bundle yet.** The compiler leaves out every
  provisional entry: a non-production build skips it, and a production build is refused
  (`tools/world-bundle-compiler/src/compile.rs`, ADR-0021 §4.5). So a chest whose palette key
  is provisional has no entry in `WorldBase`.
  - Today these are the 2 ready claims on appearances 28827 and 28828, out of 219, with
    2 of the 234 placements.
  - They are **not ready to serve until admission.** On the non-production pin they are
    unbound with `NO_ENTRY`, left out and counted (§1.5). They are listed by claim key as
    expected `NO_ENTRY` in the CHEST-PLACE-BIND-1 task record. A production pin cannot exist
    while the bundle has provisional keys, so they never reach the production boot gate.
  - **Admission:** CHEST-APPEARANCE-ADMIT-1, a content-lane packet allocated by the control
    plane after this batch and outside its DAG. It admits the two donor keys as Oteryn
    definitions under ADR-0021 §4.5, so the compiler keeps their entries. The two claims then
    bind under this rule with no change here.

Otherwise it is **unbound**, with exactly one reason:

- `NO_CRYSTAL_UNIQUE_ID`: one of the 234 ready placements has only a Canary id today;
- `CELL_OUT_OF_BOUNDS`;
- `NO_ENTRY`;
- `AMBIGUOUS_ENTRY`: two or more entries match;
- `APPEARANCE_MISMATCH`: the entry's palette appearance id is absent or differs.

The rule is exact, so the binding is never inferred. A claim is bound only when all its
placements are bound.

### 1.5 Which claims a World serves, and the production gate

- **Served set.**
  - It contains the `plain` records with `readiness: ready` whose reward items and backpack
    resolve to admitted, materializable Item definitions.
  - Variants and claims that are not ready stay out (`WAITING_*`).
  - Of the served candidates, only bound claims (§1.4) are served.
- **Boot gate.**
  - The node computes the binding report at Content activation.
  - On a production pin, any unbound candidate refuses boot with
    `BootError::ContentActivation("reward claim binding")`.
  - On a non-production pin, unbound candidates are left out, and one event line counts them
    by reason.
  - This is the provisional-key gate of ADR-0021 §4.5, applied to chests.
- **Activation artifact.** A bundle World activates one stageable server artifact through the
  existing controller. Its digest is the `server_artifact_digest` that the node records in
  `game_content_activations` (migration 0008). No column is added.
  - **Server artifact.** `WorldActivationServerV1` is one canonical byte sequence, built by one
    function in `apps/game-server/src/content/world_activation.rs` (WORLD-CONTENT-SERVE-1). It
    is the domain tag `oteryn:world-activation/server/v1`, followed by:
    - the bundle digest and the pin's `content_revision`;
    - the pin's `entry_start`, as native `(x, y, floor)`;
    - each served claim, in canonical `PlacementKey` order, with its bound cell and bundle
      `placement_key` and its quest transition (CHEST-QUEST-BIND-1);
    - the canonical projection of every runtime definition the served path reads: the
      `ItemDefinitionFacts` (definition, stack class, container capacity, container-slot
      pattern) of each reward Item and backpack, ascending by definition;
    - the digest of the quest catalogue the World loads (QUEST-CAT-BOOT-1).
  - Changing any of these changes the bytes and so the digest. A semantic change to a referenced
    Item therefore gives a new activation, even when the claims and the bundle are unchanged.
  - `entry_start` is in the artifact because the node places first logins there (§2.4). A
    change to it alone is a new activation.
  - **Client artifact.** `WorldActivationClientV1` is `oteryn:world-activation/client/v1`
    followed by the bundle digest, because the client reads nothing beyond the bundle-bound
    view.
  - **Frame binding.** The digest is SHA-256 over `oteryn:world-activation/frame/v1` and the
    bundle's frame id (`global-target-2026-09-27`).
  - **Limits.** The server artifact is at most 1 MiB, checked with the existing
    `FirstProductionLimits` check (234 placements and their definitions are a few KiB).
  - **Staging.** `StagedGeneration::stage` recognizes the server artifact by its tag, as it
    recognizes the native source-world carrier. It:
    - checks both SHA-256 values against the expectation before decoding;
    - decodes the artifact and checks it canonically;
    - returns a `GenerationIdentity` with the pin's revisions.
  - **Activation.** `activate_world_bundle(controller, quiescence, world, issuance, inputs)`
    builds both artifacts from the loaded bundle and the served claims. It calls `stage_primary`,
    then `activate` with the boot quiescence guard, as `activate_native_entry_room` does. It
    returns a `WorldBundleContentPin`, which holds:
    - the identity and the activation sequence;
    - the frame binding and `entry_start`;
    - the served claims.
  - The pin is not `Clone`, and only this function produces it, as for
    `NativeEntryContentPin`. Nothing bypasses the controller's authorization.
  - **Issuer.** `oteryn-game-ops content activate` gets a bundle-World mode, `--world-pin <file>
    --bundle <path>`. It:
    - loads and verifies the bundle against the pin;
    - computes the binding report;
    - builds the two artifacts with the same function and issues their digests and the frame
      digest.
  - The issuer never issues the native room digests for a bundle World. The native mode is
    unchanged.
  - **Node.** At boot, `activate_world_bundle` stages the bytes it built against the scope's
    recorded issuance. A mismatch refuses boot with `BootError::ContentActivation("digest")`,
    as for the native room (`serve.rs` `activate_content`).

### 1.6 Identities: canonical in durable rows, digest-bound only in memory

- A served chest's content `PlacementKey` is canonical and stable across bundles:
  `oteryn:placement/<claim key without its "oteryn:" prefix>/<placement index>`. That key is the
  `source_placement` in the MINT intent and audit (`reward_claim_mint.rs`).
- The bundle `placement_key` and the cell come from the binding. They live only in memory,
  and the next activation rebuilds them (ADR-0021 §4.2: the bundle key is not a canonical
  identity).
- Claim uniqueness stays `(character, claim family, claim production key)` (migration 0012).
  The claim is per Character, so it is shared by every channel of the World. A Character who
  took a chest in one channel gets `NOTHING_TO_USE` in another.
- **Until MAP-WIRE-1 is accepted:**
  - the `USE` target is still the canonical `PlacementKey` bytes in `WorldObjectTarget.placement`
    (ITEM-USE-WIRE-1), exactly as for the entry chest;
  - reach is still the existing rule (same floor, Chebyshev distance 1), measured from the bound
    cell;
  - the content, ruleset and sim revisions of the chest `USE` come from the served World, not
    from the `entry_chest` constants.

### 1.7 Persistence and fencing

- **Tables.** No new table or migration. Claims, MINT reservations and receipts use the
  migration 0012 tables, and activations use the migration 0008 rows.
- **Fences.** A chest MINT stays fenced by the session's `CurrentCharacterItemFence` and the
  session generation (DUR-03). The binding adds no fence and writes nothing.
- **Volatile state.** A chest's map state stays in the overlay (ADR-0021 §4.4,
  MAP-OVERLAY-1a). It is never written to PostgreSQL.
- **Reset.** A World reset to a new bundle rebinds every claim. A claim already taken stays
  taken, because it is keyed by the claim and not by the placement.

### 1.8 Seeing the map is a separate decision

- The client cannot draw the imported map or address a bundle entry until MAP-WIRE-1 (a
  contract candidate) is accepted, and until MAP-WIRE-2 (`MAP_STATE_V1`) and MAP-CLIENT-1 are
  packeted.
- This batch makes the server side serve and pass on the live path, which a protocol client
  can drive. Whether the wire target becomes the bundle `placement_key` is MAP-WIRE-1's
  decision. If it does, a lookup from bundle key to canonical key is added there, through the
  binding.

## 2. Packets

### 2.1 SPAWN-ADMIT-1

```yaml
task_id: OTV2-20261004-spawn-admit-1
decision: ARCH-WORLD-CONTENT-SERVE-1 §1.1
depends_on: ["#1791 merged", OTV2-20261004-world-bundle-ci-1]
worker: oteryn-impl-worker
review: content review on the frozen head
branch: allocated by the control plane
base: main after #1791 and WORLD-BUNDLE-CI-1 merge
migration_lease: none
owned_paths:
  - content/world/spawns/**
  - tools/world-bundle-compiler/convert_spawns.py        # source path and provenance only
  - tools/world-bundle-compiler/tests/**                 # its converter tests
  - content/world/pins/<imported world slug>.json        # the pin refresh only (§1.2)
  - docs/agents/tasks/archive/OTV2-20261004-spawn-admit-1.md
validation:
  - the converter's tests, and regeneration with no diff
  - cargo test --locked -p oteryn-world-bundle-compiler
  - python3 tools/agents/validate_governance.py
  - git diff --check
```

- **Builds:** the canonical spawn family regenerated from the pinned CrystalServer file, with
  CrystalServer `00ce02a5` provenance, and the index.
- **Acceptance:**
  - the records equal the #1791 candidate records apart from the status and provenance fields;
  - the record and point counts are 54,680 and 87,815;
  - no point of a held group is present;
  - a non-production World bundle compiles with the new family within `RL-01` to `RL-03` and
    `RL-13`;
  - points whose cell cannot admit their creature are listed as compile diagnostics;
  - the frame matches the placements' frame `global-target-2026-09-27`;
  - regenerating gives no diff;
  - the imported World's pin is refreshed to the digest of this tree, and the `world-bundle.yml`
    job passes on the PR.
- **Not in scope:** the held groups, any runtime change (SPAWN-1b), a change to the
  `imports/` tree or to #1791.

### 2.2 WORLD-BUNDLE-CI-1

```yaml
task_id: OTV2-20261004-world-bundle-ci-1
decision: ARCH-WORLD-CONTENT-SERVE-1 §1.2-§1.3
depends_on: []
worker: oteryn-impl-worker
review: CI and provenance review on the frozen head
branch: allocated by the control plane
base: main
migration_lease: none
owned_paths:
  - .github/workflows/world-bundle.yml
  - content/world/pins/**                                # the schema and one non-production pin
  - tools/world-bundle-compiler/src/main.rs              # a --check-pin mode only, if needed
  - tools/world-bundle-compiler/tests/**
  - docs/agents/tasks/archive/OTV2-20261004-world-bundle-ci-1.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-world-bundle-compiler --all-targets -- -D warnings
  - cargo test --locked -p oteryn-world-bundle-compiler
  - python3 tools/repository/validate_repository_policy.py
  - python3 tools/agents/validate_governance.py
  - git diff --check
```

- **Builds:**
  - the workflow (§1.2);
  - the pin schema;
  - one non-production pin for the imported World, with its digest, revisions, `entry_start`
    and source commit;
  - the pin check.
- **Acceptance:**
  - two builds from one commit give one digest;
  - a pin with a wrong digest fails the job;
  - a production pin on a `non-production` build fails the job;
  - `entry_start` must be a walkable, non-blocking base cell inside the World bounds;
  - the artifact name carries the digest.
- **Not in scope:**
  - a production pin, production artifact storage and fetching by the node;
  - a change to the `game-gate` required set: the job is not added to it unless the control
    plane allocates that separately.

### 2.3 CHEST-PLACE-BIND-1

```yaml
task_id: OTV2-20261004-chest-place-bind-1
decision: ARCH-WORLD-CONTENT-SERVE-1 §1.4-§1.6
depends_on: [MAP-LOAD-1]
worker: oteryn-impl-worker
review: content and correctness review on the frozen head
branch: allocated by the control plane
base: main
migration_lease: none
owned_paths:
  - apps/game-server/src/content/world_reward_claims.rs  # new: reader of the reward-claim shards, served-set filter, binding report
  - apps/game-server/src/content/mod.rs                  # the mod line and re-export only
  - apps/game-server/src/map/mod.rs                      # the sparse top-level `unique` table, its accessor and unit tests only
  - apps/game-server/tests/world_reward_claims_*.rs
  - docs/agents/tasks/archive/OTV2-20261004-chest-place-bind-1.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server world_reward_claims
  - cargo run --locked -p oteryn-architecture-check
  - python3 tools/agents/validate_governance.py
  - git diff --check
```

- **Model change.** Today `WorldBase` keeps only the ids and depths of entries. The `Builder`
  drops the bundle's `unique` attribute when it pushes a tile (`map/mod.rs`). This packet keeps
  it:
  - a sparse table, ascending by entry index, holds the `unique` value of each top-level entry
    that has one, with that entry's palette appearance id (§1.4), derived from its palette key
    at load;
  - an accessor `TileView::unique(ordinal) -> Option<UniqueEntry>` reads it, where
    `UniqueEntry` is `{ unique: u16, appearance: Option<u16> }`;
  - nothing else in `map/` changes. The table costs memory only for entries with a unique id,
    and it is measured on the reference bundle and recorded in the task record.
- **Builds:** a pure function from a `WorldBase`, the reward-claim shards and the admitted Item
  registry to the served set and a binding report (§1.4, §1.5). It returns canonical
  `RewardClaimPlacement`s with their bound cells, and writes nothing. The shards are embedded
  as the quest document is.
- **Acceptance:**
  - over a small fixture bundle built by the compiler: each §1.4 reason with one vector;
  - a bound placement carries the entry's cell and `placement_key`;
  - the canonical key is stable when the fixture bundle is rebuilt with entries moved on other
    tiles;
  - a claim with one unbound placement is not served;
  - variants and non-ready claims are filtered out;
  - a reward item without an admitted definition makes its claim not served;
  - a loaded fixture bundle returns each top-level entry's `unique` exactly as the compiler wrote
    it, `None` for an entry without one, and the existing `map/` tests pass unchanged;
  - the palette appearance id is `<id>` for an `oteryn:item.tibia.i<id>` and an
    `oteryn:terrain.tibia.i<id>` key, `source_item_id` for a provisional donor key, and `None`
    for any other key;
  - a fixture chest whose appearance is not an Item, under an admitted Item or Terrain key,
    binds;
  - a fixture chest under a provisional donor key has no entry in a non-production fixture
    bundle, and is `NO_ENTRY`;
  - over the real non-production bundle, exactly the two claims on 28827 and 28828 are
    `NO_ENTRY` for that reason, listed in the task record.
- **Not in scope:** calling it from the node (§2.4), quest bindings (#1789), a compiler or
  format change, and any `map/` change beyond the `unique` table.

### 2.4 WORLD-CONTENT-SERVE-1

```yaml
task_id: OTV2-20261004-world-content-serve-1
decision: ARCH-WORLD-CONTENT-SERVE-1 §1.3, §1.5-§1.7
depends_on: [MAP-CUTOVER-1, OTV2-20261004-world-bundle-ci-1, OTV2-20261004-spawn-admit-1, OTV2-20261004-chest-place-bind-1, OTV2-20261004-chest-quest-bind-1]
worker: oteryn-hard-worker
review: hard and persistence review (Codex, final frozen head)
branch: allocated by the control plane
base: main after its dependencies merge
migration_lease: none
owned_paths:
  - apps/game-server/src/node/serve.rs                   # per-World composition, the boot gate, the event line
  - apps/game-server/src/interaction/chest_use.rs        # resolving served claims; revisions from the World
  - apps/game-server/src/gameplay_transport/mod.rs       # the chest target lookup, reach from the bound cell, revision source, the test mod line
  - apps/game-server/src/gameplay_transport/world_content_serve_tests.rs  # new
  - apps/game-server/src/content/world_activation.rs     # new: the bundle-World activation artifacts (§1.5)
  - apps/game-server/src/content/activation.rs           # activate_world_bundle and WorldBundleContentPin only
  - apps/game-server/src/content/production.rs           # the StagedGeneration::stage branch for the World activation artifact only
  - apps/game-server/src/content/mod.rs                  # the mod line and re-export only
  - apps/game-server/src/bin/oteryn-game-ops.rs          # the bundle-World mode of `content activate` only
  - docs/agents/tasks/archive/OTV2-20261004-world-content-serve-1.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server
  - cargo run --locked -p oteryn-architecture-check
  - python3 tools/agents/validate_governance.py
  - git diff --check
```

- **Builds:**
  - per-World content selection from the pin (§1.3);
  - the served claims composed over the booted base (§1.5);
  - the boot gate and its event line;
  - the activation artifacts over the bundle, the served claims, their Item definitions and the
    quest catalogue; their staging and activation through `ContentActivationController`; their
    issuer in `oteryn-game-ops content activate` (§1.5);
  - the quest catalogue loaded with the World's revision;
  - chest `USE` resolution and reach from the binding, with revisions taken from the World
    (§1.6).
- **Acceptance**, on the served path, through `gameplay_transport`, against PostgreSQL:
  - On the imported World's non-production bundle, the Beregar `firewalker_boots` chest:
    - records its quest transition as a pending obligation once;
    - the session refresh applies the transition to its track;
    - a second `USE` in another channel of the World is `NOTHING_TO_USE`.
  - A `USE` from distance 2 is refused, and nothing is written.
  - A stale session generation or item fence is refused, and nothing is written.
  - On a production pin, one unbound candidate refuses boot with `ContentActivation`. On a
    non-production pin it is left out and counted.
  - Changing the bundle digest, the served claim set, one reward Item's stack class, the quest
    catalogue or `entry_start` changes `server_artifact_digest`.
  - Issuer and node: an issuance made for a pin, followed by a change to only that pin's
    `entry_start`, refuses boot with `ContentActivation("digest")`. A fresh issuance for the
    changed pin boots.
  - A bundle World becomes active only through `stage_primary` and `activate`: activation
    without an issuance, or under a non-quiescent guard, is refused, and the controller's
    active generation has the World artifact's identity.
  - A World artifact with a changed byte, a non-canonical order or over 1 MiB fails staging.
  - `content activate --world-pin --bundle` issues digests that the node's boot check accepts.
    An issuance from the native mode, or from another bundle or claim set, refuses boot with
    `ContentActivation("digest")`. Replaying a request file reissues the same digests.
  - The fixture World still serves the entry chest, D116 and `oteryn:content/entry-r1`, and its
    existing tests pass unchanged.
  - A first login on the imported World is placed at its pinned `entry_start`.
- **Not in scope:**
  - spawns (SPAWN-1b), the map wire (§1.8), doors and other legacy-id bindings;
  - a production pin or deployment;
  - a revision migration between Worlds.

## 3. Rejected options

- **Serving imported chests in the fixture World.** It would move every fixture Character to
  a new content revision and break their quest tracks (#1789 §1.2). It would also mix D116 with
  the imported map.
- **Binding claims inside the compiler.** RewardClaim content is not part of the World Project,
  so the bundle format would have to grow. A boot-time binding over the loaded base needs no
  format change, and the production gate keeps it fail-closed.
- **Binding by cell alone, or by unique id alone.** A cell can hold several containers, and
  unique ids are not bound to cells in the Canary and CrystalServer sources. Requiring both, plus
  the appearance, is exact.
- **Requiring the chest appearance to resolve to an Item.** `appearance_tibia_id` is source
  evidence, and some chest appearances are not Items. The production gate would then refuse
  every pin. The rule compares palette appearance ids instead (§1.4).
- **Digesting only the bundle and the claims.** A semantic change to a reward Item would keep
  the digest, so two different behaviours would share one activation (§1.5).
- **The bundle `placement_key` as the durable source placement.** It changes with every
  bundle, so a claim audit would name a key that no longer exists (ADR-0021 §4.2).
- **Merging the Canary and CrystalServer spawn sets.** It needs a per-point source choice, and
  it breaks frame consistency with the CrystalServer map (§1.1).
- **Committing bundles.** That was rejected by MAP-LOAD-PACKET-1 §1.1. The repository keeps the
  digest pin and CI rebuilds the bundle.
- **A new activation column for the bundle digest.** `server_artifact_digest` already binds the
  served artifact, so a migration would add nothing a test cannot prove.

## 4. Decision test

`docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md`, mandatory decision test:

1. **Must decide now?**
   - **YES** for SPAWN-ADMIT-1, WORLD-BUNDLE-CI-1 and CHEST-PLACE-BIND-1. They are needed for any
     imported content on a node, and none of them waits on an unaccepted contract.
   - **YES** for the WORLD-CONTENT-SERVE-1 rulings, so that MAP-CUTOVER-1 and CHEST-QUEST-BIND-1
     land against a fixed target. The packet itself starts only after its dependencies merge.
2. **What concrete work is blocked?**
   - Any imported chest, quest step or spawn on a running node.
   - SPAWN-1b, which needs an admitted spawn source and a booted bundle.
   - Owner `1a` (#1789): a chest placed on the map advancing its quest.
3. **What becomes harder later?**
   - The pin file is a repository input. A production artifact store or a control-plane pin
     later needs a migration of that one file.
   - A separate imported World means its Characters do not carry over to or from the fixture
     World.
   - The exact binding rule leaves out any chest whose source data disagrees. Each one needs
     a content fix rather than a looser rule, and a production pin refuses boot until fixed.
   - Replacing the Canary spawns ties future spawn updates to CrystalServer imports.
4. **What would justify superseding it?**
   - MAP-WIRE-1 choosing another target identity, which changes only the §1.6 lookup.
   - An accepted control-plane World configuration that holds pins outside the repository.
   - A source correction that binds unique ids to cells, which allows a different exact rule.
5. **What is deliberately not decided?**
   - The map wire, the client and the wire target identity (§1.8).
   - Production pins, the artifact store and fetching by the node.
   - Running any World on a protected or production deployment.
   - The held spawn groups and their owning contracts.
   - Doors, levers and other legacy-id bindings.
   - Reward-claim variants, cooldowns and container rewards.
   - A Character move or a revision migration between Worlds.
6. **Risks and trade-offs.**
   - The boot gate couples production readiness to content quality. This is intended,
     and it matches ADR-0021 §4.5.
   - Boot-time binding costs one pass over the ready claims at activation. 234 placements
     is negligible against the bundle load, so no limit row is added.
   - WORLD-CONTENT-SERVE-1 waits on the MAP-OVERLAY-1a to 1c and MAP-CUTOVER-1 chain. The
     earlier slices are not on that chain and run now.
