# Oteryn Full Game Content & Ruleset Tree v1 — Amendment 01: mergeable tree layout

- Decision: `CONTENT_TREE_MERGEABLE_LAYOUT_V1`
- Date: 2026-09-30
- Repository: Oteryn/Oteryn-Game
- Amends: `OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md` (successor tree physical layout only)
- Admission main: `f38d7f3a6e040d69ab4b8df343f1d5921fec3c23`
- Owner direction: requested in the owner session of 2026-09-30; feasibility in [#162 comment 5913960307](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5913960307) and [correction 5913981487](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5913981487)
- Document at authoring: **CANDIDATE**; independent review and protected integration govern acceptance. The amended base contract is itself still `Status: CANDIDATE` on `main`; this amendment is accepted together with it or after it, never ahead of it.
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
| D: key-addressed shards, line-per-record serialization, no committed digests, counts or revisions, and regenerate-and-compare validation in the Merge Queue | Unrelated changes merge textually; Merge Queue tests the combination; diffs show only changed records | Generator, validator, tests and later the loader change; needs this amendment. **Selected.** |

## 3. Decision (D)

### 3.1 Key-addressed shards

- Every family index names the record field that holds the record's **identity key**. For
  every family migrated today this is `definition.identity.key`, the key the generator
  already uses through `target_id(definition["identity"])` in `world_project_v2_to_tree.py`.
  A family whose records carry no single identity key cannot use this layout until its
  index names one.
- A record's shard is a pure function of that key:
  `bucket = first 2 hex digits of sha256(utf8(identity key))`, giving 256 fixed buckets per family.
- Shard path: `<family node>/<stem>-<bucket>.json`, for example
  `content/creatures/definitions/creatures-3f.json`. Bucket names never depend on counts
  or positions; an empty bucket has no file.
- A record changes shard only when its key changes.
- A family whose committed records are a fixed small catalogue (fewer than 256 records and
  no expected growth) may use one file; its index states so.

### 3.2 Canonical serialization

- Shards are JSON Lines: line 1 is the shard header object (`schema`, `family`, `bucket`),
  every further line is exactly one record, UTF-8, sorted keys, compact separators,
  final newline.
- Records are ordered by the UTF-8 bytes of their identity key. This replaces the current
  order of the legacy record list; `canonical_bytes` stays the byte form of each line.
- No commas between records and no enclosing array, so inserting, changing or removing one
  record touches only its own line.
- The existing record shapes are unchanged; only the container changes.

### 3.3 No whole-corpus or per-commit values in committed files

A committed file may hold only values that change when that file's own records or the
family layout change. Concretely:

- **Family `index.json`**: schema, family, identity-key field, bucket function and shard
  stem. No record counts, no shard list, no legacy blob SHA.
- **`content/manifest.json`**: schema, the family → index map and the `compatibility`
  block. `admission_main`, `project_revision`, per-family `records` counts and
  `managed_files` are removed.
- **`content/project.json`**: schema, locators, `migrated_families`,
  `next_population_families` and `runtime_source`. `project_revision` is removed.
- **`content/content.lock.json`**: schema, digest algorithm and the canonical form
  identifier only. It lists no digests, counts, `admission_main`, `project_revision` or
  `legacy_blobs`. A committed per-shard digest list is rejected: two PRs that change
  different records of one bucket both rewrite that shard's line, edits to the lines of
  adjacent shards conflict, and a clean textual merge would leave a stale digest.
- **Derived, never committed**: the per-shard digests, the package digest (sha256 over the
  sorted `path` + shard-sha256 lines), the project revision (named by the package digest),
  and all totals. The generator and validator compute them; exports and consumers that need
  a package identity compute the same value from the same bytes. CI prints totals and the
  package digest as a PR summary.
- The binding to the legacy monolith (`legacy_blobs`, `legacy_source.git_blob_sha`) is
  replaced by the regenerate-and-compare check of 3.4, which proves on every Merge Queue
  candidate that the tree is exactly what the committed legacy corpus and tools generate.

### 3.4 Validation by regeneration

- One required check runs the tree generator from the committed inputs and requires zero
  diff against the committed tree, byte for byte. This check, not a lock, pins the exact
  bytes.
- It runs on `pull_request` **and on `merge_group`**, because the Merge Queue candidate is
  where two textually merged PRs are first combined. It is a required check of the Merge
  Queue for any change to its inputs.
- Its path trigger covers every generator input: `content/**` (including
  `content/world/**`), `imports/**`, `tools/content-migration/**`, the authoring tools the
  generator reads and the evidence files it names. Today `content-tree-migration.yml` and
  `g4-canonical-worldproject-package-seed.yml` run on `pull_request` only and
  `merge-group-gate.yml` has no content lane, so this check is new work (slice 2) and is a
  workflow and required-check change that needs its own owner authorization.
- Until that check is live in the Merge Queue, `legacy_blobs` and the existing pins are not
  removed: slice 1 may change the layout, but it keeps a committed binding until slice 2
  replaces it.
- Tests and validators assert structure, identity, reference integrity and the derived
  digests against the files; they do not pin whole-corpus hashes, byte lengths or totals in
  hand-written code.
- Integrity is not weakened: the exact bytes are pinned by reproducible regeneration on
  every queue candidate instead of by hand-edited constants.

### 3.5 Residual conflicts

A textual conflict remains when two PRs:

- change the same record; or
- insert, change or remove records on **adjacent lines of the same bucket** (Git treats
  touching hunks as one conflict).

The second case needs both records in one of 256 buckets and next to each other in key
order; with about 23 records per bucket in the largest creature-derived families it is
rare but real. The Merge Queue cannot resolve it: the author of the later PR merges
`main`, regenerates and freezes a new candidate. A clean textual merge whose regenerated
output differs from the merged files (for example a cross-record derived field) fails the
queue's regenerate-and-compare check and is handled the same way.

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
   A contributor's own PR is not enqueued; it records the contributed head in its task
   record and on #162.
2. One integrator, the active control plane on #162, merges the ready source changes onto
   one integration branch from current `main`, takes generated files from `main`, runs the
   restage once and freezes one candidate.
3. That candidate gets one CI run, one independent review and one Merge Queue entry.
4. Dropping a failing contribution returns the integration branch to AUTHORING. The next
   state is a new candidate with a new freeze, fresh CI and the review the bound policy
   requires; no evidence carries over from the previous batch head. The dropped
   contributor's PR stays open for repair and joins a later batch.
5. When the batch merges, each contributor's PR is closed as superseded by the integration
   PR, with the integration PR linked, and its task record is archived by the integration PR.
6. A single ready content change with no other content change pending goes alone.
7. The batch size limits of `AGENTS.md` ("Work in batches") apply.

## 6. Implementation slices (after acceptance)

1. Tree generator and validator: identity-key buckets, JSON Lines shards, count-free
   indexes, manifest and project without per-commit fields, digest-free lock, derived
   digests and totals; regenerate once. No record content change. `legacy_blobs` and
   existing pins stay until slice 2.
2. CI (needs owner authorization for the workflow and required-check change):
   regenerate-and-compare on `pull_request` and `merge_group` with the full input trigger
   of 3.4; then remove `legacy_blobs` and the tree pins from hand-written tests and
   replace them with structural assertions and the CI summary.
3. Loader: the tree reader used by the runtime switch reads the new layout and computes
   the package digest.

Each slice is its own PR with the checks selected by its paths.

**Rollback.** Reverting slice 1 or 2 is safe: the tree is not the runtime source
(`legacy_until_separately_qualified`), so a revert only restores the previous tree layout
and pins. Slice 3 ships with the runtime switch and follows that switch's own rollback.

## 7. Excluded

- No change to `content/world/**`, WorldProject v2 locators, their manifest or lock, or
  `g4-canonical-worldproject-package-seed.yml`.
- No runtime switch; that remains separately qualified.
- No change to record shapes, identities, provenance fields or the admission contracts.
- Not option C.

## 8. Acceptance

- Independent review of this candidate on its exact frozen head.
- After slices 1 and 2, targeted merge cases, each two synthetic PRs from the same base,
  checked with `git merge-tree` and then through the Merge Queue:
  1. different records of the **same bucket**, not on adjacent lines: no textual conflict,
     and the merge group passes regenerate-and-compare;
  2. records in **adjacent buckets**: no textual conflict, and the merge group passes;
  3. new records in two different families: no textual conflict, and the merge group passes;
  4. the same record changed by both PRs: textual conflict, as intended;
  5. a textually clean merge whose committed output is stale: the merge group fails
     regenerate-and-compare.
- No committed file under `content/**` other than record shards changes in cases 1–3.
- The derived package digest is identical across two regenerations of the same head.
