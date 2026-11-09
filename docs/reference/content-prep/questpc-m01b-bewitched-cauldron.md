# Bewitched: first personal cauldron survey

This binds only the chosen recipe's first objective, “Reach the witch cauldron
beneath Green Claw Swamp”. Its explicit adaptation replaces rotating ingredient
lists, eight-hour global phases and favour points with a fixed daily contribution.
The approach does not assert that the source global event is active, supply any
ingredients, credit a bane-bringer kill, finish brewing or grant a reward.

The recipe retains these TIBIAWIKI_STRUCTURED reference revisions:

- https://tibia.fandom.com/index.php?curid=50849&oldid=1190949
- https://tibia.fandom.com/index.php?curid=50859&oldid=1190948

The source bodies were not reacquired for this slice; their recorded hashes and
chosen recipe are checked locally. The resolver rejects a changed first objective,
a removed personal adaptation or changed spoiler provenance.

OTS_HYPOTHESIS_ONLY item-name cross-check, read at the exact Crystal revision:
https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml
names item12023 “huge cauldron”. The existing native placement region
`content/world/placements/region-z12-x127-y124.b3` retains its corner at
(32732,31935,12), beside the other huge-cauldron pieces at x32730..32732,
y31932..31935. No source objects or ground are changed. The resolver verifies the
corner's exact public tile stack [7272,12023] before selecting an approach.

Chosen Oteryn entry: clear tile (32733,31935,-12), source ground7272, directly
east of that corner. Test arrival starts at (32734,31935,-12), also ground7272,
and uses the native STEP West caller. The source tile footprint, house status and
approach ground are checked against the retained B3 bytes, not an invented room.

The resolver is initially prepared but inactive; native dispatch proof is required
before reporting this route as PG-qualified. Daily re-entry, ingredients, brewing,
attack and completion remain separate later-stage obligations.
