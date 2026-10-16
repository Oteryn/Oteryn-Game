# WEATHER-1

```yaml
task_id: WEATHER-1
title: "WEATHER-1: deterministic visual model, preset map and candidate codec"
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/weather-1
issue: 1622
pr: 2202
base_sha: c9241ccddcddb886b82d3d18b454e4a3c7b6d3aa
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 1 WEATHER-1 (row 17); wave 2"
class: "impl"
migration: no
registry: yes
owner: WEATHER-1 worker (family G)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-16
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/world_weather.rs
  - apps/game-server/src/world_weather_tests.rs
  - crates/protocol-oteryn/src/world_weather.rs
slot_only_paths:
  - apps/game-server/src/lib.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/capabilities.rs
  - crates/protocol-oteryn/src/lib.rs
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
public_contracts: []
depends_on:
  - "WEATHER-SEAM-1: read-only World binding and canonical occurrence; required before offer"
  - "D1027: protocol-owner sign-off; candidate codec is not sign-off"
  - "ITEM-LOOK-1 #2182 -> WEATHER-1 -> SEC-1 registry order"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Source and accepted scope

Qualified Rust source candidate: `fd2700ea3b646493dc5b8391b655cea32238f125`,
on `skeleton/weather-1`, parent `e4eedab0440ea6f72751318c818a34d6d7306d98`.
The original model base is the `base_sha` above. This record-only successor
changes this card; all three Rust files remain byte-identical to the qualified
candidate. Its resulting record head is retained in the lead's commit receipt.
The `main` head observed before D1111 authoring was
`24fc332f256638e3b784bff1b3c85b18e2665a76`; it has not been merged into this branch.
Local evidence directories are not repository contracts or production paths.

CP [D1023–D1028](https://github.com/Oteryn/Oteryn-Game/pull/2202#issuecomment-6216591471)
authorize direct-slot selection and define binding/publication gates. Owner
[presets](https://github.com/Oteryn/Oteryn-Game/pull/2202#issuecomment-6216811445)
are rain, fog and snow. CP [D1111](https://github.com/Oteryn/Oteryn-Game/pull/2202#issuecomment-6221188957)
reviewed `e4eedab0`: **model OK, NOT READY**, and assigned the static preset map
to WEATHER-1 in its owned module, without a content file. D1027 still requires
protocol-owner sign-off. This authoring allocation does not grant it.

## Owned implementation

- Existing pure model: checked absolute elapsed /900000ms, exactly16 selector
  entries, `deterministic_decision_u64`, purpose `weather_visual_v1`, draw0;
  no clock read, default/clear preset, timer, history or mutable RNG.
- D1111 static map: index1 rain, index2 fog, index3 snow. Explicit authored
  selector weights are6/16,5/16,5/16. These are owned policy data, not a fallback
  for missing World binding or an asset/rendering admission. The World-config
  owner must explicitly bind the policy and its retained revision.
- Owned codec candidate: `crates/protocol-oteryn/src/world_weather.rs`, following
  existing strict protobuf helpers. Required status field1:1 Unavailable or2
  Active. Active additionally requires32-byte public revision token field2,
  absolute uint64 slot field3 (present even at0), and preset1..16 field4.
  Unavailable forbids Active fields. Maximum input64bytes; encoder at most49.
  Unknown/repeated fields, wrong types, mixed/missing fields, wrong token length,
  truncation, integer overflow and preset0/17 refuse. Exact field layout and
  token shape are **proposed for D1027 review**, with no schema/registry approval
  implied. No capability/domain/message IDs or client command are allocated.
- Authored tests cover the static map, native clock/light and independent SIM
  decision invariance, plus candidate codec bounds, shapes and malformed input.
  Native clock/SIM invariance is a component proof. Actual transport/gameplay
  consumer isolation is an integration gate; no dummy gameplay state proves it.

## Acceptance and remaining gates

- [x] Direct-slot arithmetic and SIM primitive accepted by D1023/D1024 and CP head review.
- [x] D1111 rain/fog/snow map and explicit selector authored in owned source.
- [x] Owned candidate codec and negative tests authored; no registration/sign-off claim.
- [x] Residue checked against observed main and exact CP ruling before this batch.
- [x] Actual native component compilation, strict lint, discovery and11 tests:6 server +5 protocol.
- [ ] Production head integration and normal all-target/package CI qualification after the caller slot.
- [ ] Protocol-owner D1027 sign-off, accepted schema and slot registry bindings.
- [ ] WEATHER-SEAM-1 actual World/private seed/clock/policy binding and canonical occurrence.
- [ ] Real caller plus server module declaration/capability/transport in one allocated slot.
- [ ] No binding means no offer; binding loss emits one full Unavailable only while current.
- [ ] A stale owner emits nothing; replay/restart/Channel moves never reseed or reroll.
- [ ] Live visual-only gameplay isolation and WEATHER-CLIENT-1 consumer/assets.

No standalone `lib.rs` declaration, lint allowance, path harness, fixture binding,
client/renderer edit, content file or slot wiring is part of this batch. Shared
registration follows ITEM-LOOK-1 and merge of main at the allocated slot.
SEC-1 is not a blocker. The owning transport must revalidate current owner and
session at publication; the model and codec cannot supply that authority.

## Validation

Independent source review of the codec, map and record: **SOURCE PASS**.
Actual native component qualification of `fd2700ea`: **11 passed,0 failed,0 ignored**
(6 server,5 protocol). The lead retained `qualification.json`, SHA256
`71890c4f2b23ae83f1c0edb2bc95593463f2d6f5ad2a4ca23fa6f26fd32c7ed6`.

The private full-source checkout used only two additional `#[cfg(test)]` module
declarations in the real server/protocol crate roots. `cargo fmt --all --check`
passed. Ordinary Cargo selected both actual native library-unit compilations:

`cargo test --offline --locked -p oteryn-game-server -p oteryn-protocol-oteryn --lib --no-run --message-format=json`

Both actual selected invocations used real Clippy1.94 with identical Cargo
arguments plus `-D warnings`; no synthesized test/cfg arguments, lint allowances
or path harness were introduced. Strict lint/codegen, fresh native ELF discovery,
`--list`, once-only source inventories and all11 literal `--exact` tests passed.
Owned formatting, whitespace and governance checks passed;124 governance tests
passed. This record-only successor reruns only cheap governance/whitespace and
verifies the exact three qualified Rust hashes, without repeating Rust.

This is **component qualification, not production head integration**. The
canonical crate roots still do not register Weather. Normal all-target Clippy,
whole-package runtime/duplicate-source CI guards, protocol-owner approval,
WEATHER-SEAM-1 and actual caller/transport/client behavior are not discharged by
the private component proof. No PostgreSQL invocation was needed for the11
selected component tests. No whole-baseline rerun or production offer is claimed.
The lead handles independent outcome review and publication preflight; this
record update performs no push or GitHub mutation. The feature remains NOT READY.

Later caller/slot integration retains the repository's normal package gates:

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings`
- `cargo clippy --locked -p oteryn-protocol-oteryn --all-targets --quiet -- -D warnings`
- `cargo test --locked -p oteryn-game-server --quiet`
- `cargo test --locked -p oteryn-protocol-oteryn --quiet`
- unchanged repository guards and required CI/protection


## Checkpoint

```yaml
last_progress: fd2700ea native components PASS11; record-only successor reuses three exact Rust hashes
status: in_progress
branch: skeleton/weather-1
blocker: D1027 sign-off, WEATHER-SEAM-1, production head integration and allocated real-caller slot
next_action: lead independent outcome review and publishing preflight; preserve NOT READY integration gates
```
