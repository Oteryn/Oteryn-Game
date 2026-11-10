# M03 chosen Pits checkpoint

Reused `qm03_world_admission.py` from collecting head
`856a420d64cc0ea5c9e8cb2a54742b45c49b9914`; no branch merge.
Only its Pits row is activated here, preserving the existing authored
25 Years time-machine entry and ungated return.

The selected Pits recipe requires level80 and Premium. Rope and Shovel
remain route tools; this checkpoint does not invent a carried-item gate.
Its `s1` counter is declared 0..1, so the callback uses minimum0 rather
than legacy storage's -1. `track_initials` declares that default in the
loader; it never rewrites an existing track or changes unspecified legacy defaults. Entry is nonterminal and failed admission returns
the player to the tile they came from. Egress has no Premium guard.

The chosen checkpoint is retained floor354 at `(32791,32331,-10)`, ordinal0
in `content/world/placements/region-z10-x128-y126.b3`. This is a chosen
checkpoint at a donor destination, not a claim that the donor checks
Premium on that destination tile or that the whole original quest is playable.
Crystal201189f8 `movements_holy_tible_tile.lua:1-4` records that destination:
https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_pits_of_inferno_quest/movements_holy_tible_tile.lua
Fetched source SHA256: `7a8b42975e202802bfc331125eb9744daa5fadd438a2e1ec02ed2cc5180b2f5a`.
The script's Holy Tible condition remains separate; this change does not
replace that condition or certify other entrance paths.
