# Container pages

Capability 606 requires capabilities 4 and 14 and gates command 6375. Command
21 still opens and closes views in domain 14. Browse Field keeps its own seek
command.

Command 6375 has four bytes: the open ordinary view id, a little-endian u16
offset, and a page size from 1 through 20. The offset is a multiple of the
page size. The server re-observes the container and verifies reach and
custody under its usual rules. It refuses a page size above the container's
capacity, a nonexistent or Browse Field view and an offset outside the direct
contents. Offset zero is valid for an empty container.

An accepted reply contains a little-endian u16 offset, a little-endian u16
total entry count and one byte for the accepted page size. The replacement
domain 14 view follows the result. It retains the original capacity, and its
entries are the requested window in display order. Entries outside the page
are no longer exposed by this view. Refresh retains the page size and clamps
the offset to the last page when contents shrink. Opening, replacing or
reconnecting uses the existing view behavior. A refused request returns an
empty payload. The same session view-command rate limit covers seek, open,
close and up.

[Crystal aligns seek offsets and re-sends the current view](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/game/game.cpp#L5175-L5196).
