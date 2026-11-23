# Protocol/foundation test-value repair qualification

Baseline: `245ba7838e58318a2d851857b6c2b2d7c757aa09`, draft PR #3430,
task `OTV2-20261122-test-ci-value-audit`, issue #3429. These receipts qualify
the seven source changes whose SHA256 values are in
`qualification/qualification_analyser.json`. They do not qualify a later GitHub
head or assert whole-repository audit completion.

## Changes and independent oracles

| Path | Change | Boundary retained or strengthened |
| --- | --- | --- |
| `crates/identity/src/lib.rs` | Pin the complete verifier and S256 challenge for `[7_u8; 32]`; retain Debug redaction. | Challenge bytes, not merely their length. The literals were independently calculated with Python `hashlib.sha256` and URL-safe Base64. This is a fixed test vector, not a claim of an RFC-published vector. |
| `crates/protocol-oteryn/src/daily_boost.rs` | Replace the private test-only decoder with literal wire bytes. | Big-endian epoch `9`, one-byte UTF-8 name lengths and `rat`/`ferumbras` payload bytes, consistent with `daily-boost-v1.md`. The real UI decoder is a separate production implementation. |
| `crates/protocol-oteryn/src/guild_war.rs` | Require `LimitExceeded` for an oversized request. | An unrelated malformed tag no longer satisfies the size assertion. |
| `crates/protocol-oteryn/src/contract_registries.rs` | Compare every complete aggregate entry with the independently parsed fragment in numeric ordinal order. | A changed field in an otherwise correctly counted entry is detected; existing base, uniqueness and order checks remain. |
| `crates/protocol-oteryn/src/analyser_catalogue.rs` | Use 17 unique nonzero IDs for the 16-item limit. | Duplicate-ID rejection cannot conceal removal of the count guard. |
| `crates/protocol-oteryn/src/analyser_party.rs` | Use 2049 unique, otherwise valid prices for the 2048-price limit. | Duplicate-price rejection cannot conceal removal of the count guard. |
| `crates/gameplay/foundation/src/scheduled_events.rs` | Remove only `w05d_annual_calendar_retains_holiday_sources_for_production`. | The retained adjacent test directly checks the same holiday names on dates, half-open exclusions, invalid dates and year wrapping. Five behavioral tests remain in the module. |

All seven changes are confined to test code or a private `#[cfg(test)]` helper.
Production library behavior is unchanged. No remote publication or deployment
was performed by this lane.

## Rust execution

Execution used the authorized aarch64 VPS, pinned Rust/Cargo `1.100.0`, two Cargo
jobs, an isolated detached worktree and a separate target directory under
`/home/ubuntu/oteryn-work`. The original qualified worktree was not changed.
The initial offline attempt lacked the locked `uuid` version; ordinary
`--locked` dependency retrieval supplied the genuine dependency. No dependency
version or lockfile was substituted. Subsequent protocol probes ran offline.

| Run | Result |
| --- | --- |
| Original baseline: identity + protocol + gameplay-foundation, `--lib` | 1313 PASS: 7 + 911 + 395; zero failures or ignored tests. |
| Final seven-file candidate: same packages and scope | 1312 PASS: 7 + 911 + 394; zero failures or ignored tests. |
| Candidate Clippy for the same three packages, `--all-targets -- -D warnings` | PASS. |
| Initial five-file candidate `cargo fmt --all --check` | PASS. |
| Final seven changed files, pinned `rustfmt --check` | PASS. |
| `git diff --check` | PASS. |

Vendored Tokio emitted pre-existing deprecation/missing-doc warnings during
tests. Clippy returned zero for the requested package scope. The count decreases
by exactly one because the proved-subsumed Debug smoke was removed.

## Negative experiments

The same mutation was applied to baseline and candidate in the disposable
worktree. Every source was restored before final qualification. The scripts
record the exact commands, exit status, elapsed time, test summaries and final
file hashes. A Rust compilation error is not accepted as a detected mutation:
negative unit probes require exit 101 and a `test result: FAILED` summary.

| Mutation | Baseline result | Candidate result |
| --- | --- | --- |
| Replace PKCE challenge with equally long `A` characters | 5 PASS | 4 PASS, 1 FAIL at fixed challenge equality |
| Change daily boost encoder and old test decoder from BE to LE | 1 PASS | 1 FAIL at literal byte equality |
| Remove guild-war request length guard | 1 PASS | 1 FAIL: `Malformed` differs from `LimitExceeded` |
| Add an extra field to the aggregate supplied to the registry helper | 3 PASS | 1 PASS, 2 FAIL at full fragment equality |
| Remove analyser catalogue item count guard | 1 PASS | 1 FAIL on the unique oversized request |
| Remove analyser party price count guard | 2 PASS | 1 PASS, 1 FAIL on the unique oversized request |

The registry experiment is explicitly an **oracle-input mutation**. It proves
that the stronger assertion catches field corruption which the old helper
missed; it is not a claim that a production builder mutant was executed.

`mutation_probe.py` and `analyser_mutation_probe.py` reproduce the experiments
from the named baseline when supplied the respective candidate source maps.
`qualification/` contains actual result JSON and logs. The registry failure's
two large JSON operands are omitted from the checked-in excerpt; that excerpt
records the unchanged original log's full remote path, byte count and SHA256.

## Semantic audit boundary

`semantic_review.json` records only manually read test bodies and the stated
implementation/caller scope, with per-file oracle, disposition, limitations and
source hashes. `inventory.json` is a mechanical routing inventory and is never
counted as semantic review. Other protocol, session and foundation files remain
open until an explicit semantic disposition is recorded. A passing package run
does not replace that review.

## Additional appearance limit repair

After the seven-file run, full review of the appearance tests and their five
production modules found the same masked limit in
`crates/protocol-oteryn/src/appearance_tests/mappings_tests.rs`: 257 identical
mount names still fail the ordered/unique-name rule if the count limit is
removed. The fixture now proves that 256 sorted unique names encode/decode and
that adding the 257th returns exactly `AppearanceError::LimitExceeded`.

`appearance_mutation_probe.py` removed only the production mount-count guard
inside the disposable checkout. Baseline: 1 PASS. Candidate: 1 FAIL at the
oversize assertion. After restoring production, all 911 protocol library tests,
protocol Clippy with all targets and denied warnings, changed-file formatting
and diff checking passed. `appearance_qualification/results.json` and its logs
record this additional run and exact candidate/restored-production hashes.
The checked-in production catalogue file has no changes.
