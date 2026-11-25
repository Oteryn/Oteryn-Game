use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use super::fixture::{Harness, debug, id};

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;

use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

pub(super) const ROOT: &str = "oteryn:quest-progress/questpc7/goblin/";

const QUEST: &str = "oteryn:quest.goblin_merchant_quest";

const TOOLS: [(u32, i64); 5] = [(17668, 3), (17669, 1), (17670, 1), (17671, 1), (17676, 1)];

fn conversation(name: &str, word: &str) -> TestResult<RewardCallback> {
    // Only supplies conversation words to the shared wire driver; it does not
    // add a transition to the runtime catalogue or write quest state.
    Ok(serde_json::from_value(serde_json::json!({
        "key":format!("oteryn:quest-reward/questpc7-goblin-{name}"),
        "quest":QUEST,"npc":"oteryn:npc.rafzan","keywords":[word],
        "topic":0,"next_topic":0,"storage":[],"take":[],"give":null,
        "outfit":null,"achievement":null,"reply":[],"sources":[]
    }))?)
}

async fn quantity(h: &Harness, item: u32) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key=$2")
        .bind(id(41).as_slice()).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}

pub(super) async fn receipts(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2")
        .bind(id(41).as_slice()).bind(QUEST).fetch_one(&h.pool).await?)
}

async fn money(h: &Harness) -> TestResult<i64> {
    Ok(
        quantity(h, 3031).await?
            + quantity(h, 3035).await? * 100
            + quantity(h, 3043).await? * 10000,
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    exercise_with_probes(owner,a,h,&[]).await.map(|_| ())
}

async fn exercise_with_probes(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    h: &Harness,
    probes: &[(Position,u16)],
) -> TestResult<EntityRef> {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC7_GOBLIN_INTRO POSTGRES_STARTED version={version}");
    let mut selected = a.continuity.selected_capabilities.as_slice().to_vec();
    selected.extend([3, 4, 6, 11, 12, 14, 15, 18]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    a.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    let [rafzan] = super::classic_quests_fixtures::install(owner, a, ["oteryn:npc.rafzan"]).await?;
    assert_eq!(receipts(h).await?, 0);
    let intro = conversation("intro", "swamp jobs")?;
    let (result, _, last) =
        super::qengine_rewards_fixtures::wire_reward(owner, a, rafzan, &intro, false).await?;
    assert_eq!(result, NpcIntentDisposition::Ok);
    for suffix in [
        "programme",
        "signs",
        "odours",
        "captures",
        "urine",
        "trunks",
        "stalkers",
    ] {
        assert_eq!(
            super::questpc7_ferumbras_fixtures::value(owner, a, &format!("{ROOT}{suffix}")).await?,
            0,
            "fresh accepted job {suffix}"
        );
    }
    assert_eq!(receipts(h).await?, 1, "one atomic seven-track start");
    for (item, _) in TOOLS {
        assert_eq!(
            quantity(h, item).await?,
            0,
            "jobs do not manufacture merchant stock"
        );
    }
    let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
        npc_actor: rafzan,
        text: "swamp jobs".into(),
    })
    .map_err(debug)?;
    let replay = super::quest_world_callbacks_fixtures::wire(owner, a, last, 7, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&replay).map_err(debug)?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(receipts(h).await?, 1);
    let kit = conversation("kit", "buy swamp kit")?;
    let (poor, _, _) = super::qengine_rewards_fixtures::wire_reward(owner, a, rafzan, &kit, false).await?;
    assert_eq!(poor, NpcIntentDisposition::Rejected);
    assert_eq!(receipts(h).await?, 1);
    for (item, _) in TOOLS {
        assert_eq!(quantity(h, item).await?, 0);
    }
    for &(position,appearance) in probes {
        assert_eq!(super::questpc7_ferumbras_fixtures::use_at(owner,a,position,appearance).await?,
            oteryn_protocol_oteryn::world_object::UseDisposition::Rejected);
        assert_eq!(receipts(h).await?,1,"accepted job without purchased tool cannot progress");
        for (item,_) in TOOLS { assert_eq!(quantity(h,item).await?,0); }
    }
    let fence = owner
        .current_quest_fence(a.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: 33293,
                y: 31677,
                floor: -15,
            },
            item: "oteryn:item.tibia.i3043".into(),
            count: 1,
        },
        33000,
        true,
    )
    .await?;
    assert_eq!(money(h).await?, 10000);
    let (paid, _, last) =
        super::qengine_rewards_fixtures::wire_reward(owner, a, rafzan, &kit, false).await?;
    assert_eq!(paid, NpcIntentDisposition::Ok);
    assert_eq!(
        money(h).await?,
        8875,
        "3x75+200+400+150+150 exact donor prices"
    );
    for (item, count) in TOOLS {
        assert_eq!(quantity(h, item).await?, count);
    }
    assert_eq!(receipts(h).await?, 2);
    let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
        npc_actor: rafzan,
        text: "buy swamp kit".into(),
    })
    .map_err(debug)?;
    let replay = super::quest_world_callbacks_fixtures::wire(owner, a, last, 7, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&replay).map_err(debug)?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(money(h).await?, 8875);
    for (item, count) in TOOLS {
        assert_eq!(quantity(h, item).await?, count);
    }
    assert_eq!(receipts(h).await?, 2);
    let unfinished: bool=sqlx::query_scalar("SELECT completed_character_revision IS NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(id(41).as_slice()).bind(QUEST).fetch_one(&h.pool).await?;
    assert!(
        unfinished,
        "introduction and shopping do not complete the six jobs"
    );
    Ok(rafzan)
}

// Exact sign sites from Task:Rafzan oldid401277; source B3 witnesses are
// independently checked by authoring tests before these fixture cells are used.
const SIGN_SITES: [Position; 3] = [
    Position { x: 33055, y: 32091, floor: -7 },
    Position { x: 33026, y: 32091, floor: -7 },
    Position { x: 33032, y: 32083, floor: -7 },
];

const SIGN_DECOY: Position = Position { x: 33055, y: 32089, floor: -7 };

pub(super) fn sign_objects() -> Vec<(Position, u16)> {
    // A fixture corridor, not an asserted source map layout. Target coordinates,
    // appearances and native Item bindings are exact independently audited facts.
    let mut objects = Vec::new();
    for x in 33025..=33056 { for y in 32082..=32093 {
        objects.push((Position { x, y, floor: -7 }, 100));
    }}
    objects
}

pub(super) fn sign_native_items() -> Vec<(Position, u32)> {
    SIGN_SITES.into_iter().chain([SIGN_DECOY]).map(|p| (p, 17679)).collect()
}

async fn use_site_tool(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    site: Position,
    appearance: u16,
    tool: u32,
) -> TestResult<(oteryn_protocol_oteryn::world_object::UseDisposition, Vec<u8>)> {
    use oteryn_protocol_oteryn::world_object::{
        MapUseTarget, UseIntent, UseTarget, decode_use_intent, encode_item_use_intent,
    };
    super::m44_source_lifecycle_fixtures::wait_for_world_action(owner, admitted).await?;
    let target = decode_use_intent(
        &super::questpc7_hoist_story_fixtures::target(owner, site, appearance)?,
    )
    .map_err(debug)?;
    let reference = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| {
            index.definition_ref("Item", &format!("oteryn:item.tibia.i{tool}"), "definition-r1")
        })
        .ok_or("site tool definition")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(target)),
        use_with_position: None,
    })
    .map_err(debug)?;
    let (next, disposition, actual) = super::rust_lottery_fixtures::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        Some(reference),
        None,
    )
    .await?;
    *admitted = next;
    Ok((disposition, actual))
}

pub(super) async fn exercise_signs(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::{UseDisposition, decode_use_result};
    use super::questpc7_ferumbras_fixtures::{use_at, value, walk};
    let version: String = sqlx::query_scalar("SHOW server_version").fetch_one(&h.pool).await?;
    eprintln!("QUESTPC7_GOBLIN_SIGNS POSTGRES_STARTED version={version}");
    assert_eq!(use_at(owner, admitted, SIGN_SITES[0], 17679).await?, UseDisposition::NothingToUse);
    assert_eq!(receipts(h).await?, 0);
    exercise(owner, admitted, h).await?;
    assert_eq!(use_at(owner, admitted, SIGN_DECOY, 17679).await?, UseDisposition::NothingToUse);
    assert_eq!(quantity(h, 17668).await?, 3, "same appearance elsewhere cannot consume a sign");
    assert_eq!(receipts(h).await?, 2);
    let approaches = [
        Position { x: 33054, y: 32090, floor: -7 },
        Position { x: 33027, y: 32090, floor: -7 },
        Position { x: 33031, y: 32083, floor: -7 },
    ];
    for (index, (site, approach)) in SIGN_SITES.into_iter().zip(approaches).enumerate() {
        walk(owner, admitted, approach).await?;
        let command = admitted.continuity.next_command_id;
        let (result, payload) = use_site_tool(owner, admitted, site, 17679, 17668).await?;
        assert_eq!(result, UseDisposition::Committed);
        assert_eq!(quantity(h, 17668).await?, 2 - i64::try_from(index)?);
        assert_eq!(value(owner, admitted, &format!("{ROOT}signs")).await?, i64::try_from(index + 1)?);
        assert_eq!(value(owner, admitted, &format!("{ROOT}sign-site-{}", index + 1)).await?, 1);
        let expected_receipts = 3 + i64::try_from(index)?;
        assert_eq!(receipts(h).await?, expected_receipts);
        let replay = super::quest_world_callbacks_fixtures::wire(owner, admitted, command, 2, payload).await?;
        assert_eq!(decode_use_result(&replay).map_err(debug)?, UseDisposition::Committed);
        assert_eq!(receipts(h).await?, expected_receipts, "exact command replay has no new effect");
        assert_eq!(use_at(owner, admitted, site, 17680).await?, UseDisposition::NothingToUse);
        assert_eq!(quantity(h, 17668).await?, 2 - i64::try_from(index)?);
        assert_eq!(receipts(h).await?, expected_receipts, "fresh duplicate visit cannot spend another sign");
    }
    assert_eq!(money(h).await?, 8875);
    assert_eq!(value(owner, admitted, &format!("{ROOT}programme")).await?, 0);
    for suffix in ["odours", "captures", "urine", "trunks", "stalkers"] {
        assert_eq!(value(owner, admitted, &format!("{ROOT}{suffix}")).await?, 0);
    }
    for (item, count) in TOOLS.into_iter().skip(1) { assert_eq!(quantity(h, item).await?, count); }
    let finished: bool = sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2")
        .bind(id(41).as_slice()).bind(QUEST).fetch_one(&h.pool).await?;
    assert!(!finished, "three signs do not complete the six-job programme");
    let map = owner.bundle_map().ok_or("sign map")?;
    let first = SIGN_SITES[0];
    let target = |appearance| -> TestResult<_> {
        let payload = super::questpc7_hoist_story_fixtures::target(owner, first, appearance)?;
        oteryn_protocol_oteryn::world_object::decode_use_intent(&payload).map_err(debug)
    };
    let placed = target(17680)?;
    assert_eq!(placed.expected_revision, 1);
    let due = map.facts.next_movement_decay_time().ok_or("shared sign reset")?;
    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    assert!(map.facts.expire_movement_items(due) >= 1);
    let restored = target(17679)?;
    assert_eq!(restored.placement, placed.placement);
    assert_eq!(restored.expected_revision, 2, "the shared site restores the cracked stone");
    Ok(())
}

const POND: Position = Position { x: 33134, y: 32093, floor: -10 };

pub(super) fn pond_objects() -> Vec<(Position, u16)> {
    // Test travel floor; only the exact native pond target claims a source placement.
    (33132..=33135).flat_map(|x| (32091..=32094).map(move |y| (Position { x,y,floor:-10 },100))).collect()
}

pub(super) async fn exercise_pond(owner: &mut ComposedFreshAdmission<'_, '_, '_>, a: &mut AdmittedSession, h: &Harness) -> TestResult {
    use super::questpc7_ferumbras_fixtures::{use_at,value};
    use oteryn_protocol_oteryn::world_object::{UseDisposition,decode_use_result};
    let version: String = sqlx::query_scalar("SHOW server_version").fetch_one(&h.pool).await?;
    eprintln!("QUESTPC7_GOBLIN_POND POSTGRES_STARTED version={version}");
    exercise(owner,a,h).await?;
    let command=a.continuity.next_command_id;
    let payload=super::questpc7_hoist_story_fixtures::target(owner,POND,9527)?;
    assert_eq!(use_at(owner,a,POND,9527).await?,UseDisposition::Committed);
    assert_eq!(quantity(h,17671).await?,0);
    assert_eq!(value(owner,a,&format!("{ROOT}urine")).await?,1);
    assert_eq!(receipts(h).await?,3);
    let replay=super::quest_world_callbacks_fixtures::wire(owner,a,command,2,payload).await?;
    assert_eq!(decode_use_result(&replay).map_err(debug)?,UseDisposition::Committed);
    assert_eq!(use_at(owner,a,POND,9527).await?,UseDisposition::Rejected);
    assert_eq!(receipts(h).await?,3);
    assert_eq!(quantity(h,17671).await?,0);
    assert_eq!(money(h).await?,8875);
    for suffix in ["programme","signs","odours","captures","trunks","stalkers"] {
        assert_eq!(value(owner,a,&format!("{ROOT}{suffix}")).await?,0);
    }
    for (item,count) in TOOLS.into_iter().filter(|(item,_)| *item!=17671) { assert_eq!(quantity(h,item).await?,count); }
    Ok(())
}

const TRUNKS: [Position; 3] = [Position{x:33101,y:32047,floor:-10}, Position{x:33143,y:32051,floor:-10}, Position{x:33136,y:32076,floor:-10}];

pub(super) fn trunk_objects() -> Vec<(Position,u16)> {
    // Synthetic travel floor; positive target cells and native identities are source-pinned.
    (33099..=33145).flat_map(|x|(32045..=32078).map(move|y|(Position{x,y,floor:-10},100))).collect()
}

pub(super) fn trunk_items() -> Vec<(Position,u32)> { TRUNKS.into_iter().map(|p|(p,17677)).collect() }

pub(super) async fn exercise_trunks(owner:&mut ComposedFreshAdmission<'_, '_, '_>,a:&mut AdmittedSession,h:&Harness)->TestResult {
    use super::questpc7_ferumbras_fixtures::{use_at,value,walk};
    use oteryn_protocol_oteryn::world_object::{UseDisposition,decode_use_result};
    let version:String=sqlx::query_scalar("SHOW server_version").fetch_one(&h.pool).await?;
    eprintln!("QUESTPC7_GOBLIN_TRUNKS POSTGRES_STARTED version={version}");
    exercise_with_probes(owner,a,h,&[(TRUNKS[0],17677)]).await?;
    let approaches=[Position{x:33100,y:32046,floor:-10},Position{x:33142,y:32050,floor:-10},Position{x:33135,y:32075,floor:-10}];
    for (index,(site,approach)) in TRUNKS.into_iter().zip(approaches).enumerate() {
        if index==1 { walk(owner,a,Position{x:33128,y:32048,floor:-10}).await?; }
        if index==2 { walk(owner,a,Position{x:33135,y:32060,floor:-10}).await?; }
        walk(owner,a,approach).await?;
        assert_eq!(use_at(owner,a,site,17677).await?,UseDisposition::Rejected,
            "bare use cannot borrow any hammer from the backpack");
        let command=a.continuity.next_command_id;
        let (result, payload) = use_site_tool(owner, a, site, 17677, 17676).await?;
        assert_eq!(result, UseDisposition::Committed);
        assert_eq!(quantity(h,17676).await?,i64::from(index < 2),"the third trunk breaks the hammer");
        assert_eq!(value(owner,a,&format!("{ROOT}trunks")).await?,i64::try_from(index+1)?);
        assert_eq!(value(owner,a,&format!("{ROOT}trunk-site-{}",index+1)).await?,1);
        let expected=3+i64::try_from(index)?;
        assert_eq!(receipts(h).await?,expected);
        let replay=super::quest_world_callbacks_fixtures::wire(owner,a,command,2,payload).await?;
        assert_eq!(decode_use_result(&replay).map_err(debug)?,UseDisposition::Committed);
        assert_eq!(use_at(owner,a,site,17678).await?,UseDisposition::NothingToUse);
        assert_eq!(receipts(h).await?,expected);
    }
    for (item,count) in TOOLS { assert_eq!(quantity(h,item).await?,if item == 17676 { 0 } else { count }); }
    assert_eq!(money(h).await?,8875);
    for suffix in ["programme","signs","odours","captures","urine","stalkers"] { assert_eq!(value(owner,a,&format!("{ROOT}{suffix}")).await?,0); }
    let finished:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(id(41).as_slice()).bind(QUEST).fetch_one(&h.pool).await?;
    assert!(!finished);
    Ok(())
}

pub(super) fn stalker_objects() -> Vec<(Position,u16)> {
    // Travel-only test terrain; the five positive spawn positions are source-pinned.
    (33010..=33077).flat_map(|x|(32106..=32162).map(move|y|(Position{x,y,floor:-7},100))).collect()
}

pub(super) async fn stalker_death(owner:&mut ComposedFreshAdmission<'_, '_, '_>,a:&mut AdmittedSession,key:&str,at:Position,command:u64)->TestResult {
    let target={
        let mut runtime=owner.runtime.lock().await;
        let policy=runtime.companion_policy(key).map_err(debug)?;
        let actor=runtime.admit_encounter_actor(key,policy.maximum_health,super::quest_world_callbacks_fixtures::local(at)).map_err(debug)?;
        runtime.install_creature_policy(actor,key).map_err(debug)?;actor
    };
    super::questpc7_native_deaths_fixtures::lethal_creature(owner,a,target,command,key).await?;
    let mut runtime=owner.runtime.lock().await;
    assert!(!runtime.corpse_projection_pinned(target),"settlement must release the actor before fixture retirement");
    runtime.remove_encounter_actor(target).map_err(debug)?;
    Ok(())
}

pub(super) async fn exercise_stalkers(owner:&mut ComposedFreshAdmission<'_, '_, '_>,a:&mut AdmittedSession,h:&Harness)->TestResult {
    use super::questpc7_ferumbras_fixtures::{value,walk};
    let version:String=sqlx::query_scalar("SHOW server_version").fetch_one(&h.pool).await?;
    eprintln!("QUESTPC7_GOBLIN_STALKERS POSTGRES_STARTED version={version}");
    let rafzan = exercise_with_probes(owner,a,h,&[]).await?;
    super::questpc7_native_deaths_fixtures::prepare_native_hunter(owner,a).await?;
    let creature="oteryn:creature.marsh_stalker";
    // Wrong region and wrong species produce ordinary kills without quest credit.
    stalker_death(owner,a,creature,Position{x:33075,y:32110,floor:-7},10000).await?;
    assert_eq!(value(owner,a,&format!("{ROOT}stalkers")).await?,0);
    stalker_death(owner,a,"oteryn:creature.rat",Position{x:33074,y:32108,floor:-7},10001).await?;
    assert_eq!(value(owner,a,&format!("{ROOT}stalkers")).await?,0);
    let sites=[(33057,32109),(33052,32119),(33043,32136),(33054,32149),(33026,32155)];
    for (index,(x,y)) in sites.into_iter().enumerate() {
        if index==4 {walk(owner,a,Position{x:33038,y:32150,floor:-7}).await?;}
        walk(owner,a,Position{x:x-1,y:y-1,floor:-7}).await?;
        stalker_death(owner,a,creature,Position{x,y,floor:-7},10002+u64::try_from(index)?).await?;
        assert_eq!(value(owner,a,&format!("{ROOT}stalkers")).await?,i64::try_from(index+1)?,"duplicate lethal notifications/drains credit once");
    }
    walk(owner,a,Position{x:33045,y:32159,floor:-7}).await?;
    stalker_death(owner,a,creature,Position{x:33046,y:32160,floor:-7},10007).await?;
    assert_eq!(value(owner,a,&format!("{ROOT}stalkers")).await?,5,"terminal objective is capped");
    assert_eq!(receipts(h).await?,7);
    for (item,count) in TOOLS {assert_eq!(quantity(h,item).await?,count);}
    for suffix in ["programme","signs","odours","captures","urine","trunks"] {assert_eq!(value(owner,a,&format!("{ROOT}{suffix}")).await?,0);}
    for (x, y) in [(33059, 32150), (33073, 32134), (33073, 32109)] {
        walk(owner, a, Position { x, y, floor: -7 }).await?;
    }
    // Arrange the earned-count boundary while keeping this task's combat and
    // payment on their real producers.
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
        h, owner, a, &[(format!("{ROOT}reports"), 499)], 79,
    ).await?;
    owner.refresh_quest_session(a.game_session_id, true).await;
    let report = conversation("report-kill", "kill")?;
    let platinum = quantity(h, 3035).await?;
    let (paid, _, _) = super::qengine_rewards_fixtures::wire_reward(owner, a, rafzan, &report, false).await?;
    assert_eq!(paid, NpcIntentDisposition::Ok);
    assert_eq!(quantity(h, 3035).await?, platinum + 1);
    assert_eq!(value(owner,a,&format!("{ROOT}reports")).await?, 500);
    let awards:i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/task_manager'")
        .fetch_one(&h.pool).await?;
    assert_eq!(awards, 1, "the five-hundredth report grants Task Manager");
    let get = conversation("accept-kill", "get kill")?;
    let _ = super::qengine_rewards_fixtures::wire_reward(owner, a, rafzan, &get, false).await?;
    assert_eq!(value(owner,a,&format!("{ROOT}stalkers")).await?, -1, "the 20-hour cooldown refuses a new start");
    assert_eq!(quantity(h,3035).await?,platinum + 1);
    let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
        .fetch_one(&h.pool).await?;
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
        h, owner, a, &[
            (format!("{ROOT}stalkers/last-report"), now - 72_000),
            (format!("{ROOT}captures"), 3),
        ], 78,
    ).await?;
    owner.refresh_quest_session(a.game_session_id, true).await;
    let (started, _, _) = super::qengine_rewards_fixtures::wire_reward(owner, a, rafzan, &get, false).await?;
    assert_eq!(started, NpcIntentDisposition::Ok);
    assert_eq!(value(owner,a,&format!("{ROOT}stalkers")).await?, 0, "the task reopens after 20 hours");
    assert_eq!(value(owner,a,&format!("{ROOT}reports")).await?, 500, "starting again cannot pay a report");
    let guards = conversation("report-guards", "guards")?;
    let (paid, _, _) = super::qengine_rewards_fixtures::wire_reward(owner, a, rafzan, &guards, false).await?;
    assert_eq!(paid, NpcIntentDisposition::Ok);
    assert_eq!(quantity(h, 3035).await?, platinum + 2, "guards reports the finished task even with the original offer unvisited");
    assert_eq!(value(owner, a, &format!("{ROOT}captures")).await?, -1);
    assert_eq!(value(owner, a, &format!("{ROOT}reports")).await?, 501);
    Ok(())
}

pub(super) async fn exercise_perfume_items(owner:&mut ComposedFreshAdmission<'_, '_, '_>,a:&mut AdmittedSession,h:&Harness)->TestResult {
    let version:String=sqlx::query_scalar("SHOW server_version").fetch_one(&h.pool).await?;
    eprintln!("QUESTPC7_GOBLIN_PERFUME_ITEMS POSTGRES_STARTED version={version}");
    exercise(owner,a,h).await?;
    let native=owner.active_generation.and_then(|g|g.native_gameplay()).ok_or("native")?;
    for (item,weight,stack) in [(17672,350,"NonStackable"),(17673,350,"NonStackable"),(17674,350,"NonStackable"),(17675,350,"NonStackable"),(17681,0,"NonStackable"),(3120,400,"StackCapable")] {
        let policy=native.item_policy_for_source_id(item).ok_or_else(||format!("missing source Item policy {item}"))?;
        let record=serde_json::to_value(policy.record())?;
        assert_eq!(record["production_definition"]["production_key"],format!("oteryn:item.tibia.i{item}"));
        assert_eq!(record["production_definition"]["revision_ref"],"definition-r1");
        assert_eq!(record["admission"]["materializable"],true);
        assert_eq!(record["admission"]["stack_class"],stack);
        assert_eq!(record["semantics"]["physical"]["value"]["weight"]["value"],weight);
        assert_eq!(record["admission"]["legal_destinations"],if item==17681 {serde_json::json!(["Ground"])}else{serde_json::json!(["CharacterInventory","Ground"])});
        assert_eq!(quantity(h,item).await?,0,"source admission alone grants no quest item");
    }
    assert_eq!(receipts(h).await?,2,"only the real introduction and paid kit commit");
    assert_eq!(money(h).await?,8875);
    Ok(())
}


pub(super) const PERFUME_GAS: Position = Position { x: 33067, y: 32121, floor: -7 };

pub(super) async fn exercise_perfume_finish(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::gameplay_transport::{connection::UseCommand, item_view::ItemKey};
    use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseDisposition, UseIntent, UseTarget, decode_use_intent};
    let [rafzan] = super::classic_quests_fixtures::install(owner, admitted, ["oteryn:npc.rafzan"]).await?;
    let mut sequence = admitted.continuity.next_command_id;
    for word in ["hi", "perfume", "yes"] {
        let result = super::qengine_rewards_fixtures::talk(owner, admitted, rafzan, sequence, word).await?;
        assert_eq!(result, NpcIntentDisposition::Ok, "{word}");
        sequence = sequence.checked_add(1).ok_or("NPC command overflow")?;
    }
    assert_eq!(quantity(h, 17670).await?, 1, "Rafzan supplies the first gatherer");
    let actor = admitted.runtime_actor.ok_or("perfume actor")?;
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "perfume fence")?.ok_or("perfume fence")?.item_fence();
    let handle = std::num::NonZeroU64::new(1).ok_or("fixture handle")?;
    for item in [17672_u32, 17674, 3120] {
        // The fresh-corpse and live Rotworm sample have independent mounted cases.
        super::quest_world_callbacks_fixtures::input_item(owner, fence, &Offering {
            position: PERFUME_GAS, item: format!("oteryn:item.tibia.i{item}"), count: 1,
        }, 890 + u64::from(item), true).await?;
    }
    let instance = |item| async move {
        let bytes: Vec<u8> = sqlx::query_scalar(
            "SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1 LIMIT 1"
        ).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?;
        Ok::<_, Box<dyn std::error::Error>>(ItemKey::Instance(bytes.try_into().map_err(|_| "perfume item")?))
    };
    let gas_target = decode_use_intent(&super::questpc7_hoist_story_fixtures::target(owner, PERFUME_GAS, 17681)?).map_err(debug)?;
    let gas = UseIntent {
        target: UseTarget::Item(handle),
        use_with: None, use_with_map_item: Some(MapUseTarget::Object(gas_target)), use_with_position: None,
    };
    let command = |id| UseCommand { game_session_id: fence.game_session_id, command_id: id, item_fence: Some(fence) };
    let refused = owner.tame_mount_impl(actor, command(77000), gas.clone(), Some(instance(17672).await?), None, true)
        .await.map_err(|()| "gas tool dispatch")?.ok_or("gas route")?;
    assert_eq!(refused.disposition, UseDisposition::Rejected);
    assert_eq!((quantity(h,17672).await?,quantity(h,17673).await?), (1,0));
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
        h, owner, admitted, &[(format!("{ROOT}odours"), 1)], 76,
    ).await?;
    owner.refresh_quest_session(admitted.game_session_id, true).await;
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "perfume fence")?.ok_or("perfume fence")?.item_fence();
    let command = |id| UseCommand { game_session_id: fence.game_session_id, command_id: id, item_fence: Some(fence) };
    super::m44_source_lifecycle_fixtures::wait_for_world_action(owner, admitted).await?;
    let outcome = owner.tame_mount_impl(actor, command(77001), gas, Some(instance(17672).await?), None, true)
        .await.map_err(|()| "gas tool dispatch")?.ok_or("gas route")?;
    assert_eq!(outcome.disposition, UseDisposition::Committed);
    assert_eq!((quantity(h,17672).await?,quantity(h,17673).await?), (0,1));
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
        h, owner, admitted,
        &[("oteryn:quest-progress/goblin_merchant/rotworm_sample".into(), 1)],
        77,
    ).await?;
    owner.refresh_quest_session(admitted.game_session_id, true).await;
    let cheese = UseIntent { target: UseTarget::Item(handle), use_with:None,
        use_with_map_item:Some(MapUseTarget::Item(std::num::NonZeroU64::new(2).ok_or("cheese handle")?)),
        use_with_position:None };
    let outcome = owner.use_cooking_conversion(actor, command(77002), cheese, Some(instance(17674).await?), Some(instance(3120).await?)).await.ok_or("cheese route")?;
    assert_eq!(outcome.disposition, UseDisposition::Committed);
    assert_eq!((quantity(h,17674).await?,quantity(h,3120).await?,quantity(h,17675).await?), (0,0,1));
    assert_eq!(super::questpc7_outfit_items_fixtures::talk(owner, admitted, rafzan, &mut sequence, "perfume").await?, NpcIntentDisposition::Ok);
    assert_eq!((quantity(h,17675).await?,quantity(h,3035).await?), (0,2));
    assert_eq!(super::questpc7_ferumbras_fixtures::value(owner,admitted,&format!("{ROOT}reports")).await?,1);
    Ok(())
}
