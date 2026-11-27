-- Shared source chains and lifetimes are installed by the preceding corpse migrations.
-- Crystal 201189f8 data/items/items.xml. Extend the installed corpse
-- registry only. The existing MINT provenance and scheduler own every stage.
DO $ordinary_corpse_slice2$
DECLARE
    chain_case CONSTANT TEXT := 'WHEN ''oteryn:item.tibia.i54678'' THEN';
    duration_case CONSTANT TEXT := 'WHEN game_summer_corpse_chain(p_key) IS NOT NULL';
    chains CONSTANT TEXT := $chains$
    WHEN 'oteryn:item.tibia.i276' THEN ARRAY[276,277,278,279] -- items.xml:475,481,487,492
    WHEN 'oteryn:item.tibia.i865' THEN ARRAY[865,866,863,864] -- items.xml:2993,2999,2984,2988
    WHEN 'oteryn:item.tibia.i909' THEN ARRAY[909,910,911,275] -- items.xml:3145,3151,3157,471
    WHEN 'oteryn:item.tibia.i949' THEN ARRAY[949,951,955,956] -- items.xml:3224,3232,3249,3255
    WHEN 'oteryn:item.tibia.i4024' THEN ARRAY[4024,4156,4157] -- items.xml:12110,12786,12792
    WHEN 'oteryn:item.tibia.i4239' THEN ARRAY[4239,4321,4322,4323] -- items.xml:13231,13614,13620,13625
    WHEN 'oteryn:item.tibia.i4240' THEN ARRAY[4240,4241,4242] -- items.xml:13237,13243,13248
    WHEN 'oteryn:item.tibia.i4247' THEN ARRAY[4247,4248,4246] -- items.xml:13261,13266,13256
    WHEN 'oteryn:item.tibia.i5960' THEN ARRAY[5960,3987,3991,3992] -- items.xml:16199,11925,11951,11957
    WHEN 'oteryn:item.tibia.i5976' THEN ARRAY[5976,4034,4035,4036] -- items.xml:16290,12164,12170,12176
    WHEN 'oteryn:item.tibia.i5977' THEN ARRAY[5977,4038,4039,4040] -- items.xml:16295,12182,12188,12193
    WHEN 'oteryn:item.tibia.i5978' THEN ARRAY[5978,4041,4042,4147] -- items.xml:16301,12197,12204,12738
    WHEN 'oteryn:item.tibia.i5980' THEN ARRAY[5980,4045,4046,4147] -- items.xml:16313,12221,12228,12738
    WHEN 'oteryn:item.tibia.i5985' THEN ARRAY[5985,4067,4068,4069] -- items.xml:16343,12322,12328,12334
    WHEN 'oteryn:item.tibia.i5986' THEN ARRAY[5986,4070,4071,4072] -- items.xml:16348,12339,12345,12350
    WHEN 'oteryn:item.tibia.i5991' THEN ARRAY[5991,4086,4087,4088] -- items.xml:16378,12421,12428,12434
    WHEN 'oteryn:item.tibia.i6002' THEN ARRAY[6002,4121,4122,4123] -- items.xml:16442,12604,12611,12617
    WHEN 'oteryn:item.tibia.i6003' THEN ARRAY[6003,4126,4127,4128] -- items.xml:16448,12632,12639,12645
    WHEN 'oteryn:item.tibia.i6013' THEN ARRAY[6013,4164,4165,4143] -- items.xml:16505,12836,12843,12720
    WHEN 'oteryn:item.tibia.i6014' THEN ARRAY[6014,4166,4167,4143] -- items.xml:16511,12849,12856,12720
    WHEN 'oteryn:item.tibia.i6015' THEN ARRAY[6015,4168,4169,4143] -- items.xml:16517,12862,12869,12720
    WHEN 'oteryn:item.tibia.i6016' THEN ARRAY[6016,4170,4171,4172] -- items.xml:16523,12875,12881,12886
    WHEN 'oteryn:item.tibia.i6026' THEN ARRAY[6026,4200,4201,4202] -- items.xml:16578,13032,13039,13045
    WHEN 'oteryn:item.tibia.i6027' THEN ARRAY[6027,4203,4204,4205] -- items.xml:16584,13050,13055,13059
    WHEN 'oteryn:item.tibia.i6032' THEN ARRAY[6032,4218,4219,4220] -- items.xml:16611,13122,13128,13133
    WHEN 'oteryn:item.tibia.i6033' THEN ARRAY[6033,4221,4222,4223] -- items.xml:16617,13137,13143,13148
    WHEN 'oteryn:item.tibia.i6036' THEN ARRAY[6036,4230,4231,4232] -- items.xml:16635,13187,13193,13198
    WHEN 'oteryn:item.tibia.i6037' THEN ARRAY[6037,4233,4234,4235] -- items.xml:16641,13203,13209,13213
    WHEN 'oteryn:item.tibia.i6040' THEN ARRAY[6040,4324,4325,4326] -- items.xml:16659,13629,13635,13640
    WHEN 'oteryn:item.tibia.i6041' THEN ARRAY[6041,4327,4328,4329] -- items.xml:16665,13644,13650,13655
    WHEN 'oteryn:item.tibia.i6043' THEN ARRAY[6043,4333,4334,4335] -- items.xml:16677,13676,13682,13686
    WHEN 'oteryn:item.tibia.i6048' THEN ARRAY[6048,4348,4349,4350] -- items.xml:16707,13746,13752,13757
    WHEN 'oteryn:item.tibia.i6054' THEN ARRAY[6054,4366,4367,4368] -- items.xml:16743,13835,13841,13845
    WHEN 'oteryn:item.tibia.i6055' THEN ARRAY[6055,4369,4370,4371] -- items.xml:16749,13849,13855,13859
    WHEN 'oteryn:item.tibia.i6058' THEN ARRAY[6058,4372,4373,4374] -- items.xml:16767,13863,13869,13873
    WHEN 'oteryn:item.tibia.i6059' THEN ARRAY[6059,4375,4376,4377] -- items.xml:16773,13877,13883,13887
    WHEN 'oteryn:item.tibia.i6060' THEN ARRAY[6060,4385,4386,4387] -- items.xml:16779,13925,13931,13935
    WHEN 'oteryn:item.tibia.i6065' THEN ARRAY[6065,5523,5530,5531] -- items.xml:16809,15168,15209,15214
    WHEN 'oteryn:item.tibia.i6077' THEN ARRAY[6077,5765,5766,5767] -- items.xml:16881,15620,15626,15631
    WHEN 'oteryn:item.tibia.i6311' THEN ARRAY[6311,6312,6313,6314] -- items.xml:17412,17418,17423,17428
    WHEN 'oteryn:item.tibia.i6323' THEN ARRAY[6323,6324,6325,6326] -- items.xml:17471,17476,17481,17486
    WHEN 'oteryn:item.tibia.i6324' THEN ARRAY[6324,6325,6326] -- items.xml:17476,17481,17486
    WHEN 'oteryn:item.tibia.i6331' THEN ARRAY[6331,6332,6333,6334] -- items.xml:17509,17515,17520,17525
    WHEN 'oteryn:item.tibia.i6335' THEN ARRAY[6335,6336,6337,6338] -- items.xml:17529,17535,17540,17545
    WHEN 'oteryn:item.tibia.i6560' THEN ARRAY[6560,4247,4248,4246] -- items.xml:18195,13261,13266,13256
    WHEN 'oteryn:item.tibia.i7091' THEN ARRAY[7091,7092,7093,7094] -- items.xml:18729,18734,18739,18744
    WHEN 'oteryn:item.tibia.i7316' THEN ARRAY[7316,7317,7318,7319] -- items.xml:19046,19052,19058,19063
    WHEN 'oteryn:item.tibia.i7330' THEN ARRAY[7330,7331,7332,7333] -- items.xml:19122,19128,19134,19139
    WHEN 'oteryn:item.tibia.i7538' THEN ARRAY[7538,7539,7540] -- items.xml:20790,20795,20800
    WHEN 'oteryn:item.tibia.i7893' THEN ARRAY[7893] -- items.xml:21281
    WHEN 'oteryn:item.tibia.i8105' THEN ARRAY[8105,8106,8107,8108] -- items.xml:22525,22531,22537,22543
    WHEN 'oteryn:item.tibia.i8137' THEN ARRAY[8137,8140] -- items.xml:22697,22715
    WHEN 'oteryn:item.tibia.i8957' THEN ARRAY[8957,8958,8959,8960] -- items.xml:23823,23828,23833,23838
    WHEN 'oteryn:item.tibia.i9001' THEN ARRAY[9001,9002,9003,9004] -- items.xml:23937,23942,23947,23952
    WHEN 'oteryn:item.tibia.i9009' THEN ARRAY[9009,9010,9011,9012] -- items.xml:23977,23982,23987,23992
    WHEN 'oteryn:item.tibia.i9024' THEN ARRAY[9024,9021,9022,9023] -- items.xml:24082,24067,24072,24077
    WHEN 'oteryn:item.tibia.i9591' THEN ARRAY[9591,8118] -- items.xml:25813,22596
    WHEN 'oteryn:item.tibia.i9607' THEN ARRAY[9607,9608,9609,9610] -- items.xml:25914,25920,25926,25930
    WHEN 'oteryn:item.tibia.i10333' THEN ARRAY[10333,10330,10331,10332] -- items.xml:27062,27045,27052,27057
    WHEN 'oteryn:item.tibia.i10355' THEN ARRAY[10355,10352,10353,10354] -- items.xml:27152,27138,27144,27148
    WHEN 'oteryn:item.tibia.i10359' THEN ARRAY[10359,10356,10357,10358] -- items.xml:27172,27158,27164,27168
    WHEN 'oteryn:item.tibia.i10363' THEN ARRAY[10363,10360,10361,10362] -- items.xml:27192,27178,27184,27188
    WHEN 'oteryn:item.tibia.i10367' THEN ARRAY[10367,10364,10365,10366] -- items.xml:27212,27198,27204,27208
    WHEN 'oteryn:item.tibia.i10371' THEN ARRAY[10371,10368,10369,10370] -- items.xml:27232,27218,27224,27228
    WHEN 'oteryn:item.tibia.i10383' THEN ARRAY[10383,10382] -- items.xml:27244,27239
    WHEN 'oteryn:item.tibia.i10403' THEN ARRAY[10403] -- items.xml:27452
    WHEN 'oteryn:item.tibia.i10458' THEN ARRAY[10458,10459,10460,10461] -- items.xml:27726,27732,27738,27742
    WHEN 'oteryn:item.tibia.i11589' THEN ARRAY[11589] -- items.xml:28732
    WHEN 'oteryn:item.tibia.i11590' THEN ARRAY[11590] -- items.xml:28736
    WHEN 'oteryn:item.tibia.i12494' THEN ARRAY[12494,12497,12498,12499] -- items.xml:30265,30283,30289,30294
    WHEN 'oteryn:item.tibia.i12495' THEN ARRAY[12495,12500,12501,12502] -- items.xml:30271,30298,30304,30308
    WHEN 'oteryn:item.tibia.i12497' THEN ARRAY[12497,12498,12499] -- items.xml:30283,30289,30294
    WHEN 'oteryn:item.tibia.i12500' THEN ARRAY[12500,12501,12502] -- items.xml:30298,30304,30308
    WHEN 'oteryn:item.tibia.i12512' THEN ARRAY[12512,12513,12514,12515] -- items.xml:30344,30349,30354,30359
    WHEN 'oteryn:item.tibia.i12525' THEN ARRAY[12525,12536,12537,12538] -- items.xml:30402,30459,30465,30470
    WHEN 'oteryn:item.tibia.i13787' THEN ARRAY[13787,13794,13795,13796] -- items.xml:31680,31685,31690,31695
    WHEN 'oteryn:item.tibia.i13845' THEN ARRAY[13845,13846,13847,13848] -- items.xml:31753,31758,31763,31768
    WHEN 'oteryn:item.tibia.i13857' THEN ARRAY[13857,13858,13864,13859] -- items.xml:31815,31822,31857,31828
    WHEN 'oteryn:item.tibia.i13870' THEN ARRAY[13870,13871,13872] -- items.xml:31882,31887,31892
    WHEN 'oteryn:item.tibia.i13937' THEN ARRAY[13937,13938,13939,13940] -- items.xml:31956,31961,31966,31971
    WHEN 'oteryn:item.tibia.i13973' THEN ARRAY[13973,13970,13971,13972] -- items.xml:31997,31983,31988,31993
    WHEN 'oteryn:item.tibia.i15872' THEN ARRAY[15872,15873,15874,15875] -- items.xml:33928,33933,33938,33943
    WHEN 'oteryn:item.tibia.i15906' THEN ARRAY[15906,15907,15908,15909] -- items.xml:33975,33980,33985,33990
    WHEN 'oteryn:item.tibia.i15988' THEN ARRAY[15988,15989,15990] -- items.xml:34022,34027,34032
    WHEN 'oteryn:item.tibia.i15991' THEN ARRAY[15991,15992,15993,15994] -- items.xml:34036,34041,34046,34051
    WHEN 'oteryn:item.tibia.i16063' THEN ARRAY[16063,16064,16065] -- items.xml:34070,34076,34081
    WHEN 'oteryn:item.tibia.i16079' THEN ARRAY[16079,16080,16081,16082] -- items.xml:34144,34149,34154,34159
    WHEN 'oteryn:item.tibia.i16083' THEN ARRAY[16083,16084,16085,16086] -- items.xml:34163,34169,34175,34180
    WHEN 'oteryn:item.tibia.i16149' THEN ARRAY[16149,16150,16151,16152] -- items.xml:34658,34663,34668,34673
    WHEN 'oteryn:item.tibia.i17426' THEN ARRAY[17426,17433,17434,17435] -- items.xml:35547,35585,35590,35595
    WHEN 'oteryn:item.tibia.i17449' THEN ARRAY[17449,17450] -- items.xml:35664,35669
    WHEN 'oteryn:item.tibia.i18110' THEN ARRAY[18110,18111,18112,18113] -- items.xml:37130,37136,37142,37148
    WHEN 'oteryn:item.tibia.i18114' THEN ARRAY[18114,18115,18116,18117] -- items.xml:37152,37158,37164,37170
    WHEN 'oteryn:item.tibia.i18118' THEN ARRAY[18118,18119,18120,18121] -- items.xml:37174,37180,37186,37192
    WHEN 'oteryn:item.tibia.i18122' THEN ARRAY[18122,18123,18124,18125] -- items.xml:37196,37202,37208,37214
    WHEN 'oteryn:item.tibia.i18126' THEN ARRAY[18126,18127,18128,18129] -- items.xml:37218,37224,37230,37236
    WHEN 'oteryn:item.tibia.i18130' THEN ARRAY[18130,18131,18132,18133] -- items.xml:37240,37246,37252,37257
    WHEN 'oteryn:item.tibia.i18134' THEN ARRAY[18134,18135,18136] -- items.xml:37261,37267,37273
    WHEN 'oteryn:item.tibia.i18138' THEN ARRAY[18138,18139,18140] -- items.xml:37283,37289,37295
    WHEN 'oteryn:item.tibia.i18142' THEN ARRAY[18142,18143,18144] -- items.xml:37305,37311,37317
    WHEN 'oteryn:item.tibia.i18146' THEN ARRAY[18146,18147,18148] -- items.xml:37327,37333,37339
    WHEN 'oteryn:item.tibia.i18153' THEN ARRAY[18153,18150,18151,18152] -- items.xml:37365,37349,37355,37361
    WHEN 'oteryn:item.tibia.i18157' THEN ARRAY[18157,18154,18155,18156] -- items.xml:37387,37371,37377,37383
    WHEN 'oteryn:item.tibia.i18254' THEN ARRAY[18254,18255,18256] -- items.xml:37916,37922,37928
    WHEN 'oteryn:item.tibia.i18285' THEN ARRAY[18285,18282,18283,18284] -- items.xml:38086,38070,38076,38082
    WHEN 'oteryn:item.tibia.i18289' THEN ARRAY[18289,18286,18287,18288] -- items.xml:38108,38092,38098,38104
    WHEN 'oteryn:item.tibia.i18318' THEN ARRAY[18318,18316,18317,4128] -- items.xml:38272,38260,38266,12645
    WHEN 'oteryn:item.tibia.i18962' THEN ARRAY[18962,18963,18964,18965] -- items.xml:38796,38802,38808,38813
    WHEN 'oteryn:item.tibia.i18966' THEN ARRAY[18966,18967,18968,18969] -- items.xml:38817,38823,38829,38834
    WHEN 'oteryn:item.tibia.i19041' THEN ARRAY[19041,19043,19044,19045] -- items.xml:38957,38969,38975,38980
    WHEN 'oteryn:item.tibia.i19042' THEN ARRAY[19042,19046,19047,19048] -- items.xml:38963,38984,38990,38995
    WHEN 'oteryn:item.tibia.i19051' THEN ARRAY[19051,19055,19056,19057] -- items.xml:39009,39028,39033,39038
    WHEN 'oteryn:item.tibia.i20155' THEN ARRAY[20155,20152,20153,20154] -- items.xml:40792,40776,40783,40788
    WHEN 'oteryn:item.tibia.i20228' THEN ARRAY[20228,20229,20230,20231] -- items.xml:41023,41029,41035,41040
    WHEN 'oteryn:item.tibia.i20233' THEN ARRAY[20233,20234,20235,20236] -- items.xml:41044,41050,41056,41061
    WHEN 'oteryn:item.tibia.i20972' THEN ARRAY[20972,20973,20974,20975] -- items.xml:41522,41527,41532,41537
    WHEN 'oteryn:item.tibia.i20988' THEN ARRAY[20988,20989,20990,20991] -- items.xml:41598,41603,41608,41613
    WHEN 'oteryn:item.tibia.i21091' THEN ARRAY[21091,21092,21093,21094] -- items.xml:41724,41730,41737,41743
    WHEN 'oteryn:item.tibia.i21108' THEN ARRAY[21108,21109] -- items.xml:41809,41814
    WHEN 'oteryn:item.tibia.i21882' THEN ARRAY[21882,21884,21885,21886] -- items.xml:43367,43373,43379,43384
    WHEN 'oteryn:item.tibia.i21888' THEN ARRAY[21888,21889,21890,21891] -- items.xml:43388,43394,43400,43405
    WHEN 'oteryn:item.tibia.i21907' THEN ARRAY[21907,21908,21909] -- items.xml:43481,43486,43491
    WHEN 'oteryn:item.tibia.i21983' THEN ARRAY[21983,21984,21985,21986] -- items.xml:43709,43714,43719,43724
    WHEN 'oteryn:item.tibia.i22078' THEN ARRAY[22078,22079,22080] -- items.xml:44064,44069,44074
    WHEN 'oteryn:item.tibia.i22089' THEN ARRAY[22089,22090,22091,22092] -- items.xml:44144,44149,44154,44159
    WHEN 'oteryn:item.tibia.i22780' THEN ARRAY[22780,22781,22782,22783] -- items.xml:45708,45713,45718,45723
    WHEN 'oteryn:item.tibia.i22784' THEN ARRAY[22784,22785,22786] -- items.xml:45727,45732,45737
    WHEN 'oteryn:item.tibia.i23388' THEN ARRAY[23388,23389,23390,23391] -- items.xml:46106,46111,46116,46121
    WHEN 'oteryn:item.tibia.i25185' THEN ARRAY[25185,25189,25190] -- items.xml:48412,48436,48442
    WHEN 'oteryn:item.tibia.i25815' THEN ARRAY[25815,25814,25813,25812] -- items.xml:49357,49351,49345,49341
    WHEN 'oteryn:item.tibia.i26133' THEN ARRAY[26133,26134,26135] -- items.xml:51500,51504,51508
    WHEN 'oteryn:item.tibia.i27517' THEN ARRAY[27517,27516,27515,27514] -- items.xml:52214,52208,52202,52198
    WHEN 'oteryn:item.tibia.i27613' THEN ARRAY[27613,27612,27611,27610] -- items.xml:52515,52509,52503,52498
    WHEN 'oteryn:item.tibia.i28586' THEN ARRAY[28586,28585,28584,28583] -- items.xml:53717,53711,53706,53702
    WHEN 'oteryn:item.tibia.i28598' THEN ARRAY[28598,28597,28596,28595] -- items.xml:53760,53754,53749,53745
    WHEN 'oteryn:item.tibia.i28605' THEN ARRAY[28605,28604,28603] -- items.xml:53796,53791,53787
    WHEN 'oteryn:item.tibia.i28613' THEN ARRAY[28613,28612,28611] -- items.xml:53838,53833,53829
    WHEN 'oteryn:item.tibia.i28625' THEN ARRAY[28625,28624,28623] -- items.xml:53901,53896,53892
    WHEN 'oteryn:item.tibia.i28726' THEN ARRAY[28726,28727,28728,28729] -- items.xml:54525,54531,54537,54542
    WHEN 'oteryn:item.tibia.i28733' THEN ARRAY[28733,28732,28731,28730] -- items.xml:54561,54555,54550,54546
    WHEN 'oteryn:item.tibia.i28737' THEN ARRAY[28737,28736,28735,28734] -- items.xml:54582,54576,54571,54567
    WHEN 'oteryn:item.tibia.i28770' THEN ARRAY[28770,28769,28768,28767] -- items.xml:54642,54636,54631,54626
    WHEN 'oteryn:item.tibia.i28774' THEN ARRAY[28774,28773,28772,28771] -- items.xml:54663,54657,54652,54648
    WHEN 'oteryn:item.tibia.i28778' THEN ARRAY[28778,28777,28776,28775] -- items.xml:54685,54679,54673,54669
    WHEN 'oteryn:item.tibia.i28807' THEN ARRAY[28807,28808,28809] -- items.xml:54820,54826,54831
    WHEN 'oteryn:item.tibia.i28861' THEN ARRAY[28861,28860,28859] -- items.xml:55081,55076,55072
    WHEN 'oteryn:item.tibia.i28873' THEN ARRAY[28873,28872,28871] -- items.xml:55111,55106,55102
    WHEN 'oteryn:item.tibia.i30013' THEN ARRAY[30013,30012,30011,30010] -- items.xml:56079,56073,56067,56063
    WHEN 'oteryn:item.tibia.i30133' THEN ARRAY[30133,30135,30136,30137] -- items.xml:56475,56487,56493,56498
    WHEN 'oteryn:item.tibia.i30167' THEN ARRAY[30167,30166,30165,30164] -- items.xml:56648,56642,56637,56633
    WHEN 'oteryn:item.tibia.i30175' THEN ARRAY[30175,30174,30173,30172] -- items.xml:56689,56683,56678,56674
    WHEN 'oteryn:item.tibia.i30179' THEN ARRAY[30179,30178,30177,30176] -- items.xml:56710,56704,56699,56695
    WHEN 'oteryn:item.tibia.i31307' THEN ARRAY[31307,31308,31309,31310] -- items.xml:58354,58359,58365,58370
    WHEN 'oteryn:item.tibia.i31394' THEN ARRAY[31394,31393,31392,31391] -- items.xml:58622,58616,58611,58607
    WHEN 'oteryn:item.tibia.i31398' THEN ARRAY[31398,31397,31396,31395] -- items.xml:58643,58637,58632,58628
    WHEN 'oteryn:item.tibia.i31611' THEN ARRAY[31611,31610,31609,31608] -- items.xml:59450,59445,59440,59436
    WHEN 'oteryn:item.tibia.i31654' THEN ARRAY[31654,31653,31652,31651] -- items.xml:59687,59681,59676,59672
    WHEN 'oteryn:item.tibia.i31658' THEN ARRAY[31658,31657,31656,31655] -- items.xml:59708,59702,59697,59693
    WHEN 'oteryn:item.tibia.i31662' THEN ARRAY[31662,31661,31660,31659] -- items.xml:59729,59723,59718,59714
    WHEN 'oteryn:item.tibia.i33957' THEN ARRAY[33957,33956,33955,33954] -- items.xml:63725,63720,63715,63711
    WHEN 'oteryn:item.tibia.i34189' THEN ARRAY[34189,34191,34192,34193] -- items.xml:65091,65101,65106,65111
    WHEN 'oteryn:item.tibia.i35358' THEN ARRAY[35358,35359,35360,35361] -- items.xml:66009,66015,66021,66027
    WHEN 'oteryn:item.tibia.i35690' THEN ARRAY[35690,35689,35688,35687] -- items.xml:66464,66458,66452,66447
    WHEN 'oteryn:item.tibia.i36679' THEN ARRAY[36679,36678,36677,36676] -- items.xml:68032,68026,68020,68016
    WHEN 'oteryn:item.tibia.i36696' THEN ARRAY[36696,36695,36694,36693] -- items.xml:68120,68114,68108,68104
    WHEN 'oteryn:item.tibia.i36764' THEN ARRAY[36764,36763,36762,36761] -- items.xml:68350,68344,68338,68334
    WHEN 'oteryn:item.tibia.i36800' THEN ARRAY[36800,36799,36798,36797] -- items.xml:68500,68494,68488,68484
    WHEN 'oteryn:item.tibia.i36881' THEN ARRAY[36881,36880,36879,36878] -- items.xml:68700,68694,68688,68683
    WHEN 'oteryn:item.tibia.i36967' THEN ARRAY[36967,36968,36969,36970] -- items.xml:69028,69034,69040,69046
    WHEN 'oteryn:item.tibia.i39208' THEN ARRAY[39208,39207,39206,39205] -- items.xml:72234,72228,72223,72219
    WHEN 'oteryn:item.tibia.i39230' THEN ARRAY[39230] -- items.xml:72345
    WHEN 'oteryn:item.tibia.i39283' THEN ARRAY[39283,39282,39281,39280] -- items.xml:72476,72470,72465,72461
    WHEN 'oteryn:item.tibia.i39291' THEN ARRAY[39291,39290,39289,39288] -- items.xml:72518,72512,72507,72503
    WHEN 'oteryn:item.tibia.i39303' THEN ARRAY[39303,39302,39301,39300] -- items.xml:72581,72575,72570,72566
    WHEN 'oteryn:item.tibia.i39311' THEN ARRAY[39311,39310,39309,39308] -- items.xml:72623,72617,72612,72608
    WHEN 'oteryn:item.tibia.i39319' THEN ARRAY[39319,39318,39317,39316] -- items.xml:72665,72659,72654,72650
    WHEN 'oteryn:item.tibia.i39327' THEN ARRAY[39327,39326,39325,39324] -- items.xml:72707,72701,72696,72692
    WHEN 'oteryn:item.tibia.i42053' THEN ARRAY[42053,42052,42051,42050] -- items.xml:74314,74308,74302,74297
    WHEN 'oteryn:item.tibia.i42057' THEN ARRAY[42057,42056,42055,42054] -- items.xml:74337,74331,74325,74320
    WHEN 'oteryn:item.tibia.i42061' THEN ARRAY[42061,42060,42059,42058] -- items.xml:74360,74354,74348,74343
    WHEN 'oteryn:item.tibia.i42065' THEN ARRAY[42065,42064,42063,42062] -- items.xml:74383,74377,74371,74366
    WHEN 'oteryn:item.tibia.i42214' THEN ARRAY[42214,42213,42212,42211] -- items.xml:74479,74473,74467,74462
    WHEN 'oteryn:item.tibia.i42222' THEN ARRAY[42222,42221,42220,42219] -- items.xml:74525,74519,74513,74508
    WHEN 'oteryn:item.tibia.i43767' THEN ARRAY[43767,43766,43765,43764] -- items.xml:75658,75652,75646,75641
    WHEN 'oteryn:item.tibia.i48263' THEN ARRAY[48263,48262,48261] -- items.xml:81507,81502,81498
    WHEN 'oteryn:item.tibia.i49994' THEN ARRAY[49994,49993,49992,49991] -- items.xml:84062,84057,84052,84048
    WHEN 'oteryn:item.tibia.i50010' THEN ARRAY[50010,50009,50008,50007] -- items.xml:84138,84133,84128,84124
    WHEN 'oteryn:item.tibia.i51537' THEN ARRAY[51537,51540,51541,51542] -- items.xml:86945,86955,86960,86965
    WHEN 'oteryn:item.tibia.i51552' THEN ARRAY[51552,51551,51550,51549] -- items.xml:87011,87006,87001,86997
    WHEN 'oteryn:item.tibia.i51564' THEN ARRAY[51564,51563,51562,51561] -- items.xml:87049,87044,87039,87035
    WHEN 'oteryn:item.tibia.i51584' THEN ARRAY[51584,51585,51586,51587] -- items.xml:87076,87081,87086,87091
    WHEN 'oteryn:item.tibia.i52571' THEN ARRAY[52571,52570,52569,52568] -- items.xml:87986,87980,87974,87968
    WHEN 'oteryn:item.tibia.i52845' THEN ARRAY[52845,52844,52843,52842] -- items.xml:88524,88518,88512,88506
    WHEN 'oteryn:item.tibia.i53078' THEN ARRAY[53078] -- items.xml:88689
    WHEN 'oteryn:item.tibia.i53082' THEN ARRAY[53082] -- items.xml:88695
    WHEN 'oteryn:item.tibia.i53086' THEN ARRAY[53086] -- items.xml:88701
    WHEN 'oteryn:item.tibia.i53090' THEN ARRAY[53090] -- items.xml:88707
    WHEN 'oteryn:item.tibia.i53094' THEN ARRAY[53094] -- items.xml:88713
    WHEN 'oteryn:item.tibia.i53108' THEN ARRAY[53108] -- items.xml:88731
    WHEN 'oteryn:item.tibia.i53122' THEN ARRAY[53122] -- items.xml:88754
    WHEN 'oteryn:item.tibia.i53125' THEN ARRAY[53125] -- items.xml:88759
    WHEN 'oteryn:item.tibia.i53128' THEN ARRAY[53128] -- items.xml:88764
    WHEN 'oteryn:item.tibia.i53132' THEN ARRAY[53132] -- items.xml:88769
    WHEN 'oteryn:item.tibia.i54335' THEN ARRAY[54335,54336,54337] -- items.xml:89734,89739,89744
    WHEN 'oteryn:item.tibia.i54386' THEN ARRAY[54386,54387,54388,54389] -- items.xml:89889,89895,89901,89907
    $chains$;
    durations CONSTANT TEXT := $durations$
    WHEN p_key='oteryn:item.tibia.i909' THEN ARRAY[10000,900000,600000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i4024' THEN ARRAY[300000,300000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i4240' THEN ARRAY[1200000,1200000,1200000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i4247' THEN ARRAY[1200000,1200000,1200000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i6324' THEN ARRAY[300000,300000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i6335' THEN ARRAY[10000,900000,600000,600000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i6560' THEN ARRAY[180000,1200000,1200000,1200000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i7538' THEN ARRAY[300000,300000,0]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i7893' THEN ARRAY[180000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i8137' THEN ARRAY[10000,500000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i9591' THEN ARRAY[10000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i10383' THEN ARRAY[10000,500000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i10403' THEN ARRAY[3000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i11589' THEN ARRAY[500000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i11590' THEN ARRAY[500000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i12497' THEN ARRAY[300000,300000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i12500' THEN ARRAY[300000,300000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i13857' THEN ARRAY[10000,300000,60000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i13870' THEN ARRAY[300000,300000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i15988' THEN ARRAY[300000,300000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i16063' THEN ARRAY[300000,300000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i17449' THEN ARRAY[300000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i18134' THEN ARRAY[300000,300000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i18138' THEN ARRAY[300000,300000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i18142' THEN ARRAY[300000,300000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i18146' THEN ARRAY[300000,300000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i18254' THEN ARRAY[300000,300000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i21108' THEN ARRAY[10000,600000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i21907' THEN ARRAY[10000,300000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i22078' THEN ARRAY[300000,300000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i22784' THEN ARRAY[300000,300000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i25185' THEN ARRAY[10000,300000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i26133' THEN ARRAY[300000,300000,0]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i28605' THEN ARRAY[300000,300000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i28613' THEN ARRAY[300000,300000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i28625' THEN ARRAY[300000,300000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i28733' THEN ARRAY[10000,300000,300000,0]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i28770' THEN ARRAY[10000,300000,300000,300000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i28807' THEN ARRAY[300000,300000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i28861' THEN ARRAY[300000,300000,300000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i28873' THEN ARRAY[300000,300000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i33957' THEN ARRAY[10000,600000,600000,600000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i34189' THEN ARRAY[10000,600000,600000,600000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i35358' THEN ARRAY[10000,300000,300000,300000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i35690' THEN ARRAY[10000,300000,300000,300000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i39230' THEN ARRAY[10000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i48263' THEN ARRAY[5000,5000,5000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i51537' THEN ARRAY[300000,300000,60000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i51552' THEN ARRAY[300000,300000,60000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i51564' THEN ARRAY[300000,300000,60000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i51584' THEN ARRAY[300000,300000,60000,60000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i52571' THEN ARRAY[300000,300000,300000,300000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i52845' THEN ARRAY[300000,300000,300000,300000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i53078' THEN ARRAY[600000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i53082' THEN ARRAY[600000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i53086' THEN ARRAY[600000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i53090' THEN ARRAY[600000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i53094' THEN ARRAY[600000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i53108' THEN ARRAY[900000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i53122' THEN ARRAY[600000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i53125' THEN ARRAY[600000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i53128' THEN ARRAY[600000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i53132' THEN ARRAY[600000]::BIGINT[]
    WHEN p_key='oteryn:item.tibia.i54335' THEN ARRAY[10000,300000,60000]::BIGINT[]
    $durations$;
    chain_definition TEXT := pg_get_functiondef('game_summer_corpse_chain(text)'::regprocedure);
    duration_definition TEXT := pg_get_functiondef('game_source_corpse_durations_v1(text)'::regprocedure);
BEGIN
    IF (length(chain_definition)-length(replace(chain_definition,chain_case,'')))/length(chain_case) <> 1
       OR (length(duration_definition)-length(replace(duration_definition,duration_case,'')))/length(duration_case) <> 1 THEN
        RAISE EXCEPTION 'Existing corpse lifecycle registry changed';
    END IF;
    IF EXISTS (
        SELECT 1 FROM regexp_matches(chains, 'WHEN ''([^'']+)'' THEN', 'g') AS root
        WHERE game_summer_corpse_chain(root[1]) IS NOT NULL
    ) THEN
        RAISE EXCEPTION 'Ordinary corpse already has a source lifecycle';
    END IF;
    EXECUTE replace(chain_definition,chain_case,chains||chain_case);
    EXECUTE replace(duration_definition,duration_case,durations||duration_case);
END $ordinary_corpse_slice2$;

-- Existing non-container source bodies use the same original lethal MINT proof.
-- Migration 3250 already admits the shared ashes. Crystal 201189f8
-- items.xml:27452-27455 supplies the insect swarm.
DO $ordinary_plain_bodies$
DECLARE
    anchor CONSTANT TEXT := '''oteryn:item.tibia.i33792''';
    body TEXT := pg_get_functiondef('game_item_plain_corpse_source(uuid)'::regprocedure);
BEGIN
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'Existing plain corpse registry changed';
    END IF;
    EXECUTE replace(body,anchor,'''oteryn:item.tibia.i10403'',' || anchor);
END $ordinary_plain_bodies$;
