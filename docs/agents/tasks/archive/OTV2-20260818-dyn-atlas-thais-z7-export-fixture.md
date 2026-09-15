# OTV2-20260818-dyn-atlas-thais-z7-export-fixture — COMPLETED

```yaml
task_id: OTV2-20260818-dyn-atlas-thais-z7-export-fixture
status: completed
repository: blakinio/Oteryn-v2
pr: 335
final_head_sha: 8553e2b6e354a7ccb7d273d16f1a2e0cf49b6ad0
merge_sha: bf8a65ca0d6b0fbc1b6c521b16e613824b048f0d
closed_at: 2026-08-18T11:22:00+02:00
cross_repository_coordination_id: OTERYN-GAME-ATLAS-V1-THAIS-Z7
```

## Terminal result

PR #335 merged the first bounded deterministic Game-owned semantic export producer for DYN-ATLAS-001 Thais Z7.

The accepted producer boundary is `tools/game-atlas-thais-fixture/` plus its exact-head workflow. OTBM and Tibia asset metadata remain migration/import inputs on the Game side only. The emitted artifact is an explicit `oteryn-game-atlas-export-v1` semantic projection under:

- `oteryn-world-spatial-v1`;
- `oteryn-crystalserver-legacy-spatial-import-v1`;
- `oteryn-atlas-15-32-appearance-spatial-v1`.

Browser/Atlas consumers do not parse OTBM and do not reconstruct legacy stack/pattern/animation rules.

## Exact source identity

```text
Otheryn migration revision = e417c5e7c22986bf4acef0495eb47f7b72c97cce
CrystalServer revision = 5e89bf8329ea406cb4ea8f4a18f32954f13e5418
world.otbm sha256 = 3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034
legacy bounds = X 32280..32440, Y 32155..32305, Z 7
native bounds = x [32280,32441), y [32155,32306), floor -7

asset object = 15.32.zip
Drive file id = 1Dlo3bS4K1nS3mw4BhPZdlHT7lX5zRAvv
asset ZIP sha256 = 1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f
catalog sha256 = 35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85
appearance sha256 = dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075
```

## Final frozen-head artifact evidence

Producer head:

`8553e2b6e354a7ccb7d273d16f1a2e0cf49b6ad0`

Exact semantic fixture workflow:

```text
run = 32119580912 (#8)
job = 95656797494
result = SUCCESS
semantic artifact digest = sha256:d38a98acaf019b07a05c0bee922505fe4c9852b38e65644e488e92df9031da2e
workflow artifact id = 9318268404
workflow artifact zip sha256 = ec05e39be62d6826d27be19ff9c33c6cba7d1c835f79d02b8ad303b073c1ef40
workflow artifact compressed bytes = 1723168
```

The workflow artifact name is:

`dyn-atlas-001-thais-z7-semantic-export-8553e2b6e354a7ccb7d273d16f1a2e0cf49b6ad0`

Semantic/file facts:

```text
tiles = 24311
ground items = 24292
top-level visible tile items = 14990
source item tree without ground = 14993
presentation records = 39282
resolved primitives = 39282
unique appearance source ids = 862
unique sprite source ids = 990
manifest bytes = 2357
tiles.jsonl bytes = 28040344
tiles.jsonl sha256 = ff14efee3fc376d8f18432c628294c64ffe89450a59aaa498a28e6d705815984
diagnostics.json bytes = 19
diagnostics.json sha256 = 60326e4e048106d4366a2fd8fe472ccfdf06667fcd0f234977febfeaa38f31b8
verified canonical bytes = 28042720
```

`14993 - 14990 = 3` proves three nested source item descendants exist in the bounded source. They are intentionally excluded from visible spatial presentation records rather than being flattened into the map stack.

## Semantic guarantees proven

- all 24,311 selected native tile coordinates are emitted in deterministic y-major/x-minor order;
- visible ground/top-level source stack becomes explicit `PresentationOrderKey { plane: 0, order: index }`;
- concrete frame group, static default phase, pattern, layer and sprite source ID are resolved by the Game producer;
- visual coverage/displacement are explicit under the accepted 15.32 profile;
- source appearance/sprite IDs remain provenance/reference IDs;
- unresolved canonical Game entity identity remains explicit rather than invented;
- canonical artifact excludes wall-clock/runner-path metadata;
- default-deny public-field policy rejects any unallowlisted canonical manifest/tile/presentation/primitive field;
- a negative self-test proves an unexpected `text` field is rejected;
- two independent builds from identical declared inputs are byte-identical;
- deliberate artifact corruption is rejected;
- semantic artifact contains no decoded source pixel payload.

## Exact-head validation

Final frozen head `8553e2b6e354a7ccb7d273d16f1a2e0cf49b6ad0`:

```text
Game Atlas Thais Z7 fixture #8 = SUCCESS
pure producer self-tests = PASS (8/8)
Agent Governance #1639 = SUCCESS
Architecture Semantic Audit #226 = SUCCESS
Merge Authority Audit #443 = SUCCESS
Merge Gate #487 = SUCCESS
review threads = 0
review submissions = 0
```

After Draft -> Ready, GitHub correctly required a fresh aggregate merge status. Ready-state checks also passed on the same unchanged head:

```text
Agent Governance #1643 = SUCCESS
Architecture Semantic Audit #230 = SUCCESS
Merge Gate #491 = SUCCESS
```

Merge Gate #491 included Linux/Windows build, strict Clippy, tests/synthetic harness, Rust policy/metadata, supply-chain review, dependency review, CodeQL Python/Actions and final aggregate validate.

PR #335 was then squash-merged as:

`bf8a65ca0d6b0fbc1b6c521b16e613824b048f0d`

## DYN-ATLAS-001 effect

The Game-owned bounded semantic export artifact/digest blocker is **CLOSED**.

Atlas may now consume the exact artifact identity above as its immutable semantic input for the bounded Thais Z7 proof. It must validate the contract/profile/capabilities/digest and must not reopen OTBM or legacy source files as fallback authority.

Remaining programme work belongs primarily to `Oteryn/Oteryn-Atlas`: semantic ingestion/chunking, content-addressed pixel handling, browser renderer, semantic picking/inspector, deterministic navigation and measurements.

Public publication of decoded 15.32 source pixels remains separately gated on project authorization for the exact ZIP digest `1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f`; this semantic Game fixture itself contains no source pixel bytes.
