# CLRED client reducer contracts

## E12 — command 220 hireling

- Encoder: `crates/gameplay/dispatch/src/gameplay_transport/connection/routing/hireling.rs`; canonical reply codec: `protocol-oteryn/src/hireling.rs` (`encode_reply` / `decode_reply`). Exactly one byte, 0/1. No new fields or wire changes.
- Public attachment: `client_runtime::hireling::HirelingState::{apply, from_snapshot}`. `apply` takes command status and raw reply bytes. There is no hireling state domain or admission snapshot; `from_snapshot` takes an optional retained canonical reply, and no reply leaves `active` unknown.
- Accepted / duplicate replay update active state; rejected / expired outcome / malformed replies preserve it. Capability 165 is required for updates.
- Existing native hireling panel uses `apply_response`, sharing the same reducer for its already-decoded transport response. Capability 250 management remains independent.
- Test: `hireling::state::tests::hireling_reply_apply_snapshot_and_refusals` covers canonical bytes, replay, restoration, missing capability, malformed bytes, and refusal atomicity.
- No pins changed.
