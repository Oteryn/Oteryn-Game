# OTV2-20261007-client-installer-1

```yaml
task_id: OTV2-20261007-client-installer-1
title: "CLIENT-INSTALLER-1: unsigned CI installer, launcher, per-user mutexes and staged activation"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/client-installer-1-20261007
issue: 1622
pr: 1912
head_sha: "exact frozen head in the control-plane FREEZE_SHA entry"
final_head_sha: "exact frozen head in the control-plane FREEZE_SHA entry"
owner: implementation worker (control plane session_0114oBVR3osF1auvFMu6ksMH, decisions D854, D901, D907, D909)
created_at: 2026-10-07
updated_at: 2026-10-07
runtime_activation: false
owned_paths:
  - apps/client/installer/**
  - apps/client/src/bin/oteryn-launcher.rs and its apps/client/Cargo.toml bin entry
  - apps/client/src/lib.rs (release_id constant, CLIENT_BUILD, the cfg(windows) win_mutex declaration)
  - apps/client/src/main.rs (the OterynClient mutex)
  - .github/workflows/merge-gate.yml (rust_windows steps only)
  - .github/workflows/merge-authority-audit.yml (EXPECTED_MERGE_GATE_BLOB rotation only, D854)
  - apps/client/Cargo.toml lints and the windows-sys dependency, its Cargo.lock effect, apps/client/src/win_mutex.rs (D901)
  - tools/repository/validate_pr_gate_pg_sim.py (EXPECTED_EVIDENCE_JOB_SHA256["rust_windows"] only, D907)
  - content/world/pins/oteryn.json (inputs_digest only, D909)
  - docs/agents/tasks/archive/OTV2-20261007-client-installer-1.md
public_contracts: []
depends_on: []
blocks: [CLIENT-INSTALLER-1b (Merge Queue rust_windows installer coverage)]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task delivers the CLIENT-INSTALLER-0 §8 row CLIENT-INSTALLER-1: an unsigned CI installer built by Inno Setup 6.7.3. Inno Setup is pinned by URL and SHA-256 in `merge-gate.yml`.

The installer contains:
- the client exe;
- `oteryn-launcher`;
- the dev `client.env`;
- an empty `packages.json`.

`<release_id>` (`<version>+<channel>.<route>.g<sha12>`) is compiled into the client and the installer. Install and uninstall work as follows:
- **Staging:** `PrepareToInstall` stages or reuses a verified `releases\<release_id>[~n]`.
- **Activation:** happens at `ssPostInstall`, after Setup has installed its tracked files (Codex 4204201805). It is one write-through `MoveFileExW` of `current.txt`. A failed activation leaves the old pointer in place and exits with code 20.
- **Mutexes:** the per-user transaction mutex `Global\OterynClientSetup-<SID>` and the client mutex `Global\OterynClient-<SID>`. The client exits with code 3 while setup holds the transaction mutex.
- **Uninstall:** removes the release directories and the pointer.

Deferred #1894 findings:
- `/RELAUNCH` with `oteryn-launcher --after-setup` starts the client only after Setup has released its mutex.
- `/EXPECTRELEASE`, `/EXPECTCHANNEL` and `/EXPECTVERSION` refuse a mismatched installer before activation.

`ci-installer.ps1` builds two installers and exercises the following on the Windows runner:
- install, reinstall and repair;
- interrupted install;
- identity refusal;
- both mutexes;
- relaunch;
- uninstall.

## Decisions

- **D901:** the client lint table sets `unsafe_code = "deny"`.
  - All unsafe FFI is in `win_mutex.rs`, under `cfg(windows)`, with a module-level allow, a SAFETY comment on each block and RAII handle closing.
  - The dependency is `windows-sys =0.61.2`.
  - The owner added `MessageBoxW` so the windowless launcher can show its error. A `--smoke` run never opens the dialog.
- **D907:**
  - Rotate the `rust_windows` evidence-job hash.
  - The installer steps are in `merge-gate.yml` only. Merge Queue coverage is follow-up CLIENT-INSTALLER-1b.
- **D909:** regenerate the world pin `inputs_digest` with `oteryn-world-bundle-compiler pin-check`.

## Notes for review

- **Signing:** Inno's `SignTool` is wired only under `#ifdef SignTool`, because Inno aborts a compile whose signing command produces no signature. CI calls `sign.ps1` explicitly, and it no-ops. No signing, credentials, publishing or deploy are involved.
- **Fixes after CI ISCC runs:**
  - Inno predefines `FILE_ATTRIBUTE_*`, so the script no longer redeclares them.
  - Inno's `BOOL` is `LongBool`, so the kernel32 and advapi32 externals now return `Boolean`.
  - The declarations were checked against the 6.7.3 script sources.
- **Not verified locally:** ISCC and the Windows runtime behaviour (no Windows or Wine here). Both are covered by CI `rust_windows`. The Windows-target Clippy ran on a scratch copy of the client's Windows modules.

## Validation

cargo fmt --all -- --check: pass
cargo clippy --locked -p oteryn-client --all-targets -- -D warnings: pass
cargo test --locked -p oteryn-client: pass
oteryn-world-bundle-compiler pin-check: pass
python tools/repository/validate_repository_policy.py: pass
python -m unittest discover -s tools/repository: pass
python tools/agents/validate_governance.py: pass
python -m unittest discover -s tools/agents/tests: OK
git diff --check: pass

## Review and closeout

The control plane decides review on the frozen head. Merge result: squash merge of #1912. At authoring, CI, the Codex review and the Merge Queue had not yet run on this head.
