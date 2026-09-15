# Merge Queue operating model

Game uses GitHub Merge Queue as the protected `main` integration-freshness authority after the repository's moving-base canary succeeds.

The externally required status remains exactly `game-gate`. Ordinary pull requests qualify through the PR aggregate gate. Queue candidates qualify through the protected `merge_group` path, which runs repository governance, Dependency Review, CodeQL, Linux and Windows Rust validation, and supply-chain checks before publishing the same `game-gate` context.

During rollout, strict required-status freshness stays enabled until a real moving-base canary proves the queue path. The canary keeps PR A at an unchanged green head, advances `main` with an independent PR B, then places unchanged PR A into Merge Queue. Success requires `game-gate` to pass on the synthetic merge-group candidate and the queued PR to integrate without a merge-up or retrigger commit.

After a successful canary, strict freshness may be disabled so Merge Queue owns integration freshness. Required approving reviews remain `0` and required Code Owner approval remains disabled while there is one human maintainer. Review-thread resolution, linear history, squash integration, deletion protection, non-fast-forward protection, and the no-bypass baseline remain in force.

Material workflow, Merge Queue, branch-protection, or governance/control-plane changes require exact-head deterministic validation, one useful independent deep review, explicit human-owner authorization before integration or live control-plane mutation, and direct post-change GitHub readback.
