`policy-v1.json` is a synthetic Platform policy example, based on the schema in
`docs/architecture/adr/0035-first-party-client-distribution-and-updater-trust-boundary.md`
and `app/Downloads/Updater/UpdaterPolicyDocument.php` in Oteryn/Oteryn-Platform
at d06a078fc545c68626899265801e93857fa4d43c (the task's main baseline). It contains no production data.

Game compatibility inspection: Oteryn/Oteryn-Game at full SHA
857f15f376907c592bb28d854fccff18a15cdf87,
`apps/client/src/bin/oteryn-launcher.rs`, `apps/client/src/lib.rs`,
`apps/client/src/native_entry.rs`, `crates/platform-client/src/lib.rs`.
The launcher only reads `current.txt` and launches an installed executable.
No TUF metadata consumer or updater policy parser exists in those sources or
elsewhere in the inspected Rust application/crate sources at that commit.
No Game fixture was copied; this fixture does not prove launcher compatibility.

`origin-v1/` contains synthetic signed stable/beta bundles produced by the
downloads test signer. All private keys were generated in memory and discarded.
The public Roots expire in 2099 for testing only; none is a production anchor.
