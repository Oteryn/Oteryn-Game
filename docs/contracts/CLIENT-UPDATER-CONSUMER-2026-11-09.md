# Game launcher consumer of Platform update origin v1

Authority: accepted `OTERYN_V2_CLIENT_UPDATE_ORIGIN_CONTRACT.md`, copied from
Platform PR 1795, commit 3f7425341722c2a0dd1d3ea6e14924021a8f2c13.

The launcher accepts `--check-updates`, `--verify-release`, `--update`, `--repair`,
and a Windows `--updates` dialog. Stable is default; explicit `--beta` selects beta
for CLI maintenance with independent trust state. There is no channel fallback.
Build inputs `OTERYN_CLIENT_TUF_ROOT_STABLE` and `OTERYN_CLIENT_TUF_ROOT_BETA` name
reviewed public root JSON files embedded in the resulting executable. Missing
roots disable their update path, not gameplay. Runtime metadata cannot bootstrap
trust. Fixture roots are test inputs only. Production root custody, DNS/TLS and
artifact storage are operations and do not block completing this consumer row.

TUF metadata uses tough 0.18 with expiry enforcement, sequential root rotation,
threshold verification, consistent snapshots and persistent channel-specific
rollback state. Verified policy revision and release sequence cannot regress;
a signed explicit rollback authorization permits a lower release sequence.
An updated pinned anchor needs a reviewed build and migration of the trust state;
a mismatched anchor refuses rather than silently resetting rollback protection.

Owner decision B 2026-11-09, reference silent: the installable artifact is a ZIP
whose root contains `oteryn-client.exe` and its runtime/package files. ZIP paths
are relative; traversal, symlinks, duplicates and case aliases refuse. Signed
package length bounds streaming; disk capacity is checked before download and
before expansion. A failed verification never activates bytes. Installation uses
a fresh release slot (retaining the previous release), syncs files and atomically
switches `current.txt` only after comparison against every archive entry succeeds.
Repair always installs a fresh slot. Local verification downloads the current
signed package and compares installed bytes, rather than trusting mutable checksums.
A stale release requires update; verification never certifies it as current.
The install directory is a trusted, per-user operator-selected location. Windows
maintenance shares the existing setup/client mutex handshake and refuses while
play or another install is active; a filesystem lock serializes updater processes.
