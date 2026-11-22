# Item presentation binding (parity batch 13)

Status: **NEEDS-CONTRACT**. The client deliberately leaves carried-item artwork
unresolved. The inventory keeps the server's stack count in its existing
overlay; the artwork fallback never interprets a subtype or definition reference as a sprite.

`ItemEntryV1` (`docs/contracts/protocol-oteryn/v1/item_view_v1.proto`) supplies
only handle, definition reference, count and subtype. The reference is a
connection/content-generation definition index, not an appearance object ID.
Capability 25's `Look::Item` supplies another definition reference, and
`AppearanceItemDictionary` validates membership only. Neither binds pixels.
The pinned client appearance bank maps appearance IDs to sprite patterns; it
has no dictionary from session definition indices to those IDs.

The existing map protocol supplies an appearance ID for each individual map
placement. That is sufficient for drawing that placement, but is not a contract
for choosing the appearance of every carried item sharing its definition.
Do not infer a persistent inventory catalogue from nearby map tiles or names.

## Exact missing message

An accepted, negotiated **ItemPresentationBindings** snapshot/replacement is
needed, containing:

- the session content generation and the verified client appearance manifest
  identity;
- bounded, unique `(item_definition_ref, sub_type)` keys, each bound to a
  nonzero appearance object ID in that manifest;
- explicit subtype matching semantics (including whether an appearance uses
  subtype as a fluid pattern), and the count/pattern selection rule;
- replacement/revision and invalidation rules for content changes, reconnect
  and activity handoff; unknown keys remain unresolved.

This is a description of the missing contract, not a new wire schema or a
reserved capability/message number. No server emission, migration or
persistence change is made. Once that contract is accepted, the client must
validate the generation, manifest, duplicate keys and appearance membership
before inventory rendering consumes the binding (batch 14).

Reference: [edubart/otclient `src/client/item.cpp`](https://github.com/edubart/otclient/blob/master/src/client/item.cpp), `Item::setId`, `setOtbId`
and `isValid`: working item drawing resolves and validates a client appearance
ID rather than treating a server definition index as a sprite ID. Oteryn must
provide its own verified binding; OTClient's OTB identity space cannot supply it.

No protocol or rendering implementation is shipped by this report. Batch 13
remains open, and the requested red/green binding regression cannot be written
against a real accepted data path until the binding contract exists. The
existing fallback and inventory count overlay are unchanged.

Source evidence at base `9306b4eadb1209754d65ebd33c31c12291cce1fd`:

- `crates/protocol-oteryn/src/item_view.rs:126`: item entry fields.
- `crates/protocol-oteryn/src/world_map.rs:125`: per-placement appearance.
- `crates/session/src/appearance_state/item_dictionary.rs:7`: reference membership,
  with no appearance object mapping.
- `apps/client/src/bevy_shell/game_ui/projection.rs:20`: unresolved artwork.
- `apps/client/src/bevy_shell/game_ui/inventory/view.rs:340`: real stack overlay.

Existing guard test (unchanged):
`bevy_shell::game_ui::projection::tests::unavailable_item_appearance_builds_one_visible_source_label`.
It covers the existing unresolved fallback, not a verified binding.
