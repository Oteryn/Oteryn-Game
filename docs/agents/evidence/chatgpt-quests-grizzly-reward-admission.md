# Grizzly kill reward admission evidence

The accepted Grizzly catalogue is pinned to Crystal `201189f871d23597b82caf88fd611454a2ddefb8`. This comparison reads the existing native Creature policies and ordinary loot Item admission; it changes none of those facts. Q11=B permits Grizzly quest credit on the existing bounded native kill queue when the ordinary reward row is unavailable. It grants no ordinary XP, corpse Item, loot, Bestiary, Task Board or proficiency value. Existing Bosstiary-only fallback remains intact.

## Inputs

- `content/quests/killing-in-the-name-of.json` —42 task creature lists and30 boss identities.
- `content/creatures/definitions/spell-native-profiles.json` — native Creature `profile.data.profile.details.corpse_item`.
- `tools/content-schema/native-gameplay/loot-tables.json` — ordinary corpse admission `items[].materializable` and `container_capacity`.
- `crates/gameplay/content/src/content/creature_reward.rs` — existing `CorpseItemInadmissible` decision; no admission override.

There are 91 distinct task/boss Creature keys. 91 use the following 59 inadmissible corpse Items; 0 have admitted corpse Items. 0 lack a native Creature profile. Corpse admission alone does not promise all loot-table entries are admitted.

| Corpse Item | Affected native Creature keys (suffix after `oteryn:creature.`) |
|---|---|
| `oteryn:item.tibia.i10190` | `draken_warmaster` |
| `oteryn:item.tibia.i10221` | `killer_caiman` |
| `oteryn:item.tibia.i10333` | `gnarlhound` |
| `oteryn:item.tibia.i10355` | `lizard_high_guard` |
| `oteryn:item.tibia.i10359` | `lizard_legionnaire` |
| `oteryn:item.tibia.i10363` | `lizard_dragon_priest` |
| `oteryn:item.tibia.i10367` | `fazzrah`, `lizard_zaogun` |
| `oteryn:item.tibia.i10371` | `lizard_chosen` |
| `oteryn:item.tibia.i10393` | `wailing_widow` |
| `oteryn:item.tibia.i10399` | `draken_spellweaver` |
| `oteryn:item.tibia.i10430` | `terramite` |
| `oteryn:item.tibia.i10445` | `ethershreck`, `ghastly_dragon` |
| `oteryn:item.tibia.i10458` | `lancer_beetle` |
| `oteryn:item.tibia.i11571` | `brimstone_bug`, `sulphur_scuttler` |
| `oteryn:item.tibia.i11653` | `draken_elite`, `paiz_the_pauperizer` |
| `oteryn:item.tibia.i11667` | `draken_abomination` |
| `oteryn:item.tibia.i12325` | `stampor`, `tromphonyte` |
| `oteryn:item.tibia.i18099` | `werewolf` |
| `oteryn:item.tibia.i18289` | `hemming` |
| `oteryn:item.tibia.i4388` | `the_noxious_spawn` |
| `oteryn:item.tibia.i5977` | `giant_spider`, `the_old_widow` |
| `oteryn:item.tibia.i5984` | `demodras` |
| `oteryn:item.tibia.i5995` | `demon` |
| `oteryn:item.tibia.i5999` | `behemoth`, `stonecracker` |
| `oteryn:item.tibia.i6005` | `stone_golem` |
| `oteryn:item.tibia.i6021` | `ancient_scarab`, `fleshcrawler` |
| `oteryn:item.tibia.i6027` | `gargoyle` |
| `oteryn:item.tibia.i6030` | `bonebeast`, `ribstride` |
| `oteryn:item.tibia.i6034` | `badger` |
| `oteryn:item.tibia.i6043` | `kongra` |
| `oteryn:item.tibia.i6044` | `merlkin` |
| `oteryn:item.tibia.i6045` | `sibang` |
| `oteryn:item.tibia.i6046` | `crocodile`, `the_snapper` |
| `oteryn:item.tibia.i6047` | `carniphila`, `deathbine`, `tiquandas_revenge` |
| `oteryn:item.tibia.i6048` | `hydra`, `the_many` |
| `oteryn:item.tibia.i6060` | `hide`, `tarantula` |
| `oteryn:item.tibia.i6061` | `serpent_spawn` |
| `oteryn:item.tibia.i6063` | `quara_pincher`, `quara_pincher_scout` |
| `oteryn:item.tibia.i6064` | `quara_mantassin`, `quara_mantassin_scout` |
| `oteryn:item.tibia.i6065` | `quara_constrictor`, `quara_constrictor_scout`, `thul` |
| `oteryn:item.tibia.i6066` | `quara_hydromancer`, `quara_hydromancer_scout` |
| `oteryn:item.tibia.i6067` | `quara_predator`, `quara_predator_scout` |
| `oteryn:item.tibia.i6073` | `thornback_tortoise` |
| `oteryn:item.tibia.i6074` | `mammoth`, `the_bloodtusk` |
| `oteryn:item.tibia.i6301` | `wyvern` |
| `oteryn:item.tibia.i6305` | `undead_dragon`, `zanakeph` |
| `oteryn:item.tibia.i6319` | `bretzecutioner`, `destroyer` |
| `oteryn:item.tibia.i6331` | `hellhound`, `kerberos` |
| `oteryn:item.tibia.i6339` | `nightmare`, `tormentor` |
| `oteryn:item.tibia.i7282` | `ice_golem`, `shardhead` |
| `oteryn:item.tibia.i7344` | `crystal_spider`, `the_bloodweb` |
| `oteryn:item.tibia.i8915` | `bruise_payne`, `mutated_bat` |
| `oteryn:item.tibia.i8957` | `esmeralda`, `mutated_rat` |
| `oteryn:item.tibia.i8965` | `young_sea_serpent` |
| `oteryn:item.tibia.i8999` | `mutated_tiger` |
| `oteryn:item.tibia.i9005` | `nightmare_scion` |
| `oteryn:item.tibia.i9009` | `flameborn`, `hellspawn` |
| `oteryn:item.tibia.i949` | `leviathan`, `sea_serpent` |
| `oteryn:item.tibia.i9607` | `gorgo`, `medusa` |

## Runtime checks

- The exact native lethal principal, source party observation, active task run and Character revision are frozen before queue acceptance.
- The existing64-entry queue, corpse projection pins, release handshake and retry/replay rules also own work with no ordinary reward row.
- Settlement first uses the same fenced Grizzly writer, including Q8 current party membership proof and Q9 live local recipient fence.
- A delayed boss death binds its observed quest revision and cannot revive a counter reset by a later selection, task completion, boss report or special-task command. The guard reads the existing immutable command receipts through a partial index; it permits distinct queued boss deaths when no reset intervenes. The mounted task/boss writer cases cover those resets, refusal replay and success replay after a reset.
- A missing ordinary reward row reserves zero loot MINTs and reaches no ordinary XP/loot/Task Board/proficiency code.
- `grizzly_boss_portal_wire_occupied_spawn_expiry_and_relog_return` and `grizzly_boss_committed_expiry_without_reply_retries_online_return` exercise native Snapper/Crocodile deaths, duplicate callbacks/drains, zero ordinary value changes, task start after death, cancelled/restarted task run, native party join after death, and final pin release.

## Deployment preflight: existing quest pins

**PROVEN:** the previous Killing quest catalogue has 152 tracks and 107 transitions;
its v2 definition hash is
`d27c8d026a45ce872176e45899196433254eee8d946abb471b7c0505c91fcc39`.
The Grizzly extension changes that hash. An active quest row with the previous
hash is refused with `REVISION_MISMATCH`, preserving its pin, values and receipts.
The existing owner provides no automatic repin operation: see
`crates/gameplay/durability/src/durability/quest_state/writer.rs:254–281`,
`apps/game-server/migrations/0056_character_quest_state.sql:273–287`, and
QUEST-STATE-0 revision pinning, section 6. Q14=A was accepted in PR #2770,
comment `6543503592`: preserve this refusal and keep migration of old active pins
outside this delivery. No repin, reset or deletion is authorized by this evidence.

The following read-only query identifies installed active pins before deployment:

```sql
SELECT pinned_content_revision,
       encode(definition_hash, 'hex') AS definition_hash,
       count(*) AS active_characters
FROM game_character_quest_states
WHERE quest_key = 'oteryn:quest.killing_in_the_name_of_quest'
  AND completed_character_revision IS NULL
GROUP BY pinned_content_revision, definition_hash;
```

**DERIVED:** the baseline's NPC `requested_by` records are source evidence without
a mounted production caller; the existing world callbacks and reward claims do
not reference the old Killing transitions. **UNKNOWN:** whether an installed
database contains historical pins. No production database was inspected and no
production rows were observed; that is not evidence that every deployment is
empty. The mounted regression
`grizzly_legacy_active_quest_pin_refuses_without_reinterpreting_value` creates an
old pin through the actual quest writer and checks refusal, replay and unchanged
player value under the new catalogue.
