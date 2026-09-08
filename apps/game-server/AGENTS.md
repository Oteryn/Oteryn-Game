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

For a high-risk change that performs a production mutation gated by current session, lease, generation, authority or other fence evidence; authorizes PREPARE or COMMIT; installs or restores a controller; replaces an authority-bearing session; or interprets persisted recovery evidence, use the executable model:

```text
AuthorityInvariant × ConsumerBoundary × MutationOperator
```

Apply these rules before material freeze:

- distinguish immutable prepared/persisted evidence from independently resolved current authority; immutable evidence may define the expected binding, but it is not current authority evidence;
- every production mutation or authority grant that consumes current authority/fence evidence must receive evidence sufficient for that boundary and must not reconstruct supposedly current facts solely from the immutable record being validated;
- a record-derived matching helper may exist only as an explicitly test-only happy-path convenience and must not be used by negative authority, provenance or mutation cases;
- classify invariants at least as identity/binding, current liveness/authority, or temporal/provenance;
- enumerate the concrete applicable mutation operators rather than recording only a cardinality rule; consider at least missing facts, stale facts/generations, mismatched identity or binding, expired/future/non-monotonic time, provenance substitution and boundary-specific replay/concurrency operators, with explicit `NOT_APPLICABLE` evidence where an operator cannot apply;
- each negative case changes exactly one applicable invariant while leaving unrelated facts semantically valid;
- cover every applicable authority-consuming mutation boundary, including fenced durable writes, compatibility and typed versions, rather than treating scenario count as authority coverage;
- run focused RED → minimal GREEN, deterministic affected validation, a finding-family sweep and a whole-diff adversarial self-review before freezing the material candidate.

On a material P0/P1 report, first verify applicability and correctness against the exact reviewed head. A verified rejection with exact evidence preserves the frozen candidate and does not require repair or re-review. Only an accepted/verified material finding supersedes the reviewed generation; repair it test-first, then sweep sibling APIs, protocol versions, direct and reconciled paths, fenced durable writes, restart, retry/replay, concurrent replacement and PostgreSQL reload where applicable before requesting another deep review. Historical terminal outcomes may retain typed disposition without current live-authority equality, but they must never reacquire controller authority through a weaker compatibility path.

Every P0/P1 report requires an explicit verified disposition: accepted and repaired, or rejected with exact evidence. Every P2 requires an explicit `fixed`, `accepted` or `deferred` disposition. External AI review remains advisory under the META-owned policy and is never merge authority; repository gates, protection and Merge Queue remain authoritative.
