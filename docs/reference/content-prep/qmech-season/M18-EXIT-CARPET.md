# M18 source exit carpet

The real B3 tile at (33597,30994,14), the exit in Crystal
`data-global/scripts/quests/the_first_dragon/actions_lever.lua:40`, contains
accepted Item410 and ornate carpet17400. Its existing identity and presentation
were already admitted, but it lacked a native physical profile, so map boot
correctly refused to infer walkability.

The normal source Item importer now qualifies only that existing carpet identity:

- Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
  `data/items/items.xml:35502-35504`: ornate carpet, floor decoration.
- The pinned `data/items/appearances.dat` object17400 has `clip=true` and
  `unmove=true`, with no unpass/unsight/take flags.
- `src/items/items.cpp:233-244` reads unpass/unsight/take/unmove into movement,
  projectile, pickup and movement flags. The resulting profile is nonblocking,
  immovable and Ground-only, with top order1. It grants no inventory custody.

Source URLs:
https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml#L35502
https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/items/items.cpp#L233

`first_dragon_item_profiles.py` verifies the pinned source hashes, preserves
accepted canonical facts, generates the capture and registers it through the
ordinary Item provider. The equipment cohort receipt is generated from its exact
four-identity source set. No migration, command, capability or ordinal is allocated.

## Native corpse closure

The same normal Item producer admits the existing corpse identities 25065,
25066 and 25067, Ground-only. Crystal201189f `data/items/items.xml:48202-48217`
defines capacities20/10 and decay10s→500s→60s→removal. The importer resolves
these exact decay references and preserves already accepted facts.
https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml#L48202

The boss source explicitly has `monster.loot = {}` at line74 and corpse25065
at line19; the existing native reward owner already supports this null canonical
loot binding by settling an empty corpse table. No synthetic loot is added.
https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/monster/bosses/the_first_dragon.lua#L74
