# T005 — Platform updater compatibility evidence

Resolved by owner 2026-11-09: Platform PR #1795 supplies the accepted
`docs/contracts/OTERYN_V2_CLIENT_UPDATE_ORIGIN_CONTRACT.md`. The consumer is now
`apps/client/src/updater.rs` and `crates/platform-client/src/updates.rs`; public
Root provisioning, DNS/TLS/storage are operations. The inspection below is
historical evidence of the previously missing input, not a current blocker.

Checked 2026-11-09 against Oteryn/Oteryn-Platform main commit
`bc6f88346e6aaf1347f0a5a798b33a5ede00bc04`.

## Correction to the previous blocker

Platform **does have** an accepted distribution architecture and a Rust metadata
GET implementation. The earlier search for `installer_sha256` only found the
Game candidate because Platform uses TUF target metadata, not that unsigned
channel-pointer shape. Absence of that field does not prove absence of an
updater interface.

The accepted authority is
[Platform ADR 0035](https://github.com/Oteryn/Oteryn-Platform/blob/bc6f88346e6aaf1347f0a5a798b33a5ede00bc04/docs/architecture/adr/0035-first-party-client-distribution-and-updater-trust-boundary.md).
[CLIENT_DISTRIBUTION_ARCHITECTURE](https://github.com/Oteryn/Oteryn-Platform/blob/bc6f88346e6aaf1347f0a5a798b33a5ede00bc04/docs/architecture/CLIENT_DISTRIBUTION_ARCHITECTURE.md)
requires publisher-authenticated TUF roles, durable freshness/rollback protection,
exact channel/platform/architecture selection, and separation from Game admission.
HTTPS or a browser-supplied checksum alone is not update authority.

## Existing implementation

The [Downloads README](https://github.com/Oteryn/Oteryn-Platform/blob/bc6f88346e6aaf1347f0a5a798b33a5ede00bc04/portal-rs/crates/downloads/README.md)
and [HTTP router](https://github.com/Oteryn/Oteryn-Platform/blob/bc6f88346e6aaf1347f0a5a798b33a5ede00bc04/portal-rs/crates/downloads/src/http.rs)
define these GET routes, disabled in the binary unless the downloads flag is enabled:

- `/updates/{channel}/metadata/{filename}`;
- `/updates/{channel}/policy-v1.json`;
- `/updates/{channel}/targets/{target}` for consistent-snapshot policy bytes.

[Verification](https://github.com/Oteryn/Oteryn-Platform/blob/bc6f88346e6aaf1347f0a5a798b33a5ede00bc04/portal-rs/crates/downloads/src/verification.rs)
uses `tough` with bounded public metadata, Root/Targets/Snapshot/Timestamp and a
channel-scoped delegation. The convenience policy endpoint is not independently
trusted. Package bytes must come from a separately supplied distribution origin;
Platform does not proxy them. No production endpoint activation is proved here.

## Compatibility inputs still needed

1. The public Root bootstrap and agreed client TUF profile. No production trust
   anchor is present in Game; inventing one or trusting web metadata as Root
   would change the accepted trust boundary.
2. Explicit channel mapping. Platform's [policy types](https://github.com/Oteryn/Oteryn-Platform/blob/bc6f88346e6aaf1347f0a5a798b33a5ede00bc04/portal-rs/crates/downloads/src/domain.rs)
   accept `stable` and `beta`; Game's installer candidate uses `dev`,
   `preproduction` and `stable`. Implicit fallback is forbidden by ADR 0035.
3. Release and activation identity mapping. Platform release IDs are opaque
   identifiers whose grammar excludes `+`; Game compiled release IDs include
   `+`. Neither equality nor version ordering can be assumed. Installer format,
   installation confirmation and recovery must bind the selected trusted target
   to the actual active Game release.
4. The artifact distribution origin and its agreement with that profile.
   The synthetic Platform fixture uses a ZIP target; it is not a contract for
   Game's executable installer.

The README explicitly states:

> The TUF profile, route names and policy parser compatibility require a real
> launcher source/contract before enabling distribution. No production activation
> or client compatibility is claimed.

Its [fixture provenance](https://github.com/Oteryn/Oteryn-Platform/blob/bc6f88346e6aaf1347f0a5a798b33a5ede00bc04/portal-rs/crates/downloads/tests/fixtures/SOURCE.md)
likewise states that the fixture does not prove launcher compatibility.

## Delivered Game slice and row status

Game's startup package identity/size/SHA-256 verifier and active-release launcher
diagnostic remain implemented and tested through real executable callers:
`executable_verifies_packages_then_rejects_same_size_corruption` and
`launcher_diagnosis_runs_the_active_release_verifier` in
`apps/client/tests/package_integrity.rs`.

T005 remains PARTIAL for TUF client trust state and update/repair execution.
This is a cross-repository profile/bootstrap/identity compatibility gap, **not**
a claim that Platform has no metadata implementation. No capability, protocol
command, private signing key, stand-in service or production activation is added.
