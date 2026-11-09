# Bosstiary catalogue over boss status command 133

This is an additive read-only request form on existing capability 117 and command
133. No capability or command registration is added. Legacy cooldown and taint
requests/replies retain their exact encoding. Unsupported request forms fail
closed on older servers.

A catalogue request is `BCT1`, `after:u32`, `category:u8`, `search_bytes:u8`, then
UTF-8 search text. Integers are little endian. The cursor is an ordinal into the
canonical boss catalogue, initially zero. Categories are 0 all, 1 bane, 2 archfoe,
3 nemesis. Search is a case-insensitive display-name substring, at most 48 bytes,
with no NUL. Each query scans at most 64 canonical definitions and returns at
most 16 matching qualified entries. The cursor advances over missing qualified
presentations and nonmatching definitions.

A reply is `BCP1`, `next:u32` (zero means end), total character boss points `u64`,
entry count `u8`, then entries: key (u16 byte length + UTF-8, at most 512 bytes),
display name (same layout, at most 128 bytes), category `u8` (1..3), kill count
`u64`, three strictly increasing nonzero kill thresholds `u64`, three strictly
increasing cumulative stage point awards `u16`, appearance id `u32`, and object
appearance flag `u8` (0/1). Appearance zero means no qualified portrait is
available. Replies are at most 16,384 bytes. Prowess, expertise and mastery are
derived from the reported thresholds; no progress is written by browsing.

Identity/category and stage rules come from the canonical Bosstiary owner. Names
and appearance ids come from the admitted generation's content-pinned creature
policy/profile. The durable read uses the same retained admission, current
session, lease, scope, recovery and Character interpretation checks as cooldown
reads. It joins only the page's at most 16 keys to that character's kill counts
and reads total points in the same transaction. Missing rows mean zero kills;
retired or unrelated keys cannot fill a page and hide the selected bosses.

The client keeps separate catalogue and cooldown cursors, preserves confirmed
cards on refusal/malformed replies, rejects mismatched categories or backwards
continuations, and invalidates the catalogue at an activity scope change. Portrait
pixels are loaded through the existing pinned appearance/sprite manifest and
sheet decoder; missing art is displayed as unavailable.
