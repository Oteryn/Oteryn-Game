# Winter Update 2026: client delta (live 15.33.124f6c vs test 15.33.c68982)

Source: the diff in the local scratch folder `tsdiff/` (summary, staticdata, appearances,
client strings) produced from the live `Tibia` package and the test `TibiaExternal`
package, plus a direct protobuf read of the test `appearances-*.dat` and
`staticdata-*.dat` for names, looktypes and NPC sale data that the diff text left out.
Raw packages were only read, never copied into the repository.

## Counts

| Asset | New | Removed | Changed |
|---|---:|---:|---:|
| appearances object | 1,028 | 3 (50227, 54653, 54654) | 43,515 |
| appearances outfit | 21 | 1 (1895) | 0 |
| appearances effect | 3 (350, 351, 353) | 0 | 243 |
| appearances missile | 0 | 0 | 76 |
| staticdata f1 creatures (bestiary) | 12 (total 839) | 0 | - |
| staticdata f3 achievements | 10 (total 378) | 0 | - |
| staticdata f5 bosses (bosstiary) | 6 (total 444) | 0 | - |
| staticdata f6 quest lines | 2 (total 104) | 0 | - |
| staticdata f2 / f4 | 0 (21 / 995) | 0 | - |
| client.en.qm strings | 64 | 52 | - |
| proficiencies json / staticmapdata | unchanged | | |

**What the 43,515 object CHG entries mean.** A byte comparison of objects 100, 102, 3031
and 22118 between the live and test `appearances.dat` shows the only difference is the
sprite id list inside frame group field 3 (every sprite id shifted by a constant
offset). The CHG population is a sprite-sheet repack, not a gameplay change; the 243
effect and 76 missile CHG entries have the same shape. Oteryn's asset import needs a
re-import of the sprite catalogue at the new client version, nothing per object.
(Sample of four objects; a full-field census was not run.)

## New creatures (staticdata f1; race id, name, looktype)

| Race | Name | Looktype | Feature |
|---:|---|---:|---|
| 2883 | omega bulwark | 1988 | Banner's Return |
| 2884 | omega bastion | 1989 | Banner's Return |
| 2885 | omega battler | 1990 | Banner's Return |
| 2886 | omega moonbreaker | 1988 (colours 10/10/10/10) | Banner's Return (Builder, Red Gullet) |
| 2887 | omega cinderflayer | 1989 (colours 77) | Banner's Return (Builder) |
| 2888 | omega rampart | 1990 (colours 86/91/91/91) | Banner's Return (Builder) |
| 2896 | giant rat | 1993 | I Smell a Rat |
| 2897 | greedcarrier rat | 1994 | I Smell a Rat |
| 2898 | hoarder rat | 1995 | I Smell a Rat |
| 2899 | greed elemental | 1998 | I Smell a Rat (Greed Dimension) |
| 2900 | greedweaver | 1999 | I Smell a Rat (Greed Dimension) |
| 2901 | greater greed | 2000 | I Smell a Rat (Greed Dimension) |

The three Builder variants reuse the base omega looktypes with fixed colours; the f1
records carry difficulty field 4 = 4 (base) and 5 (Builders). The "Builder" grouping
is from thread 568 (stronger omega knights in the Red Gullet); which race ids are the
Builders is inferred from the coloured looktypes and the higher field 4, not stated.

## New bosses (staticdata f5; boss id, name, looktype)

| Id | Name | Looktype | Feature |
|---:|---|---:|---|
| 2867 | Rathilda | 1997 | I Smell a Rat, Temple of Greed |
| 2868 | Ratasha | 1997 | I Smell a Rat |
| 2869 | Rathena | 1997 | I Smell a Rat |
| 2870 | Rattila | 1996 | I Smell a Rat |
| 2871 | The Greedking | 2001 | I Smell a Rat |
| 2902 | Banner's Nemesis | 1992 | Banner's Return (Archfoe, f5 field 3 = 1) |

## New achievements (staticdata f3; id, name, points, description gist)

| Id | Name | Points | Gist | Feature |
|---:|---|---:|---|---|
| 618 | Forged in Conflict | 1 | fought at the highest scorch level | Banner's Return (Red Gullet scorch) |
| 619 | Ratvocat | 1 | paperwork | I Smell a Rat |
| 620 | Feeling Lucky | 1 | tried your luck 100 times | I Smell a Rat (likely Greed gambling) |
| 621 | Just One More | 1 | "the next one will be the big win" | I Smell a Rat (likely Greed gambling) |
| 623 | Enemy Mine | 2 | defeated Banner's nemesis Boson with Captain Banner in the Red Gullet | Banner's Return |
| 624 | Omega Knight | 1 | studied Omega plans and equipment designs | Banner's Return (blueprint 55776) |
| 630 | Riding in Style | 1 | ratmobile mount | I Smell a Rat (key 55628) |
| 631 | Riding in Luxury | 1 | ratmobile mount | I Smell a Rat (key 55626) |
| 632 | Riding in Prestige | 1 | ratmobile mount | I Smell a Rat (key 55627) |
| 633 | Rat Pack | 1 | collected the whole rat pack | I Smell a Rat (rats 55655, 55898-55901) |

Ids 622 and 625-629 are absent from the test staticdata (secret or later-patch
achievements are possible; not inferable).

## New quest lines (staticdata f6)

| Id | Name |
|---:|---|
| 192 | I smell a rat |
| 195 | Banner's Return |

Missions and quest-log texts are not in staticdata; they come from the server.

## Outfits (looktypes)

New: 1985-2002 and 2025-2027 (21). Removed: 1895.

| Looktype | Attributed to |
|---|---|
| 1988, 1989, 1990 | omega knights (base and Builder colourings) |
| 1992 | Banner's Nemesis |
| 1993, 1994, 1995 | giant rat, greedcarrier rat, hoarder rat |
| 1996 | Rattila |
| 1997 | Rathilda, Ratasha, Rathena |
| 1998, 1999, 2000 | greed elemental, greedweaver, greater greed |
| 2001 | The Greedking |
| 1985, 1986, 1987, 1991, 2002, 2025, 2026, 2027 | not attributed by client data. Candidates: the new Banner's Return outfit (wiki changelist: "a new outfit"), the three ratmobile mounts (achievements 630-632; the three large records 2025-2027 are the likeliest), NPCs (Captain Banner, Hushwell patients). UNVERIFIED |

## Effects and missiles

New effects 350, 351, 353 (names not in client data; candidates: Red Gullet fire
spit, frenzy cue, greed effects). No new missiles. The thread 562 "frenzy" icon is a
creature icon, not an effect; its wire value is not in this diff (see
`i-smell-a-rat.md`).

## New objects

1,028 new object ids. 105 carry a name or NPC sale data (listed below); 923 are
unnamed (map tiles, walls, decorations and animation frames of the new areas, ranges
54756-56102, mostly contiguous). Item pairs with the same name (rings, boss bags) are
the usual inactive/active or two-variant pairs.

### Banner's Return (gear, Red Gullet shop, valuables)

| Id | Name | Sale data (test client) |
|---:|---|---|
| 55131 | corrupted moonsilver sabatons | knight boots |
| 55132 | corrupted moonsilver scorchsteps | paladin boots |
| 55228 | corrupted moonsilver footwraps | monk boots |
| 55229 | corrupted moonsilver flamestrider | sorcerer legs |
| 55230 | corrupted moonsilver ashstrider | druid legs |
| 55236 / 55237 | draining ring of the gullet | Ash Sentry sells for 5,000 blood cinders |
| 55242 / 55243 | siphoning ring of the gullet | Ash Sentry 5,000 cinders |
| 55244 / 55245 | rending ring of the gullet | Ash Sentry 5,000 cinders |
| 55555 / 55556 | warding ring of the gullet | Ash Sentry 5,000 cinders |
| 55246 | volcanic charge | Ash Sentry 500 cinders |
| 55251 | dark shackles | Ash Sentry 750 cinders (likely a monster trap) |
| 55338 | omega seal | Ash Sentry 50 cinders; Yasir buys 12,700 gold |
| 55339 | blood onyx | Ash Sentry 50 cinders; Yasir buys 12,200 gold |
| 55340 | blood onyx core | Ash Sentry 250 cinders; Yasir buys 64,000 gold |
| 55341 | broken blood onyx amulet | Yasir buys 214,000 gold |
| 55335 | piece of red gullet metal | Yasir buys 1,600 gold |
| 55336 | broken gullet lenses | Yasir buys 3,750 gold |
| 55337 | gullet moonstone splinter | Yasir buys 3,100 gold |
| 55411 | piece of corrupted moonstone | Ash Sentry 15 cinders; 18 jewel/gem traders (Augustin ... Yonan) buy 15,000 gold |
| 55412 | onyx turmaline | Ash Sentry 60 cinders; same traders buy 68,000 gold |
| 55549-55554 | (unnamed; flags suggest decorations) | Ash Sentry 25,000 / 40,000 / 40,000 / 40,000 / 100,000 / 25,000 cinders |
| 55775 | omega knight arms seal | Ash Sentry 120,000 cinders |
| 55776 | blueprint for an omega knight helmet | quest / achievement 624 |
| 55777-55780, 55824 | earthen / floaty / watery / fiery orb, empty orb encased in blood onyx | quest items |
| 55557 | ancient key | quest item (either quest) |
| 55786-55791 | omega bulwark / bastion / battler / moonbreaker / cinderflayer / rampart soul core | soul cores |

### I Smell a Rat (gear, valuables, bosses, mounts, pets)

| Id | Name | Sale data / note |
|---:|---|---|
| 55504 | ratguard | knight shield |
| 55505 | verdant ratbook | druid spellbook |
| 55506 | arcane ratbook | sorcerer spellbook |
| 55433 | rat claws | monk weapon |
| 55494-55498 | black / soil / verdant / dusk / pearl rat quiver | paladin quivers (thread 561 names "Dark", the client says "black") |
| 55750 | rodent quiver | quiver (not in thread 561; UNVERIFIED role) |
| 55749 | piece of ratwear | quest / crafting item |
| 54769 | musk catcher | quest tool (thread 563: musk from summoned rats is allowed) |
| 55825 | labyrinth voucher | quest item |
| 55379 / 55380 / 55381 | cheese of rot / pestilence / plague | Yasir buys 15,000 gold each |
| 55382 | wheel of cheese | Yasir buys 75,000 gold |
| 55632, 55634, 55635 | rat teeth, rat ears, rat tail | Yasir buys 150 / 350 / 250 gold |
| 55636, 55637 | hoarder claw, greed gland | Yasir buys 600 / 800 gold |
| 55638, 55639 | miasmic pustule, virulent pustule | Yasir buys 400 / 1,500 gold |
| 55640, 55641 | almost a treasure, rat family painting | Yasir buys 1,200 / 3,500 gold |
| 55642, 55643, 55644 | pact of greed, condensed greed, eye of greed | Yasir buys 8,000 / 4,000 / 600 gold |
| 55892, 55893 | Greedkings crown, Greedkings claw | Yasir buys 300,000 / 150,000 gold |
| 55652 / 56022, 55653 / 56023, 55654 / 56024 | Ratashas / Rathildas / Rathenas bag | boss reward bags (two variants each) |
| 55855 / 56025, 55856 / 56026, 55858 / 56027 | Greedkings bag / equipment bag / jewel bag | boss reward bags |
| 55894, 55895, 55896 | bag, rat dusk bag, rat pearl bag | containers |
| 55626, 55627, 55628 | cheesy luxury / prestigious / stylish ratmobile key | mount keys (achievements 631, 632, 630) |
| 55655, 55898-55901 | red / green / blue / pink / golden rat | collectibles (achievement 633 Rat Pack) |
| 55792-55797 | giant rat / greedcarrier rat / hoarder rat / greed elemental / greedweaver / greater greed soul core | soul cores |

### Other

| Id | Name | Note |
|---:|---|---|
| 55891 | greed king santa | Ruprecht (seasonal NPC) sells for 250 gold |

## Client strings (64 new, 52 removed)

Grouped by feature; each feature file quotes the strings it relies on.

- **Bounty / Elite (16 new):** Elite toggle text (monsters deal 100% more and take 50%
  less damage, 100% more XP and dust by difficulty, dust only for Premium), "Task
  Difficulty", difficulty descriptions (Beginner easy; Adept easy+medium; Expert
  medium+hard; Master hard+challenging), Silver/Gold/Platinum chance text, reroll token
  text now saying the free token comes from the Daily Reward Wall (cap 10), "Bounty
  Reroll Token". Removed: the "Claim Daily" reroll button and the old difficulty
  sentence.
- **Daily reward (14 new):** Cosmetic Tokens (balance, "%1 Cosmetic Tokens", "not enough
  Cosmetic Tokens"), "%1x Prey Wildcards", "Temple Teleport Scroll", "%1 minutes XP
  Boost %2%" (the boost now carries its own percentage), separate "Daily Reward XP
  boost" and "Store XP boost" lines (each with remaining hours or paused), "Reward
  Streak Bonus" (up to 7% chance of double weapon proficiency XP), "Claim", Premium
  reward hints. Removed: the resting-area streak bonus texts, the per-account reward
  table and the single shared "XP boost" line, "Already claimed today".
- **Gem Atelier (17 new):** "Open Gem Atelier", "Gem Vessel", "Modify Gem", "Replace
  Vessel", "Select a Vessel to Modify Gems", reshape rules (temple only, Premium only,
  gem must be unlocked, minimum fragments and gold per position), "Each node grants +%1
  Damage / Healing and unlocks a vessel", "one promotion point for each fully enhanced
  mod ... +5 Health, Mana and Capacity for each mod grade", "Revelation Mastery %1",
  "Promotion Points", import-warning for older Wheel configurations, dismantle requires
  removal from its vessel. Removed: Vessel Resonance / VR texts, Domain Affinity, domain
  switch, "cannot dismantle the last gem of this domain", mod grade dependency text.
- **QoL (5 new):** "Target Options:", "Cast in facing direction", "Cast towards cursor
  position", "Cast towards target", "Misc.: Create Mark". Removed: "Aim at target".
