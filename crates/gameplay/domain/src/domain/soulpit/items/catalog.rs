//! Donor Item catalogue, not native identity authority.
//! Crystal 201189f871d23597b82caf88fd611454a2ddefb8: full runtime union of XML primarytype
//! SoulCores and appearances.dat market.category26 (items.cpp:215-219).
//! Game.getSoulCoreItems iterates Item::items by source id (game_functions.cpp:323-338).
//! XML blob ea4196beae8f77be898f05de89db81de134886b6; appearances blob37ce9c8ee497b7edffe809f40d02c5d12ec4ccf0.

pub const SOUL_CORE_SOURCE_IDS: &[u32] = &[
    47381, 47382, 47383, 47384, 47385, 47386, 47387, 47388, 47389, 47390, 47391, 47392, 47393,
    47394, 47395, 47396, 47397, 47398, 47399, 47400, 47401, 47402, 47403, 47404, 47405, 47406,
    47407, 47408, 47409, 47410, 47411, 47412, 47413, 47414, 47415, 47416, 47417, 47418, 47419,
    47420, 47421, 47422, 47423, 47424, 47425, 47426, 47427, 47428, 47429, 47430, 47431, 47432,
    47433, 47434, 47435, 47436, 47437, 47438, 47439, 47440, 47441, 47442, 47443, 47444, 47445,
    47446, 47447, 47448, 47449, 47450, 47451, 47452, 47453, 47454, 47455, 47456, 47457, 47458,
    47459, 47460, 47461, 47462, 47463, 47465, 47466, 47467, 47468, 47470, 47471, 47472, 47473,
    47474, 47475, 47476, 47477, 47478, 47479, 47480, 47481, 47482, 47483, 47484, 47485, 47486,
    47487, 47488, 47489, 47490, 47491, 47492, 47493, 47494, 47495, 47496, 47497, 47498, 47499,
    47500, 47501, 47502, 47503, 47504, 47505, 47506, 47507, 47508, 47509, 47510, 47511, 47512,
    47513, 47514, 47515, 47516, 47517, 47518, 47519, 47520, 47521, 47522, 47523, 47524, 47525,
    47526, 47527, 47528, 47529, 47530, 47531, 47532, 47533, 47534, 47535, 47536, 47537, 47538,
    47539, 47540, 47541, 47542, 47543, 47544, 47545, 47546, 47547, 47548, 47549, 47550, 47551,
    47552, 47553, 47554, 47555, 47556, 47557, 47558, 47559, 47560, 47561, 47562, 47563, 47564,
    47565, 47566, 47567, 47568, 47569, 47570, 47571, 47572, 47573, 47574, 47575, 47576, 47577,
    47578, 47579, 47580, 47581, 47582, 47583, 47584, 47585, 47586, 47587, 47588, 47589, 47590,
    47591, 47592, 47593, 47594, 47595, 47596, 47598, 47600, 47601, 47602, 47603, 47604, 47605,
    47606, 47607, 47608, 47613, 47619, 47620, 47621, 47622, 47623, 47624, 47625, 47626, 47627,
    47628, 47629, 47631, 47632, 47633, 47634, 47635, 47636, 47637, 47638, 47639, 47640, 47641,
    47642, 47643, 47644, 47645, 47646, 47653, 47654, 47655, 47656, 47657, 47658, 47659, 47660,
    47661, 47662, 47667, 47668, 47669, 47670, 47671, 47672, 47673, 47674, 47675, 47676, 47677,
    47678, 47679, 47680, 47681, 47682, 47683, 47684, 47685, 47686, 47687, 47688, 47689, 47690,
    47691, 47697, 47698, 47699, 47700, 47701, 47702, 47703, 47704, 47705, 47706, 47707, 47708,
    47709, 47710, 47711, 47712, 47713, 47714, 47715, 47716, 47717, 47718, 47719, 47722, 47723,
    47724, 47725, 47726, 47727, 47729, 47730, 47731, 47732, 47733, 47734, 47735, 47737, 47738,
    47739, 47740, 47741, 47742, 47743, 47744, 47745, 47746, 47747, 47748, 47749, 47750, 47751,
    47752, 47753, 47754, 47755, 47756, 47757, 47758, 47759, 47760, 47761, 47762, 47763, 47764,
    47765, 47766, 47767, 47768, 47769, 47770, 47771, 47772, 47773, 47774, 47775, 47776, 47777,
    47778, 47779, 47780, 47781, 47783, 47784, 47786, 47787, 47788, 47789, 47790, 47791, 47792,
    47793, 47794, 47795, 47796, 47797, 47798, 47799, 47810, 47812, 47813, 47814, 47815, 47816,
    47817, 47818, 47819, 47820, 47821, 47822, 47823, 47824, 47825, 47826, 47827, 47828, 47829,
    47830, 47831, 47832, 47838, 47839, 47840, 47842, 47844, 47845, 47846, 47847, 47848, 47849,
    47850, 47851, 47852, 47853, 47854, 47855, 47856, 47857, 47858, 47859, 47860, 47861, 47862,
    47863, 47864, 47865, 47866, 47867, 47868, 47869, 47870, 47871, 47872, 47873, 47874, 47875,
    47876, 47877, 47878, 47879, 47880, 47881, 47882, 47883, 47884, 47886, 47888, 47890, 47891,
    47892, 47893, 47896, 47899, 47900, 47901, 47902, 47903, 47904, 47905, 47906, 47907, 47908,
    47909, 47910, 47911, 47913, 47914, 47915, 47916, 47917, 47918, 47919, 47920, 47921, 47922,
    47923, 47924, 47925, 47926, 47927, 47928, 47929, 47930, 47931, 47933, 47934, 47935, 47936,
    47937, 47938, 47939, 47940, 47941, 47942, 47943, 47944, 47945, 47946, 47947, 47948, 47949,
    47950, 47951, 47952, 47953, 47954, 47955, 47956, 47957, 47958, 47959, 47961, 47962, 47963,
    47964, 47965, 47966, 47967, 47968, 47969, 47973, 47974, 47975, 47976, 47977, 47978, 47979,
    47980, 47981, 47984, 47985, 47986, 47987, 47988, 47989, 47990, 47991, 47992, 47993, 47994,
    47995, 47996, 47997, 47998, 47999, 48000, 48001, 48002, 48003, 48004, 48005, 48006, 48032,
    48033, 48034, 48035, 48036, 48037, 48038, 48039, 48040, 48041, 48042, 48043, 48044, 48045,
    48046, 48047, 48048, 48049, 48050, 48051, 48052, 48053, 48054, 48055, 48056, 48057, 48058,
    48059, 48060, 48061, 48062, 48063, 48064, 48065, 48066, 48067, 48068, 48069, 48070, 48071,
    48072, 48073, 48074, 48075, 48076, 48077, 48078, 48079, 48080, 48081, 48082, 48083, 48084,
    48085, 48086, 48087, 48089, 48091, 48092, 48093, 48094, 48095, 48096, 48097, 48098, 48099,
    48929, 48930, 48931, 48932, 48933, 48934, 48935, 48936, 48937, 48938, 48939, 48940, 48941,
    48942, 48943, 48944, 48945, 48946, 48947, 48948, 48949, 48950, 48951, 48952, 48953, 48954,
    48955, 48956, 48957, 48958, 48959, 48960, 48961, 48962, 48963, 48964, 48965, 48966, 48967,
    48968, 48969, 48970, 48971, 48972, 48973, 48974, 48975, 48976, 48977, 48978, 48979, 48980,
    48981, 48982, 48983, 48984, 48985, 48986, 48987, 48988, 48989, 48990, 48991, 48992, 48993,
    48994, 48995, 48996, 48997, 48998, 48999, 49000, 49001, 49002, 49003, 49004, 49005, 49006,
    49007, 49008, 49009, 49010, 49011, 49012, 49013, 49014, 49015, 49016, 49017, 49018, 49019,
    49020, 49021, 49022, 49023, 49024, 49025, 49026, 49027, 49028, 49029, 49030, 49031, 49032,
    49033, 49034, 49035, 49036, 49037, 49038, 49039, 49040, 49041, 49073, 49074, 49075, 49076,
    49077, 49078, 49079, 49080, 49081, 49082, 49084, 49085, 49086, 49087, 49088, 49089, 49090,
    49092, 49094, 49095, 49162, 49163, 49164, 50103, 50104, 50105, 50106, 50107, 50108, 50330,
    51641, 51642, 51643, 51644, 51645, 51646, 51647, 51648, 51649, 51650, 51651, 51652, 51653,
    51654, 51721, 52358, 52359, 52360, 52361, 52800, 52801, 52802, 52807, 52808, 52809, 52810,
    52811, 52812, 52813, 52814, 52815, 52865, 52866, 53142, 53143, 53144, 53145, 53146, 53147,
    53148, 53149, 53150, 53151, 53152, 53153, 53154, 53155, 54587, 54588, 54589, 54590, 54591,
    54592, 54593, 54594, 54595, 54596, 54597, 54598, 54599, 54600, 54601, 54602, 54603, 54604,
    54630, 54631, 54632,
];

const NAMES: &[(u32, &str)] = &[
    (47381, "orc warlord soul core"),                // items.xml:78306
    (47382, "war wolf soul core"),                   // items.xml:78311
    (47383, "orc rider soul core"),                  // items.xml:78316
    (47384, "orc soul core"),                        // items.xml:78321
    (47385, "orc shaman soul core"),                 // items.xml:78326
    (47386, "orc warrior soul core"),                // items.xml:78331
    (47387, "orc berserker soul core"),              // items.xml:78336
    (47388, "necromancer soul core"),                // items.xml:78341
    (47389, "warlock soul core"),                    // items.xml:78346
    (47390, "hunter soul core"),                     // items.xml:78351
    (47391, "valkyrie soul core"),                   // items.xml:78356
    (47392, "sheep soul core"),                      // items.xml:78361
    (47393, "troll soul core"),                      // items.xml:78366
    (47394, "bear soul core"),                       // items.xml:78371
    (47395, "bonelord soul core"),                   // items.xml:78376
    (47396, "ghoul soul core"),                      // items.xml:78381
    (47397, "slime soul core"),                      // items.xml:78386
    (47398, "rat soul core"),                        // items.xml:78391
    (47399, "cyclops soul core"),                    // items.xml:78396
    (47400, "minotaur mage soul core"),              // items.xml:78401
    (47401, "minotaur archer soul core"),            // items.xml:78406
    (47402, "minotaur soul core"),                   // items.xml:78411
    (47403, "rotworm soul core"),                    // items.xml:78416
    (47404, "wolf soul core"),                       // items.xml:78421
    (47405, "snake soul core"),                      // items.xml:78426
    (47406, "minotaur guard soul core"),             // items.xml:78431
    (47407, "spider soul core"),                     // items.xml:78436
    (47408, "deer soul core"),                       // items.xml:78441
    (47409, "dog soul core"),                        // items.xml:78446
    (47410, "skeleton soul core"),                   // items.xml:78451
    (47411, "dragon soul core"),                     // items.xml:78456
    (47412, "demon soul core"),                      // items.xml:78461
    (47413, "poison spider soul core"),              // items.xml:78466
    (47414, "demon skeleton soul core"),             // items.xml:78471
    (47415, "giant spider soul core"),               // items.xml:78476
    (47416, "dragon lord soul core"),                // items.xml:78481
    (47417, "fire devil soul core"),                 // items.xml:78486
    (47418, "lion soul core"),                       // items.xml:78491
    (47419, "polar bear soul core"),                 // items.xml:78496
    (47420, "scorpion soul core"),                   // items.xml:78501
    (47421, "wasp soul core"),                       // items.xml:78506
    (47422, "bug soul core"),                        // items.xml:78511
    (47423, "wild warrior soul core"),               // items.xml:78516
    (47424, "ghost soul core"),                      // items.xml:78521
    (47425, "fire elemental soul core"),             // items.xml:78526
    (47426, "orc spearman soul core"),               // items.xml:78531
    (47427, "green djinn soul core"),                // items.xml:78536
    (47428, "winter wolf soul core"),                // items.xml:78541
    (47429, "frost troll soul core"),                // items.xml:78546
    (47430, "witch soul core"),                      // items.xml:78551
    (47431, "behemoth soul core"),                   // items.xml:78556
    (47432, "cave rat soul core"),                   // items.xml:78561
    (47433, "monk soul core"),                       // items.xml:78566
    (47434, "priestess soul core"),                  // items.xml:78571
    (47435, "orc leader soul core"),                 // items.xml:78576
    (47436, "pig soul core"),                        // items.xml:78581
    (47437, "goblin soul core"),                     // items.xml:78586
    (47438, "elf soul core"),                        // items.xml:78591
    (47439, "elf arcanist soul core"),               // items.xml:78596
    (47440, "elf scout soul core"),                  // items.xml:78601
    (47441, "mummy soul core"),                      // items.xml:78606
    (47442, "dwarf geomancer soul core"),            // items.xml:78611
    (47443, "stone golem soul core"),                // items.xml:78616
    (47444, "vampire soul core"),                    // items.xml:78621
    (47445, "dwarf soul core"),                      // items.xml:78626
    (47446, "dwarf guard soul core"),                // items.xml:78631
    (47447, "dwarf soldier soul core"),              // items.xml:78636
    (47448, "stalker soul core"),                    // items.xml:78641
    (47449, "hero soul core"),                       // items.xml:78646
    (47450, "rabbit soul core"),                     // items.xml:78651
    (47451, "swamp troll soul core"),                // items.xml:78656
    (47452, "amazon soul core"),                     // items.xml:78661
    (47453, "banshee soul core"),                    // items.xml:78666
    (47454, "ancient scarab soul core"),             // items.xml:78671
    (47455, "blue djinn soul core"),                 // items.xml:78676
    (47456, "cobra soul core"),                      // items.xml:78681
    (47457, "larva soul core"),                      // items.xml:78686
    (47458, "scarab soul core"),                     // items.xml:78691
    (47459, "hyaena soul core"),                     // items.xml:78696
    (47460, "gargoyle soul core"),                   // items.xml:78701
    (47461, "lich soul core"),                       // items.xml:78706
    (47462, "crypt shambler soul core"),             // items.xml:78711
    (47463, "bonebeast soul core"),                  // items.xml:78716
    (47465, "efreet soul core"),                     // items.xml:78721
    (47466, "marid soul core"),                      // items.xml:78726
    (47467, "badger soul core"),                     // items.xml:78731
    (47468, "skunk soul core"),                      // items.xml:78736
    (47470, "elder bonelord soul core"),             // items.xml:78741
    (47471, "gazer soul core"),                      // items.xml:78746
    (47472, "yeti soul core"),                       // items.xml:78751
    (47473, "chicken soul core"),                    // items.xml:78756
    (47474, "crab soul core"),                       // items.xml:78761
    (47475, "lizard templar soul core"),             // items.xml:78766
    (47476, "lizard sentinel soul core"),            // items.xml:78771
    (47477, "lizard snakecharmer soul core"),        // items.xml:78776
    (47478, "kongra soul core"),                     // items.xml:78781
    (47479, "merlkin soul core"),                    // items.xml:78786
    (47480, "sibang soul core"),                     // items.xml:78791
    (47481, "crocodile soul core"),                  // items.xml:78796
    (47482, "carniphila soul core"),                 // items.xml:78801
    (47483, "hydra soul core"),                      // items.xml:78806
    (47484, "bat soul core"),                        // items.xml:78811
    (47485, "panda soul core"),                      // items.xml:78816
    (47486, "centipede soul core"),                  // items.xml:78821
    (47487, "tiger soul core"),                      // items.xml:78826
    (47488, "elephant soul core"),                   // items.xml:78831
    (47489, "flamingo soul core"),                   // items.xml:78836
    (47490, "butterfly soul core (purple)"),         // items.xml:78841
    (47491, "dworc voodoomaster soul core"),         // items.xml:78846
    (47492, "dworc fleshhunter soul core"),          // items.xml:78851
    (47493, "dworc venomsniper soul core"),          // items.xml:78856
    (47494, "parrot soul core"),                     // items.xml:78861
    (47495, "terror bird soul core"),                // items.xml:78866
    (47496, "tarantula soul core"),                  // items.xml:78871
    (47497, "serpent spawn soul core"),              // items.xml:78876
    (47498, "spit nettle soul core"),                // items.xml:78881
    (47499, "smuggler soul core"),                   // items.xml:78886
    (47500, "bandit soul core"),                     // items.xml:78891
    (47501, "assassin soul core"),                   // items.xml:78896
    (47502, "dark monk soul core"),                  // items.xml:78901
    (47503, "butterfly soul core (blue)"),           // items.xml:78906
    (47504, "butterfly soul core (red)"),            // items.xml:78911
    (47505, "water elemental soul core"),            // items.xml:78916
    (47506, "quara predator soul core"),             // items.xml:78921
    (47507, "quara predator scout soul core"),       // items.xml:78926
    (47508, "quara constrictor soul core"),          // items.xml:78931
    (47509, "quara constrictor scout soul core"),    // items.xml:78936
    (47510, "quara mantassin soul core"),            // items.xml:78941
    (47511, "quara mantassin scout soul core"),      // items.xml:78946
    (47512, "quara hydromancer soul core"),          // items.xml:78951
    (47513, "quara hydromancer scout soul core"),    // items.xml:78956
    (47514, "quara pincher soul core"),              // items.xml:78961
    (47515, "quara pincher scout soul core"),        // items.xml:78966
    (47516, "pirate marauder soul core"),            // items.xml:78971
    (47517, "pirate cutthroat soul core"),           // items.xml:78976
    (47518, "pirate buccaneer soul core"),           // items.xml:78981
    (47519, "pirate corsair soul core"),             // items.xml:78986
    (47520, "carrion worm soul core"),               // items.xml:78991
    (47521, "enlightened of the cult soul core"),    // items.xml:78996
    (47522, "acolyte of the cult soul core"),        // items.xml:79001
    (47523, "adept of the cult soul core"),          // items.xml:79006
    (47524, "novice of the cult soul core"),         // items.xml:79011
    (47525, "pirate skeleton soul core"),            // items.xml:79016
    (47526, "pirate ghost soul core"),               // items.xml:79021
    (47527, "tortoise soul core"),                   // items.xml:79026
    (47528, "thornback tortoise soul core"),         // items.xml:79031
    (47529, "mammoth soul core"),                    // items.xml:79036
    (47530, "blood crab soul core"),                 // items.xml:79041
    (47531, "toad soul core"),                       // items.xml:79046
    (47532, "seagull soul core"),                    // items.xml:79051
    (47533, "son of verminor soul core"),            // items.xml:79056
    (47534, "green frog soul core"),                 // items.xml:79061
    (47535, "azure frog soul core"),                 // items.xml:79066
    (47536, "coral frog soul core"),                 // items.xml:79071
    (47537, "crimson frog soul core"),               // items.xml:79076
    (47538, "orchid frog soul core"),                // items.xml:79081
    (47539, "island troll soul core"),               // items.xml:79086
    (47540, "massive water elemental soul core"),    // items.xml:79091
    (47541, "hand of cursed fate soul core"),        // items.xml:79096
    (47542, "undead dragon soul core"),              // items.xml:79101
    (47543, "lost soul soul core"),                  // items.xml:79106
    (47544, "betrayed wraith soul core"),            // items.xml:79111
    (47545, "dark torturer soul core"),              // items.xml:79116
    (47546, "spectre soul core"),                    // items.xml:79121
    (47547, "destroyer soul core"),                  // items.xml:79126
    (47548, "diabolic imp soul core"),               // items.xml:79131
    (47549, "defiler soul core"),                    // items.xml:79136
    (47550, "wyvern soul core"),                     // items.xml:79141
    (47551, "fury soul core"),                       // items.xml:79146
    (47552, "phantasm soul core"),                   // items.xml:79151
    (47553, "hellhound soul core"),                  // items.xml:79156
    (47554, "hellfire fighter soul core"),           // items.xml:79161
    (47555, "juggernaut soul core"),                 // items.xml:79166
    (47556, "blightwalker soul core"),               // items.xml:79171
    (47557, "nightmare soul core"),                  // items.xml:79176
    (47558, "nomad soul core (basic)"),              // items.xml:79181
    (47559, "massive fire elemental soul core"),     // items.xml:79186
    (47560, "plaguesmith soul core"),                // items.xml:79191
    (47561, "frost dragon soul core"),               // items.xml:79196
    (47562, "penguin soul core"),                    // items.xml:79201
    (47563, "chakoya tribewarden soul core"),        // items.xml:79206
    (47564, "braindeath soul core"),                 // items.xml:79211
    (47565, "barbarian skullhunter soul core"),      // items.xml:79216
    (47566, "barbarian bloodwalker soul core"),      // items.xml:79221
    (47567, "frost giant soul core"),                // items.xml:79226
    (47568, "ice golem soul core"),                  // items.xml:79231
    (47569, "silver rabbit soul core"),              // items.xml:79236
    (47570, "chakoya toolshaper soul core"),         // items.xml:79241
    (47571, "chakoya windcaller soul core"),         // items.xml:79246
    (47572, "crystal spider soul core"),             // items.xml:79251
    (47573, "ice witch soul core"),                  // items.xml:79256
    (47574, "barbarian brutetamer soul core"),       // items.xml:79261
    (47575, "barbarian headsplitter soul core"),     // items.xml:79266
    (47576, "frost giantess soul core"),             // items.xml:79271
    (47577, "dire penguin soul core"),               // items.xml:79276
    (47578, "dark magician soul core"),              // items.xml:79281
    (47579, "dark apprentice soul core"),            // items.xml:79286
    (47580, "poacher soul core"),                    // items.xml:79291
    (47581, "goblin leader soul core"),              // items.xml:79296
    (47582, "dwarf henchman soul core"),             // items.xml:79301
    (47583, "dryad soul core"),                      // items.xml:79306
    (47584, "squirrel soul core"),                   // items.xml:79311
    (47585, "dragon hatchling soul core"),           // items.xml:79316
    (47586, "dragon lord hatchling soul core"),      // items.xml:79321
    (47587, "cat soul core"),                        // items.xml:79326
    (47588, "undead jester soul core"),              // items.xml:79331
    (47589, "cyclops smith soul core"),              // items.xml:79336
    (47590, "cyclops drone soul core"),              // items.xml:79341
    (47591, "troll champion soul core"),             // items.xml:79346
    (47592, "grynch clan goblin soul core"),         // items.xml:79351
    (47593, "frost dragon hatchling soul core"),     // items.xml:79356
    (47594, "deepsea blood crab soul core"),         // items.xml:79361
    (47595, "sea serpent soul core"),                // items.xml:79366
    (47596, "young sea serpent soul core"),          // items.xml:79371
    (47598, "skeleton warrior soul core"),           // items.xml:79376
    (47600, "massive earth elemental soul core"),    // items.xml:79381
    (47601, "energy elemental soul core"),           // items.xml:79386
    (47602, "earth elemental soul core"),            // items.xml:79391
    (47603, "bog raider soul core"),                 // items.xml:79396
    (47604, "wyrm soul core"),                       // items.xml:79401
    (47605, "wisp soul core"),                       // items.xml:79406
    (47606, "goblin assassin soul core"),            // items.xml:79411
    (47607, "goblin scavenger soul core"),           // items.xml:79416
    (47608, "grim reaper soul core"),                // items.xml:79421
    (47613, "vampire bride soul core"),              // items.xml:79426
    (47619, "mutated rat soul core"),                // items.xml:79431
    (47620, "worker golem soul core"),               // items.xml:79436
    (47621, "undead guardian soul core"),            // items.xml:79441
    (47622, "mutated bat soul core"),                // items.xml:79446
    (47623, "werewolf soul core"),                   // items.xml:79451
    (47624, "haunted treeling soul core"),           // items.xml:79456
    (47625, "zombie soul core"),                     // items.xml:79461
    (47626, "acid blob soul core"),                  // items.xml:79466
    (47627, "death blob soul core"),                 // items.xml:79471
    (47628, "mercury blob soul core"),               // items.xml:79476
    (47629, "mutated tiger soul core"),              // items.xml:79481
    (47631, "nightmare scion soul core"),            // items.xml:79486
    (47632, "hellspawn soul core"),                  // items.xml:79491
    (47633, "nightstalker soul core"),               // items.xml:79496
    (47634, "mutated human soul core"),              // items.xml:79501
    (47635, "gozzler soul core"),                    // items.xml:79506
    (47636, "damaged worker golem soul core"),       // items.xml:79511
    (47637, "crazed beggar soul core"),              // items.xml:79516
    (47638, "gang member soul core"),                // items.xml:79521
    (47639, "gladiator soul core"),                  // items.xml:79526
    (47640, "mad scientist soul core"),              // items.xml:79531
    (47641, "infernalist soul core"),                // items.xml:79536
    (47642, "war golem soul core"),                  // items.xml:79541
    (47643, "furious troll soul core"),              // items.xml:79546
    (47644, "troll legionnaire soul core"),          // items.xml:79551
    (47645, "evil sheep soul core"),                 // items.xml:79556
    (47646, "medusa soul core"),                     // items.xml:79561
    (47653, "acolyte of darkness soul core"),        // items.xml:79566
    (47654, "nightslayer soul core"),                // items.xml:79571
    (47655, "bane of light soul core"),              // items.xml:79576
    (47656, "duskbringer soul core"),                // items.xml:79581
    (47657, "shadow hound soul core"),               // items.xml:79586
    (47658, "doomsday cultist soul core"),           // items.xml:79591
    (47659, "midnight spawn soul core"),             // items.xml:79596
    (47660, "midnight warrior soul core"),           // items.xml:79601
    (47661, "herald of gloom soul core"),            // items.xml:79606
    (47662, "bride of night soul core"),             // items.xml:79611
    (47667, "orc marauder soul core"),               // items.xml:79616
    (47668, "eternal guardian soul core"),           // items.xml:79621
    (47669, "lizard zaogun soul core"),              // items.xml:79626
    (47670, "draken warmaster soul core"),           // items.xml:79631
    (47671, "draken spellweaver soul core"),         // items.xml:79636
    (47672, "lizard chosen soul core"),              // items.xml:79641
    (47673, "insect swarm soul core"),               // items.xml:79646
    (47674, "lizard dragon priest soul core"),       // items.xml:79651
    (47675, "lizard legionnaire soul core"),         // items.xml:79656
    (47676, "lizard high guard soul core"),          // items.xml:79661
    (47677, "killer caiman soul core"),              // items.xml:79666
    (47678, "gnarlhound soul core"),                 // items.xml:79671
    (47679, "terramite soul core"),                  // items.xml:79676
    (47680, "wailing widow soul core"),              // items.xml:79681
    (47681, "lancer beetle soul core"),              // items.xml:79686
    (47682, "sandcrawler soul core"),                // items.xml:79691
    (47683, "ghastly dragon soul core"),             // items.xml:79696
    (47684, "lizard magistratus soul core"),         // items.xml:79701
    (47685, "lizard noble soul core"),               // items.xml:79706
    (47686, "draken elite soul core"),               // items.xml:79711
    (47687, "draken abomination soul core"),         // items.xml:79716
    (47688, "brimstone bug soul core"),              // items.xml:79721
    (47689, "souleater soul core"),                  // items.xml:79726
    (47690, "bane bringer soul core"),               // items.xml:79731
    (47691, "cake golem soul core"),                 // items.xml:79736
    (47697, "berrypest soul core"),                  // items.xml:79741
    (47698, "boar soul core"),                       // items.xml:79746
    (47699, "stampor soul core"),                    // items.xml:79751
    (47700, "draptor soul core"),                    // items.xml:79756
    (47701, "undead cavebear soul core"),            // items.xml:79761
    (47702, "crustacea gigantica soul core"),        // items.xml:79766
    (47703, "midnight panther soul core"),           // items.xml:79771
    (47704, "iron servant soul core"),               // items.xml:79776
    (47705, "golden servant soul core"),             // items.xml:79781
    (47706, "diamond servant soul core"),            // items.xml:79786
    (47707, "clay guardian soul core"),              // items.xml:79791
    (47708, "yielothax soul core"),                  // items.xml:79796
    (47709, "shaburak demon soul core"),             // items.xml:79801
    (47710, "askarak demon soul core"),              // items.xml:79806
    (47711, "wild horse soul core"),                 // items.xml:79811
    (47712, "slug soul core"),                       // items.xml:79816
    (47713, "dromedary soul core"),                  // items.xml:79821
    (47714, "deepling scout soul core"),             // items.xml:79826
    (47715, "bog frog soul core"),                   // items.xml:79831
    (47716, "thornfire wolf soul core"),             // items.xml:79836
    (47717, "crystal wolf soul core"),               // items.xml:79841
    (47718, "elf overseer soul core"),               // items.xml:79846
    (47719, "troll guard soul core"),                // items.xml:79851
    (47722, "deepling warrior soul core"),           // items.xml:79856
    (47723, "deepling guard soul core"),             // items.xml:79861
    (47724, "deepling spellsinger soul core"),       // items.xml:79866
    (47725, "shark soul core"),                      // items.xml:79871
    (47726, "fish soul core"),                       // items.xml:79876
    (47727, "crawler soul core"),                    // items.xml:79881
    (47729, "swarmer soul core"),                    // items.xml:79886
    (47730, "spitter soul core"),                    // items.xml:79891
    (47731, "waspoid soul core"),                    // items.xml:79896
    (47732, "deepling worker soul core"),            // items.xml:79901
    (47733, "insectoid worker soul core"),           // items.xml:79906
    (47734, "crystalcrusher soul core"),             // items.xml:79911
    (47735, "mushroom sniffer soul core"),           // items.xml:79916
    (47737, "water buffalo soul core"),              // items.xml:79921
    (47738, "enraged crystal golem soul core"),      // items.xml:79926
    (47739, "drillworm soul core"),                  // items.xml:79931
    (47740, "humongous fungus soul core"),           // items.xml:79936
    (47741, "magma crawler soul core"),              // items.xml:79941
    (47742, "enslaved dwarf soul core"),             // items.xml:79946
    (47743, "lost berserker soul core"),             // items.xml:79951
    (47744, "hideous fungus soul core"),             // items.xml:79956
    (47745, "vulcongra soul core"),                  // items.xml:79961
    (47746, "wiggler soul core"),                    // items.xml:79966
    (47747, "emerald damselfly soul core"),          // items.xml:79971
    (47748, "salamander soul core"),                 // items.xml:79976
    (47749, "marsh stalker soul core"),              // items.xml:79981
    (47750, "corym charlatan soul core"),            // items.xml:79986
    (47751, "corym skirmisher soul core"),           // items.xml:79991
    (47752, "corym vanguard soul core"),             // items.xml:79996
    (47753, "swampling soul core"),                  // items.xml:80001
    (47754, "adventurer soul core"),                 // items.xml:80006
    (47755, "lost husher soul core"),                // items.xml:80011
    (47756, "lost basher soul core"),                // items.xml:80016
    (47757, "lost thrower soul core"),               // items.xml:80021
    (47758, "vampire count soul core"),              // items.xml:80026
    (47759, "vicious manbat soul core"),             // items.xml:80031
    (47760, "shadow pupil soul core"),               // items.xml:80036
    (47761, "blood priest soul core"),               // items.xml:80041
    (47762, "white shade soul core"),                // items.xml:80046
    (47763, "elder wyrm soul core"),                 // items.xml:80051
    (47764, "nightfiend soul core"),                 // items.xml:80056
    (47765, "blood hand soul core"),                 // items.xml:80061
    (47766, "gravedigger soul core"),                // items.xml:80066
    (47767, "tarnished spirit soul core"),           // items.xml:80071
    (47768, "rorc soul core"),                       // items.xml:80076
    (47769, "leaf golem soul core"),                 // items.xml:80081
    (47770, "forest fury soul core"),                // items.xml:80086
    (47771, "roaring lion soul core"),               // items.xml:80091
    (47772, "wilting leaf golem soul core"),         // items.xml:80096
    (47773, "furious fire elemental soul core"),     // items.xml:80101
    (47774, "shock head soul core"),                 // items.xml:80106
    (47775, "sight of surrender soul core"),         // items.xml:80111
    (47776, "guzzlemaw soul core"),                  // items.xml:80116
    (47777, "silencer soul core"),                   // items.xml:80121
    (47778, "choking fear soul core"),               // items.xml:80126
    (47779, "feversleep soul core"),                 // items.xml:80131
    (47780, "retching horror soul core"),            // items.xml:80136
    (47781, "demon outcast soul core"),              // items.xml:80141
    (47783, "terrorsleep soul core"),                // items.xml:80146
    (47784, "frazzlemaw soul core"),                 // items.xml:80151
    (47786, "glooth golem soul core"),               // items.xml:80156
    (47787, "metal gargoyle soul core"),             // items.xml:80161
    (47788, "blood beast soul core"),                // items.xml:80166
    (47789, "rustheap golem soul core"),             // items.xml:80171
    (47790, "glooth anemone soul core"),             // items.xml:80176
    (47791, "moohtant soul core"),                   // items.xml:80181
    (47792, "minotaur amazon soul core"),            // items.xml:80186
    (47793, "execowtioner soul core"),               // items.xml:80191
    (47794, "mooh'tah warrior soul core"),           // items.xml:80196
    (47795, "minotaur hunter soul core"),            // items.xml:80201
    (47796, "worm priestess soul core"),             // items.xml:80206
    (47797, "glooth blob soul core"),                // items.xml:80211
    (47798, "rot elemental soul core"),              // items.xml:80216
    (47799, "devourer soul core"),                   // items.xml:80221
    (47810, "abyssal calamary soul core"),           // items.xml:80226
    (47812, "high voltage elemental soul core"),     // items.xml:80231
    (47813, "glooth bandit soul core"),              // items.xml:80236
    (47814, "glooth brigand soul core"),             // items.xml:80241
    (47815, "raging fire soul core"),                // items.xml:80246
    (47816, "dawnfire asura soul core"),             // items.xml:80251
    (47817, "midnight asura soul core"),             // items.xml:80256
    (47818, "tainted soul soul core"),               // items.xml:80261
    (47819, "omnivora soul core"),                   // items.xml:80266
    (47820, "werebear soul core"),                   // items.xml:80271
    (47821, "wereboar soul core"),                   // items.xml:80276
    (47822, "werebadger soul core"),                 // items.xml:80281
    (47823, "vicious squire soul core"),             // items.xml:80286
    (47824, "renegade knight soul core"),            // items.xml:80291
    (47825, "vile grandmaster soul core"),           // items.xml:80296
    (47826, "ogre brute soul core"),                 // items.xml:80301
    (47827, "ogre savage soul core"),                // items.xml:80306
    (47828, "ogre shaman soul core"),                // items.xml:80311
    (47829, "clomp soul core"),                      // items.xml:80316
    (47830, "grimeleech soul core"),                 // items.xml:80321
    (47831, "vexclaw soul core"),                    // items.xml:80326
    (47832, "hellflayer soul core"),                 // items.xml:80331
    (47838, "reality reaver soul core"),             // items.xml:80336
    (47839, "sparkion soul core"),                   // items.xml:80341
    (47840, "breach brood soul core"),               // items.xml:80346
    (47842, "dread intruder soul core"),             // items.xml:80351
    (47844, "instable sparkion soul core"),          // items.xml:80356
    (47845, "instable breach brood soul core"),      // items.xml:80361
    (47846, "stabilizing reality reaver soul core"), // items.xml:80366
    (47847, "stabilizing dread intruder soul core"), // items.xml:80371
    (47848, "cave parrot soul core"),                // items.xml:80376
    (47849, "orclops doomhauler soul core"),         // items.xml:80381
    (47850, "orclops ravager soul core"),            // items.xml:80386
    (47851, "broken shaper soul core"),              // items.xml:80391
    (47852, "twisted shaper soul core"),             // items.xml:80396
    (47853, "iron servant replica soul core"),       // items.xml:80401
    (47854, "shaper matriarch soul core"),           // items.xml:80406
    (47855, "misguided bully soul core"),            // items.xml:80411
    (47856, "misguided thief soul core"),            // items.xml:80416
    (47857, "putrid mummy soul core"),               // items.xml:80421
    (47858, "faun soul core"),                       // items.xml:80426
    (47859, "pooka soul core"),                      // items.xml:80431
    (47860, "twisted pooka soul core"),              // items.xml:80436
    (47861, "pixie soul core"),                      // items.xml:80441
    (47862, "boogy soul core"),                      // items.xml:80446
    (47863, "weakened frazzlemaw soul core"),        // items.xml:80451
    (47864, "enfeebled silencer soul core"),         // items.xml:80456
    (47865, "nymph soul core"),                      // items.xml:80461
    (47866, "barkless devotee soul core"),           // items.xml:80466
    (47867, "barkless fanatic soul core"),           // items.xml:80471
    (47868, "dark faun soul core"),                  // items.xml:80476
    (47869, "orc cult inquisitor soul core"),        // items.xml:80481
    (47870, "orc cult minion soul core"),            // items.xml:80486
    (47871, "minotaur cult follower soul core"),     // items.xml:80491
    (47872, "minotaur cult prophet soul core"),      // items.xml:80496
    (47873, "minotaur cult zealot soul core"),       // items.xml:80501
    (47874, "cult believer soul core"),              // items.xml:80506
    (47875, "cult enforcer soul core"),              // items.xml:80511
    (47876, "stonerefiner soul core"),               // items.xml:80516
    (47877, "lost exile soul core"),                 // items.xml:80521
    (47878, "deepworm soul core"),                   // items.xml:80526
    (47879, "diremaw soul core"),                    // items.xml:80531
    (47880, "cave devourer soul core"),              // items.xml:80536
    (47881, "tunnel tyrant soul core"),              // items.xml:80541
    (47882, "chasm spawn soul core"),                // items.xml:80546
    (47883, "fox soul core"),                        // items.xml:80551
    (47884, "werefox soul core"),                    // items.xml:80556
    (47886, "lava lurker soul core"),                // items.xml:80561
    (47888, "ravenous lava lurker soul core"),       // items.xml:80566
    (47890, "frost flower asura soul core"),         // items.xml:80571
    (47891, "true dawnfire asura soul core"),        // items.xml:80576
    (47892, "true midnight asura soul core"),        // items.xml:80581
    (47893, "true frost flower asura soul core"),    // items.xml:80586
    (47896, "floating savant soul core"),            // items.xml:80591
    (47899, "falcon knight soul core"),              // runtime appearance/XML Item 47899
    (47900, "falcon paladin soul core"),             // items.xml:80596
    (47901, "brain squid soul core"),                // items.xml:80601
    (47902, "biting book soul core"),                // items.xml:80606
    (47903, "ink blob soul core"),                   // items.xml:80611
    (47904, "burning book soul core"),               // items.xml:80616
    (47905, "icecold book soul core"),               // items.xml:80621
    (47906, "energetic book soul core"),             // items.xml:80626
    (47907, "energuardian of tales soul core"),      // items.xml:80631
    (47908, "deathling scout soul core"),            // items.xml:80636
    (47909, "rage squid soul core"),                 // items.xml:80641
    (47910, "squid warden soul core"),               // items.xml:80646
    (47911, "animated feather soul core"),           // items.xml:80651
    (47913, "skeleton elite warrior soul core"),     // items.xml:80656
    (47914, "undead elite gladiator soul core"),     // items.xml:80661
    (47915, "deathling spellsinger soul core"),      // items.xml:80666
    (47916, "lumbering carnivor soul core"),         // items.xml:80671
    (47917, "spiky carnivor soul core"),             // items.xml:80676
    (47918, "menacing carnivor soul core"),          // items.xml:80681
    (47919, "ripper spectre soul core"),             // items.xml:80686
    (47920, "gazer spectre soul core"),              // items.xml:80691
    (47921, "burster spectre soul core"),            // items.xml:80696
    (47922, "thanatursus soul core"),                // items.xml:80701
    (47923, "arachnophobica soul core"),             // items.xml:80706
    (47924, "crazed winter vanguard soul core"),     // items.xml:80711
    (47925, "crazed winter rearguard soul core"),    // items.xml:80716
    (47926, "crazed summer vanguard soul core"),     // items.xml:80721
    (47927, "crazed summer rearguard soul core"),    // items.xml:80726
    (47928, "soul-broken harbinger soul core"),      // items.xml:80731
    (47929, "insane siren soul core"),               // items.xml:80736
    (47930, "cobra assassin soul core"),             // items.xml:80741
    (47931, "cobra scout soul core"),                // items.xml:80746
    (47933, "burning gladiator soul core"),          // items.xml:80751
    (47934, "priestess of the wild sun soul core"),  // items.xml:80756
    (47935, "black sphinx acolyte soul core"),       // items.xml:80761
    (47936, "crypt warden soul core"),               // items.xml:80766
    (47937, "lamassu soul core"),                    // items.xml:80771
    (47938, "feral sphinx soul core"),               // items.xml:80776
    (47939, "sphinx soul core"),                     // items.xml:80781
    (47940, "manticore soul core"),                  // items.xml:80786
    (47941, "young goanna soul core"),               // items.xml:80791
    (47942, "adult goanna soul core"),               // items.xml:80796
    (47943, "ogre ruffian soul core"),               // items.xml:80801
    (47944, "ogre rowdy soul core"),                 // items.xml:80806
    (47945, "ogre sage soul core"),                  // items.xml:80811
    (47946, "cobra vizier soul core"),               // items.xml:80816
    (47947, "flimsy lost soul soul core"),           // items.xml:80821
    (47948, "mean lost soul soul core"),             // items.xml:80826
    (47949, "freakish lost soul soul core"),         // items.xml:80831
    (47950, "cursed prospector soul core"),          // items.xml:80836
    (47951, "evil prospector soul core"),            // items.xml:80841
    (47952, "bony sea devil soul core"),             // items.xml:80846
    (47953, "many faces soul core"),                 // items.xml:80851
    (47954, "cloak of terror soul core"),            // items.xml:80856
    (47955, "vibrant phantom soul core"),            // items.xml:80861
    (47956, "brachiodemon soul core"),               // items.xml:80866
    (47957, "branchy crawler soul core"),            // items.xml:80871
    (47958, "capricious phantom soul core"),         // items.xml:80876
    (47959, "infernal phantom soul core"),           // items.xml:80881
    (47961, "infernal demon soul core"),             // items.xml:80886
    (47962, "rotten golem soul core"),               // items.xml:80891
    (47963, "turbulent elemental soul core"),        // items.xml:80896
    (47964, "courage leech soul core"),              // items.xml:80901
    (47965, "mould phantom soul core"),              // items.xml:80906
    (47966, "druid's apparition soul core"),         // items.xml:80911
    (47967, "knight's apparition soul core"),        // items.xml:80916
    (47968, "paladin's apparition soul core"),       // items.xml:80921
    (47969, "sorcerer's apparition soul core"),      // items.xml:80926
    (47973, "distorted phantom soul core"),          // items.xml:80931
    (47974, "werehyaena soul core"),                 // items.xml:80936
    (47975, "werehyaena shaman soul core"),          // items.xml:80941
    (47976, "werelion shaman soul core"),            // items.xml:80946
    (47977, "werelioness shaman soul core"),         // items.xml:80951
    (47978, "lion knight soul core"),                // items.xml:80956
    (47979, "lion paladin soul core"),               // items.xml:80961
    (47980, "usurper warlock soul core"),            // items.xml:80966
    (47981, "agrestic chicken soul core"),           // items.xml:80971
    (47984, "crypt warrior soul core"),              // items.xml:80976
    (47985, "exotic cave spider soul core"),         // items.xml:80981
    (47986, "pirat cutthroat soul core"),            // items.xml:80986
    (47987, "pirat scoundrel soul core"),            // items.xml:80991
    (47988, "pirat bombardier soul core"),           // items.xml:80996
    (47989, "pirat mate soul core"),                 // items.xml:81001
    (47990, "exotic bat soul core"),                 // items.xml:81006
    (47991, "lavaworm soul core"),                   // items.xml:81011
    (47992, "tremendous tyrant soul core"),          // items.xml:81016
    (47993, "varnished diremaw soul core"),          // items.xml:81021
    (47994, "streaked devourer soul core"),          // items.xml:81026
    (47995, "eyeless devourer soul core"),           // items.xml:81031
    (47996, "blemished spawn soul core"),            // items.xml:81036
    (47997, "afflicted strider soul core"),          // items.xml:81041
    (47998, "lavafungus soul core"),                 // items.xml:81046
    (47999, "cave chimera soul core"),               // items.xml:81051
    (48000, "venerable girtablilu soul core"),       // items.xml:81056
    (48001, "girtablilu warrior soul core"),         // items.xml:81061
    (48002, "bashmu soul core"),                     // items.xml:81066
    (48003, "juvenile bashmu soul core"),            // items.xml:81071
    (48004, "hulking carnisylvan soul core"),        // items.xml:81076
    (48005, "poisonous carnisylvan soul core"),      // items.xml:81081
    (48006, "dark carnisylvan soul core"),           // items.xml:81086
    (48032, "parder soul core"),                     // items.xml:81091
    (48033, "jungle moa soul core"),                 // items.xml:81096
    (48034, "two-head turtle soul core"),            // items.xml:81101
    (48035, "foam stalker soul core"),               // items.xml:81106
    (48036, "naga archer soul core"),                // items.xml:81111
    (48037, "naga warrior soul core"),               // items.xml:81116
    (48038, "makara soul core"),                     // items.xml:81121
    (48039, "sulphider soul core"),                  // items.xml:81126
    (48040, "sulphur spouter soul core"),            // items.xml:81131
    (48041, "gore horn soul core"),                  // items.xml:81136
    (48042, "sabretooth soul core"),                 // items.xml:81141
    (48043, "emerald tortoise soul core"),           // items.xml:81146
    (48044, "undertaker soul core"),                 // items.xml:81151
    (48045, "nighthunter soul core"),                // items.xml:81156
    (48046, "hulking prehemoth soul core"),          // items.xml:81161
    (48047, "stalking stalk soul core"),             // items.xml:81166
    (48048, "mantosaurus soul core"),                // items.xml:81171
    (48049, "headpecker soul core"),                 // items.xml:81176
    (48050, "noxious ripptor soul core"),            // items.xml:81181
    (48051, "gorerilla soul core"),                  // items.xml:81186
    (48052, "shrieking cry-stal soul core"),         // items.xml:81191
    (48053, "mercurial menace soul core"),           // items.xml:81196
    (48054, "crape man soul core"),                  // items.xml:81201
    (48055, "liodile soul core"),                    // items.xml:81206
    (48056, "boar man soul core"),                   // items.xml:81211
    (48057, "harpy soul core"),                      // items.xml:81216
    (48058, "carnivostrich soul core"),              // items.xml:81221
    (48059, "rhindeer soul core"),                   // items.xml:81226
    (48060, "iks pututu soul core"),                 // items.xml:81231
    (48061, "iks aucar soul core"),                  // items.xml:81236
    (48062, "iks chuka soul core"),                  // items.xml:81241
    (48063, "cursed ape soul core"),                 // items.xml:81246
    (48064, "iks ahpututu soul core"),               // items.xml:81251
    (48065, "mycobiontic beetle soul core"),         // items.xml:81256
    (48066, "meandering mushroom soul core"),        // items.xml:81261
    (48067, "oozing carcass soul core"),             // items.xml:81266
    (48068, "darklight construct soul core"),        // items.xml:81271
    (48069, "converter soul core"),                  // items.xml:81276
    (48070, "darklight matter soul core"),           // items.xml:81281
    (48071, "oozing corpus soul core"),              // items.xml:81286
    (48072, "darklight emitter soul core"),          // items.xml:81291
    (48073, "weretiger soul core"),                  // items.xml:81296
    (48074, "white weretiger soul core"),            // items.xml:81301
    (48075, "werecrocodile soul core"),              // items.xml:81306
    (48076, "feral werecrocodile soul core"),        // items.xml:81311
    (48077, "werepanther soul core"),                // items.xml:81316
    (48078, "white tiger soul core"),                // items.xml:81321
    (48079, "bloated man-maggot soul core"),         // items.xml:81326
    (48080, "rotten man-maggot soul core"),          // items.xml:81331
    (48081, "walking pillar soul core"),             // items.xml:81336
    (48082, "wandering pillar soul core"),           // items.xml:81341
    (48083, "sopping carcass soul core"),            // items.xml:81346
    (48084, "sopping corpus soul core"),             // items.xml:81351
    (48085, "darklight source soul core"),           // items.xml:81356
    (48086, "darklight striker soul core"),          // items.xml:81361
    (48087, "cunning werepanther soul core"),        // items.xml:81366
    (48089, "albino dragon soul core"),              // items.xml:81371
    (48091, "iks yapunac soul core"),                // items.xml:81376
    (48092, "bulltaur brute soul core"),             // items.xml:81381
    (48093, "bulltaur alchemist soul core"),         // items.xml:81386
    (48094, "bulltaur forgepriest soul core"),       // items.xml:81391
    (48095, "dragolisk soul core"),                  // items.xml:81396
    (48096, "wardragon soul core"),                  // items.xml:81401
    (48097, "mega dragon soul core"),                // items.xml:81406
    (48098, "mitmah scout soul core"),               // items.xml:81411
    (48099, "mitmah seer soul core"),                // items.xml:81416
    (48929, "black sheep soul core"),                // items.xml:81913
    (48930, "squidgy slime soul core"),              // items.xml:81918
    (48931, "husky soul core"),                      // items.xml:81923
    (48932, "massive energy elemental soul core"),   // items.xml:81928
    (48933, "evil sheep lord soul core"),            // items.xml:81933
    (48934, "hot dog soul core"),                    // items.xml:81938
    (48935, "vampire pig soul core"),                // items.xml:81943
    (48936, "doom deer soul core"),                  // items.xml:81948
    (48937, "killer rabbit soul core"),              // items.xml:81953
    (48938, "berserker chicken soul core"),          // items.xml:81958
    (48939, "demon parrot soul core"),               // items.xml:81963
    (48940, "infernal frog soul core"),              // items.xml:81968
    (48941, "undead mine worker soul core"),         // items.xml:81973
    (48942, "undead prospector soul core"),          // items.xml:81978
    (48943, "ghoulish hyaena soul core"),            // items.xml:81983
    (48944, "sandstone scorpion soul core"),         // items.xml:81988
    (48945, "grave guard soul core"),                // items.xml:81993
    (48946, "tomb servant soul core"),               // items.xml:81998
    (48947, "sacred spider soul core"),              // items.xml:82003
    (48948, "death priest soul core"),               // items.xml:82008
    (48949, "elder mummy soul core"),                // items.xml:82013
    (48950, "honour guard soul core"),               // items.xml:82018
    (48951, "feverish citizen soul core"),           // items.xml:82023
    (48952, "white deer soul core"),                 // items.xml:82028
    (48953, "starving wolf soul core"),              // items.xml:82033
    (48954, "shaburak lord soul core"),              // items.xml:82038
    (48955, "shaburak prince soul core"),            // items.xml:82043
    (48956, "askarak lord soul core"),               // items.xml:82048
    (48957, "askarak prince soul core"),             // items.xml:82053
    (48958, "insectoid scout soul core"),            // items.xml:82058
    (48959, "filth toad soul core"),                 // items.xml:82063
    (48960, "firestarter soul core"),                // items.xml:82068
    (48961, "horse soul core (taupe)"),              // items.xml:82073
    (48962, "horse soul core (brown)"),              // items.xml:82078
    (48963, "horse soul core (gray)"),               // items.xml:82083
    (48964, "terrified elephant soul core"),         // items.xml:82088
    (48965, "nomad soul core (blue)"),               // items.xml:82093
    (48966, "nomad soul core (female)"),             // items.xml:82098
    (48967, "ladybug soul core"),                    // items.xml:82103
    (48968, "manta ray soul core"),                  // items.xml:82108
    (48969, "calamary soul core"),                   // items.xml:82113
    (48970, "jellyfish soul core"),                  // items.xml:82118
    (48971, "northern pike soul core"),              // items.xml:82123
    (48972, "spidris soul core"),                    // items.xml:82128
    (48973, "kollos soul core"),                     // items.xml:82133
    (48974, "spidris elite soul core"),              // items.xml:82138
    (48975, "hive overseer soul core"),              // items.xml:82143
    (48976, "deepling brawler soul core"),           // items.xml:82148
    (48977, "deepling master librarian soul core"),  // items.xml:82153
    (48978, "deepling tyrant soul core"),            // items.xml:82158
    (48979, "deepling elite soul core"),             // items.xml:82163
    (48980, "grave robber soul core"),               // items.xml:82168
    (48981, "crypt defiler soul core"),              // items.xml:82173
    (48982, "damaged crystal golem soul core"),      // items.xml:82178
    (48983, "modified gnarlhound soul core"),        // items.xml:82183
    (48984, "stone devourer soul core"),             // items.xml:82188
    (48985, "armadile soul core"),                   // items.xml:82193
    (48986, "weeper soul core"),                     // items.xml:82198
    (48987, "orewalker soul core"),                  // items.xml:82203
    (48988, "lava golem soul core"),                 // items.xml:82208
    (48989, "cliff strider soul core"),              // items.xml:82213
    (48990, "ironblight soul core"),                 // items.xml:82218
    (48991, "dragonling soul core"),                 // items.xml:82223
    (48992, "infected weeper soul core"),            // items.xml:82228
    (48993, "pigeon soul core"),                     // items.xml:82233
    (48994, "little corym charlatan soul core"),     // items.xml:82238
    (48995, "walker soul core"),                     // items.xml:82243
    (48996, "seacrest serpent soul core"),           // items.xml:82248
    (48997, "renegade quara constrictor soul core"), // items.xml:82253
    (48998, "renegade quara hydromancer soul core"), // items.xml:82258
    (48999, "renegade quara mantassin soul core"),   // items.xml:82263
    (49000, "renegade quara pincher soul core"),     // items.xml:82268
    (49001, "renegade quara predator soul core"),    // items.xml:82273
    (49002, "minotaur invader soul core"),           // items.xml:82278
    (49003, "noble lion soul core"),                 // items.xml:82283
    (49004, "redeemed soul soul core"),              // items.xml:82288
    (49005, "gloom wolf soul core"),                 // items.xml:82293
    (49006, "ghost wolf soul core"),                 // items.xml:82298
    (49007, "elder forest fury soul core"),          // items.xml:82303
    (49008, "diamond servant replica soul core"),    // items.xml:82308
    (49009, "golden servant replica soul core"),     // items.xml:82313
    (49010, "haunted dragon soul core"),             // items.xml:82318
    (49011, "ice dragon soul core"),                 // items.xml:82323
    (49012, "stone rhino soul core"),                // items.xml:82328
    (49013, "swan maiden soul core"),                // items.xml:82333
    (49014, "goldhanded cultist soul core"),         // items.xml:82338
    (49015, "goldhanded cultist bride soul core"),   // items.xml:82343
    (49016, "orc cultist soul core"),                // items.xml:82348
    (49017, "orc cult priest soul core"),            // items.xml:82353
    (49018, "orc cult fanatic soul core"),           // items.xml:82358
    (49019, "cult scholar soul core"),               // items.xml:82363
    (49020, "mole soul core"),                       // items.xml:82368
    (49021, "arctic faun soul core"),                // items.xml:82373
    (49022, "flying book soul core"),                // items.xml:82378
    (49023, "cursed book soul core"),                // items.xml:82383
    (49024, "guardian of tales soul core"),          // items.xml:82388
    (49025, "knowledge elemental soul core"),        // items.xml:82393
    (49026, "lacewing moth soul core"),              // items.xml:82398
    (49027, "hibernal moth soul core"),              // items.xml:82403
    (49028, "percht soul core"),                     // items.xml:82408
    (49029, "schiach soul core"),                    // items.xml:82413
    (49030, "baleful bunny soul core"),              // items.xml:82418
    (49031, "animated snowman soul core"),           // items.xml:82423
    (49032, "gryphon soul core"),                    // items.xml:82428
    (49033, "orger soul core"),                      // items.xml:82433
    (49034, "roast pork soul core"),                 // items.xml:82438
    (49035, "cow soul core"),                        // items.xml:82443
    (49036, "loricate orger soul core"),             // items.xml:82448
    (49037, "bellicose orger soul core"),            // items.xml:82453
    (49038, "white lion"),                           // items.xml:82458
    (49039, "iks churrascan soul core"),             // items.xml:82463
    (49040, "rabid wolf soul core"),                 // items.xml:82468
    (49041, "ragged rabid wolf soul core"),          // items.xml:82473
    (49073, "nibblemaw soul core"),                  // items.xml:82478
    (49074, "candy floss elemental soul core"),      // items.xml:82483
    (49075, "goggle cake soul core"),                // items.xml:82488
    (49076, "candy horror soul core"),               // items.xml:82493
    (49077, "rootthing bug tracker soul core"),      // items.xml:82498
    (49078, "rootthing amber shaper soul core"),     // items.xml:82503
    (49079, "rootthing nutshell soul core"),         // items.xml:82508
    (49080, "quara raider soul core"),               // items.xml:82513
    (49081, "quara plunderer soul core"),            // items.xml:82518
    (49082, "quara looter soul core"),               // items.xml:82523
    (49084, "truffle soul core"),                    // items.xml:82528
    (49085, "sugar cube soul core"),                 // items.xml:82533
    (49086, "gingerbread man soul core"),            // items.xml:82538
    (49087, "chocolate blob soul core"),             // items.xml:82543
    (49088, "honey elemental soul core"),            // items.xml:82548
    (49089, "angry sugar fairy soul core"),          // items.xml:82553
    (49090, "truffle cook soul core"),               // items.xml:82558
    (49092, "fruit drop soul core"),                 // items.xml:82563
    (49094, "sugar cube worker soul core"),          // items.xml:82568
    (49095, "cream blob soul core"),                 // items.xml:82573
    (49162, "wafer paper butterfly soul core"),      // items.xml:82603
    (49163, "ominous soul core"),                    // items.xml:82608
    (49164, "soul prism"),                           // runtime appearance/XML Item 49164
    (50103, "spellreaper inferniarch soul core"),    // items.xml:84297
    (50104, "hellhunter inferniarch soul core"),     // items.xml:84302
    (50105, "brinebrute inferniarch soul core"),     // items.xml:84307
    (50106, "sineater inferniarch soul core"),       // items.xml:84312
    (50107, "broodrider inferniarch soul core"),     // items.xml:84317
    (50108, "gorger inferniarch soul core"),         // items.xml:84322
    (50330, "monk's apparition soul core"),          // items.xml:86019
    (51641, "norcferatu heartless soul core"),       // runtime appearance/XML Item 51641
    (51642, "varg soul core"),                       // runtime appearance/XML Item 51642
    (51643, "norcferatu nightweaver soul core"),     // runtime appearance/XML Item 51643
    (51644, "dworc shadowstalker soul core"),        // runtime appearance/XML Item 51644
    (51645, "orclops bloodbreaker soul core"),       // runtime appearance/XML Item 51645
    (51646, "gloom maw soul core"),                  // runtime appearance/XML Item 51646
    (51647, "cinder wyrmling soul core"),            // runtime appearance/XML Item 51647
    (51648, "bramble wyrmling soul core"),           // runtime appearance/XML Item 51648
    (51649, "headwalker soul core"),                 // runtime appearance/XML Item 51649
    (51650, "bluebeak soul core"),                   // runtime appearance/XML Item 51650
    (51651, "hawk hopper soul core"),                // runtime appearance/XML Item 51651
    (51652, "shell drake soul core"),                // runtime appearance/XML Item 51652
    (51653, "crusader soul core"),                   // runtime appearance/XML Item 51653
    (51654, "lion hydra soul core"),                 // runtime appearance/XML Item 51654
    (51721, "ink splash soul core"),                 // runtime appearance/XML Item 51721
    (52358, "muglex clan footman soul core"),        // items.xml:87828
    (52359, "muglex clan assassin soul core"),       // items.xml:87833
    (52360, "corrupted ghost soul core"),            // items.xml:87838
    (52361, "corrupted skeleton soul core"),         // items.xml:87843
    (52800, "raubritter skirmisher soul core"),      // runtime appearance/XML Item 52800
    (52801, "raubritter marksman soul core"),        // runtime appearance/XML Item 52801
    (52802, "raubritter chastener soul core"),       // runtime appearance/XML Item 52802
    (52807, "cyclursus soul core"),                  // runtime appearance/XML Item 52807
    (52808, "crypt fiend soul core"),                // runtime appearance/XML Item 52808
    (52809, "walking dread soul core"),              // runtime appearance/XML Item 52809
    (52810, "crypt construct soul core"),            // runtime appearance/XML Item 52810
    (52811, "haunted hunter soul core"),             // runtime appearance/XML Item 52811
    (52812, "creepy crawler soul core"),             // runtime appearance/XML Item 52812
    (52813, "night harpy soul core"),                // runtime appearance/XML Item 52813
    (52814, "roaming dread soul core"),              // runtime appearance/XML Item 52814
    (52815, "crypt mage soul core"),                 // runtime appearance/XML Item 52815
    (52865, "stag soul core"),                       // runtime appearance/XML Item 52865
    (52866, "imperial soul core"),                   // runtime appearance/XML Item 52866
    (53142, "pirate navigator soul core"),           // runtime appearance/XML Item 53142
    (53143, "pirate gunner soul core"),              // runtime appearance/XML Item 53143
    (53144, "pirate cook soul core"),                // runtime appearance/XML Item 53144
    (53145, "pirate quartermaster soul core"),       // runtime appearance/XML Item 53145
    (53146, "sea captain soul core"),                // runtime appearance/XML Item 53146
    (53147, "lizard henchman soul core"),            // runtime appearance/XML Item 53147
    (53148, "lizard magician soul core"),            // runtime appearance/XML Item 53148
    (53149, "lizard swordmaster soul core"),         // runtime appearance/XML Item 53149
    (53150, "lizard executioner soul core"),         // runtime appearance/XML Item 53150
    (53151, "lizard commander soul core"),           // runtime appearance/XML Item 53151
    (53152, "infernoid blob soul core"),             // runtime appearance/XML Item 53152
    (53153, "infernoid hound soul core"),            // runtime appearance/XML Item 53153
    (53154, "infernoid soul soul core"),             // runtime appearance/XML Item 53154
    (53155, "infernoid spiritual soul core"),        // runtime appearance/XML Item 53155
    (54587, "winged jaracal soul core"),             // items.xml:90131
    (54588, "iceplume strider soul core"),           // items.xml:90136
    (54589, "silverfrost sentinel soul core"),       // items.xml:90141
    (54590, "true feverbloom asura soul core"),      // items.xml:90146
    (54591, "radiant warden soul core"),             // items.xml:90151
    (54592, "devoted radiant warden soul core"),     // items.xml:90156
    (54593, "radiant paragon soul core"),            // items.xml:90161
    (54594, "devoted radiant paragon soul core"),    // items.xml:90166
    (54595, "radiant acolyte soul core"),            // items.xml:90171
    (54596, "devoted radiant acolyte soul core"),    // items.xml:90176
    (54597, "radiant templar soul core"),            // items.xml:90181
    (54598, "devoted radiant templar soul core"),    // items.xml:90186
    (54599, "radiant inquisitor soul core"),         // items.xml:90191
    (54600, "devoted radiant inquisitor soul core"), // items.xml:90196
    (54601, "radiant zealot soul core"),             // items.xml:90201
    (54602, "devoted radiant zealot soul core"),     // items.xml:90206
    (54603, "moonspawn blightspitter soul core"),    // items.xml:90211
    (54604, "moonspawn oozecrown soul core"),        // items.xml:90216
    (54630, "jaracal soul core"),                    // items.xml:90248
    (54631, "moonstone excavator soul core"),        // items.xml:90253
    (54632, "moonstone overseer soul core"),         // items.xml:90258
];

pub fn is_soul_core_output(source_id: u32) -> bool {
    SOUL_CORE_SOURCE_IDS.binary_search(&source_id).is_ok()
}

pub fn soul_core_name(source_id: u32) -> Option<&'static str> {
    NAMES
        .binary_search_by_key(&source_id, |(id, _)| *id)
        .ok()
        .map(|index| NAMES[index].1)
}
