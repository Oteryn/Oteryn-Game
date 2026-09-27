# Fixed one-creature death and corpse structural evidence

Status: **VALIDATING**. Allocation: #162 comment 5849676308. Protected base:
`f6b267126d6a4ee505f614a7b740ab16aa86b7b6`.

## Physical boundary

The existing `ChannelActorCarrier` remains the physical owner of one admitted
fixture creature, its exact actor generation, current position and one retained
Ability commit. Combat cannot supply lethality, position, identity or replay
facts. The carrier alone issues an opaque, single-use receipt when its private
record proves one applied positive-HP to zero transition for the exact current
actor. Receipt issuance fallibly copies the complete plan bytes into one
bounded `Box<[u8]>`; projection moves that allocation rather than allocating
again. The receipt also carries the exact actor reference, damage, original HP
and immutable owner position context/revision. The original owner commit keeps
its own bounded plan bytes for replay comparison, so retained state is exactly
one commit binding plus one projection binding, each at most 4096 bytes.

The carrier retains exactly one `Option<RuntimeCorpseProjection>` outside the
fixed slot array. There is no corpse map, vector, queue or second index. An
identical retry returns the same logical projection; a different or stale
receipt fails without replacement. A retained corpse blocks another fixture
creature admission until a future, separately accepted retirement/decay child.
Administrative removal never creates a death or corpse.

Dead creature lookup and position mutation fail closed. Ability may still
commit a lethal transition for an unpositioned prerequisite fixture, but Combat
then returns `PositionUnavailable`, and the position cannot be invented after
death.

## Focused matrix

`channel_owner_combat_death_tests.rs` covers client and AI lineage, exact
position/context/revision, nonlethal absence, pre-commit failure, identical
Ability and projection replay, injected failure before projection, lost
response after projection, missing position, dead lookup and position-mutation
rejection, modified receipt bytes/position, a different lethal occurrence after
the first death, administrative removal, stale actor/owner authority,
retained-corpse max+1 admission rejection, and the existing 4096/4097-byte
commit-binding boundary. Rejections compare the complete owned slot/projection
state relevant to the operation.

## Reference classification

- Canary `opentibiabr/canary@47dfd51f45280a59a1d3e50ba7edd573d7234446`,
  `src/creatures/creature.cpp`, blob
  `89a4af117d4de4e9373ddcb96c2513006c22c6ab`, SHA-256
  `f77f2c811f0d05a315ddd316a5d61f309814bc477ca290281a2b09e80c403683`.
- CrystalServer `zimbadev/crystalserver@5e89bf8329ea406cb4ea8f4a18f32954f13e5418`,
  `src/creatures/creature.cpp`, blob
  `d1e52e5881ea75cdc5835e0563dc842b38372f85`, SHA-256
  `c38b9b326b8b6943e33c5b06ea53a8d3d081784453b4a600acfec9185e75e39a`.

Both are `OTS_HYPOTHESIS_ONLY` evidence for lethal -> death -> corpse ordering.
They do not define Oteryn identity, authority, formulas, idempotency, resource
bounds, persistence or parity. No source code is copied.

## Qualification handoff

The complete source candidate was published at
`a98033b59092f5dd1a74e69bb67ccb962b5b6254` and checked from a fresh native
Linux LF checkout, avoiding Windows CRLF conversion in byte-sensitive tests.

- `cargo test --offline --locked -p oteryn-game-server --lib --no-default-features`:
  **608 passed, 0 failed, 2 ignored**. The ten new Combat tests passed inside
  this run.
- `cargo test --offline --locked -p oteryn-game-server --lib runtime_actor_carrier --no-default-features`:
  **53 passed, 0 failed** in the authoring checkout.
- `cargo clippy --offline --locked -p oteryn-game-server --all-targets --no-default-features -- -D warnings`:
  **PASS** in the fresh Linux checkout.
- `cargo fmt --all -- --check`: **PASS**.
- `python3 -I tools/agents/validate_governance.py`: **PASS**.
- `python3 -I tools/repository/validate_repository_policy.py`: **PASS**
  (`23 files`, `42 workflows`).
- `git diff --check` and clean-checkout readback: **PASS**.

After native-entry activation PR #953 integrated as protected
`main@139202f1aa790e573c0ae5d41f4e529fdc503016`, this task returned explicitly
to authoring and merged that protected head. The only textual conflict was the
`foundation/mod.rs` re-export list. Its resolution preserves the union of
`ChannelContentPin` and the lethal/death/corpse types; the carrier implementation
auto-merged without a manual behavior edit. The composed authoring head
`2250fac64fe1edfe48a893269703d8a28d8126d2` was checked from a fresh native
Linux LF checkout:

- full game-server library: **612 passed, 0 failed, 2 ignored**;
- the ten Combat death tests and three native-entry activation tests passed in
  that run;
- strict all-target Clippy with `-D warnings`: **PASS**;
- rustfmt, governance (`22 documents`, `9 lanes`), repository policy
  (`23 files`, `43 workflows`), diff check and clean-checkout readback: **PASS**.

The final documentation-only successor must preserve the composed Rust blobs,
pass the document-aware validators on its exact SHA and receive fresh hosted
exact-head CI plus independent review. Candidate-specific evidence from the
superseded pre-composition head is not terminal evidence for that successor.
No production or process-restart claim is made. Merge Queue and protected-main
readback remain control-plane owned.
