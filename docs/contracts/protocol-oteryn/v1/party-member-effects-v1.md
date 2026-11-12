# PARTY_MEMBER_EFFECTS_V1 (383)

Capability 383 requires PARTY_V1 (42), adds no command, and is offered only in
BUNDLE_WORLD_OFFERED_CAPABILITIES. Query 46 remains the authoritative party
projection. Only when 383 is selected may a member entry contain optional
length-delimited field 5: exactly twenty bytes, five big-endian u32 remaining
millisecond durations in Train, Protect, Enchant, Heal, Enlighten Party order.
Zero denotes no active effect. Missing field means the extension was not
selected or that the member is outside the current Channel runtime; the client
must not infer effects from membership. Invitations never contain field 5.

The server resolves the committed membership through current actor/session
leases under its runtime and spell-state guards. It reads production conditions
at one owner-clock instant; expired or removed effects become zero on refresh.
Clients replace the entire member projection on refresh and count down from
receipt using elapsed monotonic time, hiding effects at zero. Remaining time is
presentation data and grants no effect or membership authority.

Legacy peers that do not select 383 receive the original entry encoding.
Duplicate fields, wrong byte lengths and effects on invitations are rejected.
