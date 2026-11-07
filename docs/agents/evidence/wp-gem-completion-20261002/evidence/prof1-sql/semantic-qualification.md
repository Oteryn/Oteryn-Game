# PROF-1 exact-candidate SQL behavioral qualification

Classification: PROVEN for the local PostgreSQL boundaries below. This is preparatory evidence; migration 0032 has not been admitted to the repository or runtime. D283b admission remains the complete migration/writer/integrity/support gate, including the control-plane disposition of the 678-line SQL batch.

Candidate SHA-256: `9bde3bfcc946e18600913e295a983998321f4fdfd9dd244bf5b28a28e04142b5`.
PostgreSQL: 17.6, isolated local container `oteryn-wp-pg-20261001`. All 30 present baseline migration files through 0031 applied in a unique lane-owned database. Candidate then installed locally in that isolated database solely for executable tests.

`semantic-results.json`: 161/161 actual cases pass. The independent seed and all positive controls must pass before negative evidence is accepted. Every case begins a fresh transaction and rolls it back; no history is reused between cases. Seed independently declares account/root revision 1, named Character, progression revision 1, level 1, XP 0, eight context revisions v1, Harmony/time 0. Receipts do not supply current root or progression authority. This intentionally minimal fixture qualifies the SQL persistence boundary; it does not simulate an authenticated bootstrap, current gameplay session, lease, content admission, Protection Zone or active-content N.

Covered executable boundaries:

- Header CHECKs for UUID, successor, cause, command/digest lengths, XP/level preservation, source context grammar and timestamp; Character FK; all eight context values bind to independent current progression.
- Composite line/header FK binds occurrence, Character, revision and cause. Source Item/definition grammar, same definition key, duplicate receipt/Item rejection and cause direction CHECKs execute.
- Arrays: NULL container, empty, >7, multidimensional, non-one lower bounds, invalid -1/3 values reject. One/seven all-NULL slots and actual choices 0/2 pass. Both line and current-row constraints execute.
- Header missing lines, perk selection with two lines, missing root/state successor, initial relation and future-root mismatch reject. Two training tracks commit consistently.
- First-line zero/unfilled seed, previous-line definition/revision/progress/selection continuity, row/line existence and exact latest row values/revision/occurrence execute. Old-header late append rejects.
- Immutable header/line UPDATE, DELETE, TRUNCATE (CASCADE included to reach the trigger), and current-row DELETE, TRUNCATE, rekey, definition-key change, decreasing progress and unexplained values reject.
- All seven inherited kinds XP/death/stance/Bestiary/charm/monk/build have valid successors, tip-context rejection and cross-kind duplicate-revision rejection. WP→each and each→WP pass. XP→WP continuity rejection and Harmony/forced-Serene preservation execute. i64::MAX progress passes; i64 overflow rejects.
- Real runtime role training, selection and first migration succeed with existing grants and revoked direct guard EXECUTE.

`concurrency-results.json`: two real connections race on the same current Character root. The second absolute successor waits for the first transaction's row lock; the first fully validates and commits. The second then rejects with SQLSTATE 23514 (`Character root permits only one exact revision successor`). Readback is root revision 2, progression revision 2, one header, one line and one current track.

No SQL defect was established by these cases. No authoritative candidate edit or patch is proposed. Some early harness construction attempts failed on baseline name/generated-column setup or SQL quoting; these were fixture failures, discarded before the independently valid seed passed. They are not counted as candidate evidence.

The Rust support in `character_proficiency_postgres_cases.rs` is a scratch candidate produced by the handwritten fixture helper `build_rust_support.py`, with the same manually designed scenarios. It is test source, not generated production data and does not receive a generated-data batch waiver. Both repository wrappers will need their approved path-loaded module only when the complete gate is ready. The standalone scratch Cargo harness carries copies of baseline migrations and the candidate solely to compile the proposed support and run its SQLx typed error assertions. Its local-only runner is separate from the canonical support's pinned admin URL and admitted-0032 precondition; it cannot qualify repository admission.

Actual Rust/sqlx replay: the support compiled against the repository-vendored SQLx/Tokio dependencies on Rust 1.94.0. Two separate scratch wrapper targets (`authority`, `progression`) each executed all 161 cases with typed PostgreSQL database error-code assertions, PASS (0.93s / 0.80s). This does not claim the canonical repository wrappers are wired or that migration 0032 is admitted. Log: `rust-harness.log`; summary/pins: `semantic-qualification.json`.
