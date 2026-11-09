# T067 — native session consumer compatibility

Platform main inspected at `bc6f88346e6aaf1347f0a5a798b33a5ede00bc04` on 2026-11-09.
T067 remains PARTIAL; no working quick-login or stay-logged-in claim is made.

## Missing Platform contract

An accepted, operational native session-family enrollment and rotation contract
for the `session` and `remembered` persistence classes is missing. The named
`POST /api/v1/game-auth/device-sessions`, `/native-characters`, `/tickets`, and
`/revoke` routes exist as candidates; their existence does not establish this
session-class contract or a working Rust authority.

Evidence at the pinned Platform revision:

- [Native session candidate](https://github.com/Oteryn/Oteryn-Platform/blob/bc6f88346e6aaf1347f0a5a798b33a5ede00bc04/docs/contracts/NATIVE_REMEMBERED_DEVICE_SESSION_CONTRACT_CANDIDATE.md#L333):
  DB-1 is explicitly unaccepted; it requires a process-only session family and
  opt-in remembered family with a device key, replacing the older opt-in-only flow.
- [Rust device router](https://github.com/Oteryn/Oteryn-Platform/blob/bc6f88346e6aaf1347f0a5a798b33a5ede00bc04/portal-rs/crates/gameauth/src/devices_http.rs#L69):
  `routes` installs `UnavailableAuthority` even when the feature flag is enabled.
- [PHP controller](https://github.com/Oteryn/Oteryn-Platform/blob/bc6f88346e6aaf1347f0a5a798b33a5ede00bc04/app/Http/Controllers/GameAuth/DeviceSessions/NativeDeviceSessionController.php#L44):
  enrollment requires `explicitConsent: true` and returns protocol version 1;
  it does not implement session-class enrollment with device proof.
- [Strict enrollment decoder](https://github.com/Oteryn/Oteryn-Platform/blob/bc6f88346e6aaf1347f0a5a798b33a5ede00bc04/app/Http/Controllers/GameAuth/DeviceSessions/NativeDeviceSessionRequest.php#L44):
  accepts exactly `protocol_version: 1` and `remember_device: true`; it rejects
  session persistence and device-proof enrollment members.
- [Native authentication contract](https://github.com/Oteryn/Oteryn-Platform/blob/bc6f88346e6aaf1347f0a5a798b33a5ede00bc04/docs/contracts/OTCLIENT_GAME_AUTH_CONTRACT.md#L178):
  the client must not persist or use an OAuth refresh token. Substituting the
  general OAuth refresh grant would violate the native bootstrap contract.

The Game consumer cannot supply Platform authority or invent the missing
versioned enrollment request. Existing individual confirmations remain available;
the combined T067 row cannot be marked DONE. No credentials or candidate keys
were persisted, and no disabled candidate was activated.
