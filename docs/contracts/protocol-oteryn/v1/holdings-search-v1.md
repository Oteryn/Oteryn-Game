# Holdings search V1

Capability 506 requires 199 and extends command 388 with selector 134.
Request: selector:u8, offset:u16le, location:u8 (0 all, 1 inventory,
2 depot, 3 stash, 4 inbox), search_length:u8 and UTF-8 search (0..96 bytes,
no NUL or line breaks). Search is trimmed and case insensitive, matching
an item key or its name in the active authoritative item content.
Location and search constrain the full fenced custody snapshot before its
count and ordered 24-row page. The legacy offset and total bounds remain.
Reply: selector:u8, legacy_page_length:u16le, the complete selector-130 page,
then one name per row (length:u16le, UTF-8 bytes, 1..512 bytes, no NUL or line
breaks). Names align exactly with page rows. An unknown content key retains
its key as its label. Trailing bytes and malformed fields are rejected.
Unselected connections retain selector 130 and unchanged legacy bytes.
