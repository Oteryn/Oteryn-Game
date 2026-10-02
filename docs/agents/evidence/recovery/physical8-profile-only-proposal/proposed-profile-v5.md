# Physical8: proposed Item artifact v5 and resource profile v2

Status: PROPOSAL ONLY, NOT ACCEPTED, NOT IMPLEMENTED. This external packet proposes a profile-only owning decision before any production enum/schema/codec release. It changes no repository file, runtime admission, protocol, Item identity, native field or existing resource row. Actual inspected source is published Forge4f `4f690c1e8a8966a4d074dde94d5a884dbb34fb94`. The original Physical8 inventory/source57/native49 checkpoints and the recovered uncompiled prototypes remain immutable; no prototype transplant is proposed.

The owning v1 profile section1 requires independent frozen tool/evidence review, exact registry/governance checks and a LIVE coordinator acceptance checkpoint tied to exact profile/registry/tool/evidence blob hashes BEFORE schema/codec release. A draft, synthetic measurement, user authorization for enrichment or historical accepted source is not that checkpoint. This successor remains held at that gate.

## Proposed grammar and caller

Introduce a distinct `OTERYN_REFERENCE_PLAYABLE_ARTIFACT/v5`, magic `OTRPA05\0` (hex4f54525041303500), header profile_version5, typed body record_version2, compiler `OTERYN_REFERENCE_PLAYABLE_COMPILER/v5` and canonicalization `OTERYN_REFERENCE_PLAYABLE_CANONICALIZATION/v5`. Resource owner is proposed `D6_M1_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE/v2` in a NEW `docs/architecture/OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V2.md`. Explicit profile selection is required; an enum literal appearing in a record must not silently choose a different source universe or reinterpret a v4 artifact.

Modifier ELEMENT byte values remain Death1/Earth2/Energy3/Fire4/Holy5/Ice6; append Physical7. These are ReferenceModifierElement codes, not ResistanceKind.Physical10 or WeaponElement codes. All37 ModifierKind IDs/order and six parameter cases remain unchanged. ELEMENT stays one byte. Preserve all other typed groups, limits and client allowlist. JSON PHYSICAL is a new ModifierElement literal only on the newly accepted source/schema carrier; any existing native source schema/version metadata affected by that extension must be explicitly disposed in the live owning checkpoint rather than silently widening a historical schema declaration. No network/protocol ABI or cross-process Rust layout promise is made by repr(u8).

Use a separate explicit v5 current-Item-family constructor/caller over the COMPLETE actual current canonical Item projection, exactly34031 identities at this proposal cut. Retain every Item, including materializable=false and partial Known semantics; do not drop fields or records to meet the profile. Bind the exact full family/key/revision identity-set digest and actual source input digests to the new qualification receipt/metadata. Both projections use that same full universe. Missing/extra/duplicate/different identity or unresolved accepted cross-Item target rejects before allocation/staging. Known values remain Known and unsupported Known fields reject the WHOLE artifact, never become UNKNOWN or disappear.

The old protected B1 artifact fixture remains a DIFFERENT exact38157 universe. Existing v4 `for_source` and `accepts_item_count` requirements and registry V1 rows remain unchanged. No generic partial-family exception, constant mutation or4126 fabricated identities is permitted. Native current34031 versus historical38157 is a difference of source universes, not a missing-item backlog. Other non-Item families remain outside this Item-only artifact: actual materialized Reference source contains57320 records, of which34031 are Items. Do not feed all57320 through an Item-only codec.

## Compatibility requirements

- New reader accepts every retained v1–v4 golden under its original profile, count and limits; no old bytes are reinterpreted as v5. Old1..6 ModifierElement wire values and JSON spellings remain exact.
- Existing v1–v3 have no admitted typed ModifierElement body. Existing v4 still rejects ELEMENT7 at encode and on actual lazy body lookup decode, even after the shared Rust enum learns Physical. Do not call unrestricted from_wire(7) from the v4 decoder.
- Preserve v4's header/index/integrity parse followed by lazy body lookup. A forged integrity-correct v4 body carrying7 may reach the legacy lazy header/index boundary but must fail lookup; do not add an eager whole-body semantic scan to claim earlier rejection.
- New v5 permits ELEMENT7 and rejects0/8/255, with all other count/order/state/source guards unchanged. Both server and client-safe projected bodies must preserve complete Physical vectors and all Known sibling fields.
- Old reader rejects new magic/profile before body semantics. Profile/compiler/canonicalization/version/package pairing and source-universe mismatches reject; mixed v4/v5 generation pairs cannot stage partially. Artifact v5 availability does not activate a runtime/client consumer.

## Proposed exact bounds, not accepted measurements

Current observed native identity count is34031. The old grammar's maximum body records are3555 server/3433 client bytes; the external synthetic v5 experiment also produced those exact lengths with Physical7. This supports a proposed equal-width hypothesis, not a measured full-artifact or max+1 qualification. The proposed successor limits retain the existing semantic maxima and use a NEW count-bound resource family, appended to the registry without changing V1 rows:

| Proposed dimension | Limit | Evidence status |
|---|---:|---|
| current Item records, each projection | exactly34031 | actual4f full identity set observed; final successor must rebind exact identities |
| Modifier ELEMENT code domain |1..7, byte width1 | proposed append-only domain; invalid0/8/255 test required |
| server/client record |3555 /3433 bytes | observed synthetic records only; complete max/max+1 remains pending |
| manifest |7500 bytes | inherited carrier proposal |
| index |36379143 bytes | calculated4+34031*1069; not observed maximum |
| server/client body |120980205 /116828423 bytes | calculated34031*3555 /34031*3433 |
| server/client artifact |157367048 /153215266 bytes | calculated200+7500+index+body |
| combined pair |310582314 bytes | calculated checked sum; not RSS or allocator budget |

No public hard maximum may be released from these arithmetic calculations alone. Every inherited atom/collection dimension, every new count/element-code boundary, full header/index/body/projection and pair ceiling needs independent encode/decode max and max+1 evidence on the accepted profile shape. The existing Python maximal fixture needs explicit v5 selection, complete profile metadata and registry-linked output. Existing v1 tool/original evidence stays immutable; a successor tool/evidence receipt is separately versioned and pinned.

An actual4f JSON-only observation found12100 Known names with maximum46UTF-8 bytes and zero Known descriptions. This is current source atom observation only, not an artifact test or proof that all possible/current-future sources fit every dimension. Any later predecessor atom or Known field exceeding the proposed ceiling must hold and obtain an honest owning adjustment; no truncation/loss.

## Exact-blob acceptance checklist before codec release

1. Allocate a separate PROFILE-ONLY authoring branch after Root's currently serial publication lanes. Create new profile doc, append-only candidate registry rows, successor independent oracle/tool and its source-pinned evidence; no native Physical promotion or production codec change in that profile slice.
2. Freeze exact Git tree/commit and profile, registry, tool, evidence SHA256/Git blob IDs. Independent review must validate selected current34031 universe, old38157 preservation, exact grammar/metadata, source/field no-loss rules and every finite resource dimension. Run actual governance and registry validation on that stopped profile-only candidate.
3. Collect missing full-family/oracle/projection/header/profile/golden and max/max+1 evidence without representing current synthetic-record output as complete. Recompute exact bounded arithmetic and source input identity sets. All measurements distinguish semantic maxima, scenario observations and encoded pair bounds.
4. Owning live coordinator checkpoint explicitly records `D6_M1_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE/v2 = ACCEPTED` for those exact blobs, selected artifact input universe and v5 grammar/caller. Any changed profile/tool/evidence/registry bytes invalidate that acceptance binding and require a fresh checkpoint.
5. Only then allocate minimal fresh production schema/codec diff against the actual current predecessor; do not transplant the historical prototype. Implement profile-scoped encode/lazy-decode and source carrier/version disposition, independent oracle and tests. Full Rust/schema production acceptance remains a separate gate.
6. Published Modifier26 and Mantra49 proofs pin the whole old reference_playable.rs and compiler dependencies. Preserve their V1 facts/files/history exactly. A NEW dependency qualification/receipt and versioned current compiler/packet closure must truthfully pin the new enum/model bytes, prove identical26 and49 facts/rows/typed kind order, and retain historical reproduction separately. Do not relabel the old pin as current or silently repin frozen V1 artifacts. Current source57proof/native49 scope and eight Physical holds remain intact until their own successor integration.
7. Separately qualify/promote the exact eight whole vectors16atoms from immutable source57. Recheck full current source/identity/member/name/World/native-state guards, complete own-ID frame and cutoff. Preserve50239 historical1177239 and postcut1207393 separately, continuityUNKNOWN. Atomic/idempotent full vectors; no partial Physical omission or inferred modifier evaluation/priority/target binding. If prior494/796 remains actual, only then conditional502/812; recompute final counts.
8. Full all57320/34031 canonical/native comparison and both actual current-family projection artifacts including nonmaterializable records; old goldens/limits/contexts/packets protected. No D278, gameplay admission, lifecycle, protocol/runtime or client activation change follows automatically.

Prototype measurement reference: physical8-external-prototype-measurement-20261002.json, oracleSHA b77934f4b3c1c7a99442ecadc854b5d81412a83a3cbd52485971d334ccfb2d87. v4 Physical7 rejected, v5 Physical7 synthetic record roundtrip passed, invalid0/8/255 rejected. Production Rust, full-artifact, full-family max/max+1 and live owning acceptance are NOT established.
