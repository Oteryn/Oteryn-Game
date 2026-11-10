# Generic server modal v1 (CGMECH T159 / O040)

Game owns dialog content, identities, choices and accepted answers. The client only
presents the descriptor and returns opaque control ids. A modal answer grants no
authority: its rule callback must use the existing fenced gameplay owner.

Capability 450 supplies commands 736 (query), 737 (answer), and 6196 (start a
Hireling Store selection).
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
bytes; both edges are 1..96. The complete reply is at most the existing 64 KiB command-result budget. Inline pixels
are data, never a URL or executable content.

## Hireling Store selection

Command 6196 uses the command 185 Store purchase request and optional Hireling
customization encoding. It requires selected capabilities 450, 154 and 250 and
the exact current connected actor. Only the admitted Hireling rename and sex-change
offers are accepted. Rename supplies a valid name without a Hireling identity;
sex change supplies no customization. The quote and purchase-request UUID are
retained unchanged until the answer. No debit or reservation occurs at this step.
The client retains its confirmed request until the matching modal answer is
acknowledged; opening the selector alone cannot discard its retry identity.

The response is a modal descriptor. The server reads the player's bounded owned
Hireling list, maps choice ids to those exact identities, and provides Select and
Cancel buttons. Repeating the same pending request returns the same descriptor;
a different request cannot replace it. Query 736 returns that pending descriptor.
Cancel clears the selection without buying anything. Select resolves the stored
identity and invokes the ordinary command 185 purchase owner, which rechecks the
quote, ownership, lamp state, session fences, purchase mode and funds. Its existing
purchase UUID and receipts prevent repeated debit or delivery. An uncertain result
keeps the exact submitted answer retryable; another choice or Cancel cannot abandon
that potentially paid purchase. A decoded result acknowledges the answer, and the
next query presents the purchase result as an informational dialog. Disconnect
discards pending connection state; it cannot cancel a purchase already retained
by the durable Store owner.

Rule sources: `docs/reference/tibia-manual/interface.md` (official interface
capture, 2026-09-28) and `products.md:115-119` (Hireling name and sex services).
Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
`data/libs/systems/hireling.lua:676-708` defines choices and Select/Cancel;
`data/modules/scripts/gamestore/init.lua:1914-1948,1981-2020` consumes the selected
Hireling for rename and sex change and refuses an active Hireling. Canary
`47dfd51f45280a59a1d3e50ba7edd573d7234446` has the same selection helper.
The OTS callback behavior is compatibility evidence (`OTS_HYPOTHESIS_ONLY`);
the official manual owns the service semantics. Exercise rewards and GM teleport
are outside this slice: the former needs the donor's reward eligibility producer;
the latter already exists behind `staff-tools-v1.md`'s off-host signature over a
canonical request containing the exact destination Character. Selecting another
target cannot reuse that signature, and Game cannot mint a new signing authority.

## Validation

`server_modal_wire_round_trip_validates_choices_buttons_and_image_bounds` covers
choice/button identity, optional cancellation, image length, duplicate controls,
canonical flags and trailing data. End-to-end transport evidence belongs to the
client-gap rows when the owner and native UI are connected.

`server_modal_hireling_store_selection_dispatches_cancel_rename_and_sex_once`
drives the real connection dispatcher and PostgreSQL Store owner, including
cancellation, duplicate answers, and an uncertain Platform response.
`store_hireling_confirmation_queues_server_modal_selection` drives the native
Store confirmation into the play queue. `store_hireling_without_identity_requests_server_choice_before_purchase`
covers the frozen quote retry and capability fallback.
