# Familiar appearance

Capability 579 `FAMILIAR_APPEARANCE_V1` depends on appearance capability 25.
APP-3's strict protobuf layouts, command 25 and domain 21 stay unchanged.

Command 6324 queries the familiar choices with an empty payload. Command
6325 changes the chosen familiar appearance. Its bounded UTF-8 JSON object
contains exactly `root_revision` (positive u64) and `key` (the chosen
definition key). Maximum 320 bytes. Unknown or ungranted keys are rejected.
The revision fences a change from an older catalogue. Choosing the current
selection again confirms the current view without a write.

Both commands return a UTF-8 JSON object with `root_revision`, `choices`
and `selected`. Each choice has `key` and `look_type` (positive u32).
Choices are unique and sorted by key, at most 256, and include only active
familiar outfit appearances the character has been granted. If native keys
share one persisted look, the first key in ascending order is canonical. Keys use ASCII
letters, digits and `:/._-`, at most 256 bytes. `selected` is either the
authoritative selected choice or null if the saved look is absent from the
permitted choices. Maximum 65536 bytes. Invalid requests have empty rejected
results. These commands run in the character's current Channel. Game uses
the current Character and session-generation fence for changes. Familiar lifecycle and summons continue to own active companions.

Crystal 201189f8 filters the outfit dialog's familiar list through the
character's grants and publishes the saved familiar look in that dialog.
[Crystal outfit window](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/server/network/protocol/protocolgame.cpp#L8636-L8757)

[TibiaWiki familiar reference](https://www.tibiawiki.com.br/wiki/Familiars)
