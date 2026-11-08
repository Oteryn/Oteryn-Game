# Generic server modal v1 (CGMECH T159 / O040)

Game owns dialog content, identities, choices and accepted answers. The client only
presents the descriptor and returns opaque control ids. This feature never grants
items, cosmetics, commercial entitlements or local authority.

This authoring slice defines the codec. Capability 449 and commands 736 (query)
and 737 (answer) remain unadvertised until their server and client handlers land.
The transport uses the existing gated command envelope; no legacy frame changes.

## Request

Query has an empty payload. An answer is a little-endian nonzero u32 dialog id,
a u8 button id, and a u8 choice-present flag. Flag 0 ends the payload; flag 1 is
followed by the u8 choice id. Other flags and trailing bytes are invalid.

The current connection owner must verify the exact pending dialog and both ids.
An ordinary button requires a listed choice when choices exist. The declared
cancel button may be selected without a choice. A stale or unknown answer has
no effect. Reconnection must not inherit a prior connection's pending dialog.

## Reply

The first byte discriminates: 0 means no dialog; 1 precedes a dialog descriptor;
2 precedes an accepted answer encoded as above. No reply permits trailing bytes.

A descriptor contains, in order: u32 dialog id, title, message, choice list,
button list, cancel-button-present flag and optional u8 cancel id, then an
image-present flag and optional image. Integers are little endian. Strings have
a u16 byte length followed by UTF-8. Lists have a u8 count followed by u8 id /
string pairs. Presence flags accept only 0 and 1.

Bounds: title and labels 256 bytes, message 8192 bytes, 32 choices, 8 buttons.
The id is nonzero, the button list is nonempty and ids are unique within each
list. Text is nonempty and contains no NUL. A declared cancel id must name a
button. Images carry u16 width / height and exactly width * height * 4 RGBA
bytes; both edges are 1..128. The complete reply is at most 96 KiB. Inline pixels
are data, never a URL or executable content.

## Validation

`server_modal_wire_round_trip_validates_choices_buttons_and_image_bounds` covers
choice/button identity, optional cancellation, image length, duplicate controls,
canonical flags and trailing data. End-to-end transport evidence belongs to the
client-gap rows when the owner and native UI are connected.
