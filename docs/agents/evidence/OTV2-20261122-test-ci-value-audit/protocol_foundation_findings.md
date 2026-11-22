# Protocol, session and foundation test-value audit — semantic wave 2

## Evidence basis and scope

Baseline: `8e9f0e16631aab1beb8416f8d5ca707aa3031083` (`cef077886adb38c7b86bdb5d56b877a8833e7b1e` tree). Repository root: `<baseline-checkout>`. The parent was editing unrelated files during this wave; the scoped scanner verified **zero modified source blobs** in this lane against the baseline.

Scope: `crates/protocol-oteryn`, `crates/session`, `crates/session-tcp`, `crates/identity`, `crates/foundation`, `crates/gameplay/foundation`, `crates/simulation-determinism`, and `tests/security/auth`. Related production consumers and runner helpers were read only to resolve test ownership and oracle independence. Applicable `crates/simulation-determinism/AGENTS.md` keeps those primitives protocol/persistence/UI neutral; no proposal moves session or security authority into that crate.

PROVEN means source/command output directly inspected. DERIVED means an inference from that source. UNKNOWN marks absent execution or incomplete mapping. RECOMMENDATION is a proposed change. **No Rust compilation, Rust execution, real PostgreSQL journey, or game process was run in this wave.**

| Scope | Rust sources read lexically | Literal baseline test attributes | Extracted ordinary function bodies |
| --- | ---: | ---: | ---: |
| protocol-oteryn | 347 | 920 | 920 |
| session | 188 | 302 | 301 |
| session-tcp | 3 | 3 | 3 |
| identity | 1 | 7 | 7 |
| foundation | 2 | 3 | 3 |
| gameplay/foundation | 54 | 395 | 395 |
| simulation-determinism | 1 | 7 | 7 |
| security/auth | 7 | 13 | 13 |
| **Total** | **603** | **1,650** | **1,649** |

The one unmatched template is `crates/session/src/tests/status_projection_tests.rs:50`: `#[test] fn $name()` inside `status_exchange!`, instantiated six times. This is a concrete reason source counts are not runtime case counts. The scanner does not expand Rust macros or evaluate Cargo targets/features.

The manual index contains **119 bounded scope rows across 55 paths**, including production/caller traces and partial helper reviews. It does not represent 119 fully executed tests or 55 whole files semantically completed. Of the 408 test-bearing files in these eight inventory groups, 370 had no manual row in this wave; the other 38 remain partial except for the exact scopes explicitly listed.

Artifacts, all under `./`:

- `protocol_foundation_reviewed_scopes.tsv`: exact test names, helpers or caller scopes, dispositions and unreviewed remainder.
- `protocol_foundation_remaining.tsv`: all 408 test-bearing files; unvisited files remain `UNREVIEWED_STATIC_SCAN`.
- `scan_protocol_oracles.py`: reproducible lexical scanner, not a Rust parser or test runner.
- `protocol_foundation_case_inventory.tsv` and `protocol_foundation_case_inventory_summary.json`: 1,649 extracted bodies, candidate hints, two repeated-body groups, and the known macro exception.
- `protocol_foundation_test_proposal.patch`: four-file test-only proposal, **not applied**.
- `protocol_foundation_oracle_evidence.json`: independently computed PKCE/vector facts and proposal limitations.

## Findings and proposed dispositions

### PF-01 — The PKCE “deterministic” test does not verify the challenge (PROVEN weak oracle)

`crates/identity/src/lib.rs:375`, `pkce_is_deterministic_and_redacted`, calls `PkceMaterial::from_entropy(&[7; 32])` and checks only `challenge().len() >= 43` plus the verifier's redacted Debug text. The other four PKCE tests check entropy rejection and verifier lengths. None of those assertions fixes the challenge bytes or the verifier bytes for a known input.

A wrong constant of 43 characters satisfies this case's challenge predicate. That is a **DERIVED counterexample to the assertion**, not an executed Rust mutation result. The redaction assertion remains useful and should stay.

The current shipping caller, `apps/client/src/native_entry/authorize.rs:13-30`, obtains `PkceMaterial` from secure entropy and passes its challenge to the authorization URL. The real portal journey in `tests/security/auth/tests/web_identity_pkce_isolation.rs` validates mismatch refusal and token/code isolation, but its `support/mod.rs:310` helper constructs the challenge independently using Base64url/SHA-256. It therefore does not, by itself, qualify the shipping `PkceMaterial::from_entropy` generator. This is not a claim that all client login tests were exhaustively mapped.

**RECOMMENDATION:** strengthen the existing test with a fixed verifier/challenge vector and retain redaction; do not delete identity or portal tests. Proposed rename: `pkce_matches_fixed_s256_vector_and_redacts_verifier`.

For 32 input bytes equal to `0x07`, Python standard-library Base64url without padding and SHA-256 independently produced:

```text
verifier  BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc
challenge 3Ev4DHdHPRMPoN6GukAY_pi7IUAF5qWJHRK6kURvnoE
```

These are synthetic test values. The evidence JSON records how they were calculated. They are not presented as an externally published RFC vector. Rust agreement is still UNKNOWN pending the focused check.

A smaller naming issue exists at `identity/src/lib.rs:465`: `callback_requires_exact_state` only parses and compares the returned state text. The real mismatch refusal is tested by `loopback_redirect_returns_the_code_for_the_exact_state` and `shipping_callback_rejects_mismatched_state`. Rename the parser case to describe percent-decoding/field retention if touched; do not remove the real mismatch cases.

### PF-02 — Daily boost roundtrip uses a decoder that ships only in tests (PROVEN)

`crates/protocol-oteryn/src/daily_boost.rs:38` defines a complete private `decode_view` under `#[cfg(test)]`. The sole `query_and_view_round_trip` case at line 70 encodes a view and decodes it through that test-only implementation.

The production producer is `crates/gameplay/dispatch/src/gameplay_transport/daily_boost.rs:172` calling `wire::encode_view`. The shipping client decoder is different code: `crates/client-ui/src/features/0151-daily-boost/protocol.rs:31`, called by `apply_result`; the app forwards to it through `apps/client/src/features/0151-daily-boost/protocol.rs`. Consequently the protocol roundtrip does not exercise the consumer that interprets the result in the product.

There is valuable existing client coverage: `controls_encode_reply_and_refusal` builds a literal payload and applies it to the real panel; the app's daily-boost play tests inspect the resulting creature and epoch. Those should remain. The reviewed subset does not establish complete producer-to-client black-box coverage.

**RECOMMENDATION:** replace the protocol test-only decoder with a literal documented wire vector in the existing case. The contract is explicit in `docs/contracts/protocol-oteryn/v1/daily-boost-v1.md#view`: u64 big endian epoch, then two one-byte lengths and nonempty UTF-8 keys. For epoch 9, `rat`, `ferumbras`, the proposed expected payload is 22 bytes. The patch removes the private decoder copy and preserves query acceptance/refusal checks.

This reduces duplicated test implementation while improving independence. It does not change the production client decoder, protocol or runtime behavior. Future consumer integration checks should call the real public result application boundary rather than recreating another parser.

### PF-03 — Guild-war oversize assertion also passes on an unrelated malformed-tag failure (PROVEN structure; mutant survival DERIVED)

`crates/protocol-oteryn/src/guild_war.rs:122`, `round_trip`, includes:

```rust
assert!(decode_guild_war_intent(&[0; 65]).is_err());
```

The production decoder first rejects lengths above 64 with `CyclopediaWireError::LimitExceeded`. If that guard were removed, the same input's first zero tag would reach the default match arm and return `Malformed`. The current assertion accepts either result. Thus it cannot distinguish the intended early resource bound from an unrelated parse error.

**RECOMMENDATION:** retain the two meaningful intent roundtrips and change this existing assertion to the exact expected `Err(CyclopediaWireError::LimitExceeded)`. This is in the scratch proposal. No new test count is required. Broader request/result byte vectors and invalid UUID/tag cases remain a possible future coverage improvement, not a requirement to add dozens of mirroring cases now.

Current consumers exist in dispatch `gameplay_transport/guild_war.rs` and the client `features/0103-guild-war/protocol.rs`; this is a test-oracle weakness in a current contract, not dead feature code.

### PF-04 — Registry aggregation checks structural shape, not “every fragment and nothing else” (PROVEN scope mismatch)

`crates/protocol-oteryn/src/contract_registries.rs:39`, helper `check`, is described as checking the aggregate is the base plus every fragment. The two callers are `protocol_registry_is_its_base_and_fragments` and `resource_limits_registry_is_its_base_and_fragments`.

The helper compares top-level keys, non-collection base values, collection length against the number of `.json` files, uniqueness of IDs, and protocol ID ordering. It does **not** read and compare each fragment's content to the corresponding aggregate entry. A changed non-ID payload field that preserves counts, keys and ordering is invisible to those two assertions. This is a DERIVED assertion counterexample, not proof that the current builder emits bad content.

The actual builder `crates/protocol-oteryn/build.rs:196-282` reads fragment bodies and splices them into the output. Python `tools/contracts/tests/test_registry_fragments.py` checks its own split/assemble implementation against committed trees. Rust `capability_fragments_tests.rs` separately checks generated capability facts, fixed golden fragments and many invalid registrations. These provide real adjacent protection; none should be discarded. They also do not make the exact claim in this helper universally true for every field of every aggregated registry entry.

**RECOMMENDATION:** either narrow the test/helper name and comment to structural integrity or strengthen the existing helper by parsing sorted fragment JSON and comparing complete entries to the built aggregate. Derive expectations directly from source fragment files rather than invoking the same Rust `assemble` implementation for expected and actual. No patch is included for this item because the ordering and relevant build validation should be resolved with the registry owner first.

### PF-05 — Debug-string calendar smoke is subsumed by adjacent observable behavior (PROVEN assertions; retirement recommended)

`crates/gameplay/foundation/src/scheduled_events.rs:382`, `w05d_annual_calendar_retains_holiday_sources_for_production`, parses `SCHEDULE` and checks that Debug output contains `halloween` and `piece_of_cake`.

The next case, `w05d_annual_windows_are_half_open_and_wrap_across_years`, parses the same schedule and directly asserts the two event names at their exact calendar boundaries, plus absence outside the windows and year-wrap behavior. That is stronger evidence for the production behavior represented by the smoke case. A harmless Debug-format change could fail the first case without breaking the schedule consumer.

Current consumers call `Schedule::seasonal_at`, including `crates/gameplay/durability/src/durability/world_event.rs:42-43`, and production rate callers use `current_rates`; no consumer dependence on the inspected Debug string was found in the bounded search.

**RECOMMENDATION:** remove this one Debug-only case while keeping the adjacent boundary and arithmetic tests. The patch proposes that removal. This is a small maintenance improvement; no material runtime saving is claimed. Name searches in available `.github`, tools, docs, apps, crates and tests found no external filter/registry references to this case.

### PF-06 — “Accepts only semantic IDs” is a positive constructor smoke, not an exclusivity test (PROVEN; exact redundancy UNKNOWN)

`crates/gameplay/foundation/src/foundation/admission_tests/tests_reconnect_history.rs:360`, `fresh_admission_accepts_only_semantic_foundation_ids`, decodes one valid UUID into three typed IDs, calls `FreshAdmissionFacts::new(...)`, propagates errors, and returns success. It contains no rejection or type-negative assertion.

The production signature in `foundation/admission.rs:141` enforces the typed arguments, and `new` also rejects zero nonce/lease/scope. Numerous stronger admission tests traverse that constructor through `facts -> for_test -> new` in the fixture helpers. However this smoke uses the same raw UUID in three namespaces, whereas common fixtures use distinct values. Exact behavioral redundancy is therefore **not proven**.

**RECOMMENDATION:** clarify the intended contract. If it is successful construction from typed values, rename the case. If it is compile-time rejection of raw/mixed ID types, that requires an appropriate compile-fail/public type boundary check, not this runtime success case. Avoid creating such infrastructure just to protect an obvious signature without a demonstrated regression risk. No removal or patch is proposed for this item.

### PF-07 — Synthetic/captured/session/topology execution modes must remain visible (PROVEN distinction, not a blanket defect)

`crates/session/src/tests/e2e_c01.rs` has two input modes. With `E2E_ALL_CAPTURE_DIR`, `payloads()` loads server capture bytes and an expected text file. Without it, the test constructs payloads using the shipping codec and an in-process duplex peer. In both modes it exercises the real Session state application, then malformed-delta rejection with unchanged state. It emits no own `e2e_all PASS` marker.

The synthetic mode is useful session-contract coverage. Its ordinary Cargo PASS is not evidence that a real server produced or replayed captured bytes. The caller must prove the capture environment and artifact chain when it credits an end-to-end row.

In `tests/security/auth`, the four real journeys return success early when `GAME_PLATFORM_INPUT` is absent: one directly in `web_identity_pkce_isolation`, three through `support::run`. The PG observer helper additionally returns early without `OTERYN_TEST_POSTGRES_ADMIN_URL`. These source paths do not exercise the named topology in ordinary environment-free crate runs.

The canonical qualification caller is stronger: `tools/qualification/game_platform/lifecycle.sh:155-218` compiles the targets, supplies the topology environment, selects exact cases and requires exact terminal evidence markers. Missing input cannot silently earn a successful real journey through that caller. No failing live qualification was observed in this lane.

**RECOMMENDATION:** keep those real journeys and separate counts for ordinary unit assertions, captured replay and topology execution. An explicit ignored/feature-qualified lane or richer skip metadata may improve reporting, but changing such selection requires updating the canonical caller together. Do not disable them or call their verified runner “fake green” based only on the early-return source.

## False positives that were resolved

### Two exact-text groups are seven distinct length-reader checks

The scanner found two repeated-body groups consisting of `f26_borrowed_length_bounds` wrappers. Five pass `super::read_bytes` and two pass `super::bytes`. Each resolves to a different local production reader:

| Module | Production reader definition |
| --- | --- |
| account_achievements.rs | `read_bytes` at 177 |
| actor_spell.rs | `read_bytes` at 260 |
| charm_wire.rs | `read_bytes` at 68 |
| world_object.rs | `read_bytes` at 247 |
| world_spatial.rs | `read_bytes` at 162 |
| actor_spell_item_v2.rs | `bytes` at 56 |
| actor_spell_v2.rs | `bytes` at 41 |

The shared helper in `count_bounds_tests.rs` verifies accepted slice length/pointer/cursor and rejection before body traversal for huge declared lengths and small backing buffers. Identical wrapper text is correct parameterization here. **Keep all seven bindings.**

### “No direct assert” is not “no oracle”

The scanner flagged 132 ordinary test bodies with no literal `assert!` family macro. Many call `codec`, an asserting helper, or return a checked error. Examples manually resolved:

- `session_tests_push::a_base_mismatch_an_unselected_domain_and_an_unregistered_type_poison_the_session` calls `assert_idle_push_poisons`, which verifies the exact error class and subsequent `SessionUnusable` behavior.
- The two `admission_authority_publication` security-revision wrappers call a helper that changes different provenance facts and expects `Stale`.
- `decoder_fuzz_support::codec` checks generated roundtrips and fixed seed validity/garbage refusal; `property::fuzz_decoder` deliberately checks absence of panic. Truncation iteration alone is a robustness probe, not an assertion that every prefix must fail.
- `tests/security/auth::check` returns an error when its predicate fails. Those cases need not use an assertion macro to fail correctly.

Unreviewed members of the 132 remain candidates. They were not all semantically cleared.

### Test-only wrapper versus copied test-only implementation

`protocol-oteryn/src/item_view.rs:443` has a `#[cfg(test)] decode_entry` wrapper that directly calls the production `decode_entry_for(input, false)`. That wrapper is not equivalent to PF-02's separate parser implementation. Do not flag every `cfg(test)` entrypoint as a fake consumer.

## Protected coverage and public boundaries

| Area and representative reviewed tests | Why retained |
| --- | --- |
| Protocol `seam_server_encoders_match_independent_proto_vectors` | Literal byte vectors for Accepted, Resumed and Error; expectations do not call the encoder under test |
| Protocol `envelope_tests` and selected `state_sync_tests` | Exact error types, pre-admission generation, direction, duplicate/wrong-wire fields, malformed lengths and identifiers |
| F26 count/length tests | Adversarial declared lengths, actual backed lengths, cursor/pointer invariants, allocation/iteration instrumentation and supported bounds; a generic happy path cannot replace these |
| Session recovery's three reviewed cases | Changed session ID/generation/sequence/capability selection, join target mismatch, unacknowledged command identity and precise server refusal remain observable at the public byte-stream boundary |
| session-tcp's three tests | Actual local TLS sockets, named certificate refusal and actual socket nodelay; not mock configuration-only assertions |
| identity's entropy bounds and actual loopback case | Current min/max material constraints, HTTP callback path, exact state mismatch and listener shutdown |
| foundation's bounded text, cancellation flag and ManualClock deadline | Cheap current primitive outcomes; the synchronous cancellation flag case alone does not qualify pending async wakeups |
| simulation-determinism's seven tests | Fixed decision/hash vectors, order independence, purpose/draw separation, named rounding, overflow, semantic time and duplicate key rejection |
| gameplay foundation FND04 parser/security cases | Stale security revision, extreme numeric dates without panic, exact compact JWS structure, noncanonical base64url, invalid UTF-8 and duplicate JSON members |
| gameplay snapshot facade's three tests | Calls public `SnapshotBarrier` begin/chunk/commit and verifies duplicate domains and additive unknown fields at the assembly boundary |

This wave found targeted weak assertions and one small retirement candidate. It did not establish that these security, codec, arithmetic or fencing suites should be replaced wholesale by process black-box tests.

## Reviewable proposal and verification status

The scratch patch changes only four test areas:

1. Strengthen the existing PKCE case with literal independently calculated verifier/challenge values, retaining redaction.
2. Replace the daily-boost test-only decoder with the documented literal payload oracle in its existing case.
3. Require the precise guild-war length-limit error.
4. Remove the one Debug-string holiday smoke, retaining the adjacent behavior cases.

Net test count: one case removed; three existing cases strengthened; no additional test cases. `git apply --check protocol_foundation_test_proposal.patch` returned exit 0. The PKCE literal values and 22-byte daily-boost payload were independently calculated/checked with Python's standard library. Those checks do not compile or execute the proposed Rust code.

**UNKNOWN before landing:** Rust compilation/lint/format and actual relevant target execution. The parent should run the repository-required focused gates for `oteryn-identity`, the affected `oteryn-protocol-oteryn` cases, and the `scheduled_events::tests` subset in `oteryn-gameplay-foundation`. Preserve current security review requirements if the proposal is adopted. Do not claim the Rust patch is tested because it applies cleanly.

## Remaining work for the next wave

The remaining TSV is the authoritative file queue; partial rows do not mark their other tests complete.

1. Protocol feature modules beyond the named scopes: compare frozen wire/spec oracles, max-bound errors and malformed permutations with existing client/server tests. Prioritize modules whose only local coverage is one roundtrip, without treating all roundtrips as useless.
2. Session's many capture-replay modules: map environment mode, real input producer, captured expected output and caller evidence. Keep public Session state/refusal/replay tests; identify cases with only synthetic fixture interpretation before attributing end-to-end coverage.
3. Gameplay foundation: continue per-case current caller and unique failure-mode mapping through authority publication, reconnect bridge, condition state and owner timers. The current reviewed security/parser sample is not 395 completed case reviews.
4. Identity/foundation boundary gaps: determine existing coverage of pending cancellation wakeups, time overflow/backwards movement and state/parser failure cases across current callers before adding tests.
5. Security auth: the admitted journey body and device rotation/blocker helper cases remain largely unreviewed in this wave. Trace assertions to real service state and preserve the strict canonical evidence caller.
6. Actual execution economics: collect target/config/shard-specific case lists and durations in the qualified runner. No speedup, mutation score or full runtime coverage percentage can be inferred from this lexical scan.

No repository files were modified by this lane, no external changes were published, and no tests or gates were disabled. The scratch proposal is explicitly separate from verified implementations already owned by the parent.
