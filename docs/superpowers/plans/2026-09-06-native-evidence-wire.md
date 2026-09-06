# Native evidence wire codec implementation plan —346

Accepted source330/339 and protected source-resource/addendum344 govern; this plan selects no new protocol or authority. Execute only after exact protected allocation plus registry342 readback. One branch agent/native-evidence-wire-346, one exclusive worktree/writer, 60-minute windows preserving provenance/counters. Work remains control plane.

## Exact five paths

- `apps/game-server/src/admission_evidence.rs`
- `apps/game-server/tests/admission_evidence_wire.rs`
- `apps/game-server/src/lib.rs`
- `docs/agents/tasks/active/OTV2-20260906-native-evidence-wire-346.md`
- `docs/superpowers/plans/2026-09-06-native-evidence-wire.md`

lib.rs is one module export only under Work's explicit247 amendment. Existing serde/serde_json/base64 dependencies are available; no Cargo or Foundation mutation is allocated.

## Closed wire families

All requests contain integer version and exact operation name. All observed responses contain exactly eight envelope fields: version, operation, result="observed", source_authority, source_revision, decision_identity, source_observed_at, clock_uncertainty_seconds.

| Operation | Version | Request binding fields | Additional observed fields |
|---|---:|---|---|
| ReadAccountSecurityV1 | 1 | account_id, independently configured purpose, scope | same bindings, allowed, minimum_valid_generation |
| ReadFreshSigningTrustV1 | 1 | issuer, profile, independently configured key_purpose, key_id | same bindings, trusted, public_key |
| ReadRecoveryAccountSecurityV2 | 2 | account_id, purpose=platform_security, scope=existing_actor_recovery | same bindings, allowed, minimum_valid_generation |
| ReadRecoverySigningTrustV2 | 2 | issuer=urn:oteryn:platform:game-recovery, profile=oteryn-reauth-recovery-v1, key_purpose=existing_actor_recovery, key_id | same bindings, trusted, public_key |

Account observed has13 and trust observed14 mandatory fields. Fresh issuer/profile are urn:oteryn:platform:game-admission and oteryn-pre-admission-v1. Do not infer fresh evidence key_purpose from credential fresh_entry. V1 unspecified literal bindings remain independently supplied bounded expected values; no default production descriptor or interoperability claim.

Failure body is exactly version, operation, result with one of not_found/unavailable/unauthorized/unsupported. No generation/key/source facts are fabricated. Transport failure has no trusted body. No HTTP method/path/status contract is invented.

## Bounded representation

Request1024/response8192 raw bytes including whitespace/escapes; root object only,16members,64decoded ASCII name bytes,256decoded string bytes unless stricter. Check caps before retaining/materializing peer-sized values; reject unknown keys and nested arrays/objects before consuming them into a DOM. Full raw byte cap does not replace per-field or allocation checks.

Source authority1–128 ASCII matching [A-Za-z0-9._:/-]+ and exact expected descriptor identity. Positive revisions/generation are canonical decimal u64 strings (20digit maximum), decision_identity is exact canonical source revision. Time is nonnegative signed64 Unix decimal string (19digits), uncertainty canonical u64 string permits0; later authenticated freshness arithmetic remains separate. Account UUID lowercase canonical hyphenated36 and native semantics, never integer compatibility. KeyID1–64 ASCII [A-Za-z0-9._-]+. Public key exactly43 canonical unpadded base64url chars→32bytes, validate trailing bits. JSON booleans only; selected version is integer. No fallback between purposes/versions.

## Execution and evidence

1. Independently construct positive golden request/response examples for every operation and failure family, with inline fixture provenance. Fixture V1 literal choices demonstrate parameter binding only.
2. Implement minimal closed typed request encoding and bounded response visitor after real failing tests. Do not implement Foundation source/seal traits, accepted floors or local revisions.
3. Add missing/duplicate/unknown/wrong scalar/nested/trailing/version/operation negatives, mutating one invariant with otherwise valid independently expected bindings.
4. Exercise account/source/purpose/scope/issuer/profile/key-purpose/key-ID substitutions; both fresh↔recovery directions. Prove denial/untrust and absent results do not manufacture facts or positive authorization.
5. Test max/max+1/overflow, signs/leading zeroes, invalid UUID/key grammar, invalid base64 padding/trailingbits, raw escapes/decoded expansion/truncation and allocation-before-rejection. Semantically fixed encodings use exact widths plus nearest representable values and exact generic byte comparison.
6. Run affected library/integration, strict all-target Clippy/fmt and selected repository checks. Stage material checkpoints, pause custody for Work native publication/CI; no local-only completion credit. Final independent exact-head review and protected Merge Queue required.

## Separate downstream work

Actual authenticated mTLS/HTTP, rollback-protected descriptor, four-operation shared accounting, durable V1/V2 Account floor and distinct signing namespaces, acknowledged PostgreSQL publication and current adoption remain separately allocated. Decoded DTOs are never current authority. No B/338/247 readiness follows from this codec alone.

## Execution checkpoint

Admitted Work5559516855 atad7273e3e91a4e4254abb9aa2710c7e0c9754afe, window1 13:27–14:27UTC. Closed flat parser uses fixed inline slots and manual bounded Unicode decoding; no generic DOM or escaped-string heap scratch. Independent fixture literals remain V1 binding demonstrations only. Tests cover four families and failure variants, each required field, substitutions, numeric and encoding boundaries, truncated input and fixed-buffer rejection before growth. Foundation mutations/SQL/TLS/live source are not exercised or claimed. Independent review then exact-head CI remain required.
