# CL-006 — Overlong guild name from the native form terminates the play session

Status: **CONFIRMED P2 by independent lead source review.**
Base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`.

## Concrete trigger and trace

In a live admitted Windows session with guild capability40 selected and no guild request pending, open the guild form, enter30 ASCII characters into the name field and press Found. `apps/client/src/features/0040-guild/native.rs:108-115` creates the unrestricted edit control; `control_text` at204-209 reads up to63 UTF16 units. The action at161 constructs `Intent::Found` without validating the wire name limit. The UI request builder (`crates/client-ui/src/features/0040-guild/mod.rs:32-76`) accepts it, sets pending and returns a raw GuildCommand. The native caller173-174 puts it on the actual play queue.

`apps/client/src/features/0040-guild/protocol.rs:8-14` invokes `session.guild(command)`; its actual implementation (`apps/client/src/play/session_adapter.rs:287-292`) calls `oteryn_client_runtime::guild::send`. Runtime `features/0040-guild/protocol.rs:27-39` encodes the request. `crates/protocol-oteryn/src/guild/codec.rs:104-115` rejects names longer than29 UTF8 bytes. Runtime maps that local encoding failure to `SessionError::CapabilityNotSelected`, before any server packet is sent.

The queue loop returns this error from its Guild arm (`apps/client/src/play/session_loop.rs:331-338`). The common tail accepts only `ActivityScopeChanged` as recoverable; every other error calls `outbox.end(public_class(&ended))` and returns (`399-406`). Thus a normal invalid form value ends the admitted client session rather than displaying a local field error. `Outbox.end` clears gameplay UI state (`play/mailbox/operations.rs:109-164`).

## Counterguards and limits

The selected and pending guards do not bound name length. Win32 input truncation at63 UTF16 units still permits the30 ASCII trigger; there is no29-byte field restriction in the fully inspected native module. The wire limit itself is correct and prevents malformed server requests. Empty names are **not** the claimed trigger: this codec allows them and the server may provide an ordinary business refusal. Invite names over29 bytes and invalid rank values may share the path, but the minimal finding needs only Found with30 ASCII characters and no existing guild.

Impact: local user input unnecessarily disconnects play and requires reentry. No server mutation, durable item/character loss or remote denial-of-service vector is claimed.

Validation: static full native/UI/runtime caller trace and exact encoder check. No Windows execution, Rust suite or SQL run was performed.

## Independent lead verification

The lead independently read the entire native guild form, client UI builder, runtime encoder adapter, native result adapter and opening input/view modules, plus exact codec and terminal session-loop ranges. The real Windows shell calls guild_input (windows_shell/input.rs105); open_guild refreshes membership and calls the native opener without pre-setting its Windows open flag. After that initial request completes, Found with30 ASCII is accepted locally and reaches the29-byte encoder guard; its error leaves guild_run and the common loop ends play. The existing selected/pending guards do not cap name length. No server packet, server mutation or native Windows execution is claimed.
