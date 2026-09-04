# Game server bootstrap governance

This directory owns the native Oteryn Game Server process composition root.

During `OTV2-IMPL-BOOTSTRAP` it is intentionally **foundation-only**:

- no gameplay socket/listener;
- no wire framing or `protocol-oteryn` schema/IDs;
- no Game Session/admission credential handling;
- no persistence/database adapter;
- no gameplay movement/combat/content semantics;
- no production deployment behavior.

The executable must fail closed when invoked for real gameplay before a later coordinator allocation merges the accepted Foundation protocol/runtime/admission seams.

Any later change that introduces protocol, session, admission, persistence, public identifiers, fencing, multichannel authority or security semantics is high risk under root governance and requires the owning accepted contract plus genuinely independent exact-head review.

Do not introduce Canary/legacy-Tibia compatibility code or an alternate login authority here.

## Authority and recovery qualification discipline

For a high-risk change that can authorize PREPARE or COMMIT, install or restore a controller, replace an authority-bearing session, or interpret persisted recovery evidence, use the executable model:

```text
AuthorityInvariant × ConsumerBoundary × MutationOperator
```

Apply these rules before material freeze:

- distinguish immutable prepared/persisted evidence from independently resolved current authority; immutable evidence may define the expected binding, but it is not current authority evidence;
- production authority-granting APIs must consume independently supplied current facts and must not reconstruct those facts solely from the immutable record they are validating;
- a record-derived matching helper may exist only as an explicitly test-only happy-path convenience and must not be used by negative authority, provenance or mutation cases;
- classify invariants at least as identity/binding, current liveness/authority, or temporal/provenance, and mutate exactly one applicable invariant per negative case while leaving unrelated facts semantically valid;
- cover every applicable authority-consuming boundary, including compatibility and typed versions, rather than treating scenario count as authority coverage;
- run focused RED → minimal GREEN, deterministic affected validation, a finding-family sweep and a whole-diff adversarial self-review before freezing the material candidate.

After a material P0 or P1 finding, the reviewed generation is superseded. Repair it test-first, then sweep sibling APIs, protocol versions, direct and reconciled paths, restart, retry/replay, concurrent replacement and PostgreSQL reload where applicable before requesting another deep review. Historical terminal outcomes may retain typed disposition without current live-authority equality, but they must never reacquire controller authority through a weaker compatibility path.

Every P0/P1 finding requires either a verified repair or a verified rejection with exact evidence. Every P2 requires an explicit `fixed`, `accepted` or `deferred` disposition. External AI review remains advisory under the META-owned policy and is never merge authority; repository gates, protection and Merge Queue remain authoritative.
