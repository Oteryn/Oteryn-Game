# Installed package verification

The client checks `packages.json` before creating the login runtime or loading
presentation packages. The manifest schema is `oteryn.client.packages.v1` from
[CLIENT-INSTALLER-0 section 4](../../../docs/architecture/CLIENT-INSTALLER-0_WINDOWS_CLIENT_INSTALLER_CONTRACT_CANDIDATE.md#4-package-manifest-packagesjson).

All five release fields must match the running executable: release ID, client
version, channel, full 40-digit Game commit and target triple. The build script
captures the real Git commit and Cargo target; a release build cannot substitute
an unknown commit. An empty package list still undergoes every identity check.

Each package has a unique lowercase ID, semantic version, recognized provenance
source and nonempty reference. Its canonical file path must remain under the
release's `packages/` directory. Absolute paths, parent traversal, alternate
streams and symlink escapes are rejected. The client checks exact file size and
streams SHA-256 before permitting startup. Malformed JSON, an unknown schema,
missing files, wrong identity or corruption refuses startup and explains how to
repair the installation with the matching trusted installer.

Input budgets are a 1 MiB manifest, at most 1024 packages and IDs of at most 128
bytes. Hashing uses a fixed 64 KiB buffer. Only an explicitly compiled local
`dev.local` build may start without a manifest. If one exists it is checked.
`oteryn-client --check-packages` always requires a manifest, prints the result and
exits before login or native UI. It never contacts a service.

`oteryn-launcher --check-install` runs the matching active client's checker, so
a repaired directory such as `<release_id>~1` uses its own executable identity.
A retained older release is not compared to the newer launcher's identity or
started with a potentially unsupported new diagnostic flag. Diagnostics are
bounded and direct the player to the active release's trusted installer.

Automatic installer download/execution is **not enabled**. The distribution
interface is owned by Platform. CLIENT-INSTALLER-0 section 5.1 says:

> until it exists the updater is disabled and the client behaves exactly as today. No Game-hosted stand-in server is built.

An accepted Platform channel-pointer endpoint and serving contract remain
necessary to implement update/repair execution; local package verification does
not create distribution authority or a replacement service.
