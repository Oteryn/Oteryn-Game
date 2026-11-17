# Game client native session consumer

Authority: accepted Platform `OTERYN_V2_NATIVE_REMEMBERED_DEVICE_SESSION_CONTRACT.md`,
merged main commit `45d3413619f85a5289a221ee47412350761a56a9` (#1796). Public synthetic
vectors are copied byte-for-byte with their manifest and provenance under
`fixtures/remembered-device-v1`. The historical candidate/blocker does not govern
this implementation. Platform remains the credential, revocation and generation
owner; Game consumes its HTTP contract and the existing native Gateway ticket.
No Game-side purchased balance or new character-write authority is introduced.

## Choices and caller

Options > Miscellaneous / confirmations and login exposes process-session login,
explicit opt-in “Remember this device”, quick login, logout/exit confirmations,
reset confirmations, and native sign-out. The reference row is
`TIBIA-CLIENT-PARITY-2026-11-08.md:111`, CTX `optionsmenu_stay_logged_in`,
`qm-strings.txt:3459`. It identifies the surface without fixing Oteryn session rules;
the accepted Platform contract fixes those rules.

Owner decision B 2026-11-09, reference silent: quick login chooses the last
successfully selected character/channel only when the fresh owner feed and public
directory still mark that exact selection available. Otherwise it shows the normal
chooser. It does not recover an old actor, bypass admission, retry a predecessor,
or confuse WorldId with ChannelId. Game logout returns to character selection;
explicit native sign-out and entry cancellation revoke the family. Closing the
client preserves an explicitly remembered family; process-only state dies with it.
Logout/exit confirmations default on and may be disabled on the common page;
reset restores both. Existing transaction/payment/security confirmations are not
silently disabled by these local controls. Smoke automation bypasses message boxes.

`native_entry/authorize.rs` calls the consumer before browser bootstrap. A valid
retained credential rotates through the owner-character feed and single-use ticket
routes. An unavailable/expired/refused family is forgotten; browser PKCE is the
fallback. Fresh browser sign-in enrolls before normal OAuth ticket consumption.
No OAuth access/refresh token or password is persisted. The feature switch is
`OTERYN_NATIVE_REMEMBERED_DEVICES_ENABLED`, literal `true`, default off. It controls
client consumption, never Platform activation or production rollout.

## Retained state and serialization

The session class uses a fresh Ed25519 key and four-member enrollment proof in
process memory only. The remembered class uses the explicitly accepted original
unbound protocol-1 variant and stores only its device session in Windows Credential
Manager, scoped to the canonical trusted HTTPS origin plus OAuth client UUID. It
makes no device-bound recovery claim and stores no exportable remembered signing
key. Platforms without this OS-vault adapter offer process-only login.

An OS file lock and in-process mutex serialize the family across processes and
async operations. Persistent credentials are reloaded under that lock on each
attempt. The predecessor is removed from the vault before a potentially consuming
request. The validated successor is committed to the vault before characters or a
ticket reach their caller. Ambiguous/failed rotations clear local state and revoke
with the known predecessor where reachable; there is no automatic network retry.
A vault write failure revokes the new successor and returns no result. Explicit
sign-out deletes local state even when server revocation fails. Vault errors deny
credential use; they do not fall back to plaintext files.

Only lock files, a non-secret last selection, and local option booleans go to the
ordinary configuration directory. Session options are global, outside character
profiles, and excluded from settings exports/imports. Files contain no credentials,
keys, OAuth tokens or ticket responses. Typed secrets redact Debug. Request errors
never echo credential-bearing inputs or response bodies.

The production HTTP adapter enforces HTTPS even on loopback, exact headers/bodies,
no redirects/retries, bounded responses, canonical credentials, family/absolute
expiry continuity, changed successors, and fresh bounded ticket deadlines. Tests
use an explicit private test CA without disabling hostname/TLS verification.

Platform feature activation, OS-vault runtime qualification on supported desktop
hosts, TLS/redaction deployment and recovery DB-9 review remain enablement gates;
this row supplies the consumer, not production rollout or restart actor recovery.
