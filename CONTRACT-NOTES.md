# WCONTRIB decisions

Horse Station follows the owner's three-hour-or-zero-horses activation rule and three-hour raid expiry. A late observation preserves the original deadline; observing after expiry restores the stable immediately.

This rule intentionally differs from the retained rental-triggered 24-hour raid: Crystal 9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d `data-global/npc/palomino.lua:104-112,148-160`, SHA256 `595387918194d62554ba02e66c381e945fc065a0c0b3b2658ce773fe52f293da`. Native initial stables contain nine horses: `data-global/scripts/globalevents/spawn/horse_station_world_change.lua:1-9,18-22,39-43`, SHA256 `631b28ccc0120d2f8d9d74c1f9540b40989065b6785d7d4331f495389c92e07b`.

Dog Collar issuance uses an atomic gift: eligibility, earning and native backpack MINT commit together. A full backpack refuses without earning or setting the once flag. A successful collar remains in custody and exact replays remain valid after the disappearance ends; there is no unearned pending grant.

World live changes insert revision one once and advance existing rows with guarded UPDATEs. PostgreSQL executes BEFORE INSERT guards before an UPSERT conflict arm, so UPSERT is deliberately unsuitable for the immutable revision chain.

Steamship requires 200 shared Coal and opens only at the next save; each retiring epoch funds its following saved day. Junkar's retained destinations and prices remain the travel authority.

Swamp Fever uses an owner-authorized conservative threshold of 100 medicine pouches12517 to cure/prevent the following saved day's epidemic. The retained donor establishes the medicine and individual Ottokar exchange, but not a shared numeric cure threshold. The individual exchange remains separate.

Twisted Waters uses 1,000 corpse deposits per retiring epoch, advancing one pollution stage when met and recovering one stage otherwise. Yawno and explicit dirty-water use accept the final portable aged Swamp Troll corpse4178. The selected native lake water component is projected as clean/dirty/stinking; only a successful Shimmer catch starts the sourced personal 20-hour restriction. These deposit/recovery and lake appearance choices are conservative authored rules.

Rathleton uses twelve fixed ballot identities and one earned right per explicit cast. Unknown alternate-Monday parity is anchored to Monday2000-01-03UTC; the active epoch's save timestamp owns the period. Ties retain incumbents. Native missing entrance selectors are replaced by explicit Marvin passages to sourced dungeon/first-pipe destinations, checked against current promoted rank and elected dungeon inside the existing travel transaction. Rank admission also uses native Alaistar/Flint trades and citizen boat routes. Sources and exact hashes are recorded in the owned WORLDCHG-Q rows.

Horse returns restore one shared horse, bounded at nine; restoring all nine ends a raid early. Reduced stable cohorts use the first N members of the sourced nine-horse layout. Existing native raid scheduling, Channel target admission, actor cleanup and generation guards own every cohort.

Noodles uses the accepted92-permille saved-day disappearance rule and thirteen documented Thais roaming anchors. An escort belongs to one Character across Channels, preserves its collar, and can return only beside the actual King. The personal repeat interval is fourteen days. A return guarantees Dog Lady on the next saved day; the next reset releases any unfinished escort and selects the new day's disappearance. Dog Lady uses her existing dog presentation32, never an invented human appearance.

The shared NPC reward/access engine is reused from the quest-engine lane at5a6de28c142de185594217ce1e5cbc7f4b6e6e4e. Its unrelated binary map restoration is excluded. Required WCONTRIB NPCs are bound through the existing retained-source placement converter; missing legacy travel-service declarations use the native route catalogue and greeting adapter.

King `noodles` dialogue and collar use share the same World RETURN action. Spatial commits and receipt observation bind the original player/NPC context; ambiguous outcomes retain that context instead of reconstructing it after either actor moves.
