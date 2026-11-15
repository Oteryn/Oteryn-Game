# Exact factor-ten weight audit for #3213

Compared `85c3ace69^` with `85c3ace69`, matching records by item identity within each JSON file (never by array index). Repeated canonical/native occurrences are deduplicated. All weights below are centi-ounces.

The five corrected rows have capture decimal-place errors; see `weight-corrections.json`. Other rows agree with the retained Global capture and its wiki cross-check and are left unchanged. The capture reports 1.50 oz and 2.00 oz for the two reward bags, and 10.00 oz for the soul cores and soul prism. These retained values match the capture/wiki marker; no fresh independent Global observation was available (the wiki API returned HTTP 402). No other exact factor-ten item changes were found in the changed JSON definitions/profiles.

| Item | Name | Before | #3213 | Capture (oz) | Verdict |
|---|---|---:|---:|---|---|
| i3031 | a gold coin | 10 | 1 | 0.01 oz | Wrong; corrected to 10 |
| i9147 | a glob of grease | 100 | 10 | 0.10 oz | Wrong; corrected to 100 |
| i19371 | a glass of goo | 1700 | 170 | 1.70 oz | Wrong; corrected to 1700 |
| i25729 | a dark moon mirror | 200 | 2000 | 20.00 oz | Wrong; corrected to 200 |
| i25730 | a lit moon mirror | 200 | 2000 | 20.00 oz | Wrong; corrected to 200 |
| i34109 | a bag you desire | 1500 | 150 | 1.50 oz | Matches Global capture/wiki; retained |
| i43895 | a bag you covet | 20 | 200 | 2.00 oz | Matches Global capture/wiki; retained |
| i47381 | an orc warlord soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47382 | a war wolf soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47383 | an orc rider soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47384 | an orc soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47385 | an orc shaman soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47386 | an orc warrior soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47387 | an orc berserker soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47388 | a necromancer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47389 | a warlock soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47390 | a hunter soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47391 | a valkyrie soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47392 | a sheep soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47393 | a troll soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47394 | a bear soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47395 | a bonelord soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47396 | a ghoul soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47397 | a slime soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47398 | a rat soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47399 | a cyclops soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47400 | a minotaur mage soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47401 | a minotaur archer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47402 | a minotaur soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47403 | a rotworm soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47404 | a wolf soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47405 | a snake soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47406 | a minotaur guard soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47407 | a spider soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47408 | a deer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47409 | a dog soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47410 | a skeleton soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47411 | a dragon soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47412 | a demon soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47413 | a poison spider soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47414 | a demon skeleton soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47415 | a giant spider soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47416 | a dragon lord soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47417 | a fire devil soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47418 | a lion soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47419 | a polar bear soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47420 | a scorpion soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47421 | a wasp soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47422 | a bug soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47423 | a wild warrior soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47424 | a ghost soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47425 | a fire elemental soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47426 | an orc spearman soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47427 | a green djinn soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47428 | a winter wolf soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47429 | a frost troll soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47430 | a witch soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47431 | a behemoth soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47432 | a cave rat soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47433 | a monk soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47434 | a priestess soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47435 | an orc leader soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47436 | a pig soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47437 | a goblin soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47438 | an elf soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47439 | an elf arcanist soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47440 | an elf scout soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47441 | a mummy soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47442 | a dwarf geomancer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47443 | a stone golem soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47444 | a vampire soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47445 | a dwarf soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47446 | a dwarf guard soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47447 | a dwarf soldier soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47448 | a stalker soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47449 | a hero soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47450 | a rabbit soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47451 | a swamp troll soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47452 | an amazon soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47453 | a banshee soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47454 | an ancient scarab soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47455 | a blue djinn soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47456 | a cobra soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47457 | a larva soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47458 | a scarab soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47459 | a hyaena soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47460 | a gargoyle soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47461 | a lich soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47462 | a crypt shambler soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47463 | a bonebeast soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47465 | an efreet soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47466 | a marid soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47467 | a badger soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47468 | a skunk soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47470 | an elder bonelord soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47471 | a gazer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47472 | a yeti soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47473 | a chicken soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47474 | a crab soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47475 | a lizard templar soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47476 | a lizard sentinel soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47477 | a lizard snakecharmer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47478 | a kongra soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47479 | a merlkin soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47480 | a sibang soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47481 | a crocodile soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47482 | a carniphila soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47483 | a hydra soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47484 | a bat soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47485 | a panda soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47486 | a centipede soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47487 | a tiger soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47488 | an elephant soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47489 | a flamingo soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47490 | a butterfly soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47491 | a dworc voodoomaster soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47492 | a dworc fleshhunter soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47493 | a dworc venomsniper soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47494 | a parrot soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47495 | a terror bird soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47496 | a tarantula soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47497 | a serpent spawn soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47498 | a spit nettle soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47499 | a smuggler soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47500 | a bandit soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47501 | an assassin soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47502 | a dark monk soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47503 | a butterfly soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47504 | a butterfly soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47505 | a water elemental soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47506 | a quara predator soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47507 | a quara predator scout soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47508 | a quara constrictor soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47509 | a quara constrictor scout soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47510 | a quara mantassin soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47511 | a quara mantassin scout soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47512 | a quara hydromancer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47513 | a quara hydromancer scout soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47514 | a quara pincher soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47515 | a quara pincher scout soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47516 | a pirate marauder soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47517 | a pirate cutthroat soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47518 | a pirate buccaneer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47519 | a pirate corsair soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47520 | a carrion worm soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47521 | an enlightened of the cult soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47522 | an acolyte of the cult soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47523 | an adept of the cult soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47524 | a novice of the cult soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47525 | a pirate skeleton soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47526 | a pirate ghost soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47527 | a tortoise soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47528 | a thornback tortoise soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47529 | a mammoth soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47530 | a blood crab soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47531 | a toad soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47532 | a seagull soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47533 | a son of verminor soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47534 | a green frog soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47535 | an azure frog soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47536 | a coral frog soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47537 | a crimson frog soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47538 | an orchid frog soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47539 | an island troll soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47540 | a massive water elemental soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47541 | a hand of cursed fate soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47542 | an undead dragon soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47543 | a lost soul soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47544 | a betrayed wraith soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47545 | a dark torturer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47546 | a spectre soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47547 | a destroyer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47548 | a diabolic imp soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47549 | a defiler soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47550 | a wyvern soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47551 | a fury soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47552 | a phantasm soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47553 | a hellhound soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47554 | a hellfire fighter soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47555 | a juggernaut soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47556 | a blightwalker soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47557 | a nightmare soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47558 | a nomad soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47559 | a massive fire elemental soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47560 | a plaguesmith soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47561 | a frost dragon soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47562 | a penguin soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47563 | a chakoya tribewarden soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47564 | a braindeath soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47565 | a barbarian skullhunter soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47566 | a barbarian bloodwalker soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47567 | a frost giant soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47568 | an ice golem soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47569 | a silver rabbit soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47570 | a chakoya toolshaper soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47571 | a chakoya windcaller soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47572 | a crystal spider soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47573 | an ice witch soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47574 | a barbarian brutetamer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47575 | a barbarian headsplitter soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47576 | a frost giantess soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47577 | a dire penguin soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47578 | a dark magician soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47579 | a dark apprentice soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47580 | a poacher soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47581 | a goblin leader soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47582 | a dwarf henchman soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47583 | a dryad soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47584 | a squirrel soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47585 | a dragon hatchling soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47586 | a dragon lord hatchling soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47587 | a cat soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47588 | an undead jester soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47589 | a cyclops smith soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47590 | a cyclops drone soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47591 | a troll champion soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47592 | a grynch clan goblin soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47593 | a frost dragon hatchling soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47594 | a deepsea blood crab soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47595 | a sea serpent soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47596 | a young sea serpent soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47598 | a skeleton warrior soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47600 | a massive earth elemental soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47601 | an energy elemental soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47602 | an earth elemental soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47603 | a bog raider soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47604 | a wyrm soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47605 | a wisp soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47606 | a goblin assassin soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47607 | a goblin scavenger soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47608 | a grim reaper soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47613 | a vampire bride soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47619 | a mutated rat soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47620 | a worker golem soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47621 | an undead gladiator soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47622 | a mutated bat soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47623 | a werewolf soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47624 | a haunted treeling soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47625 | a zombie soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47626 | an acid blob soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47627 | a death blob soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47628 | a mercury blob soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47629 | a mutated tiger soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47631 | a nightmare scion soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47632 | a hellspawn soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47633 | a nightstalker soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47634 | a mutated human soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47635 | a gozzler soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47636 | a damaged worker golem soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47637 | a crazed beggar soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47638 | a gang member soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47639 | a gladiator soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47640 | a mad scientist soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47641 | an infernalist soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47642 | a war golem soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47643 | a furious troll soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47644 | a troll legionnaire soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47645 | an evil sheep soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47646 | a medusa soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47653 | an acolyte of darkness soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47654 | a nightslayer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47655 | a bane of light soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47656 | a duskbringer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47657 | a shadow hound soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47658 | a doomsday cultist soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47659 | a midnight spawn soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47660 | a midnight warrior soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47661 | a herald of gloom soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47662 | a bride of night soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47667 | an orc marauder soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47668 | an eternal guardian soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47669 | a lizard zaogun soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47670 | a draken warmaster soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47671 | a draken spellweaver soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47672 | a lizard chosen soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47673 | an insect swarm soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47674 | a lizard dragon priest soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47675 | a lizard legionnaire soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47676 | a lizard high guard soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47677 | a killer caiman soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47678 | a gnarlhound soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47679 | a terramite soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47680 | a wailing widow soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47681 | a lancer beetle soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47682 | a sandcrawler soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47683 | a ghastly dragon soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47684 | a lizard magistratus soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47685 | a lizard noble soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47686 | a draken elite soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47687 | a draken abomination soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47688 | a brimstone bug soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47689 | a souleater soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47690 | a bane bringer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47691 | a cake golem soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47697 | a berrypest soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47698 | a boar soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47699 | a stampor soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47700 | a draptor soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47701 | an undead cavebear soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47702 | a crustacea gigantica soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47703 | a midnight panther soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47704 | an iron servant soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47705 | a golden servant soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47706 | a diamond servant soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47707 | a clay guardian soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47708 | a yielothax soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47709 | a shaburak demon soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47710 | an askarak demon soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47711 | a wild horse soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47712 | a slug soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47713 | a dromedary soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47714 | a deepling scout soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47715 | a bog frog soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47716 | a thornfire wolf soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47717 | a crystal wolf soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47718 | an elf overseer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47719 | a troll guard soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47722 | a deepling warrior soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47723 | a deepling guard soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47724 | a deepling spellsinger soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47725 | a shark soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47726 | a fish soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47727 | a crawler soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47729 | a swarmer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47730 | a spitter soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47731 | a waspoid soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47732 | a deepling worker soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47733 | an insectoid worker soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47734 | a crystalcrusher soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47735 | a mushroom sniffer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47737 | a water buffalo soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47738 | an enraged crystal golem soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47739 | a drillworm soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47740 | a humongous fungus soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47741 | a magma crawler soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47742 | an enslaved dwarf soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47743 | a lost berserker soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47744 | a hideous fungus soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47745 | a vulcongra soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47746 | a wiggler soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47747 | an emerald damselfly soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47748 | a salamander soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47749 | a marsh stalker soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47750 | a corym charlatan soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47751 | a corym skirmisher soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47752 | a corym vanguard soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47753 | a swampling soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47754 | an adventurer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47755 | a lost husher soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47756 | a lost basher soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47757 | a lost thrower soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47758 | a vampire viscount soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47759 | a vicious manbat soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47760 | a shadow pupil soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47761 | a blood priest soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47762 | a white shade soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47763 | an elder wyrm soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47764 | a nightfiend soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47765 | a blood hand soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47766 | a gravedigger soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47767 | a tarnished spirit soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47768 | a rorc soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47769 | a leaf golem soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47770 | a forest fury soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47771 | a roaring lion soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47772 | a wilting leaf golem soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47773 | a furious fire elemental soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47774 | a shock head soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47775 | a sight of surrender soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47776 | a guzzlemaw soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47777 | a silencer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47778 | a choking fear soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47779 | a terrorsleep soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47780 | a retching horror soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47781 | a demon outcast soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47783 | a feversleep soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47784 | a frazzlemaw soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47786 | a glooth golem soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47787 | a metal gargoyle soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47788 | a blood beast soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47789 | a rustheap golem soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47790 | a glooth anemone soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47791 | a moohtant soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47792 | a minotaur amazon soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47793 | an execowtioner soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47794 | a mooh'tah warrior soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47795 | a minotaur hunter soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47796 | a worm priestess soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47797 | a glooth blob soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47798 | a rot elemental soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47799 | a devourer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47810 | an abyssal calamary soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47812 | a high voltage elemental soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47813 | a glooth bandit soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47814 | a glooth brigand soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47815 | a raging fire soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47816 | a dawnfire asura soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47817 | a midnight asura soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47818 | a tainted soul soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47819 | an omnivora soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47820 | a werebear soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47821 | a wereboar soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47822 | a werebadger soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47823 | a vicious squire soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47824 | a renegade knight soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47825 | a vile grandmaster soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47826 | an ogre brute soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47827 | an ogre savage soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47828 | an ogre shaman soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47829 | a clomp soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47830 | a grimeleech soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47831 | a vexclaw soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47832 | a hellflayer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47838 | a reality reaver soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47839 | a sparkion soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47840 | a breach brood soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47842 | a dread intruder soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47844 | an instable sparkion soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47845 | an instable breach brood soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47846 | a stabilizing reality reaver soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47847 | a stabilizing dread intruder soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47848 | a cave parrot soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47849 | an orclops doomhauler soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47850 | an orclops ravager soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47851 | a broken shaper soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47852 | a twisted shaper soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47853 | an iron servant replica soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47854 | a shaper matriarch soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47855 | a misguided bully soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47856 | a misguided thief soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47857 | a putrid mummy soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47858 | a faun soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47859 | a pooka soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47860 | a twisted pooka soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47861 | a pixie soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47862 | a boogy soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47863 | a weakened frazzlemaw soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47864 | an enfeebled silencer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47865 | a nymph soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47866 | a barkless devotee soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47867 | a barkless fanatic soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47868 | a dark faun soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47869 | an orc cult inquisitor soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47870 | an orc cult minion soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47871 | a minotaur cult follower soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47872 | a minotaur cult prophet soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47873 | a minotaur cult zealot soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47874 | a cult believer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47875 | a cult enforcer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47876 | a stonerefiner soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47877 | a lost exile soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47878 | a deepworm soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47879 | a diremaw soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47880 | a cave devourer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47881 | a tunnel tyrant soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47882 | a chasm spawn soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47883 | a fox soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47884 | a werefox soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47886 | a lava lurker soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47888 | a ravenous lava lurker soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47890 | a frost flower asura soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47891 | a true dawnfire asura soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47892 | a true midnight asura soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47893 | a true frost flower asura soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47896 | a floating savant soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47900 | a falcon paladin soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47901 | a brain squid soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47902 | a biting book soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47903 | an ink blob soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47904 | a burning book soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47905 | an icecold book soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47906 | an energetic book soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47907 | an energuardian of tales soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47908 | a deathling scout soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47909 | a rage squid soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47910 | a squid warden soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47911 | an animated feather soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47913 | a skeleton elite warrior soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47914 | an undead elite gladiator soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47915 | a deathling spellsinger soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47916 | a lumbering carnivor soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47917 | a spiky carnivor soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47918 | a menacing carnivor soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47919 | a ripper spectre soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47920 | a gazer spectre soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47921 | a burster spectre soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47922 | a thanatursus soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47923 | an arachnophobica soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47924 | a crazed winter vanguard soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47925 | a crazed winter rearguard soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47926 | a crazed summer vanguard soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47927 | a crazed summer rearguard soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47928 | a soul-broken harbinger soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47929 | an insane siren soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47930 | a cobra assassin soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47931 | a cobra scout soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47933 | a burning gladiator soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47934 | a priestess of the wild sun soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47935 | a black sphinx acolyte soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47936 | a crypt warden soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47937 | a lamassu soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47938 | a feral sphinx soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47939 | a sphinx soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47940 | a manticore soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47941 | a young goanna soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47942 | an adult goanna soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47943 | an ogre ruffian soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47944 | an ogre rowdy soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47945 | an ogre sage soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47946 | a cobra vizier soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47947 | a flimsy lost soul soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47948 | a mean lost soul soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47949 | a freakish lost soul soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47950 | a cursed prospector soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47951 | an evil prospector soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47952 | a bony sea devil soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47953 | many faces soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47954 | a cloak of terror soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47955 | a vibrant phantom soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47956 | a brachiodemon soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47957 | a branchy crawler soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47958 | a capricious phantom soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47959 | an infernal phantom soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47961 | an infernal demon soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47962 | a rotten golem soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47963 | a turbulent elemental soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47964 | a courage leech soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47965 | a mould phantom soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47966 | a druid's apparition soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47967 | a knight's apparition soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47968 | a paladin's apparition soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47969 | a sorcerer's apparition soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47973 | a distorted phantom soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47974 | a werehyaena soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47975 | a werehyaena shaman soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47976 | a werelion soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47977 | a werelioness soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47978 | a usurper knight soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47979 | a usurper archer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47980 | a usurper warlock soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47981 | an agrestic chicken soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47984 | a crypt warrior soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47985 | an exotic cave spider soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47986 | a pirat cutthroat soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47987 | a pirat scoundrel soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47988 | a pirat bombardier soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47989 | a pirat mate soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47990 | an exotic bat soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47991 | a lavaworm soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47992 | a tremendous tyrant soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47993 | a varnished diremaw soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47994 | a streaked devourer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47995 | an eyeless devourer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47996 | a blemished spawn soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47997 | an afflicted strider soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47998 | a lavafungus soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i47999 | a cave chimera soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48000 | a venerable girtablilu soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48001 | a girtablilu warrior soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48002 | a bashmu soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48003 | a juvenile bashmu soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48004 | a hulking carnisylvan soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48005 | a poisonous carnisylvan soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48006 | a dark carnisylvan soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48032 | a parder soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48033 | a jungle moa soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48034 | a two-headed turtle soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48035 | a foam stalker soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48036 | a naga archer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48037 | a naga warrior soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48038 | a makara soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48039 | a sulphider soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48040 | a sulphur spouter soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48041 | a gore horn soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48042 | a sabretooth soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48043 | an emerald tortoise soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48044 | an undertaker soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48045 | a nighthunter soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48046 | a hulking prehemoth soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48047 | a stalking stalk soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48048 | a mantosaurus soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48049 | a headpecker soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48050 | a noxious ripptor soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48051 | a gorerilla soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48052 | a shrieking cry-stal soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48053 | a mercurial menace soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48054 | a crape man soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48055 | a liodile soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48056 | a boar man soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48057 | a harpy soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48058 | a carnivostrich soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48059 | a rhindeer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48060 | an iks pututu soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48061 | an iks aucar soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48062 | an iks chuka soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48063 | a cursed ape soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48064 | an iks ahpututu soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48065 | a mycobiontic beetle soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48066 | a meandering mushroom soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48067 | an oozing carcass soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48068 | a darklight construct soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48069 | a converter soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48070 | a darklight matter soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48071 | an oozing corpus soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48072 | a darklight emitter soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48073 | a weretiger soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48074 | a white weretiger soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48075 | a werecrocodile soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48076 | a feral werecrocodile soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48077 | a werepanther soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48078 | a white tiger soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48079 | a bloated man-maggot soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48080 | a rotten man-maggot soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48081 | a walking pillar soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48082 | a wandering pillar soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48083 | a sopping carcass soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48084 | a sopping corpus soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48085 | a darklight source soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48086 | a darklight striker soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48087 | a cunning werepanther soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48089 | an albino dragon soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48091 | an iks yapunac soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48092 | a bulltaur brute soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48093 | a bulltaur alchemist soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48094 | a bulltaur forgepriest soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48095 | a dragolisk soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48096 | a wardragon soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48097 | a mega dragon soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48098 | a mitmah scout soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48099 | a mitmah seer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48929 | a black sheep soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48930 | a squidgy slime soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48931 | a husky soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48932 | a massive energy elemental soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48933 | an evil sheep lord soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48934 | a hot dog soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48935 | a vampire pig soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48936 | a doom deer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48937 | a killer rabbit soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48938 | a berserker chicken soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48939 | a demon parrot soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48940 | an infernal frog soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48941 | an undead mine worker soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48942 | an undead prospector soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48943 | a ghoulish hyaena soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48944 | a sandstone scorpion soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48945 | a grave guard soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48946 | a tomb servant soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48947 | a sacred spider soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48948 | a death priest soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48949 | an elder mummy soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48950 | an honour guard soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48951 | a feverish citizen soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48952 | a white deer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48953 | a starving wolf soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48954 | a shaburak lord soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48955 | a shaburak prince soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48956 | an askarak lord soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48957 | an askarak prince soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48958 | an insectoid scout soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48959 | a filth toad soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48960 | a firestarter soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48961 | a horse soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48962 | a horse soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48963 | a horse soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48964 | a terrified elephant soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48965 | a nomad soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48966 | a nomad soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48967 | a ladybug soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48968 | a manta ray soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48969 | a calamary soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48970 | a jellyfish soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48971 | a northern pike soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48972 | a spidris soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48973 | a kollos soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48974 | a spidris elite soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48975 | a hive overseer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48976 | a deepling brawler soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48977 | a deepling master librarian soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48978 | a deepling tyrant soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48979 | a deepling elite soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48980 | a grave robber soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48981 | a crypt defiler soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48982 | a damaged crystal golem soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48983 | a modified gnarlhound soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48984 | a stone devourer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48985 | an armadile soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48986 | a weeper soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48987 | an orewalker soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48988 | a lava golem soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48989 | a cliff strider soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48990 | an ironblight soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48991 | a dragonling soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48992 | an infected weeper soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48993 | a pigeon soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48994 | a little corym charlatan soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48995 | a walker soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48996 | a seacrest serpent soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48997 | a renegade quara constrictor soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48998 | a renegade quara hydromancer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i48999 | a renegade quara mantassin soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49000 | a renegade quara pincher soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49001 | a renegade quara predator soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49002 | a minotaur invader soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49003 | a noble lion soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49004 | a redeemed soul soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49005 | a gloom wolf soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49006 | a ghost wolf soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49007 | an elder forest fury soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49008 | a diamond servant replica soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49009 | a golden servant replica soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49010 | a haunted dragon soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49011 | an ice dragon soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49012 | a stone rhino soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49013 | a swan maiden soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49014 | a goldhanded cultist soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49015 | a goldhanded cultist bride soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49016 | an orc cultist soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49017 | an orc cult priest soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49018 | an orc cult fanatic soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49019 | a cult scholar soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49020 | a mole soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49021 | an arctic faun soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49022 | a flying book soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49023 | a cursed book soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49024 | a guardian of tales soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49025 | a knowledge elemental soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49026 | a lacewing moth soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49027 | a hibernal moth soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49028 | a percht soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49029 | a schiach soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49030 | a baleful bunny soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49031 | an animated snowman soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49032 | a gryphon soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49033 | an orger soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49034 | a roast pork soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49035 | a cow soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49036 | a loricate orger soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49037 | a bellicose orger soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49038 | a white lion soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49039 | an iks churrascan soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49040 | a rabid wolf soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49041 | a ragged rabid wolf soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49073 | a nibblemaw soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49074 | a candy floss elemental soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49075 | a goggle cake soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49076 | a candy horror soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49077 | a rootthing bug tracker soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49078 | a rootthing amber shaper soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49079 | a rootthing nutshell soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49080 | a quara raider soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49081 | a quara plunderer soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49082 | a quara looter soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49084 | a truffle soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49085 | a sugar cube soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49086 | a gingerbread man soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49087 | a chocolate blob soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49088 | a honey elemental soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49089 | an angry sugar fairy soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49090 | a truffle cook soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49092 | a fruit drop soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49094 | a sugar cube worker soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49095 | a cream blob soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49162 | a wafer paper butterfly soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49163 | an ominous soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i49164 | a soul prism | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i50103 | a spellreaper inferniarch soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i50104 | a hellhunter inferniarch soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i50105 | a brinebrute inferniarch soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i50106 | a sineater inferniarch soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i50107 | a broodrider inferniarch soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i50108 | a gorger inferniarch soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i50330 | a monk's apparition soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i52358 | a muglex clan footman soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i52359 | a muglex clan assassin soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i52360 | a corrupted ghost soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |
| i52361 | a corrupted skeleton soul core | 100 | 1000 | 10.00 oz | Matches Global capture/wiki; retained |

## Hireling regression triage

The exact requested Cook bank-only, Cook carrot/package/replay, and Steward stash/replay/ACL
cases each executed and passed on origin/main `beaaf9bc1bfff4f3d8a8b6afc3c39f4217cb80a6`
using the unmodified dispatch binary compiled from that commit and an exact origin/main
content snapshot. Each emitted `HIRELING_LIFECYCLE POSTGRES_CASE_COMPLETED`.
The in-progress generated content was preserved and restored around this baseline run.
The reported CI failures are not reproduced on that main snapshot; no additional weight
correction is justified by them.

Cook's cited `cook.rs:266` asserts a historical receipt replay.
`hireling_services/purchase.rs:388` reads the receipt and never consults item weights.
Steward's `steward.rs:34` calls `atelier.rs:137` / `hireling_services.rs:185`, which checks
current House access, Steward entitlement and NPC conversation context before stash
transfers. Those checks also do not consult item weights. No weight root cause is confirmed
for these three CI reports.

| Fixture item | Before #3213 | origin/main | Global capture | Disposition |
|---|---:|---:|---|---|
| i2854 backpack | 1800 | 1800 | 18.00 oz | Correct; retained |
| i2856 present | 600 | 600 | 6.00 oz | Correct; retained |
| i3028 small diamond | 10 | 10 | 0.10 oz | Correct; retained |
| i3031 gold coin | 10 | 1 | Erroneous 0.01 oz | Corrected to 10 |
| i3035 platinum coin | 10 | 10 | 0.10 oz | Correct; retained |
| i3043 crystal coin | 10 | 10 | 0.10 oz | Correct; retained |
| i9657 cyclops toe | 111 | 111 | 1.11 oz | Matches capture; retained |
| i10405 warmaster's wristguards | 120 | 120 | 1.20 oz | Matches capture; retained |
| i22189 ogre nose ring | 2100 | 420 | 4.20 oz | Matches Global; retained |
| i29409 carrot pie | 620 | 620 | 6.20 oz | Matches Global; retained |
| i35173 toolbox (asserted absent) | 0 | 0 | Not captured | No admitted output; retained |

## Changes larger than tenfold

An identity-matched sweep of all changed JSON profiles/definitions in #3213 finds
792 positive-weight changes of at least tenfold: the 777 exact changes above plus
the following 15. None is a starter item. The retained capture is not sufficient to
resolve every conflict independently; uncertain values remain unchanged.

| Item | Name | Before | #3213 | Capture | Verdict |
|---|---|---:|---:|---|---|
| i12045 | a music box | 200 | 2700 | 27.00 oz | Matches capture/wiki marker; independent Global confirmation uncertain; retained |
| i12057 | a reward box | 100 | 1800 | 18.00 oz | Capture/wiki conflict; Global value uncertain; retained |
| i12147 | cake | 500 | 1 | 0.01 oz | Matches capture/wiki marker; independent Global confirmation uncertain; retained |
| i12148 | cake with a cherry | 500 | 1 | 0.01 oz | Matches capture/wiki marker; independent Global confirmation uncertain; retained |
| i12152 | cake | 500 | 1 | 0.01 oz | Matches capture/wiki marker; independent Global confirmation uncertain; retained |
| i12153 | cake | 500 | 1 | 0.01 oz | Matches capture/wiki marker; independent Global confirmation uncertain; retained |
| i12154 | cake | 500 | 1 | 0.01 oz | Matches capture/wiki marker; independent Global confirmation uncertain; retained |
| i12155 | cake | 500 | 1 | 0.01 oz | Matches capture/wiki marker; independent Global confirmation uncertain; retained |
| i24949 | a costume bag | 100 | 3800 | 38.00 oz | Matches capture/wiki marker; independent Global confirmation uncertain; retained |
| i25689 | a golden brush | 1240 | 100 | 1.00 oz | Capture/wiki conflict; Global value uncertain; retained |
| i32626 | an amber | 1250 | 100 | 1.00 oz | Matches capture/wiki marker; independent Global confirmation uncertain; retained |
| i33935 | a crown | 1900 | 90 | 0.90 oz | Matches capture/wiki marker; independent Global confirmation uncertain; retained |
| i39395 | a mercurial wing | 10 | 470 | 4.70 oz | Capture/wiki conflict; Global value uncertain; retained |
| i46626 | a greater fragment | 500 | 10 | 0.10 oz | Matches capture/wiki marker; independent Global confirmation uncertain; retained |
| i903 | a badger fur | 65 | 1000 | 10.00 oz | Matches capture/wiki marker; independent Global confirmation uncertain; retained |

## Starter/capacity and node-boot triage

All 28 distinct Crystal starter item identities, including the backpack, have
identical active weights before #3213, on origin/main, and in this repair.
The complete kit totals (backpack, worn equipment and carried quantities included) are:

| Vocation | Centi-ounces | Ounces |
|---|---:|---:|
| none | 7100 | 71.00 |
| sorcerer | 18600 | 186.00 |
| druid | 18600 | 186.00 |
| paladin | 34400 | 344.00 |
| knight | 42700 | 427.00 |
| monk | 31000 | 310.00 |

The vocationless kit is 71 oz, below the Global capacity projection's minimum
580 oz at level 1 (`item_capacity.rs:86-108`). The other kits retain their
pre-#3213 weights. The largest kit is 427 oz. The historical first-entry grant
checks item identity, stack limits and container slots, not carried mass
(`durability/starter_grant.rs:98-122`). The separate Newhaven grant requires known
weights but does not compare their aggregate mass with carry capacity.

C01 (`full_row_c01.rs:189-209`) requires a legitimately equipped weapon and armor.
The provisioned no-vocation kit intentionally has no worn equipment
(`items/src/starting_loadout.rs:12`; `full_row_c19b.rs:1-5,68-84`). C19b explicitly
reports the missing reachable vocation-change route after proving rope/shovel
custody and empty equipment. Giving the no-vocation kit weapons to silence C01
would change gameplay policy and conceal that missing route. No starter weight
correction is supported by this evidence. A boot custody `Unavailable` alone
does not establish a weight cause; recovery/fence/database failures can produce
that result before any weight accounting.
