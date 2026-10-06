# CLIENT-INSTALLER-0 — Windows Client Installer and Package Contract Candidate

- DecisionStatus: `CANDIDATE`
- DeliveryStatus: `IN_REVIEW`
- ImplementationStatus: `NOT_STARTED`
- Date: 2026-10-06
- Coordination: `#1622`, decision `D853`
- Task: `OTV2-20261006-arch-client-installer-0`
- Depends on:
  - `docs/architecture/ADR-0002-repository-ownership-and-client-migration.md` (the client lives in `apps/client`)
  - `docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_CONTRACT_CANDIDATE.md` §15, §17, §18
  - `docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md`
  - `tools/qualification/login_local/README.md` (current client settings)
- Supersedes: the ALPHA-CLIENT-01 deferrals of installer/updater technology (§17.4, §24), client bundle format, and install scope and directories (§15, §24). ALPHA-CLIENT-01 now points here; its §17.1–§17.4 rules stay binding. The code-signing provider and patch/CDN format stay deferred.
- Runtime authorization: **NONE**

## 1. Purpose

The owner wants a real installer for the native Windows client that installs the client and its packages, not a bare exe. This candidate fixes the distribution unit, the installer technology, the package manifest, the updater model, the code-signing seam, the CI build and the implementation slices. It does not change protocol, identity, admission, persistence or production trust.

## 2. Distribution unit and layout

One release is one installer, `oteryn-client-<release_id>-x86_64-setup.exe`, built from one exact Game commit for one channel by one build route. The crate version alone does not identify a release: many commits share it (today every build is `0.1.0`), one commit is built once per channel with different `client.env` and `packages.json` content, and a release-job build differs from the CI build of the same commit. The release identity is therefore:

`<release_id>` = `<client_version>+<channel>.<route>.g<game_commit[0..12]>`, for example `0.1.0+dev.ci.g1a2b3c4d5e6f`.

- `<client_version>` is the `oteryn-client` crate version (today the workspace version).
- `<channel>` is the channel of section 5.1.
- `<route>` is `ci` for the unsigned `rust_windows` artifact (section 7) and `rel` for the release job (section 6, slice 4). A `rel` release is one immutable signed artifact per `<release_id>`: a rerun of the release job for the same commit and channel finds the published artifact, publishes nothing and does not re-sign (section 6).
- The suffix is the first 12 hex digits of the exact Game commit.

The build compiles all four parts in, so each `<release_id>` names exactly one payload. The client reports the exact release as `client_build` = `oteryn-client/<release_id>`, for example `oteryn-client/0.1.0+dev.ci.g1a2b3c4d5e6f`. Platform and the Gateway, the compatibility authority (ALPHA-CLIENT-01 §17.3), can therefore admit or reject one build even when another build shares its `client_version`.

- The value stays inside the existing bounds: visible ASCII and at most 64 bytes for the Platform login, and the 128-byte `client_build_id` of FND-02. To keep that guarantee, a `<release_id>` longer than 50 bytes fails the build.
- The format of the existing field changes, but the field itself and its diagnostic, non-authoritative role do not, so no protocol change is involved.
- `minimum_supported_client_version` remains a coarse client-side floor. Rejecting one bad build that shares a version with a good one is done by `client_build` at Platform/Gateway. No ordering is defined on `<release_id>`; which release is newer is decided only by the channel `sequence` (section 5).

All releases and channels share one install: one fixed Inno Setup `AppId`, one install directory and one uninstall entry. Installing another channel's installer is an explicit operator or user action that switches the channel.

### 2.1 Install directory (release payload, replaced by updates)

Per-user install, no elevation: `%LOCALAPPDATA%\Programs\Oteryn\`.

```text
%LOCALAPPDATA%\Programs\Oteryn\
  oteryn-launcher.exe            stable entry point; starts releases\<current.txt>\oteryn-client.exe
  current.txt                    the activation pointer: the one active release directory name
  releases\<release_id>[~<n>]\
    oteryn-client.exe
    client.env                   release defaults (section 2.3)
    packages.json                package manifest (section 4)
    packages\<id>-<version>.<ext>  content/asset packages listed in packages.json (none today)
  unins000.exe / unins000.dat    upstream uninstaller
```

A release directory is written complete and never modified afterwards. `current.txt` is the **only** activation pointer (ALPHA-CLIENT-01 §17.3):

A release directory is named `<release_id>`, or `<release_id>~<n>` (`<n>` a decimal integer from 1) when it replaces a broken copy of the same release; `~` never occurs in a `<release_id>`. `current.txt` holds that directory name. No directory is ever renamed or modified once it is in place, and the active directory is never moved.

1. If a directory of the release being installed already exists, which happens on a reinstall or a reactivation, the installer verifies the one `current.txt` names (or else the newest one) before reuse against the file list and SHA-256 values embedded in the installer: every payload file must be present with the exact hash, and no other file may exist. A directory that passes is reused unchanged and the copy is skipped; this is safe because a `<release_id>` names exactly one payload. Otherwise the installer writes the payload into `releases\.staging-<release_id>\` and, once it is complete, renames it (a same-volume directory rename) to a name that does not exist yet: `<release_id>` for a first install, or `<release_id>~<n>` with `<n>` one greater than any existing suffix when a broken copy (corrupted, partly deleted or quarantined) exists. A failure here leaves `current.txt` untouched, so the previous release stays active and fully consistent. A leftover staging directory is inert and is deleted by the next install.
2. Activation writes `current.txt.new`, flushes it, and replaces `current.txt` with one `MoveFileExW(MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)` rename on the same volume. That rename is the single commit point. No correctness depends on installer rollback after this point, because there is nothing left to roll back: before the rename the old release is active, after it the new one is.
3. Only after the rename does the installer delete other `releases\*` directories. When the activated release differs from the one `current.txt` named before, it keeps the directory `current.txt` now names and the one it named immediately before, which is kept for local rollback (section 5.3). On a same-`<release_id>` reinstall or repair, where `current.txt` already named a directory of the installed release, it deletes only the other directories of that same release, such as the broken copy a repair replaced, and never touches the previous distinct release. A broken directory is therefore removed only after the replacement is active. A failure during this cleanup, for example on a locked executable, is harmless; the next install repeats it.

Concurrency:

All mutex names carry the current user's SID and live in the `Global\` namespace, so they cover every session of that user and never block another Windows user on the same machine. Because the names are computed at run time, setup and the uninstaller create and check them in `[Code]` with the Windows mutex API rather than through the static `SetupMutex` and `AppMutex` directives.

- Installs and uninstalls are serialized by one transaction mutex, `Global\OterynClientSetup-<SID>`, whose name is fixed across all releases and channels. Setup creates it in `InitializeSetup`, refuses to run with an error if it already exists (`ERROR_ALREADY_EXISTS`), and holds it until it exits, which spans steps 1–3. A second installer, whether launched by the updater or by hand, therefore never interleaves steps 1–3.
- The uninstaller takes the same mutex the same way in `InitializeUninstall`, before deleting anything, and holds it until it exits. An uninstall therefore never runs during an install, and an install never runs during an uninstall.
- The client holds `Global\OterynClient-<SID>` for its lifetime. Setup checks it in `PrepareToInstall`, after it holds the transaction mutex and immediately before step 1, and the uninstaller checks it in `InitializeUninstall` after taking the transaction mutex; either refuses while any Oteryn client of that user runs. Conversely, the client first takes its own mutex and then checks the transaction mutex; if an install or uninstall holds it, the client releases its mutex, tells the user an install is in progress and exits. Because each side takes its own mutex before checking the other's, a client and a setup that start together cannot both proceed; at worst both back off. The check is in the client, not only in the launcher, so it also covers a release started directly. The updater's own installer starts only after the client has exited (section 5.2).
- Readers need no lock: the launcher reads `current.txt` once, and the rename of step 2 is atomic.

The Start-menu shortcut targets the stable `oteryn-launcher.exe` and never changes between versions. The launcher is a small, GUI-subsystem binary of the `oteryn-client` package (no console window). It reads `current.txt`, requires a value of the form `<release_id>` or `<release_id>~<n>` naming an existing `releases\<value>\oteryn-client.exe`, starts that executable with its own arguments and exits. Otherwise it shows an error that asks the user to reinstall; it never guesses another release. The client finds its release `client.env` and `packages.json` next to its own executable, never through `current.txt`, so an executable is always paired with its own configuration and manifest. This is the whole of the Oteryn-specific install logic.

### 2.2 Per-user data directory (never touched by install, update or uninstall)

`%LOCALAPPDATA%\Oteryn\`: per-user `client.env` overrides, settings, cache, logs and crash diagnostics (ALPHA-CLIENT-01 §15, §16, §17.4). Uninstall removes only the install directory, the shortcuts and the uninstall registry entry under `HKCU`. Deleting user data is a separate explicit user action, never a side effect of uninstall or update.

### 2.3 Configuration (`client.env`)

`client.env` is UTF-8 `KEY=VALUE` lines; `#` starts a comment line; no quoting, interpolation or includes. It carries the settings the client reads today (`apps/client/src/lib.rs` `NativeLoginConfig::from_env`):

| Key | Scope | Release default |
| --- | --- | --- |
| `OTERYN_PLATFORM_URL` | release default, per-user override | channel value; empty disables native login |
| `OTERYN_GATEWAY_URL` | release default, per-user override | channel value |
| `OTERYN_OAUTH_CLIENT_ID` | release default, per-user override | channel value |
| `OTERYN_WORLD` | release default, per-user override | channel value |
| `OTERYN_CHARACTER_ID` | per-user only | not shipped; a Character selection is account state, not release state |
| `OTERYN_DEV_ROOT` | per-user only | not shipped (section 2.4) |
| `OTERYN_UPDATE_URL` | release default | channel pointer URL (section 5); unset disables the updater |

Precedence, highest first: process environment, `%LOCALAPPDATA%\Oteryn\client.env`, the release `client.env` next to the running executable. The client reads both files itself. The release file is read-only payload; the per-user file is an operator/development override. Neither is the durable, versioned user-settings store of ALPHA-CLIENT-01 §15, which stays separate. `client.env` never holds a secret: the login test password that `login_local` writes as a comment is local-run evidence and must not appear in a shipped or CI-produced file.

### 2.4 Trust roots

The installer ships no trust root. `OTERYN_DEV_ROOT` stays a per-user, development-only setting that points at a locally generated root (`login_local`). Shipping a production gameplay trust root, or changing how the client validates the game endpoint, is a production-trust change that needs its owning contract (FND-02/ADR-0003 lineage) and is out of scope here.

## 3. Installer technology

**Inno Setup 6** (`ISCC.exe` command-line compiler), used as-is with a single checked-in script.

| Option | Why not first |
| --- | --- |
| MSIX | cannot install unsigned outside developer mode, so it blocks the unsigned first slice; adds package-identity and sandbox constraints |
| WiX / MSI (`cargo-wix`) | heavier authoring; per-user MSI has known quirks; `cargo-wix` targets WiX v3 by default; current WiX binaries carry the Open Source Maintenance Fee for revenue-generating users, an owner cost decision |
| NSIS | comparable capability, less safe script language |
| `cargo-packager` | younger; it wraps NSIS/WiX, adding a layer without removing one |

Inno Setup is mature, free for commercial use, supports per-user install without elevation (`PrivilegesRequired=lowest`), silent install and uninstall (needed by the updater and by CI), a built-in uninstaller and a provider-neutral signing hook (`SignTool`, section 6). Section 2.1's pointer write is the only Oteryn logic in the script (a `[Code]` step calling `MoveFileExW`); together with the launcher it is the only Oteryn install logic, and no custom installer framework is built. If an MSI is later required (enterprise deployment, Windows Installer transactional rollback), WiX replaces only the script; the layout, manifest and CI contract stay the same.

Source location: `apps/client/installer/` (the script and the release `client.env` templates per channel). It lives under the client package so the existing Windows lane classifier selects `rust_windows` when it changes.

## 4. Package manifest (`packages.json`)

```json
{
  "schema": "oteryn.client.packages.v1",
  "release": {
    "release_id": "0.1.0+dev.ci.g<12-hex>",
    "client_version": "0.1.0",
    "channel": "dev",
    "game_commit": "<40-hex Oteryn-Game commit>",
    "target": "x86_64-pc-windows-msvc"
  },
  "packages": []
}
```

Each `packages[]` entry, when one exists:

| Field | Rule |
| --- | --- |
| `id` | lowercase `[a-z0-9-]+`, unique in the manifest |
| `version` | the package's own semantic version, independent of `client_version` |
| `path` | relative to `releases\<release_id>\`, under `packages\`; no `..`, no absolute path |
| `size` | exact byte length |
| `sha256` | lowercase hex SHA-256 of the file |
| `provenance.source` | `oteryn-original`, `oteryn-generated` or `licensed-redistributable` |
| `provenance.source_ref` | Game commit and producing tool for original/generated content; the licence reference for licensed content |

Rules:

1. The client checks that `release.release_id`, `client_version`, `channel`, `game_commit` (all 40 hex digits) and `target` each equal the identity compiled into the running executable, and refuses to start on any mismatch, so a manifest from another build, route, channel or target is never accepted. It then verifies every listed package (path inside the release directory, exact size, SHA-256) before it loads any of them, and refuses to start on a mismatch, a missing file or an unknown `schema`. An empty `packages` list is valid; that is the first slice.
2. Third-party reference material (`AGENTS.md`: reference use does not grant redistribution) has no `provenance.source` value and therefore cannot be listed or shipped. The CI build rejects any file under `packages\` that is not listed in the manifest, any file in the release directory outside `packages\` other than the fixed release files `oteryn-client.exe`, `client.env` and `packages.json`, and any provenance value outside the list above. Adding a source class needs an owner decision recorded in this contract.
3. Packages are bundled in the installer while the client scene is synthetic and built in. The future asset pipeline is the extension point: it produces package files plus manifest entries in the same schema, and a later fetched-package mode downloads content-addressed files (`sha256` as the key) from the release endpoint (section 5) and verifies them against this manifest before activation. A new manifest field is additive within `v1`; a changed meaning is a new `schema`.
4. World and gameplay content stays server-authoritative (DUR-04, ADR-0021). Client packages are presentation assets only and never carry gameplay truth.

## 5. Updater model

### 5.1 Channels and endpoint ownership

Channels: `dev` (CI artifacts, operator use), `preproduction`, later `stable`. A channel is bound at build time through the release `client.env` template; the client never switches channel by itself.

Game owns what a release is: the installer, `packages.json`, `SHA256SUMS` and a release descriptor, all produced by Game CI from one exact commit. Platform owns distribution to players (web identity, commercial and control-plane responsibilities): hosting the files and serving the channel pointer. The channel pointer is a small document `{channel, sequence, release_id, client_version, installer_url, installer_sha256, minimum_supported_client_version}` with a strictly increasing `sequence`. Its endpoint and serving contract are a Platform-owned interface to agree under the cross-repository contract process; until it exists the updater is disabled and the client behaves exactly as today. No Game-hosted stand-in server is built.

### 5.2 Minimum first updater slice

At start-up, before login, if `OTERYN_UPDATE_URL` (release `client.env`) is set, the client fetches the channel pointer over HTTPS. A pointer whose `channel` differs from the running release's channel is ignored. Eligibility is decided only by the authorized `sequence`, never by version or `<release_id>` comparison.

Updater state lives in `%LOCALAPPDATA%\Oteryn\updater.json`, keyed by channel, because each channel has its own `sequence`. Per channel it holds two values:

- `accepted_sequence`: the highest sequence whose release this client has seen active. It is absent on a fresh install.
- `pending`: `{sequence, release_id}` of an update that was accepted but has not yet been confirmed.

The client changes this state only while it holds a named mutex `Global\OterynClientUpdater-<SID>`, and it writes the file through the same flush-and-`MoveFileExW` replace as `current.txt`. Every write of `accepted_sequence` takes the maximum of the stored and new values, so it never decreases.

A release is *active* when it is the running release and `current.txt` names its directory. A release started directly from a retained directory while `current.txt` names another one is running but not active. The rules are:

1. **Eligibility.** The high-water mark is the larger of `accepted_sequence` and `pending.sequence`; an absent value does not count. A pointer is eligible when either of these holds:
   - its `sequence` is greater than the high-water mark, or there is no high-water mark;
   - it is an exact retry of `pending`: the same `sequence` and the same `release_id`.

   A pointer with `pending.sequence` but a different `release_id` is ignored. Nothing is baselined before evaluation, so the first pointer a fresh install sees is evaluated like any other. A stale or replayed pointer below a failed pending update is therefore never installed.
2. **Already active.** An eligible pointer that names the active release records its `sequence` as `accepted_sequence`, clears `pending`, and installs nothing.
3. **Update or reactivation.** Any other eligible pointer is an update. This includes a lower `client_version` (an authorized downgrade, section 5.3) and a pointer to the running release when that release is not active. On acceptance the client:
   1. records `pending`; by rule 1 its `sequence` is never lower than an existing `pending`;
   2. downloads the installer to the per-user cache;
   3. verifies `installer_sha256` (and the Authenticode signature once section 6 is live);
   4. exits, and runs the installer silently with a relaunch flag.

   For a pointer to a retained release, the installer reuses the existing directory after verifying it and only repoints `current.txt`; a broken copy is replaced by a new directory (section 2.1).
4. **Confirmation.** At each start, before rules 1–3, the client compares `pending` with the active release:
   - if `pending.sequence` is not greater than `accepted_sequence`, `pending` is stale and is cleared;
   - otherwise, if the running release is active and equals `pending.release_id`, `accepted_sequence` becomes `pending.sequence` and `pending` is cleared.

   `accepted_sequence` advances only here and in rule 2, never on acceptance alone and never from a release that is running but not active.
5. **Retry.** A failed check, download, hash verification, install or relaunch leaves `accepted_sequence` and `pending` unchanged, so the same pointer stays eligible as an exact retry at the next start.
6. **Never blocks play.** None of these failures blocks play unless the running version is below `minimum_supported_client_version`. Platform and the Gateway remain the compatibility authority (`client_build`), per ALPHA-CLIENT-01 §17.3.

Updates never happen inside an active gameplay session (§17.1).

### 5.3 Rollback

- Local: a failed install leaves the previous release active (section 2.1). The previous release directory is kept. It can be started directly, which runs it without activating it, or reactivated by running its installer, which reuses the directory and repoints `current.txt`.
- Channel: rollback is a roll-forward. Platform republishes the previous payload under a new, higher `sequence`; clients on the bad version are then eligible under section 5.2 and install the older payload. A pointer whose `sequence` is not greater than `accepted_sequence` is ignored, which blocks replay of an old pointer. The installer accepts installing an older version over a newer one; activation just repoints `current.txt`. A client that the user started directly from the retained older release is not active, so it takes rule 3 rather than rule 2 and is reactivated. `accepted_sequence` never decreases.

## 6. Code signing seam

One hook: `apps/client/installer/sign.ps1 <file>`. Inno Setup calls it through `SignTool` for the installer and the uninstaller. The CI release job calls it explicitly for both shipped executables, `oteryn-client.exe` and `oteryn-launcher.exe`, before packaging. When signing is enabled (external alpha, D854), the release job verifies all four signatures (installer, uninstaller, client, launcher) and fails if any of them is missing or invalid. The release job builds with `<route>` = `rel` and publishes only signed output, so a `rel` release is always signed and never collides with the unsigned `ci` build of the same commit (section 2). Without signing configuration it exits 0 and signs nothing, so builds are unsigned and the artifact is labelled unsigned.

- Owner decision D854: builds stay unsigned until external alpha. Signing with a chosen provider is a required gate before external alpha (ALPHA-CLIENT-01 §17.2); the provider is chosen then, and the hook's body is the only provider-specific code.
- Credentials never enter the repository or a pull-request job. Signing runs only in a separate release job bound to a protected GitHub environment, using that environment's secrets or OIDC federation, on `main` commits that already passed the Merge Queue. `merge-gate.yml` never signs.
- - `packages.json` is generated per route and carries that route's own `<release_id>`, so the `ci` and `rel` manifests of one commit differ; only their package entries are identical. `SHA256SUMS` is recomputed after signing.
- A published `rel` artifact is immutable. Before building, the release job looks up the `<release_id>` in the release store; if an artifact already exists, the rerun publishes nothing and signs nothing, and that artifact stays the one payload for the `<release_id>`.

## 7. CI build

Owner direction: extend the `rust_windows` job of `merge-gate.yml`. After the existing release build and `--smoke`, the job:

1. installs Inno Setup at a pinned version, verifying the downloaded installer's SHA-256 before running it;
2. writes `packages.json` (empty `packages`, `game_commit` = the exact checked-out SHA) and the `dev` `client.env` template into the staging directory;
3. compiles the installer with `ISCC` (unsigned; `sign.ps1` no-ops) for `<release_id>` = `<client_version>+dev.ci.g<sha12>`;
4. silently installs into a temporary per-user directory, runs the installed `releases\<release_id>\oteryn-client.exe --smoke`, checks the layout of section 2.1, reinstalls the same installer (the existing release directory is reused), starts a second silent install while the first holds the transaction mutex and checks that it does not interleave, silently uninstalls, and checks that the install directory is gone and a pre-seeded per-user data directory is untouched;
5. writes `SHA256SUMS` (installer and `packages.json`) and uploads both with the installer as an Actions artifact (`actions/upload-artifact` pinned by commit SHA, short retention), with `permissions: contents: read` only.

Any failure fails `rust_windows` and therefore `game-gate`. Nothing is made optional, skipped or `continue-on-error`. `merge-gate.yml` is a protected workflow whose blob is pinned by `merge-authority-audit.yml`. Owner decision D854 authorizes that audit rotation for this extension; there is no separate installer workflow.

## 8. Slicing plan

Each slice is one PR, smallest first. Each runs the checks `CONTEXT_ROUTING.md` selects for its paths.

| Slice | Delivers | Owned paths | Validation |
| --- | --- | --- | --- |
| CLIENT-INSTALLER-1 | Unsigned installer artifact from CI: exe, launcher, `dev` `client.env`, empty `packages.json`, `<release_id>` compiled in, staged-rename install, atomic `current.txt` activation, the per-user transaction and client mutexes, uninstall | `apps/client/installer/**`; `apps/client/src/bin/oteryn-launcher.rs` and the `apps/client/Cargo.toml` bin entry; the `<release_id>` constant and `CLIENT_BUILD` in `apps/client/src/lib.rs` and the `OterynClient` mutex in `apps/client/src/main.rs`; `.github/workflows/merge-gate.yml` (`rust_windows` steps only) with the D854 `merge-authority-audit.yml` rotation; task record | `cargo fmt`/`clippy`/`test -p oteryn-client` with launcher tests for a missing, malformed and dangling `current.txt`; `rust_windows` green with the install/smoke/uninstall steps of section 7, including an interrupted-install case that leaves the previous release active, a same-`<release_id>` reinstall that keeps the previous distinct release, a reinstall that repairs a corrupted release directory, a repair that writes `<release_id>~1`, activates it and only then removes the broken directory, a concurrent second installer blocked by the transaction mutex, an uninstall refused while setup holds it, a client start refused while setup holds it, and an install refused while the client holds its mutex; `python tools/repository/validate_repository_policy.py` and matching `tools/repository/test_*.py` |
| CLIENT-INSTALLER-2 | Client reads `client.env` (section 2.3 precedence) and verifies `packages.json` (section 4 rule 1) | `apps/client/src/**` | `cargo fmt`, `cargo clippy -p oteryn-client`, `cargo test -p oteryn-client` with precedence, malformed-file, unknown-schema and hash-mismatch cases, and one rejected manifest for each mismatching `release` field; `rust_windows` |
| CLIENT-INSTALLER-3 | Package-manifest CI check: unlisted files and disallowed provenance fail the build | `apps/client/installer/**` | negative fixtures in the same job |
| CLIENT-INSTALLER-4 | Before external alpha (D854): signing release job behind a protected environment; `sign.ps1` body for the chosen provider | new release workflow, `apps/client/installer/sign.ps1` | installer, uninstaller, `oteryn-client.exe` and `oteryn-launcher.exe` each verify with `Get-AuthenticodeSignature`, and an unsigned launcher fails the job; a rerun for an already published `<release_id>` publishes and signs nothing; no secret in the repository or PR jobs; needs the provider choice |
| CLIENT-INSTALLER-5 | Updater first slice (section 5.2) and pointer anti-replay | `apps/client/src/**` | unit tests for sequence eligibility (including an authorized downgrade, a same-`client_version` release, an absent `accepted_sequence` and a pointer for another channel), a failed install retried at the next start, `pending` confirmation on relaunch, a stale `pending` cleared, a pointer below a failed `pending` rejected while its exact retry is accepted, a directly started non-active release reactivated by a rollback pointer, `accepted_sequence` never decreasing, hash mismatch, sequence replay and minimum-version; needs the Platform channel-pointer contract |
| later | Fetched packages from the asset pipeline | per section 4 rule 3 | when the asset pipeline exists |

Slice 1 is the playable-first minimum: it turns the existing release build into an installable, uninstallable artifact; its only Rust addition is the launcher. Until slice 2, the installed client reads its settings from the process environment as today.

## 9. Decisions

1. Code signing (owner, D854): unsigned until external alpha; signing is a required gate before external alpha (section 6, slice 4). The provider is chosen at that gate.
2. CI placement (owner, D854): extend `rust_windows` in `merge-gate.yml` under an owner-authorized `merge-authority-audit.yml` rotation; no separate installer workflow (section 7).
3. Channel-pointer endpoint (control plane, D607): Platform hosts it under a cross-repository contract (section 5.1); the updater stays disabled until that contract exists.

## 10. Deferred

MSI/enterprise deployment, delta patches, background or forced updates, Linux/macOS packaging, fetched-package CDN layout, user-data migration between versions, and crash-upload endpoints.
