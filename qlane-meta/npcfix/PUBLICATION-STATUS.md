# NPCFIX publication status

Branch: `claude/happy-davinci-ijz3wy-npcfix`. Main merged: `64d8bb8b8e7c7733f3840cc9ceb9134ce70b0b8c`.

## DONE: 272 owned NPC entries

A Sweaty Cyclops; Alkestios; Elvith; Olrik; Ahmet; Alesar; Fitzgerald; Malor; Enpa Rudra; Cerdras; Liane; Padreia; Altar; Chemar; Elyen Ravenlock; Kasmir; Muzir; Nah'Bob; Oblivion; Omrabas; A Dark Priestess; A Majestic Warwolf; Albert; Alexander; Amanda; An Ominous Bat; Anaztassja Moroia; Beatrice; Captain Seahorse; Chief Grarkharok; Chrystal; Eliza; Emberwing; Eremo; Grovebeast Npc; Gundralph; Gunther; Hawkhurst; Iwan; Lai; Lailene; Lesser Messenger of Heaven; Luna; Maeryn; Milos; Mirabell; Myzzi; Ortelio; Owin; Pemaret; Pino; Puffels; Rudolph; Sane Mage; Telas Golem; Thomas; Ursula; Willard; Wyrdin; A Fading Memory; A Sleeping Dragon; A Strange Chalice; A Wandering Soul; Chartan; Chrak; Esrik; Lizard Prisoner; Lizard Tunnel Guard; Murim; Ongulf; Pompan; Zalamon; Zizzle; Zlak; Ztiss; Zumtah; Zurak; Maelyrra; Toffee; Alyxo; Captain Jack Rat; Eshaya; Taya; Yonan; Etzel; Gnomission; Hemor, the guard; Humnog, the guard; Lokur; Lukosch; Malech; Maryza; Melfar; Nezil; Pukosch; Pydar; Riddler; Shortsighted Dwarf; Sigurd; Tezila; The Orc King; Tulf; Wally; Xelvar; Captain Max; Chantalle; Charlotta; Chondur; Duncan; Eleonore; John The Carpenter; Marina; Ocelus; One-Eyed Joe; Polly; Amra; Asphota; Candra; Captain Corsarah; Dhira; G Ezkho; Giri; Goldro; Haani; Isika; Jata; Javala; Kaumudi; Nagging Naga; Nekaret; Nihaka; Nilavarna; Nipuna; Omar; Pescadu; Ziyad; Charles; Ray; A Beggar; Chavis; Eathar; Edmund; Elbert; Elliott; Emily; Fabiana; Fayla; Fillias; Flint; Fyodor; Georgia Rainbird; Graham; Guardian of the Deep Seas; Guide Kunibert; Hexel; Jossel; Klesar; Kromrek; Maro; Marvin; Nicholas; Normod; Peremin; Al Dee; Dixi; Captain Marie-Denise Banner; Dream Butterfly; Eldoran the Ambitious; Glimpse of Allusion; Lieutenant Harlan; WOX; Yielothax; A Restless Soul; A Starving Dog; Baltim; Bertha; Captain Breezelda; Captain Haba; Dankwart; Eirik; Freezhild; Fynn; Hawkyr; Janz; Jessica; Kjesse; Lurik; Nelly; Nor; Rashid; Robert; Romir; Sinatuki; Sirik; Thorwulf; Zora; Albinius; Benjamin; Dorian; Eclesius; Eddy; Edgar-Ellen; Edowir; Elane; Emiliana; Emilie; Ferryman Kamil; Gaberiel; Galuna; Gamel; Gorn; Grof, The Guard; Guide Luke; Hanna; Hardek; Harsky; Hector The Mentor; Henricus; Lizzie; Lubo; Markwin; Marvik; Partos; Paulette; Robson; Doctor Marrow; A Behemoth; A Ghostly Guardian; A Strange Fellow; A Weakened Forest Fury; Allen; Captain Fearless; Demonguard; Dustrunner; Eustacio; Frank The Plank; Frans; George The Boyscout; Gnome Trooper; Gnommander; Guide Elena; H.L.; Hagor; Jack The Worker; Jean Claude; Julian; Lightfoot; Livielle; Lorbas; Maria; Nathaniel; Nelliem; Nemal; Oldrak; Romella; Shiantis; Zebron; Chuckles; Emperor Rehal; Lisander; Maris; Maritima; Oliver; Ortheus; Palimuth; Peter; Pugwah; Tarak (Inner); Tarak (Sunken); Xorlosh; Yalahari; Znozel

## Not done: 21 owned NPC entries

Rascalio; Yselda; Captain Jack; Petros; Zarifan; A Swan; Harlow (Vengoth); Eustacio's Spy Rat; Gnops Member; Captain Waverider; Maris (Mistrock); ...; Anna; Ferryman Kamil (Meluna); Ghost Captain; Hoaxette; Junkar (Mines); Junkar (Robsons); Junkar (Thais); Minzy; Zirkon

## Dropped unpublished slice

GNOPS Member; Harlow (Vengoth); Captain Waverider; Maris (Mistrock); Ferryman Kamil (Meluna); Junkar (Mines). Retained complete patch and new authoring helpers in local `/tmp/npcfix-evidence/`. No previously published cases removed.

Mandatory whole-node-boot command: `NODE_BOOT_PROFILE=sorcerer NODE_BOOT_SPELLS=1 bash tools/qualification/node_boot/run.sh`. Exact error: `NODE_BOOT_RESULT=BLOCKED reason=exact_platform_checkout_missing` (exit 2); all 12 variants fail the same prerequisite. Name-only canonical changes also alter the activation digest, so the whole unpublished slice is withdrawn.

Baseline placement PostgreSQL: 1 passed, 147.237 seconds, `POSTGRES_STARTED NPCFIX retained source placement batches`. Full quest-authoring runner before withdrawal: all 105 modules, 1131 tests passed; no new qualification is inferred from this.

Additional incoming-Main CI helper probe: `python3 tools/repository/test_validate_merge_group_pg_sim.py` stopped with `AssertionError: pwsh is required for WP1 native failure-propagation qualification`. Main files kept unchanged.

## Final publication validation

All17 generated-file stages PASS; duplicate migrations PASS; official pin-refresh PASS (no content/activation delta); `cargo check --workspace --all-targets` PASS602.244s; `cargo fmt --all --check` PASS26.842s. No changed Rust crates in this publication.

## Lane migrations

- `1230_quest_atomic_track_capacity.sql`
- `1233_quest_reward_absolute_deadlines.sql`
- `1234_quest_cooldown_repeat_occurrences.sql`
- `1235_quest_reward_workshop_clocks.sql`
- `1236_quest_reward_backpack_outputs.sql`
- `1237_quest_reward_bulk_draws.sql`
- `1238_quest_deadline_move_alias.sql`
