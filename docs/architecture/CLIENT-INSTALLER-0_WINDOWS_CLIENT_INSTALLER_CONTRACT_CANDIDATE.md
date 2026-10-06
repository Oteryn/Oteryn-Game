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
- Supersedes: the ALPHA-CLIENT-01 §17.4 line "Installer/updater technology remains deferred" (installer technology only; §17.1–§17.4 rules stay binding)
- Runtime authorization: **NONE**

## 1. Purpose

The owner wants a real installer for the native Windows client that installs the client and its packages, not a bare exe. This candidate fixes the distribution unit, the installer technology, the package manifest, the updater model, the code-signing seam, the CI build and the implementation slices. It does not change protocol, identity, admission, persistence or production trust.

## 2. Distribution unit and layout

One release is one installer, `oteryn-client-<version>-x86_64-setup.exe`, built from one exact Game commit. `<version>` is the `oteryn-client` crate version (today the workspace version); the client reports it as `client_build` (`oteryn-client/<version>`).

### 2.1 Install directory (release payload, replaced by updates)

Per-user install, no elevation: `%LOCALAPPDATA%\Programs\Oteryn\`.

```text
%LOCALAPPDATA%\Programs\Oteryn\
  current.txt                    the one active <version>, written at activation
  releases\<version>\
    oteryn-client.exe
    client.env                   release defaults (section 2.3)
    packages.json                package manifest (section 4)
    packages\<id>-<version>.<ext>  content/asset packages listed in packages.json (none today)
  unins000.exe / unins000.dat    upstream uninstaller
```

A release directory is written complete and never modified afterwards. Activation (ALPHA-CLIENT-01 §17.3) is the last install step: only after every file of `releases\<version>\` is in place does the installer's post-install step point the Start-menu shortcut at `releases\<version>\oteryn-client.exe` and replace `current.txt`. An aborted install is rolled back by the installer and leaves the shortcut and `current.txt` on the previous complete release. After successful activation, the installer deletes every other `releases\*` directory except the immediately previous one, which is kept for local rollback (section 5.3). There is no launcher process.

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

Precedence, highest first: process environment, `%LOCALAPPDATA%\Oteryn\client.env`, `releases\<current>\client.env`. The client reads both files itself. The release file is read-only payload; the per-user file is an operator/development override. Neither is the durable, versioned user-settings store of ALPHA-CLIENT-01 §15, which stays separate. `client.env` never holds a secret: the login test password that `login_local` writes as a comment is local-run evidence and must not appear in a shipped or CI-produced file.

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

Inno Setup is mature, free for commercial use, supports per-user install without elevation (`PrivilegesRequired=lowest`), silent install and uninstall (needed by the updater and by CI), a built-in uninstaller and a provider-neutral signing hook (`SignTool`, section 6). Section 2.1's activation step is the only Oteryn logic in the script (`[Code]` post-install step); no custom installer framework is built. If an MSI is later required (enterprise deployment, Windows Installer transactional rollback), WiX replaces only the script; the layout, manifest and CI contract stay the same.

Source location: `apps/client/installer/` (the script and the release `client.env` templates per channel). It lives under the client package so the existing Windows lane classifier selects `rust_windows` when it changes.

## 4. Package manifest (`packages.json`)

```json
{
  "schema": "oteryn.client.packages.v1",
  "release": {
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
| `path` | relative to `releases\<version>\`, under `packages\`; no `..`, no absolute path |
| `size` | exact byte length |
| `sha256` | lowercase hex SHA-256 of the file |
| `provenance.source` | `oteryn-original`, `oteryn-generated` or `licensed-redistributable` |
| `provenance.source_ref` | Game commit and producing tool for original/generated content; the licence reference for licensed content |

Rules:

1. The client verifies every listed package (path inside the release directory, exact size, SHA-256) before it loads any of them, and refuses to start on a mismatch, a missing file or an unknown `schema`. An empty `packages` list is valid; that is the first slice.
2. Third-party reference material (`AGENTS.md`: reference use does not grant redistribution) has no `provenance.source` value and therefore cannot be listed or shipped. The CI build rejects any packaged file that is not listed in the manifest, and any provenance value outside the list above. Adding a source class needs an owner decision recorded in this contract.
3. Packages are bundled in the installer while the client scene is synthetic and built in. The future asset pipeline is the extension point: it produces package files plus manifest entries in the same schema, and a later fetched-package mode downloads content-addressed files (`sha256` as the key) from the release endpoint (section 5) and verifies them against this manifest before activation. A new manifest field is additive within `v1`; a changed meaning is a new `schema`.
4. World and gameplay content stays server-authoritative (DUR-04, ADR-0021). Client packages are presentation assets only and never carry gameplay truth.

## 5. Updater model

### 5.1 Channels and endpoint ownership

Channels: `dev` (CI artifacts, operator use), `preproduction`, later `stable`. A channel is bound at build time through the release `client.env` template; the client never switches channel by itself.

Game owns what a release is: the installer, `packages.json`, `SHA256SUMS` and a release descriptor, all produced by Game CI from one exact commit. Platform owns distribution to players (web identity, commercial and control-plane responsibilities): hosting the files and serving the channel pointer. The channel pointer is a small document `{channel, sequence, client_version, installer_url, installer_sha256, minimum_supported_client_version}` with a strictly increasing `sequence`. Its endpoint and serving contract are a Platform-owned interface to agree under the cross-repository contract process; until it exists the updater is disabled and the client behaves exactly as today. No Game-hosted stand-in server is built.

### 5.2 Minimum first updater slice

At start-up, before login, if `OTERYN_UPDATE_URL` (release `client.env`) is set: fetch the channel pointer over HTTPS; if `client_version` is newer than the running one, offer the update. On acceptance, download the installer to the per-user cache, verify `installer_sha256` (and the Authenticode signature once section 6 is live), exit the client, and run the installer silently with a relaunch flag. A failed check or download never blocks play unless the running version is below `minimum_supported_client_version`; Platform and the Gateway remain the compatibility authority (`client_build`), per ALPHA-CLIENT-01 §17.3. Updates never happen inside an active gameplay session (§17.1).

### 5.3 Rollback

- Local: a failed install leaves the previous release active (section 2.1). The previous release directory is kept, so it can be started directly, or reactivated by reinstalling its installer.
- Channel: rollback is a roll-forward. Platform republishes the previous payload under a new, higher `sequence`. The client rejects a pointer whose `sequence` is lower than the last one it accepted (stored in the per-user data directory), which blocks replay of an old pointer.

## 6. Code signing seam

One hook: `apps/client/installer/sign.ps1 <file>`. Inno Setup calls it through `SignTool` for the installer and the uninstaller; the CI release job calls it for `oteryn-client.exe` before packaging. Without signing configuration it exits 0 and signs nothing, so builds are unsigned and the artifact is labelled unsigned.

- The signing provider is an owner decision (owner question 1). The hook's body is the only provider-specific code.
- Credentials never enter the repository or a pull-request job. Signing runs only in a separate release job bound to a protected GitHub environment, using that environment's secrets or OIDC federation, on `main` commits that already passed the Merge Queue. `merge-gate.yml` never signs.
- Signed and unsigned builds of one commit have the same `packages.json`; `SHA256SUMS` is recomputed after signing.

## 7. CI build

Owner direction: extend the `rust_windows` job of `merge-gate.yml`. After the existing release build and `--smoke`, the job:

1. installs Inno Setup at a pinned version, verifying the downloaded installer's SHA-256 before running it;
2. writes `packages.json` (empty `packages`, `game_commit` = the exact checked-out SHA) and the `dev` `client.env` template into the staging directory;
3. compiles the installer with `ISCC` (unsigned; `sign.ps1` no-ops);
4. silently installs into a temporary per-user directory, runs the installed `releases\<version>\oteryn-client.exe --smoke`, checks the layout of section 2.1, silently uninstalls, and checks that the install directory is gone and a pre-seeded per-user data directory is untouched;
5. writes `SHA256SUMS` (installer and `packages.json`) and uploads both with the installer as an Actions artifact (`actions/upload-artifact` pinned by commit SHA, short retention), with `permissions: contents: read` only.

Any failure fails `rust_windows` and therefore `game-gate`. Nothing is made optional, skipped or `continue-on-error`. Constraint: `merge-gate.yml` is a protected workflow whose blob is pinned by `merge-authority-audit.yml`; changing it needs the owner-authorized audit rotation (owner question 2).

## 8. Slicing plan

Each slice is one PR, smallest first. Each runs the checks `CONTEXT_ROUTING.md` selects for its paths.

| Slice | Delivers | Owned paths | Validation |
| --- | --- | --- | --- |
| CLIENT-INSTALLER-1 | Unsigned installer artifact from CI: exe, `dev` `client.env`, empty `packages.json`, activation step, uninstall | `apps/client/installer/**`; `.github/workflows/merge-gate.yml` (`rust_windows` steps only, after audit rotation); task record | `rust_windows` green with the install/smoke/uninstall steps of section 7; `python tools/repository/validate_repository_policy.py` and matching `tools/repository/test_*.py` |
| CLIENT-INSTALLER-2 | Client reads `client.env` (section 2.3 precedence) and verifies `packages.json` (section 4 rule 1) | `apps/client/src/**` | `cargo fmt`, `cargo clippy -p oteryn-client`, `cargo test -p oteryn-client` with precedence, malformed-file, unknown-schema and hash-mismatch cases; `rust_windows` |
| CLIENT-INSTALLER-3 | Package-manifest CI check: unlisted files and disallowed provenance fail the build | `apps/client/installer/**` | negative fixtures in the same job |
| CLIENT-INSTALLER-4 | Signing release job behind a protected environment; `sign.ps1` body for the chosen provider | new release workflow, `apps/client/installer/sign.ps1` | signed artifact verifies with `Get-AuthenticodeSignature`; no secret in the repository or PR jobs; needs owner question 1 |
| CLIENT-INSTALLER-5 | Updater first slice (section 5.2) and pointer anti-replay | `apps/client/src/**` | unit tests for version compare, hash mismatch, sequence regression and minimum-version; needs the Platform channel-pointer contract |
| later | Fetched packages from the asset pipeline | per section 4 rule 3 | when the asset pipeline exists |

Slice 1 is the playable-first minimum: it turns the existing release build into an installable, uninstallable artifact without client code changes. Until slice 2, the installed client reads its settings from the process environment as today.

## 9. Owner questions

1. Code-signing provider. a) a cloud signing service with Microsoft-managed identity validation (for example Azure Trusted Signing); b) an OV/EV certificate in a cloud HSM (for example DigiCert KeyLocker or SSL.com eSigner); c) stay unsigned until external alpha. Recommendation: c now, a before external alpha (lowest cost, no key custody, works through the section 6 hook).
2. CI placement of slice 1. a) extend `rust_windows` in `merge-gate.yml` with an owner-authorized `merge-authority-audit.yml` rotation; b) build the installer in a separate non-gate workflow first and move it into `rust_windows` later. Recommendation: a, as directed, because it gates the installer on every Windows-affecting change.
3. Release channel pointer endpoint. a) Platform hosts it under a cross-repository contract (section 5.1); b) Game publishes it as a GitHub Release asset until Platform has it. Recommendation: a; the updater stays disabled until then.

## 10. Deferred

MSI/enterprise deployment, delta patches, background or forced updates, Linux/macOS packaging, fetched-package CDN layout, user-data migration between versions, and crash-upload endpoints.
