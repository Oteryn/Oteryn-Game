# Validation and evidence limits

Base: `e2e5ced306040113dc9c395251a802f50eb5776a`. This record separates executed checks from source inspection and proposed runtime reproduction. The exact published documentation head and post-publication readback are recorded in [#3088](https://github.com/Oteryn/Oteryn-Game/issues/3088).

## Executed checks

| Check | Actual result | What it establishes |
|---|---|---|
| `cargo metadata --no-deps --offline --locked --format-version 1` using installed Rust/Cargo 1.99.1 | Exit 0; 67 workspace packages and 268 targets | Actual package/target/dependency inventory, no compilation |
| `git ls-tree -r` and inventory partition assertion | 27,776 tracked files; all assigned once to 67 packages plus 11 supplemental domains | Complete baseline inventory, not behavioral coverage |
| Independent reconstruction of coverage using exact Git blobs and line-position sets | 196 unique paths; 96 full / 100 partial; 40,516 union lines; no remaining discrepancy | Correct deduplication, range bounds, package counts and honest statuses; does not repeat every substantive review |
| Std-only source-extracted listener poll reproduction | PASS; `queued=1 accept_polls=1 follow_up_wakes=0` at the selected released-slot boundary | SEC-002 isolated liveness defect; boundary stubs replace socket/TLS/runtime; production cadence is not modeled |
| Isolated pure house/item Rust harness, including independent rebuild | Exit 0; 300 bid escrow scenarios plus notice, auction tie, quick-loot and ammunition checks passed | Only the selected pure calculations/planners; not SQL persistence or any DUR finding reproduction |
| `python tools/agents/validate_governance.py` on documentation candidate | Exit 0; 22 required policy documents and 9 project lanes validated | Documentation/governance consistency |
| `python -m unittest discover -s tools/agents/tests` | Exit 0; 129 tests passed | Repository governance tooling only, not game tests |
| Report JSON/path/range/count validation and documentation-only delta inspection | Required before publication; exact results recorded on the coordination issue | Report integrity and absence of game-code changes |

The pure harness's actual output was:

```text
PASS: 300 bid escrow scenarios (funding, conservation, release, winner burn); notice bounds; max-price auction tie; quick-loot selection/exclusivity cap; ammo underflow.
```

It directly imports the audited `house_move_out.rs` and `quick_loot.rs`. Three house-auction source copies use a scratch module layout and were byte-compared against exact baseline blobs. No copied game source was modified. The harness does not reproduce DUR-001, DUR-002 or DUR-003.

The listener reproduction contains the original poll closure with deterministic standard-library boundary stubs. Its source-to-harness body equality was explicitly checked after trimming only surrounding whitespace. The initial extraction-hash metadata was corrected by the lead; the old unverified hash is retained as provenance only in the private record. Actual normalized source/harness closure SHA-256: `6ec9fec3bd685c7f8e750f443cf6409385fea873b28de3018b6c7139fd5b0216`.

An initial documentation whitespace check found final blank lines in two generated Markdown reports; those document-only formatting issues were removed and the check was repeated. They were not game-source test failures.

## Latest-main freshness

The final prepublication read observed main at `4f0b6c62a789489fca6c37dd36d5917d096f328e`. Selected changed guards, six new migrations and primary finding blob identities were checked; see [the reconciliation](latest-main-reconciliation.md). This adds no baseline-coverage credit and no new runtime test result.

## Independent P1 checks

| Finding | Primary analysis | Independent verification | Limits |
|---|---|---|---|
| DUR-003 | Durability/economy auditor | Gameplay auditor; lead reread core chain | Static runtime route and SQL predicates; no PostgreSQL execution |
| SEC-001 | Session/protocol observation and lead reread | Gameplay auditor | Enabled supported configuration; live configuration and TLS load scenario unknown |
| GP-001 | Gameplay auditor | Durability/economy auditor; lead reread gates | Real qualification/caller/gate proof; native cast fixture not executed |

The three small infrastructure packages also received a complete independent source reread, including their embedded tests and selected dependency/caller counterchecks. See [foundation-independent.md](foundation-independent.md). They remain REVIEWED, not a production/runtime sign-off.

## Explicitly not executed

- Full Rust workspace build, Clippy or test suite.
- PostgreSQL provisioning, full migrations, integration or system suites.
- Real network/TLS admission, cast-service saturation or native public-projection delivery.
- Live deployment, production account/data or configuration inspection.
- Real crash/restart, ack-loss and controlled concurrent transaction schedules.
- A client-side native RNG prediction demonstration.

The installed Rust binaries are available via `/home/agent/.cargo/bin`; the standard Cargo registry source cache is absent. No dependency installation or heavy build was started. Existing native/SQL tests were read only where listed in the source ledger. Their presence is not a passing test result.

## Outstanding targeted reproductions

1. DUR-003: supported nested inventory -> committed death -> failed root drop -> retained pending respawn -> fresh/reconnect refusal, with a flat control and departed-owner recovery variant.
2. SEC-001: private isolated mixed observer/admission capacity scenario, with actual service configuration and all normal authorization limits.
3. GP-001: valid self/native/conjure dispatch with effective loyalty/proficiency bonus, zero-bonus control and unchanged raw binding.
4. SEC-003: valid V2 snapshots across encoder/body/envelope boundaries through the real mTLS test sink and durable acknowledgement path.
5. DUR-001: private whole-tree custody/capacity fixture and unchanged conservation assertions.
6. SEC-002: actual listener scheduling with and without outer periodic wake sources.
7. DUR-002: controlled PostgreSQL lock interleaving and abort/no-corruption assertions.
8. RNG-001: establish exact pre-command server-seed visibility before attempting one stable native prediction comparison.

These are proposed checks, not completed test results. No finding severity relies on an invented successful runtime execution.
