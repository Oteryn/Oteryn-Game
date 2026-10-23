# WEATHER-1

```yaml
task_id: WEATHER-1
title: "WEATHER-1: visual model, candidate codec and partial clock read seam"
mode: IMPLEMENT
status: completed
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
  - crates/gameplay/world/src/world_clock.rs
  - apps/game-server/src/lifted_tests/world_clock_world_config_tests.rs
  - crates/gameplay/weather/src/lib.rs
  - crates/gameplay/weather/src/tests.rs
  - crates/protocol-oteryn/src/world_weather.rs
  - docs/contracts/protocol-oteryn/v1/world_weather_v1.proto
  - apps/game-server/src/lib.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection.rs
  - crates/gameplay/dispatch/src/gameplay_transport/capabilities.rs
  - crates/gameplay/dispatch/src/gameplay_transport/capabilities_tests.rs
  - crates/protocol-oteryn/src/lib.rs
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - content/weather/**
  - tools/content-schema/weather-authoring/**
slot_only_paths: []
public_contracts: []
depends_on:
  - "D1190: owned World binding plus canonical occurrence/A2 integration required before offer"
  - "D1027: protocol-owner sign-off; candidate codec is not sign-off"
  - "ITEM-LOOK-1 #2182 -> WEATHER-1 -> SEC-1 registry order"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **DONE**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: [#2202](https://github.com/Oteryn/Oteryn-Game/pull/2202). Data-bound visual weather offer, admission snapshot/runtime stream and fail-closed binding loss are merged; rendering is the separate WEATHER-CLIENT-1 follow-up. Evidence: `crates/gameplay/dispatch/src/gameplay_transport/capabilities.rs`.

Tracking closeout: delivered on main; no active implementation ownership is retained by this historical card.

## D1190 resolved World-config source — owned leaf and private A2 proposal

[CP D1190](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6225612898)
resolves the previously missing production source: `[world]` contains seed_file,
clock_identity and whole-hour clock_epoch_utc_ms; the existing
`readiness.world_policy_revision` is the single policy source. The earlier
source-gap request below is historical and superseded, not an open decision.

Owned `world_clock.rs` now retains a non-Clone, redacted-Debug World-bound
`RetainedWorldConfig`. Its only constructor loads the actual file through
`secure_file::read_checked(FileClass::Secret)` against the process UID, with a
32-byte bound and exact length. There is no raw seed/default constructor. The
read accessor returns borrowed seed/identity/policy plus the retained clock only
for the bound WorldId. These are immutable input data, not current ownership,
session or publication authority. Three authored tests use real private temporary
files to cover exact shape, permission/symlink rejection, metadata, World mismatch,
redaction and retention after the original file changes. Their runtime is NOT_RUN.

Shared configuration and callers belong to A2 after #2187 and #2182. Exact actual
source copies and one frozen patch are prepared outside the product checkout at
`/workspace/oteryn-family-g/evidence/2202-D1190-private-A2/`. The proposal contains
NodeConfig's optional WorldConfig and validation, serve's single checked load,
borrowed GameplaySeamOwners/ComposedFreshAdmission propagation, connection clock
source/no-config offer filtering, both qualification run.sh configs with a real
once-per-session Secret file, and the one in-process qualification owner literal.
It removes DEFAULT_WORLD_CLOCK_EPOCH_UTC_MS in the same proposal as its caller
replacement. That removal is deliberately not published alone in the canonical
leaf. I-owned ReadinessConfig is byte-unchanged; no parallel node writer publishes.

The slot proposal is based on this branch's exact source; it is not a merge of
I's unpublished A2 assembly. Final A2 must compose the configured offer predicate
with I's boot-bound required-set builder/authenticated fresh/resume/recovery
selection, preserving a single effective offer source and refusing a configured
required clock when no World binding exists. Weather26 is still unoffered here;
actual weather publication, canonical occurrence and D1026 current-owner behavior
remain integration work, not claims derived from this retained loader.

Cheap owned Rustfmt, private changed-Rust formatting, shell syntax and diff/patch
application/hash checks are retained with the source freeze. New compilation,
strict lint, three loader tests, private caller/qualification runtime and all
weather integration remain NOT_RUN. Prior11 native components remain historical
source proof; the later clock delta and this loader have not been runtime-qualified.
Status NOT_FULLY_QUALIFIED, production acceptance RED. No shared product source,
registry, content pin, default seed, feature activation, runner, heavy job, commit,
push or freeze is performed by this author before the lead's source review.

## Historical D1137 authoring checkpoint — superseded by D1190 source grant

CP [D1137](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6224930839)
assigns the seam and World seed accessor in `world_clock.rs` to #2202, capability26
and state domain22 as leaf constants, registry A2 after#2182, and D1027 sign-off
through frozen protocol review. This supersedes the separate WEATHER-SEAM-1
path allocation below; it does not supply the missing production config source.
[D1122](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6224930201)
permits this draft checkpoint with explicit RED/NOT_RUN and cheap checks.

Authored here: `WorldClockSource::absolute_elapsed_ms` reads the existing clock's
retained epoch with checked subtraction and no day wrap. Both native clock
minutes and weather slot arithmetic consume this read. Existing clock tests now
assert before-epoch, epoch, midnight and maximum-time reads. Leaf protocol
constants capture26/22 and the World-spatial dependency; codec behavior is unchanged.

**Exact source gap / CP action:** there is no production World-config owner
carrying a WorldId-scoped private seed, clock identity/epoch and weather policy
revision. `node/config.rs::WorldBundleConfig` contains content/bundle pins and
start coordinates only. `gameplay_transport/connection.rs` constructs the clock
with `DEFAULT_WORLD_CLOCK_EPOCH_UTC_MS` (zero). The only clock state in
`world_clock.rs::WorldClockSource` is its epoch scalar. Neither the bundle digest,
WorldId, public revision token nor the legacy clock can become a private seed.
CP must identify or allocate the production retained World-config source and its
owner plus the minimum path authority needed to load its seed/epoch/policy facts.
A seed accessor or canonical World/clock/policy/slot occurrence cannot be honestly
integrated until that source exists. No synthetic owner, fixture, implicit policy
or default epoch is used as weather admission evidence.

D1025/D1026 remain mandatory integration gates: missing binding means no offer;
binding loss after selection yields one full Unavailable while current; stale
owner emits nothing. Current-owner/session revalidation belongs at publication.
Those behaviors and replay/restart/Channel stability remain **NOT_RUN** on a real
caller. No capability offer, registry write, shared caller or module registration
is part of this checkpoint. D1160's single new-module-line allowance is unused:
registering the incomplete server module alone would introduce unconsumed APIs.

Validation for this authoring checkpoint: `cargo fmt --all --check`, explicit
`rustfmt --edition 2024 --check` of all three changed Rust files, and
`git diff --check` **PASS**. New clock read assertions are **NOT_RUN**. Prior native
component **PASS11** remains historical evidence for `fd2700ea`; it does not qualify
this changed clock/model source. Production acceptance remains **RED** because the required binding and caller
are absent. Ordinary full package checks on this changed source are **NOT_RUN**
under D1122; historical Weather package PASS remains historical. Exact updated
source native tests, strict lint and full production integration are **NOT_RUN**.
No full qualification, protocol sign-off, freeze, push or review trigger is claimed.

## Previous source and accepted scope

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
  implied. D1137 now allocates capability26/domain22 as leaf constants; message IDs and client commands remain unallocated.
- Authored tests cover the static map, native clock/light and independent SIM
  decision invariance, plus candidate codec bounds, shapes and malformed input.
  Native clock/SIM invariance is a component proof. Actual transport/gameplay
  consumer isolation is an integration gate; no dummy gameplay state proves it.

## Acceptance and remaining gates

- [x] Direct-slot arithmetic and SIM primitive accepted by D1023/D1024 and CP head review.
- [x] D1111 rain/fog/snow map and explicit selector authored in owned source.
- [x] Owned candidate codec and negative tests authored; no registration/sign-off claim.
- [x] Residue checked against observed main and exact CP ruling before this batch.
- [x] Historical exact-source native component compilation, strict lint, discovery and11 tests:6 server +5 protocol.
- [x] Registry (capability 26, state domain 22), proto, constants and requirement rows registered; `offered: true` after the D1027 sign-off.
- [x] Canonical occurrence/token (`WorldWeatherSource`) over the retained World binding; no seed or slot in the public token.
- [x] Server module, offer seam and connection wiring (admission snapshot, `Next::Visibility` delta); e2e in `capabilities_tests.rs::weather_wire` and, on the production composed owner through `serve_admitted` (PostgreSQL lane, nested under `monk_save::postgres_tests`), `weather_wire_postgres_tests.rs`.
- [x] No binding means no offer; binding loss emits one full Unavailable; restart/reconnect/Channel move never reroll (e2e).
- [x] Production head integration (`ComposedFreshAdmission::world_weather()` returns the shipped data); normal all-target/package CI qualification runs in the PR lanes.
- [x] Protocol-owner D1027 sign-off: owner "tak" on 2026-10-18 (kind 1..9, intensity 1..3, capability 26 offered once the data loads); registry `offered` flipped to true.
- [x] A2: production offer and runtime stream. The D1448 Oteryn-designed `content/weather` data is the source; it is offered once it loads and validates, and fails closed otherwise.
- [x] Visual-only isolation at the wire: the e2e asserts only domain 22 deltas move with weather.
- [x] WEATHER-CLIENT-1 consumer/assets: handed to the client task; the server side carries the values (D1448).

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
last_progress: CP D1446 fail-closed seam wired end to end; unsourced rain/fog/snow weights removed
status: completed
branch: skeleton/weather-1
blocker: none; CI qualification of the final head
next_action: merge after CI and review are green; archive this record with the closeout
```

Lead independent source review PASS for the owned retained-config leaf; changed-file
Rustfmt/diff and governance validator +124 tests PASS. D1122 permits this
NOT_FULLY_QUALIFIED checkpoint; new native/strict/runtime remain NOT_RUN. The
private shared A2 proposal remains uncommitted to the product and slot-bound.

Native compile found two test assertions requiring Debug on intentionally
non-Debug GameplayDecisionRoot (E0277, duplicate diagnostics). Assertions now use
boolean equality, preserving the secret type without printing its bytes. First
compile exit101/runtime NOT_RUN retained; repaired source requires new native
qualification. Production loader source is unchanged.


### D1190 actual checked-loader qualification

Qualified source: `026085f720c245c0434333710a6ccfddfafc57cf` (production loader
unchanged from f32361ac; only the two secret-root test assertions repaired).
The actual native Cargo lib-unit artifact compiled successfully, fresh=false,
with exact Eventsded94838 and Reset0c745974 owned source in a private crate.
Its three loader cases each passed1/failed0/ignored0: real checked Secret read,
World mismatch/retention/redaction, invalid shape/mode/symlink and metadata.
Actual dep-info includes the exact clock and test files once, and source-at-close
matches. ELF SHA256 `89fb3206f720c42b168379022f37198e77b8b4785638ec6b0813e6589fe883d8`;
retained gzip SHA256 `57aab3f9a153bbb5eea4fd333ca46bf261a9f76140e1cdf70c2b4854cc9558a6`.
The first compile101/E0277 is preserved; it did not execute tests. No production
Debug implementation was added to the secret root. Current strict/all-target,
A2 caller/offer/occurrence and full runtime integration remain NOT_RUN.

Fresh private A2 proposal v2 is based on026085f7 and contains only eight shared
caller/config/qualification plus coherent DEFAULT-removal paths. It omits the
already-owned loader/test hunks. Exact patch SHA256
`f116b6430005f1f0f1eb68feac3718cd946edb26d4597b89fd7a6cd7da399f83`;
all before/after hashes and exact apply roundtrip retained outside the product.
The v1 proposal remains immutable historical evidence. No shared caller is pushed.


### D1446 wiring (supersedes the NOT_RUN notes above)

The unsourced 6/5/5 rain/fog/snow table was removed; no preset values are invented. Capability 26
is registered but not offered. `GameplayAuthority::world_weather()` defaults to `None`, so
production never offers 26; an authority holding a retained binding offers it dynamically
(`with_weather_offer`), sends the domain 22 snapshot at admission and one full-replacement delta per
new absolute slot, lost binding (one `Unavailable`) or restored binding. Validation of the
candidate is recorded in the PR status comment.


### D1448 Oteryn-designed values, live in production (supersedes the D1446 "never offered" state)

The owner chose Oteryn's own weather. Everything below is `OTERYN_DESIGN`, not Tibia reference data,
and weather stays visual only (no gameplay effect).

- Data: `content/weather/{index,climates,areas}.json`, validated by
  `tools/content-schema/weather-authoring/validate_weather.py` and by the server loader.
- Source: a stateless, public, sha256-based deterministic draw per area. Each of the 28 regions
  and 22 cities draws independently. Periods run inside 12-slot blocks, last 15 to 45 minutes
  (1 to 3 slots of 15 minutes, the last one truncated at the block end), and start on slot
  boundaries. Intensity is uniform 1 to 3.
- Weights per climate: temperate clear 55, cloudy 20, rain 15, storm 7, fog 3; cold clear 50, snow 35,
  blizzard 15; desert clear 70, heat haze 20, sandstorm 10; jungle/swamp clear 40, rain 35, storm 15,
  fog 10. Climates are assigned by area name (the weather content family #2246 has no climate
  taxonomy, so everything else is temperate); this assignment is an Oteryn design guess.
- Wire: `Active{binding_token, absolute_slot (period start), kind 1..9, intensity 1..3}`, at most 51 bytes.
- Production: the trait default of `FreshAdmissionAuthority::world_weather()` is `None`; the
  production `ComposedFreshAdmission` returns the shipped data. Capability 26 is in the static
  offered sets and the registry (`offered: true`, D1027 signed off), and a connection drops it
  when its authority exposes no source, so missing or invalid data still fails closed.
- Approximation: no area geometry exists server-side, so a player maps to the nearest area anchor
  (region position or city hometown temple) by x and y; 12 areas have no anchor and never win.
  Where a region and a city share an anchor, the city (lower key) wins.
- Extension point: the `WeatherSource` trait and `real_world: null | {latitude, longitude}` per
  area allow a later live real-world binding. It is not implemented.
- CLIENT follow-up: rendering of kinds 1..9 with intensity 1..3 (and of `Unavailable`) is a client
  task; the server only carries the values.
- D1027 protocol-owner sign-off was given by the owner on 2026-10-18. The card stays active until CI is green.
