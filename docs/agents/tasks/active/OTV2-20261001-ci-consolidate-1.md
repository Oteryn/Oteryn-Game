# OTV2-20261001-ci-consolidate-1

```yaml
task_id: OTV2-20261001-ci-consolidate-1
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 162
owner_authorization: D253
allocation: "#162 comment 5926106104; corrected owned paths 5926172095"
writer: Codex task 01a0f3db-9cb7-7ab0-be26-2f9dbd86643c
stage_a_branch: ci/ci-consolidate-1-stage-a
stage_b_branch: ci/ci-consolidate-1-20261001
jira: KAN-20 (parent KAN-9)
ownership_released: only after verified integration and control-plane closeout
```

## Delivery

The two-stage packet keeps the protected-base audit. Stage A rotates only the
approved PR/Merge Queue workflow blob values. Stage B activates the exact reviewed
workflows. The control plane owns review triggers and enqueue/auto-merge; Stage B
must not integrate before Stage A. Bind each final remote head in #162 after the
last authoring write; validation and review apply to that frozen head.

- Replace whole-workspace `target/` caching with immutable upstream dependency
  caching, shared across corresponding Linux/Windows PR, queue and main jobs.
  Only successful protected-main push Linux/Windows jobs publish; all other
  jobs restore only. A miss builds normally.
- Use `line-tables-only` dev/test debug artifacts and disable incremental builds
  on disposable runners. Release, assertions, overflow checks and test commands
  retain their accepted settings.
- Cancel superseded selected post-merge Linux/Windows/PostgreSQL builds by job.
  Documentation-only pushes with unselected lanes do not cancel earlier builds;
  manual dispatch remains independent.
- Absorb #813 / #809 Slice B: one reviewed routing matrix in
  `validate_pr_routing_contract.py`, used by hosted qualification and local tests.
  Preserve additional exact-tree, malformed-input and aggregate canaries.
- Absorb #814 / #809 Slice C: one candidate-side `CONTROL_CONTRACT_PINS` registry
  and comparison loop. Preserve both existing job extraction rules, semantic
  PG/SIM checks and separate protected-base approval. Tests and the PR validator
  read their expected bytes from this registry rather than duplicate hashes.

No stale draft branch is merged, rebased or published. #813/#814 closure remains
the control plane's action after this candidate freezes. KAN-20 is an aggregate
Story and remains active while its other acceptance sources, including #809,
remain open.

## Reconsidered findings

| Finding | Disposition |
|---|---|
| Queue maximum builds 5, minimum merge 1 | Already configured; retain. Concurrency improves throughput but is not a cache optimization. A failed prefix may rebuild affected later groups. |
| Windows on proven server-only queue changes | Already integrated by #1421/#1422; preserve its exact classifier and fail-closed fallback. |
| #1170 protected-base audit failure | Latest observed c8a8d884 still has obsolete MQ blob 82b0d535. Its existing writer/control-plane content train must reconcile current main and refreeze; unchanged-head reruns do not repair it. |
| Documentation selecting Windows | On #1413's exact head, protocol and server package files reference architecture paths, including bounded directory references in comments. The conservative consumer model therefore selects real package closure. No blanket docs exemption or new Rust lexer is introduced. |
| Large cache and no cache in queue/Windows/PG | Implement dependency cache and a new namespace; retain cache misses as normal builds. No old cache entry is deleted. |
| Repeated protected-main Rust builds | Implement selected-job concurrency, preserving unselected/manual behavior. |
| Four PostgreSQL fixtures repeat 727 test names | Names alone do not prove identical fixture/type context. Preserve all targets and private fixture-owner boundaries; no deduplication is authorized by this observation. |
| Metadata `edited` re-runs heavy PR gate | Current governance expressly requires metadata edits to re-run the exact-head aggregate. Removing it requires a separately qualified event/identity design; do not turn a selected test into skipped success. |
| Dedicated content/semantic validators outside canonical fan-in | Coverage gap remains a follow-up. Select required validators from owning content contracts and prove exact-head plus synthetic-group routing before adding a lane; blanket-running all standalone capture/import jobs is not justified. |
| G4 focused Rust checks duplicate workspace checks | Preserve until unique materialization/roundtrip evidence is separated and tied to the canonical aggregate. No unique checks are removed here. |
| Broad Crystal/content workflow triggers | Preserve until actual dependency/input coverage supports narrower paths. |
| First queue failure leaves other jobs running | No privileged cancellation helper or new Actions write permission is added. Cheap/expensive job ordering needs a measured latency/resource comparison. |
| sccache, alternative linkers, combining PG/Linux jobs | No additional compiler layer, custom service or fixture rewrite without cold/warm measurement and compatibility qualification. |

## Qualification and measurement

Deterministic local preparation checks include workflow syntax, cache/publication
mutation controls, PR/MQ PG/SIM credibility controls, exact-candidate routing,
post-merge routing, repository policy and governance. Linux-only fixture routes
are qualified in WSL Ubuntu; PowerShell native command propagation is retained.

Required terminal evidence remains independent exact-frozen-head review, hosted
selected CI, real `merge_group` `game-gate=SUCCESS` and protected-main readback.
Local preparation is not protected integration. After integration, compare cache
bytes, restore/extract time and cold/warm build/test timings before claiming a
measured saving. The prior successful queue baseline 36820708925 had Linux build
6m13, Clippy 3m26 and approximately 6m23 of tests/smokes; no target saving is proven.
