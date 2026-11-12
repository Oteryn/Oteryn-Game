# OTV2-20261112-pr3098-client1533-qualification

```yaml
task_id: OTV2-20261112-pr3098-client1533-qualification
title: Repair and qualify the native 15.33 client package
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/pr3098-merge-20261112
pr: 3098
base_sha: f05944d8852342d30c933cc3a0f4338676f92571
head_sha: null
owner: Codex integration lead
depends_on:
  - 3115
```

## Scope and dispositions

The owner requested batched Codex review and repair until no P0/P1/P2 remain,
light local checks, and final heavy GitHub qualification. This task instruction
supersedes the earlier review-count/P2-deferral limit for this PR.

Independent whole-client review found two P1 defects in utility actions. One
coherent repair now queues oldest-message dismissal to the Session owner, which
removes the retained line and its paired extras before normal publication, and
gives server notices a separate visibility switch across console consumers.
Player/private chat remains visible when server notices are hidden. Five focused
repository regressions cover the owner, actual session loop, metadata alignment,
mixed tabs and notice views. Existing recipient/echo assertions remain.

Three Polish labels and the reported native viewport/subsystem behavior predate
this PR; the exact relevant values/paths were compared with its server baseline.
They were not introduced by the reviewed client delta and were not silently
counted as repaired native-window behavior.

## Dependency composition and review

The complete client-only delta was reviewed against inherited server bde524ebb;
the repair at 6544f20a received an independent material-delta pass with zero
P0/P1/P2 findings. Reviewers authored none of the repair.

For the owner's explicit faster-parallel request, the reviewed #3115 server
candidate a6e312236 was merged forward locally before publication. Its 169
incoming server/main paths and the 390 client-owned paths are disjoint. All 374
present client blobs and 16 intentional deletions remain identical to the clean
reviewed client candidate. This qualifies the actual combined producer/consumer
tree on runners while preserving the landing order: #3115 first, #3098 only
after its own exact-head checks and accepted-main composition proof. No
protected-main mutation is performed by this dependency merge.

## Validation

- `cargo fmt --all --check`: pass.
- `cargo metadata --locked --no-deps --format-version 1`: pass.
- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass, 129 tests.
- `git diff --check`: pass.
- Actual utility-action Rust harness: two tests failed before the repair and
  passed afterwards. Actual Session-owner/metadata harness: two tests passed.
  Their source blobs are unchanged by the dependency merge.
- Reviewed client source preservation: all 390 paths, including deletions,
  checked after the forward dependency merge; no changed client blobs.
- Full client/native/Windows, heavy Rust, PostgreSQL and Node qualification are
  delegated to GitHub CI; they are not claimed to have passed locally.

Authoring is complete; final qualification and merge remain pending. The freeze
comment binds the exact head. Integration result: squash merge of #3098 after
#3115, followed by the full main gate. No test weakening, workflow edit,
protection bypass, force push or red-Node waiver is authorized by this record.

## Native CI compile repair after the first composed freeze

The first composed head fb0538c095baf2bf26eeba19f219e313e2333a8f failed the
native Clippy/test leaves with the same E0308 in the new chat-loop regression.
`ClientRuntime::block_on` returns `Result<F::Output, RuntimeError>`; the async
scenario already returns `Result<(), Error>`. The worker closure had retained
both layers, unlike the neighboring regression. It now explicitly returns
`Result<(), Error>` and propagates the runtime layer inside the worker. The
scenario assertions, async failure, thread-join handling and 64 MiB stack remain.
The newly added Session regression already unwraps its runtime layer correctly;
the other new tests have no runtime/thread wrapper.

A lightweight harness using the actual outer test wrapper and matching runtime
method signatures reproduced E0308 before the change. Afterward it compiled and
passed four checks: success, constructor error, runtime error and async scenario
error; each error retained its type. This is compile/type-propagation evidence,
not execution of the real chat scenario. Full native compilation and runtime
regressions still require the new composed head's CI. The earlier 424 quest-test
result belongs to the inherited server qualification; it is not new client
validation. The prior 390-path identity check above describes the first composed
freeze; this compile repair deliberately changes its chat test source.
