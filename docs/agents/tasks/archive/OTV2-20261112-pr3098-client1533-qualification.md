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

## Successor composition for runner diagnosis

The server successor b5c90b816 is merged forward with the native compile repair
before the next publication. It corrects three CI-confirmed fixture assumptions:
retired Atelier identities, capped TaskBoard token credits, and direct Templar
life-drain protection. Test-only reshape branch markers retain the strict 6214
assertions while its rejection remains unresolved. These markers are temporary
and must be removed after matched diagnosis before final integration. The
Activity handoff failure is still unclassified; a passing client occurrence is
not a waiver of the failed server occurrence.

Of the 390 reviewed client-owned paths, 389 retain their reviewed blobs or
intentional deletions. Only the documented chat regression compile fix differs.
Formatting, locked metadata, governance validation and its 129 tests, and diff
checks pass for this composition. No full native or runtime pass is claimed.
The new combined candidate requires its own runner results; #3115 must land
before #3098, with accepted-main composition and post-merge gates still required.

## Utility caller fixture repair after native execution

The native tests 3/3 job 131117755857 on composed head
5f117b87bf4e8090dec896151beef0f3f8b1c316 completed 3,781 tests with one
failure in `n022_session_utilities_use_existing_callers`. Its old assertion
expected Clear Oldest Message to mutate the mailbox projection immediately,
although the repaired production caller queues a Session-owner command. The
same fixture also retained the old Toggle Server Messages assertion against
the general player-chat filter; the first panic had prevented reaching it.

The caller fixture now requires the actual `ChatClearOldest` request, retains
both old/new projected lines until the owner processes it, toggles the separate
server-notice flag, and requires the player transcript flag to stay unchanged.
It consumes that exact request before the existing QuickLoot command assertion.
The owner-effect assertion belongs to the existing Session/chat integration
regressions, rather than a fake fixture pop. No production behavior changes.
All Clear Oldest/Toggle Server Messages fixture references were swept together.

A lightweight probe compiled the actual utility-action body and the two affected
fixture fragments with bounded shell collaborators. Both stale assertions failed
before this repair; both corrected caller contracts passed afterward. This probe
does not execute the real Session/chat loop and is not a new native runtime pass.
The full chat-loop regression's runner result remains unconfirmed here; final
composition requires its own native/Windows, PostgreSQL and Node CI evidence.
