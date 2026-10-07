# OTV2-20261007 CLIENT-INSTALLER-1

status: frozen
base_main: f80428d5
branch: claude/client-installer-1-20261007
coordination: #1622
runtime_activation: false
last_progress: unsigned CI installer, launcher and per-user mutexes authored; local validation green
next_action: control-plane review routing of the frozen head

## Goal

The CLIENT-INSTALLER-0 §8 row CLIENT-INSTALLER-1: an unsigned CI installer artifact with the
client exe, the launcher, the dev `client.env` and an empty `packages.json`; `<release_id>`
compiled in; staged-rename install with atomic `current.txt` activation; the per-user
transaction and client mutexes; uninstall. Deferred PR #1894 findings 4200478168 (relaunch only
after Setup releases its mutex: `/RELAUNCH` plus `oteryn-launcher --after-setup`) and 4200478176
(the installer's release, channel and version must match before activation: `/EXPECTRELEASE`,
`/EXPECTCHANNEL`, `/EXPECTVERSION`).

## Owned paths

- `apps/client/installer/**`
- `apps/client/src/bin/oteryn-launcher.rs` and its `apps/client/Cargo.toml` bin entry
- the `<release_id>` constant and `CLIENT_BUILD` in `apps/client/src/lib.rs` (plus the
  `#[cfg(windows)] pub mod win_mutex;` declaration the module needs)
- the `OterynClient` mutex in `apps/client/src/main.rs`
- `.github/workflows/merge-gate.yml`, `rust_windows` steps only
- `.github/workflows/merge-authority-audit.yml`, the `EXPECTED_MERGE_GATE_BLOB` rotation only (D854)
- CP D901 (sent as D607): `apps/client/Cargo.toml` lints and the `windows-sys` dependency,
  the `Cargo.lock` effect of that dependency, `apps/client/src/win_mutex.rs`
- CP D907: `tools/repository/validate_pr_gate_pg_sim.py`, the
  `EXPECTED_EVIDENCE_JOB_SHA256["rust_windows"]` constant only (rotation, no logic change)
- CP D909: `content/world/pins/oteryn.json`, the `inputs_digest` regenerated from
  `oteryn-world-bundle-compiler pin-check` after the client crate and `Cargo.lock` changed
- this record

## Decisions

- D901: client lint table with `unsafe_code = "deny"`; all unsafe code in `win_mutex.rs` under
  `cfg(windows)` with a module-level allow, a SAFETY comment per block and RAII handle closing;
  `windows-sys =0.61.2`; `SYNCHRONIZE` defined locally.
- D907: 1a rotate the `rust_windows` evidence-job hash; 2a installer steps in `merge-gate.yml`
  only. The Merge Queue (`merge-group-gate.yml` `rust_windows`) does not run them yet: follow-up
  CLIENT-INSTALLER-1b.
- Owner: `MessageBoxW` added to the D901 FFI list so the windowless launcher can show its error;
  a `--smoke` run never opens the dialog.

## Notes for review

- Signing: Inno's `SignTool` is wired only under `#ifdef SignTool`, because Inno aborts a compile
  whose signing command produces no signature; CI calls `sign.ps1` (a no-op that says signing is
  not configured) explicitly. This differs from the contract's "Inno calls it through SignTool".
- Staging and verification happen in `PrepareToInstall`; activation and clean-up wait for
  `ssPostInstall` (Codex 4204201805), so a failure while Setup installs its tracked files leaves
  `current.txt` unchanged. A staged directory left by such a failure is inert and reused or
  removed by the next install. A failed activation shows an error and exits with code 20.
- ISCC 6.7.3 predefines `FILE_ATTRIBUTE_*`; the script no longer redeclares them (CI compile
  error at 5d370fe4). Other declared names were checked against the 6.7.3 script sources.
- Inno Setup 6.7.3 is pinned by URL and SHA-256 in the workflow; the upload action is pinned by
  commit SHA like the rest of the repository.
- Not verified locally: the Inno script compile and every Windows runtime behaviour of the
  installer, uninstaller, launcher and mutexes. The PowerShell scripts were parsed with
  PowerShell 7.5.3 only; the Windows-target Clippy ran on a scratch copy of the client's Windows
  modules because the full client does not cross-compile here.
