# DUR-ONEOF-1 — insert-only audit oneof additions

```yaml
task_id: DUR-ONEOF-1
title: Reduce measured audit oneof merge contention without changing wire bytes
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-davinci-ijz3wy-duroneof
pr: null
base_sha: 1fcf3ded6
owner: Codex task worker (owner-allocated worktree)
created_at: 2026-10-24
updated_at: 2026-10-24
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/durability/src/durability/item_mint_audit.rs
  - crates/gameplay/durability/src/durability/item_mint_audit/
  - crates/gameplay/durability/src/durability/item_mint_audit.tags
  - crates/gameplay/durability/src/durability/item_fee_burn_audit.rs
  - crates/gameplay/durability/src/durability/item_fee_burn_audit.tags
  - docs/agents/tasks/DUR-ONEOF-1.md
public_contracts: []
```

## Task and authority

Owner-authorized local structural work only, under STRUCTURE-TARGET-1. No Git
mutation, publication, dependencies, migrations or schema changes. The bound
organization policy 3.1.0 was read at immutable META revision
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb` before material work.

## Investigation and decision

PROVEN: inspection at base `1fcf3ded6` used
`git log --since=30.days --format=%h -- <file>` and `git log -p`, supplemented by
`--follow` across the ADR-0019 move from `apps/game-server/src/durability/` on
2026-10-21 (`a1e58083a`). The reachable 30-day window reaches 2026-09-24 even
though the clone is shallow. A relocation is not counted as a mechanic addition.
Initial definitions and later extensions are counted separately.

| Module | Current-path commits | Commits following rename | Existing oneof | Initial variants | Extension commits / variants |
| --- | ---: | ---: | --- | ---: | ---: |
| `item_mint_audit.rs` | 5 | 21 | `OneItemOperationV1` | 1 | 12 / 12 |
| Same module | same | same | `CausationV1` | 4 | 0 / 0 |
| `item_fee_burn_audit.rs` | 5 | 10 | `FeeBurnCauseV1` | 1 | 3 / 4 |
| `reward_claim_mint_audit.rs` | 1 | 5 | none (uses operation tag 4) | 0 | 0 / 0 |
| `map_item_mint_audit.rs` | 1 | 4 | none (uses operation tag 8) | 0 | 0 / 0 |
| `bank_audit.rs` | 3 | 4 | `BankCauseV1` | 2 | 0 / 0 |
| `house_interior_audit.rs` | 1 | 2 | source / destination oneofs | 2 each | 0 / 0 |

The complete `prost::Oneof` / `prost(oneof` sweep under the durability directory
found these six oneofs and no others. Reward/map payloads and fee child modules
do not declare another oneof.

| Oneof | Extension commit | Lane | Added tags / variants |
| --- | --- | --- | --- |
| Operation | `3dcf3c82a` | B3-1 | 3 Transfer |
| Operation | `69d85f33a` | CHEST-1 | 4 RewardClaimMint |
| Operation | `3e3ed1aea` | D3-6 | 5 DecayRetire |
| Operation | `58429a26d` | GOLD-FEE-1a | 6 FeeBurn |
| Operation | `67fffa25c` | TIMED-RT-1b | 7 TimedExpiry |
| Operation | `98ca4aeaf` | MAP-OVERLAY-1b | 8 MapItemMint |
| Operation | `b4774770a` | NPC-TRADE-1 | 9 NpcTrade |
| Operation | `22a4736a2` | ITEM-USE-1a+1b | 10 ItemUse |
| Operation | `cecbfda32` | HOUSE-RUNTIME-1a | 11 HouseTransfer |
| Operation | `bcc55803a` | Forge | 13 ForgeDustLimit |
| Operation | `3eddd49b6` | TRADE-PLAYER-1 | 14 PlayerTrade |
| Operation | `d784aae73` | CRAFT-1 | 15 Craft |
| Fee cause | `ac3f7fbc6` | PREM-2b | 2 PromotionPurchase, 3 BlessingPurchase |
| Fee cause | `bc88d5914` | LOOP-MAP-1 | 4 AreaDonation |
| Fee cause | `90edf5154` | ITEM-TIMED-1 | 5 NpcRepair |

PROVEN contention: all 12 operation extensions and all 3 fee-cause extensions
replace the same respective comma-separated `tags` attribute. Operation tags
14 and 15 insert in the same enum position immediately before tag 13; the fee
tag 5 was prepended, producing order 5,1,2,3,4. The operation order at the base
was 2..11,14,15,13. Of the 12 operation extensions, 10 insert at the enum's
closing brace and 2 before `ForgeDustLimit`; of the 3 fee extensions, 2 insert
at the closing brace and 1 before `CharmUnassign`. The fee validation match in
`item_fee_burn_audit/purchase_cause.rs` was created by PREM-2b and extended by
both LOOP-MAP-1 and ITEM-TIMED-1 at the same position before the `CharmUnassign`
arm: 3 lane commits, including initial creation, and 2 shared-position inserts.
No repeated numeric tag was found: **0 tag-number collisions**. These patches do not prove
an actual Git conflict or simultaneous branch timing; those facts are UNKNOWN
from linear squash history. They do prove the owner's explicit threshold:
**12 and 3 distinct lane commits adding to the same oneof in 30 days**.

DERIVED decision: implement only the two qualifying oneofs. Sort and mark their
variant blocks and make the prost tag attributes one tag per line, removing the
shared-line replacement. Preserve every variant name, type, tag and field.
Leave the four non-qualifying oneofs structurally unchanged. Retain local raw
history evidence under ignored `target/DUR-ONEOF-1/`.

## Implementation

Both qualifying enums have sorted insertion markers. Native `tags(...)`
attributes put one integer/comment per line with trailing commas. Both schema
sites and the adjacent `.tags` golden accept inserted lines, including a new
highest tag. New variants also need wire fixtures. No generator or dependency.
Upstream prost 0.14.4 accepts the
native list through `Punctuated::parse_terminated` in its
[tag parser](https://github.com/tokio-rs/prost/blob/v0.14.4/prost-derive/src/field/mod.rs).

The unit guard checks the entire marked enum body, rejects variants outside the
markers, asserts uniqueness and strictly increasing order, compares both enum
and prost dispatch tags with the one-tag-per-line golden, and requires every
variant name/tag to have a fixture. The four other oneofs remain structurally
unchanged. Accepted wire authority remains the existing DUR-03 payload and
`docs/contracts/game-events/v1/{native_one_item_transaction,foundation}.proto`;
no contract or persistence behavior changes.

## Wire parity

PROVEN: the first three tests captured **22** nonempty encodings before either
production oneof changed. At capture, a source comparison with `HEAD` proved
that the only production-file difference was mounting the new test module.
The production source SHA-256 values were:

- Mint: `db64b654ef428accc6e97fbb81e404ab7f42d927b41dc865454db09d27ea9793`.
- Fee: `da8869d557f5cb8c102432889af78c4b9d0e8572b39fe659e32822684cb9f22b`.

The captured hex is fixed in `item_mint_audit/tests/oneof_wire_tests.rs` and
compared with the reordered code; every fixture also decodes to its original
value. These are codec fixtures rather than valid gameplay requests. Operations
encode through `OneItemTransactionV1` with interpretation revision 1, fee causes
through `OneItemFeeBurnCauseV1`, and causation through `CausationRefV1`.

| Variant | Tag | Captured wire hex | Golden bytes equal |
| --- | ---: | --- | --- |
| Operation::Mint | 2 | `080112060a0220072001` | yes |
| Operation::Transfer | 3 | `08011a040a022007` | yes |
| Operation::RewardClaimMint | 4 | `080122022001` | yes |
| Operation::DecayRetire | 5 | `08012a03308101` | yes |
| Operation::FeeBurn | 6 | `08013203108101` | yes |
| Operation::TimedExpiry | 7 | `08013a053081016000` | yes |
| Operation::MapItemMint | 8 | `080142052001288101` | yes |
| Operation::NpcTrade | 9 | `08014a054007488101` | yes |
| Operation::ItemUse | 10 | `080152054881016001` | yes |
| Operation::HouseTransfer | 11 | `08015a054881015007` | yes |
| Operation::ForgeDustLimit | 13 | `08016a06488101900107` | yes |
| Operation::PlayerTrade | 14 | `080172040a028107` | yes |
| Operation::Craft | 15 | `08017a040a028107` | yes |
| FeeCause::CharmUnassign | 1 | `0a0b0a05636861726d12028107` | yes |
| FeeCause::PromotionPurchase | 2 | `120f0a0970726f6d6f74696f6e12028107` | yes |
| FeeCause::BlessingPurchase | 3 | `1a0e0a08626c657373696e6712028107` | yes |
| FeeCause::AreaDonation | 4 | `220a0a02810710071a020809` | yes |
| FeeCause::NpcRepair | 5 | `2a070a028107308101` | yes |
| Causation::EventId | 1 | `0a028107` | yes |
| Causation::Command | 2 | `12070a028107108101` | yes |
| Causation::OperationId | 3 | `1a028107` | yes |
| Causation::TransactionId | 4 | `22028107` | yes |

## Validation

Every cargo invocation uses `/home/mole/oteryn-cache/codex-bin/cargo`, the pinned
1.99.1 toolchain, `--locked --offline`, and filtered PostgreSQL scope. Logs and
the exact expanded unit command remain under ignored `target/DUR-ONEOF-1/`.

- Baseline capture: `cargo test --locked --offline -p oteryn-gameplay-durability --lib --quiet -- oneof_wire_tests --nocapture`: PASS, 3 tests before production edits.
- Reordered wire and tag guards: `cargo test --locked --offline -p oteryn-gameplay-durability --lib --quiet -- oneof_wire_tests`: PASS, 5 tests / 22 byte fixtures.
- `cargo clippy --locked --offline -p oteryn-gameplay-durability --all-targets --quiet -- -D warnings`: PASS. An initial new-helper `chunks_exact_to_as_chunks` warning was repaired with `as_chunks::<2>()`; existing vendored Tokio/SQLx dependency diagnostics remain.
- PostgreSQL: `cargo test --locked --offline -p oteryn-game-server --test character_authority_postgres --quiet -- item_mint_postgres_cases::mint_commits_once_and_duplicate_cause_returns_identical_result --exact --nocapture`: PASS, 1 test / 582 filtered, 6.59 s, PostgreSQL `180006`. Checks the committed outbox against the frozen envelope and duplicate/replay result.
- PostgreSQL: `cargo test --locked --offline -p oteryn-game-server --test character_authority_postgres --quiet -- item_fee_burn_postgres_cases::a_fee_burns_whole_stacks_then_part_of_the_last_with_the_character_change --exact --nocapture`: PASS, 1 test / 582 filtered, 8.62 s. Neither PostgreSQL log contains a missing-URL early return.
- Governance: `python3 tools/agents/validate_governance.py`: PASS, 22 policy documents / 9 lanes.
- Governance tests: `python3 -m unittest discover -s tools/agents/tests`: PASS, 129 tests.
- Unit inventory: 440 library cases; exactly 25 use the PostgreSQL helper or admin URL. The remaining 415 are selected by positive full-name `--exact` filters, with no skip flags or unfiltered PostgreSQL run. `unit-filters.txt` stores the names and `unit-command.json` stores the exact argv.
- Full non-PostgreSQL unit run: PASS, 415 tests / 25 PG cases filtered, 22.11 s.
- `cargo fmt --all -- --check crates/gameplay/durability/src/durability/item_mint_audit/tests/oneof_wire_tests.rs`: PASS. Explicitly checks the new file under the existing `include!` mount as well as the workspace.
- `git diff --check`: PASS.

## Self-review

PASS: re-read the whole tracked diff and every added file against
`/home/mole/oteryn-cache/CODEX-CHECKLIST.md`. A structural comparison proves the
same 13 operation and 5 fee variant names, types and tags; identical wrapper
fields and tag lists; and byte-identical source everywhere else except the test
mount. Every fixed hex string equals the pre-change capture. Existing tests,
validators, SQL, locks, schema, protocol codes and runtime call sites are unchanged.

Workspace-wide `rg` over `apps/`, `crates/`, `tools/` enumerated 148 existing
oneof use sites (`workspace-oneof-sites.txt`). No enum variant, struct field,
trait method or public function is added, so no constructor/implementation or
dispatch API expansion is needed. The only new helpers are private unit-test
functions. No test is deleted, ignored or weakened.

External review: not selected under the bound risk-based policy for this
representation-only change. Publication review and CI belong to the coordinator.
No review trigger or repository upload was performed.

## Unfinished

None locally. Publication and full CI belong to the coordinating lane; no commit,
push, PR or frozen candidate was created. This card completes the authorized
local task, without claiming a protected-main lifecycle transition.
