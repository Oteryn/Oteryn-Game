-- Shared source chains and lifetimes are installed by the preceding corpse migrations.
-- Source registry data for Crystal 201189f8 data/items/items.xml.
-- The existing MINT trigger schedules every nonterminal transform. A zero
-- terminal lifetime retains the body, as in Item::canDecay.
DO $source_corpses$
DECLARE
    chain_body TEXT := pg_get_functiondef('game_summer_corpse_chain(text)'::regprocedure);
    lifetime_body TEXT := pg_get_functiondef('game_source_corpse_durations_v1(text)'::regprocedure);
    plain_body TEXT := pg_get_functiondef('game_item_plain_corpse_source(uuid)'::regprocedure);
    chain_anchor CONSTANT TEXT := 'SELECT CASE p_key';
    lifetime_anchor CONSTANT TEXT := 'SELECT CASE';
    plain_anchor CONSTANT TEXT := $$'oteryn:item.tibia.i33792'$$;
    chain_cases TEXT := '';
    lifetime_cases TEXT := '';
    source RECORD;
    item_key TEXT;
    installed_chain INTEGER[];
    installed_lifetimes BIGINT[];
BEGIN
    IF (length(chain_body)-length(replace(chain_body,chain_anchor,'')))/length(chain_anchor) <> 1
        OR (length(lifetime_body)-length(replace(lifetime_body,lifetime_anchor,'')))/length(lifetime_anchor) <> 1
        OR (length(plain_body)-length(replace(plain_body,plain_anchor,'')))/length(plain_anchor) <> 1
    THEN
        RAISE EXCEPTION 'Source corpse registry predecessor changed';
    END IF;
    -- Crystal retains these admitted roots with no timer. Candy Horror's
    -- positive timer also leaves the same body: target 48297 has no ItemType.
    FOR source IN SELECT * FROM (VALUES
        (2983, ARRAY[2983], ARRAY[0]::BIGINT[]),
        (4378, ARRAY[4378], ARRAY[0]::BIGINT[]),
        (21815, ARRAY[21815], ARRAY[0]::BIGINT[]),
        (21887, ARRAY[21887], ARRAY[0]::BIGINT[]),
        (22495, ARRAY[22495], ARRAY[0]::BIGINT[]),
        (22698, ARRAY[22698], ARRAY[0]::BIGINT[]),
        (23567, ARRAY[23567], ARRAY[0]::BIGINT[]),
        (23729, ARRAY[23729], ARRAY[0]::BIGINT[]),
        (26125, ARRAY[26125], ARRAY[0]::BIGINT[]),
        (27586, ARRAY[27586], ARRAY[0]::BIGINT[]),
        (31203, ARRAY[31203], ARRAY[0]::BIGINT[]),
        (31204, ARRAY[31204], ARRAY[0]::BIGINT[]),
        (31205, ARRAY[31205], ARRAY[0]::BIGINT[]),
        (31206, ARRAY[31206], ARRAY[0]::BIGINT[]),
        (35136, ARRAY[35136], ARRAY[0]::BIGINT[]),
        (35384, ARRAY[35384], ARRAY[0]::BIGINT[]),
        (35388, ARRAY[35388], ARRAY[0]::BIGINT[]),
        (35600, ARRAY[35600], ARRAY[0]::BIGINT[]),
        (35686, ARRAY[35686], ARRAY[0]::BIGINT[]),
        (35846, ARRAY[35846], ARRAY[0]::BIGINT[]),
        (36434, ARRAY[36434], ARRAY[0]::BIGINT[]),
        (36929, ARRAY[36929], ARRAY[0]::BIGINT[]),
        (39949, ARRAY[39949], ARRAY[0]::BIGINT[]),
        (43762, ARRAY[43762], ARRAY[0]::BIGINT[]),
        (43771, ARRAY[43771], ARRAY[0]::BIGINT[]),
        (43959, ARRAY[43959], ARRAY[0]::BIGINT[]),
        (44666, ARRAY[44666], ARRAY[0]::BIGINT[]),
        (44670, ARRAY[44670], ARRAY[0]::BIGINT[]),
        (48109, ARRAY[48109], ARRAY[0]::BIGINT[]),
        (48112, ARRAY[48112], ARRAY[0]::BIGINT[]),
        (48268, ARRAY[48268], ARRAY[0]::BIGINT[]),
        (48271, ARRAY[48271], ARRAY[0]::BIGINT[]),
        (48340, ARRAY[48340], ARRAY[0]::BIGINT[]),
        (48349, ARRAY[48349], ARRAY[0]::BIGINT[]),
        (48353, ARRAY[48353], ARRAY[0]::BIGINT[]),
        (48366, ARRAY[48366], ARRAY[0]::BIGINT[]),
        (48382, ARRAY[48382], ARRAY[0]::BIGINT[]),
        (48405, ARRAY[48405], ARRAY[0]::BIGINT[]),
        (49120, ARRAY[49120], ARRAY[0]::BIGINT[]),
        (50311, ARRAY[50311], ARRAY[0]::BIGINT[]),
        (51560, ARRAY[51560], ARRAY[0]::BIGINT[]),
        (2127, ARRAY[2127], ARRAY[2000]::BIGINT[]),
        (4388, ARRAY[4388,4389,4390], ARRAY[300000,300000,60000]::BIGINT[]),
        (5013, ARRAY[5013,4378], ARRAY[60000,0]::BIGINT[]),
        (5997, ARRAY[5997,4105,4106,4107], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (6009, ARRAY[6009,4150,4151,4152], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (6018, ARRAY[6018,4176,4177,4178], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (6038, ARRAY[6038,4236,4237,4238], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (6045, ARRAY[6045,4339,4340,4341], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (6051, ARRAY[6051,4357,4358,4359], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (6057, ARRAY[6057,4382,4383,4384], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (6061, ARRAY[6061,4388,4389,4390], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (6062, ARRAY[6062,4391,4392], ARRAY[10000,300000,120000]::BIGINT[]),
        (6072, ARRAY[6072,5624,5625,5626], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (6073, ARRAY[6073,5627,5628,5629], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (6076, ARRAY[6076,5727,5728,5729], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (6301, ARRAY[6301,6302,6303,6304], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (6347, ARRAY[6347,6348,6349,6350], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (8113, ARRAY[8113,8114,8115,8116], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (8744, ARRAY[8744,8738,8742,8743], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (8826, ARRAY[8826], ARRAY[0]::BIGINT[]),
        (8909, ARRAY[8909,8910], ARRAY[10000,900000]::BIGINT[]),
        (8961, ARRAY[8961,8962,8963,8964], ARRAY[10000,300000,300000,600000]::BIGINT[]),
        (8965, ARRAY[8965,8966,8967], ARRAY[300000,300000,600000]::BIGINT[]),
        (9092, ARRAY[9092,9089,9090,9091], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (9695, ARRAY[9695], ARRAY[0]::BIGINT[]),
        (10393, ARRAY[10393,10394,10395,10396], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (10430, ARRAY[10430,10431,10432,10433], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (11360, ARRAY[11360], ARRAY[0]::BIGINT[]),
        (12325, ARRAY[12325,12326,12327], ARRAY[300000,300000,60000]::BIGINT[]),
        (12526, ARRAY[12526,12533,12534,12535], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (12658, ARRAY[12658,12825,12826,12827], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (12717, ARRAY[12717,12718,12719,12720], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (12823, ARRAY[12823,12660,12661,12662], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (12834, ARRAY[12834,12835,12836], ARRAY[300000,300000,60000]::BIGINT[]),
        (13801, ARRAY[13801,13802,13803,13804], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (13860, ARRAY[13860,13861,13862,13863], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (13979, ARRAY[13979,13980,13981,13982], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (13983, ARRAY[13983,13984,13985,13986], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (15865, ARRAY[15865,15866,15867], ARRAY[300000,300000,60000]::BIGINT[]),
        (15907, ARRAY[15907,15908,15909], ARRAY[300000,300000,60000]::BIGINT[]),
        (16186, ARRAY[16186,16187,16188], ARRAY[300000,300000,60000]::BIGINT[]),
        (16193, ARRAY[16193,16190,16191,16192], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (17421, ARRAY[17421,17422,17423,17424], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (17622, ARRAY[17622,17623,17624,17625], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (18225, ARRAY[18225,18222,18223,18224], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (18226, ARRAY[18226,18227,18228], ARRAY[300000,300000,60000]::BIGINT[]),
        (18230, ARRAY[18230,18231,18232], ARRAY[300000,300000,60000]::BIGINT[]),
        (18242, ARRAY[18242,18243,18244], ARRAY[300000,300000,60000]::BIGINT[]),
        (18246, ARRAY[18246,18247,18248], ARRAY[300000,300000,60000]::BIGINT[]),
        (18250, ARRAY[18250,18251,18252], ARRAY[300000,300000,60000]::BIGINT[]),
        (18261, ARRAY[18261,18258,18259,18260], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (18265, ARRAY[18265,18262,18263,18264], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (18273, ARRAY[18273,18270,18271,18272], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (18306, ARRAY[18306,18307,18308,18309], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (18937, ARRAY[18937,18938,18939], ARRAY[300000,300000,60000]::BIGINT[]),
        (18953, ARRAY[18953,18954,18955,18956], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (18961, ARRAY[18961,18958,18959,18960], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (18970, ARRAY[18970,18971,18972,18973], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (18974, ARRAY[18974,18975,18976,18977], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (18978, ARRAY[18978,18979,18980,18981], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (18982, ARRAY[18982,18983,18984,18985], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (19049, ARRAY[19049,19052,19053,19054], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (19059, ARRAY[19059,19060], ARRAY[300000,60000]::BIGINT[]),
        (20058, ARRAY[20058,20059,20060,20061], ARRAY[10000,600000,300000,60000]::BIGINT[]),
        (20144, ARRAY[20144,20145,20146,20147], ARRAY[10000,600000,300000,60000]::BIGINT[]),
        (21099, ARRAY[21099,21100,21101,21102], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (21446, ARRAY[21446,21447,21448,21449], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (21893, ARRAY[21893,21894,21895,21896], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (21977, ARRAY[21977,21979], ARRAY[10000,300000]::BIGINT[]),
        (22010, ARRAY[22010,22011,22012,22013], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (22023, ARRAY[22023,22024,22025], ARRAY[300000,300000,60000]::BIGINT[]),
        (22066, ARRAY[22066,22068,22069,22070], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (22067, ARRAY[22067,22071,22072,22073], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (22776, ARRAY[22776,22777,22778,22779], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (23553, ARRAY[23553,23554,23555,23556], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (23561, ARRAY[23561,23562,23563], ARRAY[300000,300000,60000]::BIGINT[]),
        (24256, ARRAY[24256], ARRAY[0]::BIGINT[]),
        (25071, ARRAY[25071,25072,25073], ARRAY[10000,600000,60000]::BIGINT[]),
        (25085, ARRAY[25085,25086], ARRAY[10000,600000]::BIGINT[]),
        (25827, ARRAY[25827,25826,25825,25824,25821,25820], ARRAY[10000,300000,300000,60000,300000,60000]::BIGINT[]),
        (25831, ARRAY[25831,25830,25829,25828], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (25866, ARRAY[25866,25865,25864,25863], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (27521, ARRAY[27521,27520,27519,27518], ARRAY[10000,300000,300000,300000]::BIGINT[]),
        (27536, ARRAY[27536,27535,27534], ARRAY[300000,300000,60000]::BIGINT[]),
        (27555, ARRAY[27555,27554,27553,27552], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (27633, ARRAY[27633,27632,27631,27630], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (27637, ARRAY[27637,27636,27635,27634], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (27641, ARRAY[27641,27640,27639,27638], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (27722, ARRAY[27722,27723,27724,27725], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (28617, ARRAY[28617,28616,28615], ARRAY[300000,300000,60000]::BIGINT[]),
        (28639, ARRAY[28639,28638,28637], ARRAY[300000,300000,60000]::BIGINT[]),
        (28664, ARRAY[28664,28665,28666], ARRAY[300000,300000,60000]::BIGINT[]),
        (28667, ARRAY[28667,28668,28669,28670], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (28786, ARRAY[28786,28785,28784,28783], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (28794, ARRAY[28794,28795,28796,28797], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (28802, ARRAY[28802,28803,28804,28805], ARRAY[10000,300000,300000,10000]::BIGINT[]),
        (30069, ARRAY[30069,30068,30067,30066], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (30099, ARRAY[30099,30098,30097,30096], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (30159, ARRAY[30159,30158,30157,30156], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (30272, ARRAY[30272], ARRAY[300000]::BIGINT[]),
        (31386, ARRAY[31386,31385,31384,31383], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (31409, ARRAY[31409,31408,31407,31406], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (31413, ARRAY[31413,31412,31411,31410], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (31551, ARRAY[31551,31550,31549,31548], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (32702, ARRAY[32702,32701,32700,32699], ARRAY[10000,300000,300000,50000]::BIGINT[]),
        (32733, ARRAY[32733,32732,32731,32730], ARRAY[10000,300000,100000,50000]::BIGINT[]),
        (32737, ARRAY[32737,32736,32735,32734], ARRAY[10000,300000,100000,50000]::BIGINT[]),
        (32741, ARRAY[32741,32740,32739,32738], ARRAY[10000,300000,100000,50000]::BIGINT[]),
        (32744, ARRAY[32744,32743,32742], ARRAY[300000,100000,50000]::BIGINT[]),
        (33821, ARRAY[33821,33820,33819,33818], ARRAY[10000,600000,600000,600000]::BIGINT[]),
        (33977, ARRAY[33977,33976,33975,33974], ARRAY[10000,600000,600000,600000]::BIGINT[]),
        (33981, ARRAY[33981,33980,33979,33978], ARRAY[10000,600000,600000,600000]::BIGINT[]),
        (34184, ARRAY[34184,34183,34182,34181], ARRAY[10000,900000,600000,600000]::BIGINT[]),
        (34185, ARRAY[34185,34186,34187,34188], ARRAY[10000,900000,600000,600000]::BIGINT[]),
        (34245, ARRAY[34245,34246,34247], ARRAY[900000,600000,600000]::BIGINT[]),
        (36612, ARRAY[36612,36611,36610,36609], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (36684, ARRAY[36684,36683,36682,36681], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (36688, ARRAY[36688,36687,36686,36685], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (36692, ARRAY[36692,36691,36690,36689,36688,36687,36686,36685], ARRAY[10000,300000,300000,60000,10000,300000,300000,60000]::BIGINT[]),
        (36843, ARRAY[36843,36842,36841,36840], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (36963, ARRAY[36963,36964,36965,36966], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (39212, ARRAY[39212,39211,39210,39209], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (39275, ARRAY[39275,39274,39273,39272], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (39279, ARRAY[39279,39278,39277,39276], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (39295, ARRAY[39295,39294,39293,39292], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (39307, ARRAY[39307,39306,39305,39304], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (39331, ARRAY[39331,39330,39329,39328], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (39712, ARRAY[39712], ARRAY[60000]::BIGINT[]),
        (42247, ARRAY[42247,42248,42249], ARRAY[60000,300000,300000]::BIGINT[]),
        (43754, ARRAY[43754,43753,43752,43751], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (44656, ARRAY[44656,44657,44658,44659], ARRAY[300000,300000,300000,300000]::BIGINT[]),
        (48362, ARRAY[48362,48361,48360], ARRAY[5000,5000,5000]::BIGINT[]),
        (48378, ARRAY[48378,48377,48376], ARRAY[5000,5000,5000]::BIGINT[]),
        (49990, ARRAY[49990,49989,49988,49987], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (50002, ARRAY[50002,50001,50000,49999], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (51546, ARRAY[51546,51547,51548], ARRAY[300000,300000,60000]::BIGINT[]),
        (51556, ARRAY[51556,51555,51554,51553], ARRAY[300000,300000,60000,60000]::BIGINT[]),
        (51572, ARRAY[51572,51571,51570,51569], ARRAY[300000,300000,60000,60000]::BIGINT[]),
        (51595, ARRAY[51595,51594,51593,51592], ARRAY[300000,300000,60000,60000]::BIGINT[]),
        (52841, ARRAY[52841,52840,52839,52838], ARRAY[300000,300000,300000,300000]::BIGINT[]),
        (53119, ARRAY[53119], ARRAY[600000]::BIGINT[]),
        (54647, ARRAY[54647,54648,54649,54650], ARRAY[10000,300000,300000,60000]::BIGINT[]),
        (54667, ARRAY[54667,54668,54669], ARRAY[10000,300000,60000]::BIGINT[]),
        (54670, ARRAY[54670,54671,54672,54673], ARRAY[10000,300000,300000,60000]::BIGINT[])
    ) AS policy(item, stages, lifetimes) LOOP
        item_key := 'oteryn:item.tibia.i' || source.item::text;
        IF source.stages[1] IS DISTINCT FROM source.item
            OR cardinality(source.stages) <> cardinality(source.lifetimes)
        THEN
            RAISE EXCEPTION 'Invalid source corpse policy for %', item_key;
        END IF;
        installed_chain := game_summer_corpse_chain(item_key);
        installed_lifetimes := game_source_corpse_durations_v1(item_key);
        IF installed_chain = source.stages AND installed_lifetimes = source.lifetimes THEN
            CONTINUE;
        END IF;
        IF installed_chain IS NOT NULL OR installed_lifetimes IS NOT NULL THEN
            RAISE EXCEPTION 'Conflicting corpse lifecycle policy for %', item_key;
        END IF;
        chain_cases := chain_cases || format(E'\n WHEN %L THEN %L::INTEGER[]',
                                             item_key, source.stages);
        lifetime_cases := lifetime_cases || format(E'\n WHEN p_key=%L THEN %L::BIGINT[]',
                                                   item_key, source.lifetimes);
    END LOOP;
    EXECUTE replace(chain_body,chain_anchor,chain_anchor||chain_cases);
    EXECUTE replace(lifetime_body,lifetime_anchor,lifetime_anchor||lifetime_cases);
    EXECUTE replace(plain_body,plain_anchor,$plain$ 'oteryn:item.tibia.i2127', 'oteryn:item.tibia.i24256', 'oteryn:item.tibia.i35686', 'oteryn:item.tibia.i36434'$plain$||', '||plain_anchor);
END $source_corpses$;

-- A custody-only mutation preserves the exact applied stage through the
-- installed immutable handoff receipt. The retirement mutation consumes one
-- further revision, so its deferred guard proves the preceding revision.
DO $corpse_decay_custody$
DECLARE
    f TEXT;
    body TEXT;
    original CONSTANT TEXT := 'i.state_revision=r.state_revision_before+1 AND i.last_transaction_id=r.transaction_id';
    retired CONSTANT TEXT := 'i.state_revision=r.state_revision_before+2';
    handoff CONSTANT TEXT := $proof$EXISTS(
        SELECT 1 FROM game_item_decay_handoffs h
         WHERE h.item_instance_id=r.item_instance_id
           AND h.source_stage=r.source_stage AND h.stage_transaction_id=r.transaction_id
           AND h.result_revision=i.state_revision AND h.custody_transaction_id=i.last_transaction_id)$proof$;
BEGIN
    FOREACH f IN ARRAY ARRAY['game_corpse_terminal_stage_ready(uuid)','game_item_decay_retire_consistency_guard()'] LOOP
        body := pg_get_functiondef(f::regprocedure);
        IF (length(body)-length(replace(body,original,'')))/length(original) <> 1 THEN
            RAISE EXCEPTION 'Corpse terminal stage predecessor changed in %', f;
        END IF;
        body := replace(body,original,'(('||original||') OR '||handoff||')');
        IF f = 'game_item_decay_retire_consistency_guard()' THEN
            IF (length(body)-length(replace(body,retired,'')))/length(retired) <> 1 THEN
                RAISE EXCEPTION 'Corpse retirement predecessor changed';
            END IF;
            body := replace(body,retired,'('||retired||' OR EXISTS(
                SELECT 1 FROM game_item_decay_handoffs h
                 WHERE h.item_instance_id=r.item_instance_id
                   AND h.source_stage=r.source_stage AND h.stage_transaction_id=r.transaction_id
                   AND h.result_revision=i.state_revision-1))');
        END IF;
        EXECUTE body;
    END LOOP;
END $corpse_decay_custody$;

-- These admitted definitions have both movable and pickupable source facts.
-- Inventory admission still checks the active native Item policy and the
-- exact move receipt. An occupied corpse container keeps its Ground owner.
CREATE FUNCTION game_ordinary_corpse_portable_key(p_key TEXT) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE STRICT AS $$
    SELECT p_key IN (
        'oteryn:item.tibia.i2983','oteryn:item.tibia.i3994','oteryn:item.tibia.i3995',
        'oteryn:item.tibia.i3996','oteryn:item.tibia.i3998','oteryn:item.tibia.i3999',
        'oteryn:item.tibia.i4000','oteryn:item.tibia.i4024','oteryn:item.tibia.i4116',
        'oteryn:item.tibia.i4117','oteryn:item.tibia.i4118','oteryn:item.tibia.i4156',
        'oteryn:item.tibia.i4157','oteryn:item.tibia.i4188','oteryn:item.tibia.i4189',
        'oteryn:item.tibia.i4190','oteryn:item.tibia.i5565',
        'oteryn:item.tibia.i5566','oteryn:item.tibia.i5567','oteryn:item.tibia.i10332',
        'oteryn:item.tibia.i10334','oteryn:item.tibia.i10335','oteryn:item.tibia.i10440',
        'oteryn:item.tibia.i10441','oteryn:item.tibia.i10442','oteryn:item.tibia.i22698',
        'oteryn:item.tibia.i23729','oteryn:item.tibia.i31203','oteryn:item.tibia.i31204',
        'oteryn:item.tibia.i31205','oteryn:item.tibia.i31206','oteryn:item.tibia.i50311')
$$;
REVOKE ALL ON FUNCTION game_ordinary_corpse_portable_key(TEXT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_ordinary_corpse_portable_key(TEXT) TO oteryn_game_runtime;

DO $portable_ordinary_corpse$
DECLARE
    body TEXT;
    original CONSTANT TEXT := 'm.loot_table_production_key=i.definition_production_key';
    keys CONSTANT TEXT := 'i.definition_production_key IN (';
    provenance CONSTANT TEXT := 'AND g.world_id=m.death_world_id';
    guard CONSTANT TEXT := $old$OR game_item_loop_drop_ref(ground_body.corpse_ref)))),false) THEN$old$;
    timed CONSTANT TEXT := 'OR p.map_source_transaction_id IS NOT NULL';
BEGIN
    body := pg_get_functiondef('game_item_plain_corpse_source(uuid)'::regprocedure);
    IF (length(body)-length(replace(body,original,'')))/length(original) <> 1
        OR (length(body)-length(replace(body,keys,'')))/length(keys) <> 1
        OR (length(body)-length(replace(body,provenance,'')))/length(provenance) <> 1
    THEN
        RAISE EXCEPTION 'Plain corpse provenance predecessor changed';
    END IF;
    body := replace(body,original,'('||original||' OR (
        game_ordinary_corpse_portable_key(i.definition_production_key)
        AND EXISTS (
            SELECT 1 FROM game_spell_field_temporal_schedules p
            JOIN game_spell_field_temporal_receipts r USING(item_instance_id,source_stage)
            WHERE p.item_instance_id=i.item_instance_id
              AND p.source_transaction_id=m.transaction_id AND p.source_ordinal=0
              AND (p.target_family,p.target_production_key,p.target_revision)=
                  (i.definition_family,i.definition_production_key,i.definition_revision_ref)
              AND ((r.transaction_id=i.last_transaction_id AND r.state_revision_before+1=i.state_revision)
                   OR EXISTS (
                      SELECT 1 FROM game_item_decay_handoffs h
                       WHERE h.item_instance_id=r.item_instance_id
                         AND h.source_stage=r.source_stage AND h.stage_transaction_id=r.transaction_id
                         AND h.result_revision=i.state_revision AND h.custody_transaction_id=i.last_transaction_id
                   ))
        )))');
    body := replace(body,keys,'(game_ordinary_corpse_portable_key(i.definition_production_key) OR '||keys);
    body := replace(body,provenance,') AND NOT EXISTS (
        SELECT 1 FROM game_item_corpse_container_entries e WHERE e.parent_item_instance_id=i.item_instance_id
    ) '||provenance);
    EXECUTE body;

    body := pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
    IF (length(body)-length(replace(body,guard,'')))/length(guard) <> 1
        OR (length(body)-length(replace(body,timed,'')))/length(timed) <> 1 THEN
        RAISE EXCEPTION 'Portable corpse inventory predecessor changed';
    END IF;
    body := replace(body,timed,timed||$timed$
        OR (NEW.quantity_before=1 AND NEW.quantity_after=1 AND (
            (NEW.from_kind=3 AND NEW.to_kind=3 AND game_item_loop_native_corpse_source(NEW.item_instance_id))
            OR (NEW.from_kind IN (1,2,3) AND NEW.to_kind IN (1,2,3)
                AND game_ordinary_corpse_portable_key(NEW.definition_production_key)
                AND p.source_ordinal=0 AND EXISTS (
                    SELECT 1 FROM game_item_mint_receipts m
                     WHERE m.item_instance_id=p.item_instance_id
                       AND m.transaction_id=p.source_transaction_id
                       AND m.loot_purpose_key='CORPSE_MATERIALIZATION'
                       AND m.materialized_at IS NOT NULL
                ))
        ))$timed$);
    EXECUTE replace(body,guard,$new$OR game_item_loop_drop_ref(ground_body.corpse_ref)))),false)
      AND NOT (NEW.to_kind IN (1,2) AND NEW.quantity_before=1 AND NEW.quantity_after=1
        AND NEW.definition_family='Item' AND NEW.definition_revision_ref='definition-r1'
        AND game_ordinary_corpse_portable_key(NEW.definition_production_key)
        AND EXISTS (
             SELECT 1 FROM game_item_ground_locations portable_ground
             JOIN game_item_instances portable_item USING(item_instance_id)
             WHERE portable_ground.item_instance_id=NEW.item_instance_id
               AND (game_item_plain_corpse_source(NEW.item_instance_id)
                    OR game_item_loop_drop_ref(portable_ground.corpse_ref))
               AND (portable_item.definition_family,portable_item.definition_production_key,portable_item.definition_revision_ref)=
                   (NEW.definition_family,NEW.definition_production_key,NEW.definition_revision_ref)
        )) THEN$new$);
END $portable_ordinary_corpse$;
