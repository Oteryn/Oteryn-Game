# Run 8 Stag fixture isolation investigation

HEAD: 1fa04bbbddb5d96e21fecca0bc456f3c5e1109b7; local AUTHORING, no Git mutation.

Original red: `.questpc/r7-root-combined-pg.log` has the Stag read-only guard fail at shared `fresh_session.rs:39`, after boat-to-Ada None/2/3/13 succeeded; 10 other tests pass. Alone passes (`r7-root-stag-guard-alone.result.json`, 345.692s). Assertion hides the returned admission disposition.

Ranked hypotheses before repair:
1. Shared-PC publication latency ages independent admission observations beyond the existing five-second freshness boundary. Prediction: returned disposition is RejectedStaleAuthority and sample-to-commit age exceeds five seconds; independent case databases have no incumbent/membership.
2. Cross-case fixture identity or database collision. Prediction: RejectedCollision/RejectedIncumbent with an existing session, despite nanosecond-suffixed per-case databases.
3. Runtime readiness publication or bootstrap publication conflicts. Prediction: non-Applied publication dispositions or mismatched current guard records, regardless of elapsed source time.

Diagnostic change retains strict Committed assertion and prints only disposition and observation age on failure (no credentials/URLs). Combined reproduction uses QPC_SLOT=3 and only `quest_world_callbacks::literal_uses::` at two test threads.

First selected combined run (`r8-stag-combined-red.result.json`) reached both callback tests and four complete native callback cases, then both tests failed `RevisionMismatch("native gameplay input digest")` during concurrent content regeneration (0/2, 102.72s test execution). This is an invalidated content-generation snapshot, not the fresh-session failure; no production/content repair is made in this lane. Lead notified; repeat combined qualification after source/pins settle. The deterministic fixture-only regression does not load content and isolates the admission observation-age boundary.

Isolation trace: shared Harness names each database with a nanosecond suffix and uses `fence_parent(&database.name)`, so the older comment in first-vocation tests about tags sharing retained directories no longer describes the live harness. No directory/database collision repair is justified here.

Deterministic red proof: `r8-stag-freshness-red.log` returns **RejectedStaleAuthority**, sampled observation age **6s**, at the original strict Committed assertion; 0/1 in 2.82s (238.373s including queued rebuild). Runtime alone is permitted to be older: `validate_change` applies the five-second source-age bound to AccountSecurityAndPresence/FixedFreshSigningTrust, not RuntimeOwnershipAndReadiness. `fresh_source_deadline` fixes Platform security/signing eligibility to observation +5; no grant extension can fix that.

Repair: runtime readiness publishes first; only then sample DB time and construct independent current security/signing observations and signed admission request. Copy the exact previously published Runtime guard into the request's owning source; never re-age that evidence. Existing Committed/Active/generation=2 assertions remain strict. The regression asserts old runtime observation preserved exactly, fresh Platform observation, and unchanged five-second accepted deadline. No sleeps, skips, deadline changes or retries.

Shared fixture impact: `rg prepare_current_session\( apps crates tools` found 222 call/definition lines. Examined request-retaining consumers (first-vocation, busy-holder, wheel admission); signature, session identity, lease generation, bootstrap revisions and returned committed operation remain unchanged. No enum variant, struct field or trait method added.

Lead-requested coordination only: scenario.rs adds Father/legacy labels and delegates those two cases to npc_program::exercise_father before the unchanged ordinary catalogue-NPC branch. NPC worker notified. Literal callback completion marker now records each case duration.

Self-review against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md` and repository oteryn-review: read entire fixture diff and owned literal callback/scenario deltas. No production API, SQL mutation, schema, protocol, guard budget, session fencing, assertion or deadline was weakened. All source replacements use the exact previously published guard; successful admission still commits the normal claim transition and verifies current Active session/lease generation. No new public function/enum variant/struct field/trait method; constructor/match propagation is N/A. Existing literal callback real-wire assertions remain unchanged (only per-case elapsed evidence added). `rustfmt --edition 2024 fresh_session.rs` and owned-path `git diff --check` succeeded; package clippy and final PG evidence pending below.

First post-fix compilation (`r8-stag-freshness-green.result.json`, 204.846s) was invalidated by concurrent NPC/Viola authoring: missing FreshAdmissionAuthority import, moving policy out of Arc, and stale ReturnOffer dependency metadata. Owning workers notified and repaired their code. No fixture error was reported; this is **not** green execution evidence. Queued package clippy then caught NPC test referencing private durability::quest_state::QuestRefusal; NPC worker notified. Final combined PG waits for lead's stable generated-content/Rust snapshot signal.

The six-second regression isolates the stricter evidence deadline: donor fixture grants live for ten seconds, so the old request's credential is still valid for roughly four seconds at red commit while independently sourced Platform observations are already stale by one second. The repair acquires **new initial** Platform observations after runtime publication; it does not refresh or reinterpret previously published Platform evidence.

Lead announced generated content stable (register PASS 60.657s, model PASS 244.274s); launched exact combined three-test command in `r8-stag-combined-green.result.json`. Build stopped before tests at unrelated `tutorial_viola.rs:136` E0277 (`EquipmentError` cannot convert into `DurabilityError`), 34.725s. Viola worker and lead notified. Same filters will rerun after that author's repair; no admission, callback or fixture error was reported. Final package clippy belongs to lead's combined verification as requested, rather than duplicate queued builds.

Second combined build (`r8-stag-combined-green-repaired.log`) reaches lib-test compilation but new Viola test `postgres_tests/viola.rs:127` lacks the FreshAdmissionAuthority import needed for `npc_talk`. Author/lead notified. Production-only checks do not cover this import; wait for corrected lib-test/all-targets build before another own-mechanic PG retry. No fixture/callback compile error. Added scoped-six NPC scenario hook on NPC worker/lead request only after its export existed; owned NPC worker qualifies that branch.

Lead consolidated validation after simultaneous host SIGTERM/long cold builds: stopped own duplicate slot3 command on request (session exit130; retained `r8-stag-combined-final.log`). **SOURCE_STABLE**: fixture repair/regression, per-literal duration marker, and Father/legacy/scoped NPC hooks complete. No source writes pending. Lead's one warm slot6 command will run the two exact own filters (`quest_world_callbacks::literal_uses::`, `quest_world_callbacks_stag_fixture_refreshes_security_after_runtime_publication`) together with other mechanic filters, at two test threads. Expected own execution proof is 3 passing tests and 17 duration markers (16 callback cases, one deterministic fresh-session regression). Final PG/clippy proof remains with lead; no green claim from this worker yet.

## Actual central PostgreSQL execution checkpoint

Read and extracted `.questpc/r8-dispatch-central-pg-repaired.log` into **`.questpc/r8-stag-evidence.json`** (source-log SHA-256, exact command/result, exact raw markers, source line numbers, per-case durations, own passing test lines, preserved deterministic-red provenance). All three own tests passed together with catalogue and other mechanic tests under `--test-threads=2`:

- Fresh-session regression: 1 marker, **3.351517416s**.
- Literal nonzero-ordinal callbacks: 7 markers, summed case duration **325.333086956s**.
- Original read-only Stag guard test: 9 markers, summed case duration **373.038540310s**; all original None/below-minimum/minimum/completed stages and pre-startup crypt wall case completed.

Verified **17 exact POSTGRES_CASE_COMPLETED markers**, each with a positive duration: 16 real literal callback dispatch cases (12 allowed, 4 refused) plus the independent fresh-session regression. Both literal tests retained strict movement, expected chat, quest-state, receipt and revision assertions. The runtime guard remains the exact published observation; fresh initial Platform security/signing observations are sampled after runtime publication. The five-second accepted evidence deadline, Committed/Active/lease-generation assertions and all original timeouts are unchanged. No skips, sleeps, retries or assertion weakening. Deterministic red log remains intact: RejectedStaleAuthority at age6s, 0/1 in2.82s.

The **complete central batch remains red** (6 passed / 10 failed, 1077.92s execution;1140.391s command duration). This is passing evidence for the three own tests, not a claim that the whole batch or final generated artifact snapshot is qualified. Lead will rerun after new native artifacts and owns final shared clippy/fmt verification. No Cargo or source write occurred during this evidence-only extraction.
