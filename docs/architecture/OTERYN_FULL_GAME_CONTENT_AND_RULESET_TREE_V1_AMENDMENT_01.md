# Oteryn Full Game Content & Ruleset Tree v1 — Amendment 01: mergeable tree layout

- Decision: `CONTENT_TREE_MERGEABLE_LAYOUT_V1`
- Date: 2026-09-30
- Repository: Oteryn/Oteryn-Game
- Amends: `OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md` (successor tree physical layout only)
- Admission main: `f38d7f3a6e040d69ab4b8df343f1d5921fec3c23`
- Owner direction: requested in the owner session of 2026-09-30; feasibility in [#162 comment 5913960307](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5913960307) and [correction 5913981487](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5913981487)
- Document at authoring: **CANDIDATE**; independent review and protected integration govern acceptance.
- Task: `OTV2-20260930-mergeable-content-tree`
- Scope: layout, canonical serialization, lock shape and validation style of the successor tree `content/**` (not `content/world/**`). No generator, loader, runtime or content change is made by this document.

## 1. Problem

Two content PRs that change unrelated records cannot both reach `main` without a
base merge, a full restage and a new CI and review cycle. Measured on SW-1 (#1363)
and SW-2 (#1351):

| Evidence | Value |
| --- | --- |
| Files changed by SW-1 | 94, of which about 80 are generated |
| Files changed by both SW-1 and SW-2 | 58 |
| `abilities-02000-02499.json` in SW-1 | 11 records shifted in, 11 shifted out, 0 record contents changed |

Every one of these conflicts is caused by layout, not by content (**PROVEN** from the
commits above):

1. **Position-addressed shards.** Families are cut into fixed 500-record slices of an
   ordered list (`ITEM_SHARD_SIZE` in `world_project_v2_to_tree.py`). One inserted
   record shifts every later shard, and a changed count renames the last shard file
   (`abilities-05500-05889.json` → `abilities-05500-05901.json`).
2. **Whole-corpus values in committed files.** `content/content.lock.json` and
   `content/manifest.json` carry family counts and the git blob SHA of each legacy
   monolith; every family `index.json` carries `record_count` and the blob SHA of
   `content/world/definitions/reference.json`. Any content change rewrites all of them.
3. **Single-line files.** Each shard is one JSON line, so Git cannot merge two edits
   to different records of the same file.
4. **Hand-written pins.** Tests, the materializer and the tree validator pin whole-corpus
   hashes, byte lengths and counts (`TREE_SHA256`, `CREATURES`, `CREATURE_RECORDS`, ...),
   so every content PR also edits the same lines of `.rs` and `.py` files.

GitHub Merge Queue cannot regenerate files and server-side merges ignore custom
merge drivers, so the layout itself must make independent changes textually independent.

## 2. Options

| Option | Benefit | Cost / disposition |
| --- | --- | --- |
| A: serialize content PRs, or batch them through one integration branch | No code or contract change; works now | Authors wait for a train; one broken change delays the batch. **Kept as interim operation (section 5).** |
| B: one-record-per-line formatting only | Readable diffs | Leaves 2 and 4 intact: every PR still conflicts. **Rejected.** |
| C: stop committing generated content; build it in CI | Removes all generated diffs | Changes the accepted WorldProject v2 locators and the build; reviewers lose the data diff. **Deferred**, no demonstrated need beyond D. |
| D: key-addressed shards, line-per-record serialization, per-shard lock digests, derived totals and regenerate-and-compare validation | Unrelated changes merge textually; Merge Queue tests the combination; diffs show only changed records | Generator, validator, tests and later the loader change; needs this amendment. **Selected.** |

## 3. Decision (D)

### 3.1 Key-addressed shards

- A record's shard is a pure function of its canonical identity key:
  `bucket = first 2 hex digits of sha256(utf8(key))`, giving 256 fixed buckets per family.
- Shard path: `<family node>/<stem>-<bucket>.json`, for example
  `content/creatures/definitions/creatures-3f.json`. Bucket names never depend on counts
  or positions; an empty bucket has no file.
- A record changes shard only when its key changes.
- A family whose committed records are a fixed small catalogue (fewer than 256 records and
  no expected growth) may use one file; its index states so.

### 3.2 Canonical serialization

- Shards are JSON Lines: line 1 is the shard header object (`schema`, `family`, `bucket`),
  every further line is exactly one record, UTF-8, sorted keys, compact separators,
  records ordered by identity key, final newline.
- No commas between records and no enclosing array, so inserting or removing one record
  touches only its own line.
- The existing record shapes are unchanged; only the container changes.

### 3.3 Indexes, manifest and lock without whole-corpus values

- Family `index.json` names the schema, the family, the bucket function and the shard stem.
  It carries no record counts, no shard list and no legacy blob SHA, so it changes only
  when the family layout changes.
- `content/content.lock.json` lists one entry per shard, one entry per line, sorted by path:
  `{"path": ..., "sha256": ...}`. A change touches only the entries of the shards it
  changes.
- The **package digest** is derived, never committed: sha256 over the sorted
  `path` + `sha256` lines of the lock. Exports and consumers that need a package identity
  compute it; it is identical for identical content.
- Totals (family counts, authoring profile counts) are derived by the validator and
  reported by CI, never committed.
- The binding to the legacy monolith (`legacy_blobs`, `legacy_source.git_blob_sha`) is
  replaced by the regenerate-and-compare check of 3.4, which proves the tree was generated
  from the committed legacy corpus on the same head.

### 3.4 Validation by regeneration

- One required CI job runs the tree generator from the committed inputs on the PR head and
  requires zero diff against the committed tree (the pattern already used for
  `content/world` by `g4-canonical-worldproject-package-seed.yml`).
- Tests and validators assert structure, identity, reference integrity and the lock
  against the files; they do not pin whole-corpus hashes, byte lengths or totals in
  hand-written code. A total that matters to a reviewer is printed by CI as a PR summary.
- Integrity is not weakened: exact bytes are still pinned, by the lock per shard and by
  reproducible regeneration, instead of by a hand-edited constant.

### 3.5 Residual conflicts

A real conflict remains when two PRs change the same record, or insert two new records
whose keys sort next to each other in the same bucket. The second case is rare with
256 buckets and is resolved by regeneration.

## 4. Where the benefit arrives

The successor tree is not the runtime source yet (`runtime_source:
legacy_until_separately_qualified` in `content/project.json`), and every content PR still
restages the single-line `content/world/**` monolith and its pinned tests. Applying D to
the tree removes the tree's share of the conflicts (most of the 58 files) at once; full
independence of content PRs arrives when all consumers switch to the tree and the
monolith is retired, which is the accepted migration order of the tree contract, section 10.
D is applied before that switch so the tree does not need a second layout migration.

## 5. Interim operation: content integration train

Until section 4 completes, content restages are integrated in batches:

1. Authors work in parallel on their own task branches on sources and code (schemas,
   converters, stage tools, tests). Generated output on those branches is disposable.
2. One integrator, the active control plane on #162, merges the ready source changes onto
   one integration branch from current `main`, takes generated files from `main`, runs the
   restage once and freezes one candidate.
3. That candidate gets one CI run, one independent review and one Merge Queue entry.
   A failing contribution is removed from the batch and the batch is regenerated.
4. A single ready content change with no other content change pending goes alone.
5. The batch size limits of `AGENTS.md` ("Work in batches") apply.

## 6. Implementation slices (after acceptance)

1. Tree generator and validator: key buckets, JSON Lines shards, count-free indexes,
   per-shard lock, derived totals; regenerate once. No record content change.
2. CI: regenerate-and-compare job for the tree; remove tree pins from hand-written tests
   and replace them with structural assertions and the CI summary.
3. Loader: the tree reader used by the runtime switch reads the new layout and computes
   the package digest.

Each slice is its own PR with the checks selected by its paths.

## 7. Excluded

- No change to `content/world/**`, WorldProject v2 locators, their manifest or lock, or
  `g4-canonical-worldproject-package-seed.yml`.
- No runtime switch; that remains separately qualified.
- No change to record shapes, identities, provenance fields or the admission contracts.
- Not option C.

## 8. Acceptance

- Independent review of this candidate on its exact frozen head.
- After slice 1 and 2: two synthetic PRs adding records with different keys to the same
  family merge without textual conflict, and the merge group passes regenerate-and-compare.
- The derived package digest is identical across two regenerations of the same head.
