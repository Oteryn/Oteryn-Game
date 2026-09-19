# Oteryn World Project Source Profile v1 decision

- Date: 2026-09-19
- Gate: `CONTENT-504 / CW1`
- Status: `OWNER_APPROVED / PROTECTED_DELIVERY_PENDING`
- Owner acceptance: Issue `#162`, comment `5745395297`
- Execution allocation: Issue `#162`, comment `5745433817`
- Applies to: editable Oteryn Content/World project source and import/reimport only
- Does not authorize: source/runtime implementation, World Bundle selection, production resource registration, deployment or activation

## 1. Decision

`OTERYN_WORLD_PROJECT_SOURCE_PROFILE/v1` is the minimum canonical editable-source profile for the next Content/World implementation increment.

The profile uses strict UTF-8 JSON as its default physical carrier and the existing typed Reference semantic graph as its only gameplay model. A source reader produces `ReferencePlayableContentSource`; `link_reference_playable` remains the canonical validation/linking boundary and produces `CanonicalReferencePlayableContent`. A later successor compiler must consume that canonical value through the existing Content lineage. This decision does not create a second model, linker, compiler or runtime loader.

JSON Lines is not generally selected for maps or catalogues. A concrete high-cardinality record family may adopt JSONL only through a reviewed profile revision that cites representative measurements for that family and preserves every invariant in this decision. File count, bytes, record count, parse/write cost, peak memory, diff locality and coherent-save behavior are the relevant measurements. A tool author cannot select JSONL ad hoc.

This source decision is separate from the permanent World Bundle decision. Bundle serialization, compression, indexing, chunk/floor packing, patching, signing and production/full-world resource maxima remain under the measured D3 decision.

## 2. Why this decision is required now

ADR-0005 requires an editable native project with deterministic serialization, practical review, stable references, provenance, partial/coherent saves and recovery, while declaring its example directory layout illustrative. DUR-04 fixes the typed graph, exact Content Lock and compiler pipeline but deliberately leaves the physical source and bundle encodings open. The Content/World programme requires a usable canonical typed project destination before broad B5/B6 promotion.

The merged successor seam already supplies the semantic destination: `ReferencePlayableContentSource`, `PackageManifestBinding`, `ContentLockBinding` and `link_reference_playable`. Freezing the source profile now prevents each importer or Studio path from inventing its own JSON dialect, metadata authority or reimport behavior. It does not require the permanent runtime artifact choice.

## 3. Minimum project structure

One project root contains these logical roles:

```text
project.json                 canonical project root and revision commit point
manifest.json                canonical package/source manifest
content.lock.json            exact immutable dependency lock
records/                     typed definition and world records
imports/                     source snapshots, mappings, baselines and conflicts
metadata/                    non-authoritative author/editor information
```

Exact subdirectories and filenames below these roles are organizational. `manifest.json` records every project-managed `records/`, `imports/` and `metadata/` document by logical role, schema/profile revision, relative locator, byte length and SHA-256 digest. Metadata is therefore retained in the audited source fingerprint even though it has no gameplay authority. A path locates bytes; it is never `ContentKey`, `PlacementKey`, definition identity, ordering identity or runtime identity.

`manifest.json` carries the package fields needed to construct the existing `PackageManifestBinding`, required/optional feature declarations and the document inventory. Its canonical bytes supply `source_manifest_digest`. After that digest is known, `content.lock.json` supplies the existing exact `ContentLockBinding`, including the matching root package revision/provenance and exact dependency revisions. Finally, `project.json` binds the source-profile identity/revision, project revision, `manifest.json` locator/digest and `content.lock.json` locator/digest and acts as the publication commit point. This ordering is intentionally acyclic: neither `manifest.json` nor its inventory contains `project.json`, `manifest.json` itself or `content.lock.json`; the root binds the two completed objects. Floating revisions, mutable branches and network-time latest resolution are forbidden.

A metadata-only edit therefore changes the manifest and project/package provenance fingerprints. It does not by itself change typed gameplay definitions, keys or behavior. `CanonicalReferencePlayableContent` retains package/lock provenance, and a later artifact may embed that provenance, so this decision makes no claim that canonical-value equality, `content_revision`, package revision or compiled artifact bytes/digests remain unchanged. Those version and artifact rules stay with their accepted owning contracts and the successor compiler decision.

The profile does not require a file per record or one monolithic world file. A project may group records where practical, provided the manifest inventory, stable semantic identity and deterministic writer make regrouping semantically neutral.

### Canonical locator and root-containment contract

`project.json`, `manifest.json` and `content.lock.json` are fixed lowercase ASCII names at the project root in v1. They must be distinct ordinary files and cannot also appear as inventoried documents. Every inventory locator is a normalized root-relative logical locator, not an operating-system path supplied by a source package.

A canonical v1 locator uses `/` as its only separator and one or more lowercase ASCII segments. Each segment starts and ends with an ASCII letter or digit and may contain lowercase ASCII letters, digits, `_` and `-`; a `.` may separate nonempty such components. The following are invalid before filesystem access:

- an empty locator or segment, leading/trailing `/`, `.` or `..` segment;
- absolute, drive-qualified or UNC syntax;
- `\`, `:`, NUL/control bytes, percent-escaped separators, non-ASCII bytes, or a segment ending in a dot/space;
- a Windows device basename such as `con`, `prn`, `aux`, `nul`, `com1` through `com9`, or `lpt1` through `lpt9`, with or without an extension;
- a byte-duplicate locator or a locator colliding with a fixed control filename.

Lowercase ASCII plus the segment grammar gives one platform-independent textual spelling and removes Unicode-normalization, case-folding, alternate-separator, trailing-dot/space and alternate-data-stream aliases. Locator length and segment count remain finite inputs under the selected implementation budget.

Admission anchors all reads to one opened project-root directory; v1 rejects a project root that is itself a symbolic link, junction or other reparse point. Each component is resolved relative to that root without following symbolic links, junctions or other reparse points, and its actual directory-entry spelling must equal the canonical locator segment byte-for-byte. Every control or inventoried document must resolve to a root-contained ordinary regular file. V1 rejects symlinked/reparse-point directories or documents, special files and hard-linked source documents rather than attempting to prove a safe alias topology. Each admitted file must also have a unique filesystem identity; two locators resolving through a case alias, Windows short name, hard link or other platform alias to the same file fail the project. Validation/open must be one no-follow operation where supported, or must recheck the same root-contained filesystem identity immediately before consuming bytes; substitution fails closed.

Writers derive destinations only from fixed control names and already admitted canonical locators. They stage into a newly created non-aliased staging root under the writer-controlled project parent, create ordinary files without following links, and verify containment, unique identity, byte length and digest before publication. Source-provided filenames or archive members never become output paths. A changed component, symlink/reparse substitution, alias collision or destination outside the staging/project root aborts without replacing the current commit point. These are source-profile invariants, not a requirement for a new generic filesystem sandbox.

## 4. Strict JSON profile

Every JSON document must satisfy all of the following before typed construction:

1. UTF-8 without BOM; invalid Unicode and trailing non-whitespace bytes fail.
2. Exactly one JSON value of the root kind required by that document role.
3. Duplicate object member names fail before typed construction; last-key-wins behavior is forbidden.
4. Unknown fields fail except inside an explicitly typed `extensions` or author-metadata member allowed by that schema revision.
5. Unknown required features fail. Optional extensions may be retained only when their namespace and non-authoritative treatment are declared; they cannot influence compilation until promoted into a later typed schema.
6. Authoritative numbers use the integer type and unit declared by the typed schema. Floating-point interpretation, implicit units, numeric strings and lossy coercion are forbidden unless a later typed field explicitly defines them.
7. References are explicit typed keys/revisions. Display names, paths, JSON member order, array index and source numeric IDs cannot substitute for semantic identity.
8. Input lengths, declared counts and aggregate arithmetic are checked before input-sized allocation. Every implementation profile has finite admission limits and max/max+1 tests for the boundary it uses.

Use maintained upstream parsing/serialization APIs, currently the workspace `serde`/`serde_json` lineage, unless a concrete versioned capability gap is proven. Oteryn-owned code supplies schema/domain validation and duplicate-name enforcement around that parser; it must not implement a general JSON parser.

The canonical writer emits one deterministic byte representation for a typed project revision: schema-defined field order, lexically ordered map keys, preserved order for semantically ordered sequences, identity-sorted output for set-like collections, no insignificant whitespace, and one terminal LF. Locale, wall clock, filesystem enumeration, host paths and unordered collection iteration cannot affect emitted bytes. Parse -> typed link -> canonical write -> parse must preserve the same canonical graph and bytes.

For an admitted JSONL family, each nonempty LF-terminated line is one strict JSON record under the same rules. The family-specific profile defines the record kind, stable identity, ordering rule and manifest count/digest. Blank lines, comments, partial final lines and duplicate record identities fail. JSONL physical line order cannot become identity unless the typed schema explicitly declares the sequence semantically ordered.

## 5. One typed semantic authority

The source profile serializes the existing Reference successor semantics. It does not add an intermediate gameplay record model. I/O-only parsing state may exist transiently to enforce bounds and diagnostics, but it cannot become a separately versioned graph or a second set of validation rules.

Canonical gameplay meaning remains in typed fields such as definition family/revision, `ProductionKey`, evidence binding, placement identity, coordinate frame, ordered placement relation, footprints, finite states, transition bindings and owner capability requirements. The linker continues to reject wrong-family, unresolved, duplicate, incompatible, non-promotable or structurally invalid values.

Author/editor metadata may contain display/editor names, descriptions, localization references, categories, namespaced tags, notes, grouping, selection state, tool hints and editor-only deprecation/replacement guidance. Those bytes are inventoried and preserved, but they are excluded from authoritative typed gameplay semantics, client/server gameplay projections, owner selection, collision, transitions, eligibility, loot/value/RNG and runtime policy. A name, category, tag or replacement hint cannot activate or redirect a mechanic. Any metadata value later needed by gameplay must first receive an owned typed field and schema revision; compilation otherwise fails rather than interpreting the metadata.

## 6. Provenance and import records

Every imported batch records enough immutable data to reproduce and audit its candidate output:

- source repository/package identity, exact revision and file/blob digests;
- access/redistribution disposition where applicable;
- source-generation profile and revision;
- importer/decoder and mapper identity/revision;
- source record identity and source-to-native field/key mapping;
- units/transforms and explicit override rules;
- evidence classification and references;
- unsupported, unknown, lossy, ambiguous, conflicting and excluded records;
- the selected closure disposition: executable, candidate-only or blocked, with reason;
- the exact normalized imported baseline used for later reimport.

Source numeric IDs, filenames, paths, display names and appearance IDs remain provenance or mapping inputs. They do not become native stable identity by hashing or renaming. Source/import evidence, target truth, redistribution permission and runtime capability remain separate admission axes.

The project manifest and existing Content Lock bind the resulting package revision and source-manifest digest. Provenance records augment that binding; they do not replace it or create another provenance authority.

## 7. Semantic three-way reimport

Reimport compares, per stable typed identity and field:

```text
previous normalized imported baseline
vs new normalized source candidate
vs current local canonical project value
```

The deterministic outcomes are below, where `B` is the retained baseline value, `U` a changed upstream value and `L` a changed local value:

| Baseline | New source | Local project | Outcome |
|---|---|---|---|
| `B` | `B` | `B` | unchanged |
| `B` | `U` | `B` | adopt `U` and its provenance |
| `B` | `B` | `L` | retain local correction `L` |
| `B` | same changed value `X` | same changed value `X` | converge to `X` |
| `B` | `U` | different `L` | explicit conflict; no overwrite |

Add, delete, rename/move, identity remap, split/merge and transform cases use the same rule. A deletion conflicting with a local correction or retained state becomes a conflict or an explicitly approved retirement/migration. Rechunking, regrouping, file moves, canonical reserialization and reload cannot create semantic additions/deletions or new reward/value occurrences.

Conflict records carry the stable identity, typed field path, baseline/new/local values or immutable references, provenance for each side, and explicit disposition. Unresolved conflicts fail selected executable closure. Candidate-only catalogues may retain them visibly.

## 8. Coherent saves and recovery

A project revision is readable only when the fixed, root-contained `project.json`, its bound `manifest.json`, its bound `content.lock.json` and every uniquely resolved inventoried document form one validated set under the locator contract above. Writers stage changed bytes, compute and validate the complete manifest, construct and validate its exact lock, then publish `project.json` as the project-revision commit point using an atomic filesystem primitive or a journal/recovery protocol. Readers resolve from that root and never accept a mix of old and new bytes; containment, identity, digest or lock mismatch fails closed.

Partial authoring saves may update a bounded subset of records, but publication still creates a coherent project revision. Autosave/recovery state remains outside the current committed revision until it passes the same validation. A failed save or interrupted migration leaves the previous valid revision recoverable and preserves a pre-migration backup.

The exact filesystem transaction mechanism is an implementation choice because platform capabilities differ. The observable invariant is one complete validated revision or the previous revision, never a partially authoritative mixture.

## 9. Resource boundary

This decision defines resource dimensions, not numeric product maxima: total project bytes/documents/records, per-document and per-record bytes, nesting depth, string/key length, decoded field count, reference count, provenance/baseline/conflict volume, and transient parse/write memory.

The first narrow implementation proof may use explicit finite evidence limits measured against its admitted real family. Those limits must be labeled non-production and cannot be promoted to full-world or runtime maxima. Production admission requires accepted registry values and boundary tests for the concrete production profile. The D3 measurements' 32-cell chunks, 64 MiB artifact fence and 2 MiB raw-chunk fence remain harness evidence only.

## 10. Required first proof

After separate CW3 allocation, the smallest implementation proof is one real imported family through:

```text
pinned source + importer provenance
  -> canonical project JSON
  -> strict bounded parse
  -> ReferencePlayableContentSource
  -> link_reference_playable
  -> existing Content successor compiler path
```

The proof must cover byte determinism, enumeration and file-regrouping independence, malformed/duplicate/unknown-critical input, digest/lock mismatch, typed reference failures, metadata/tag authority exclusion, coherent interrupted save, and three-way unchanged/upstream/local/converged/conflict/delete outcomes. Locator negatives include `../` and nested traversal, POSIX absolute paths, drive/UNC paths, backslash and alternate-data-stream syntax, empty/dot segments, duplicate locators, case/trailing-dot/trailing-space/device-name aliases, two locators resolving to one filesystem identity, source-document/directory symlinks or reparse points, hard-linked source documents, and staged write-target escape or link substitution. Platform-specific alias cases are mandatory on every supported platform where they exist. If a JSONL family is proposed, its representative measurement and JSON-equivalent semantic/round-trip proof precede enabling it.

No separate compiler may be added for the source profile. Because the merged repository currently has the Reference linker but no Reference artifact compiler, the final arrow is future CW3 work extending the existing Content compiler lineage; this decision does not claim it exists.

## 11. Compatibility and supersession

`FIRST_PRODUCTION_CONTENT_PROFILE/v1` remains unchanged. Cross-profile loading and activation continue to fail closed. This source profile does not reinterpret bootstrap artifacts or their accepted maxima.

Supersession requires a newer accepted profile/decision supported by concrete authoring, compatibility, security or representative-corpus evidence. A later Studio container may cache or transact the same graph, but export/import through this canonical project must preserve identities, provenance, semantic equality and three-way history. A convenience database cannot become a second semantic authority.

## 12. Explicit non-decisions

This decision does not select:

- a blanket JSONL map/catalogue format;
- `.omap` or `.owb` names/extensions;
- World Bundle serializer, schema carrier, compression or chunk/floor packing;
- runtime spatial-sector dimensions, patch/CDN/signing layout or production activation;
- production/full-world numeric maxima;
- target coordinates, collision, order, footprints or other still-unproved Reference facts;
- full future NPC, quest, economy, house, event, scripting or Studio schemas;
- a universal metadata/EAV gameplay extension mechanism.

Future families add only the typed fields required by their accepted increment. They do not need to enumerate all future fields before the first real family can use the project.

## 13. Decision timing

- **Must decide now:** YES, for a single canonical editable destination before broad B5/B6 promotion and the first serialized CW3 proof.
- **Blocked without it:** importers and authoring paths could produce incompatible JSON dialects, provenance, metadata authority and reimport rules.
- **Harder later:** multiple persisted source dialects would require migration and could couple identity or gameplay behavior to layout/tags.
- **Evidence that may supersede it:** representative authoring/Studio measurements, a concrete upstream codec limitation, compatibility/security findings, or measured family-specific need for JSONL or another source carrier.
- **Deliberately deferred:** the World Bundle and production resource decision remains D3-owned.

`OWNER_DIRECTION: ACCEPTED_JSON_WITH_CONDITIONAL_MEASURED_JSONL`

`SOURCE_MODEL: EXISTING_REFERENCE_GRAPH_AND_LINKER_ONLY`

`WORLD_BUNDLE_AND_PRODUCTION_MAXIMA: DEFERRED_TO_D3`
