# First-party client update origin v1

Status: **ACCEPTED** — owner decision 2026-11-09, Oteryn-Game #3058,
option B, Game lane T005. Implements ADR 0035 and Platform plan T35.
This contract does not claim deployment or client conformance.

## Origins and paths

Public metadata origin: `https://account.oteryn.com`.
Each C below is exactly `stable` or `beta`, with independent trust and no fallback.
N is the positive decimal role version; H is the lowercase SHA-256 of target bytes.

| Resource | Exact URL template |
| --- | --- |
| Root rotation | `https://account.oteryn.com/updates/C/metadata/N.root.json` |
| Targets | `https://account.oteryn.com/updates/C/metadata/N.targets.json` |
| Snapshot | `https://account.oteryn.com/updates/C/metadata/N.snapshot.json` |
| Timestamp | `https://account.oteryn.com/updates/C/metadata/timestamp.json` |
| Channel delegation | `https://account.oteryn.com/updates/C/metadata/N.C.json` |
| Active policy convenience URL | `https://account.oteryn.com/updates/C/policy-v1.json` |
| Verified policy target | `https://account.oteryn.com/updates/C/targets/H.channels/C/policy-v1.json` |

Metadata bases are `/updates/stable/metadata/` and `/updates/beta/metadata/`.
Policy target bases are `/updates/stable/targets/` and `/updates/beta/targets/`.
Compatibility aliases `root.json`, `targets.json`, `snapshot.json` and `C.json`
resolve the active head. Versions come from verified parent metadata.

Package storage origin is `https://downloads.oteryn.com` with target bases
`/updates/stable/targets/` and `/updates/beta/targets/`.
For signed target path `channels/C/releases/R/P/A/F`, the consistent-snapshot URL is
`https://downloads.oteryn.com/updates/C/targets/channels/C/releases/R/P/A/H.F`.
R, P, A and F are exact signed release/platform/architecture/filename segments.
Platform serves policy targets only, never package bytes. Clients must route policy
and package reads to the separate bases; a single generic target base is insufficient.
Stage immutable packages before metadata activation. Missing packages never fall
back to another origin, channel, release, architecture or platform.

## Initial trust and rotation

The initial channel-specific public Root JSON is shipped inside the client build,
pinned through its reviewed release process. There is **no trust-on-first-use**.
Fetching `1.root.json` or `root.json` over HTTPS MUST NOT bootstrap trust.
`N.root.json` is served for rotation from an already trusted Root only.

Rotate sequentially N to N+1, verifying both the previous Root threshold and the
new Root self-signature threshold, versions, role keys and TUF expiry rules before
persisting trusted state. Never skip intermediate Roots, decrease versions or
replace trust with an uploaded Root. The bounded publication profile includes the
exact configured anchor and all intermediate Roots, allows 16 rotations per bundle
and requires consistent snapshots. Root custody is offline and threshold-controlled.
Threshold Root compromise requires out-of-band recovery through a new trusted
client release. Routine publication cannot repair it. Targets delegates exactly
one terminating channel role scoped to `channels/C/*`.

TUF signatures, parent hashes/lengths, expiry and rollback checks establish trust.
Policy and artifacts MUST match verified TUF target hashes and length before use
or installation. HTTPS is transport only; HTTP success or an operator checksum
cannot establish publisher authority. Private keys never touch Platform. Signing
and repository generation occur outside Platform; only public signed bytes enter it.

## HTTP and size contract

Metadata and policy use `Content-Type: application/json` and
`X-Content-Type-Options: nosniff`, preserving exact uploaded bytes.
Versioned files and hash-addressed policy targets use
`Cache-Control: public, max-age=31536000, immutable, no-transform`.
Mutable aliases, active policy and `timestamp.json` use
`Cache-Control: no-store, no-transform`.
Storage packages use `application/octet-stream` and the immutable cache directive.
Storage must preserve exact bytes. Package size is bounded by the signed target
length; there is no additional fixed package cap. Clients must stream with that
bound and check available disk before installation.

Publication permits 32 metadata files maximum, 1 MiB each, 8 MiB total; policy
is at most 256 KiB and configured public Root at most 1 MiB. Empty files,
non-files and symlink file entries are refused. Bundle directories are trusted
local operator input. Bundles contain no packages or private keys.

`OTERYN_DOWNLOADS_ENABLED` defaults off; only literal `true` enables it.
Disabled routes return 404 without connecting to the database. An unpublished
channel, missing object or expired active head returns 404. Immutable historical
files remain readable after expiry. Invalid channel/path input returns 400;
storage/internal failures return 503. Errors carry `Cache-Control: no-store`.
Artifact storage returns 404 for absent objects and 503 for outages. Clients treat
both as unavailable and never bypass verification or fall back to another channel.

## Operator publication

Prepare `metadata/` with complete signed role files and `policy-v1.json` with exact
policy target bytes. Configure `OTERYN_DOWNLOADS_ROOT_STABLE` or
`OTERYN_DOWNLOADS_ROOT_BETA` as a path to the pinned public Root, plus the existing
`OTERYN_DATABASE_URL` deployment configuration. Enable the flag explicitly, then run:

```text
oteryn-portal downloads-publish --channel stable --dir <bundle> --actor <staff-UUIDv7>
```

OS command and database access are prerequisites. The actor UUID is an audit
identity, not authentication by itself: the command checks an active account and
the existing staff/admin permission `downloads.manage`. `Downloads::publish`
rechecks access, verifies against the configured anchor and atomically records
history/actor and activates the generation. Flag off, bad arguments, unauthorized
actors, wrong Root, bad signatures, expiry, replay or inconsistent metadata fail
with nonzero exit status and no activation. Success prints the publication UUID.
There is no signing, public mutation endpoint or implicit migration.

## Fixtures and release gates

`fixtures/client-update-origin-v1/manifest.json` records exact byte counts and
SHA-256 for synthetic fixtures copied from `crates/downloads/tests/fixtures`.
These include signed synthetic stable/beta bundles generated with the downloads
test signer and copied byte-for-byte from its fixture directory. They are not
production trust anchors. Signing keys were generated in memory and discarded;
no private keys are persisted.
Real Root provisioning, DNS/TLS/storage deployment, availability and actual client
pinning/rotation/installation evidence remain separate release gates.
