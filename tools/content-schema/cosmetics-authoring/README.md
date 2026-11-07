# Permanent cosmetics authoring — D1093

This is the canonical tree-first Outfit/Mount authoring format and materializer. It leaves the
historical identity-only WorldProject packets unchanged. No production authoring document is
supplied while sex/look, starter/addon, fallback and source-conflict decisions remain unresolved.

An explicit `OTERYN_CHARACTER_COSMETICS_AUTHORING/v1` document contains the content revision,
Reference profile revision and +10 mount speed fact, client source version, RGB palette, complete
Outfit/Mount records and exactly one no-addon/no-mount fallback for each sex. Required fields
have no defaults. Unknown members, duplicate JSON members, missing facts and invalid masks or
fallbacks fail closed. The palette has 133 authored RGB entries; it is not an engine table.

Each identity, look/sex mapping, starter flag, addon count/mask, premium value, unlock sources,
palette, fallback and source binding has a separate `PROMOTED` entry. Its JSON pointer and exact
value must match the document, with a decision reference and an exact source reference/revision/
SHA256. Only `STRUCTURED_REFERENCE_DATA`, `OWNER_DECISION` and `ACCEPTED_REFERENCE_EVIDENCE`
are supported source classes. `UNKNOWN`, OTS-only and source-observation-only promotion classes
are rejected. These fields retain the owning admission decision; their strings alone do not
prove its truth or grant authority. A source observation's numeric membership never supplies
the missing sex, starter, fallback or canonical identity mapping.

The producer binds the exact existing staged source-manifest/shard bytes and every used look's
record digest. It does not parse DAT again or infer semantic facts from sprite observations.
The existing Rust source-bound compiler is reused once: only the canonical document's schema
discriminator is normalized to its private compiler format. All authored fact values are retained.

```
python tools/content-schema/cosmetics-authoring/cosmetics_authoring.py \
  --authoring content/cosmetics/outfits/authoring.json \
  --output content/cosmetics/outfits/native-cosmetics.json [--check]
python -m unittest discover -s tools/content-schema/cosmetics-authoring -p 'test_*.py'
```

`world_project_v2_to_tree.py` calls this producer before writing derived content when authoring
exists; missing authoring produces no catalogue. A stale generated cosmetics packet without its
authoring source refuses regeneration. The authored input and runtime output join the existing
manifest/lock file set only during that tool's CP slot. No index or registry is guessed here.

The native ABI requires `cosmetics: PinnedGameplayBytes`. Historical provisioning V1–V5 obtains
an explicit `availability: unavailable` document with real bytes and a checked SHA256; it exposes
no cosmetics and preserves V1–V8 artifact bytes. Provisioning V6 requires an explicit authored
pin. Binary V9 is precisely V8's twelve sections plus a thirteenth cosmetics section, retaining
the existing total artifact bound and a 16MiB cosmetics-section bound. The V8 closure requirement
is binary layout compatibility, not a gameplay dependency between loot and cosmetics.

Compile and decode validate the new section. Only the actual independently issued outer artifact
and sealed activation guard admit it. `NativeGameplayState::cosmetics()` returns a borrowed
`NativeCosmeticsView`, with sorted key iterators, the existing appearance-content interface and
the same outer source digest. A qualified source-world wrapper rebinds that digest with the rest
of the generation. No mutable catalogue, active-generation authority, unlock, sex input, mount
effect or client capability is minted. A changed valid catalogue cannot reuse an old issuance.

`synthetic-authoring.json` is RAW SYNTHETIC TEST DATA with explicit test-only promotion labels.
It is not a populated production catalogue or evidence that any real sex/starter/fallback mapping
was accepted. Product population and shared indexes/manifests/locks remain pending CP facts/slot.
## Authored earned grants (owner decision 2026-10-22)

An optional document `grant_mappings` array maps each opaque `grant_key` to one
explicit target: `Outfit` with `outfit_key`, `Addon` with `outfit_key` and
one-based `addon`, or storage-only `Mount` with `mount_key`. Each mapping carries
a nonempty, unique `compatible_provenance` list of recorded grant revisions.
Keys are unique, targets must exist, and addon identities must fit that outfit.
The whole array requires its own value-bound `/grant_mappings` promotion.

An absent mapping authorizes nothing. Matching the active content revision is
not a compatibility rule, and grant keys are never derived from cosmetic keys.
The runtime reads only declared keys for the fenced Character's account. Store
remains unavailable; premium metadata retains the V1 D61 behavior.
