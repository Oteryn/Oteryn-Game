//! L2 Regalia, alliance supplies and tome progression enter through native commands.
use super::*;

fn probe(case: &'static str, required: &[&str]) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::fixture::run(async |admin| {
        let started = std::time::Instant::now();
        let h = Harness::create_quest_reward(admin, "l2_knowledge_binding", true).await?;
        let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("POSTGRES_CASE_STARTED {case}-binding server_version={version}");
        let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
        let present = required
            .iter()
            .all(|key| callbacks.iter().any(|c| c.key.ends_with(key)));
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_BINDING_PROBE {case} duration={:?}",
            started.elapsed()
        );
        assert!(
            present,
            "complete native journey bindings absent: {required:?}"
        );
        Ok(())
    })?;
    scenario(case)
}

#[test]
fn questl2_knowledge_batch_kilmaresh_regalia_full_journey() -> TestResult {
    probe("qengine-rewards-l2-regalia", &["/l2-regalia-combine"])
}
#[test]
fn questl2_knowledge_batch_uneasy_alliance_full_journey() -> TestResult {
    probe(
        "qengine-rewards-l2-alliance",
        &["/l2-alliance-start", "/l2-alliance-finish"],
    )
}
#[test]
fn questl2_knowledge_batch_tomes_full_journey() -> TestResult {
    probe(
        "qengine-rewards-l2-tomes",
        &["/l2-cael-tome-1", "/l2-cael-tome-12"],
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    if name.ends_with("alliance") {
        return alliance(owner, admitted, h).await;
    }
    if name.ends_with("regalia") {
        return regalia(owner, admitted, h).await;
    }
    Err(format!("full native player journey has not been composed yet: {name}").into())
}

async fn regalia(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use super::questl2_journeys::quantity;
    let callback = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .find(|c| c.key.ends_with("/l2-regalia-combine"))
        .ok_or("Yonan source exchange")?;
    assert!(callback.is_paid_exchange());
    assert!(callback.storage.is_empty());
    let [npc] =
        super::super::classic_quests::install_nearby(owner, admitted, ["oteryn:npc.yonan"]).await?;
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("Regalia capabilities")?;
    assert_eq!(
        wire_reward(owner, admitted, npc, &callback, false).await?.0,
        NpcIntentDisposition::Rejected
    );
    let position = Position {
        x: 33293,
        y: 31677,
        floor: -15,
    };
    for (n, item) in [31572, 31573, 31574].into_iter().enumerate() {
        supply(owner, admitted, item, 16000 + u64::try_from(n)?, position).await?;
    }
    assert_eq!(
        wire_reward(owner, admitted, npc, &callback, false).await?.0,
        NpcIntentDisposition::Rejected
    );
    for item in 31572..=31574 {
        assert_eq!(
            quantity(h, item).await?,
            1,
            "missing fourth piece preserves all inputs"
        );
    }
    assert_eq!(quantity(h, 31576).await?, 0);
    supply(owner, admitted, 31575, 16003, position).await?;
    let (disposition, _, command) = wire_reward(owner, admitted, npc, &callback, false).await?;
    assert_eq!(disposition, NpcIntentDisposition::Ok);
    for item in 31572..=31575 {
        assert_eq!(quantity(h, item).await?, 0);
    }
    assert_eq!(quantity(h, 31576).await?, 1);
    let before = counts(h).await?;
    let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
        npc_actor: npc,
        text: "yes".into(),
    })
    .map_err(debug)?;
    let replay =
        super::super::quest_world_callbacks::wire(owner, admitted, command, 7, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&replay).map_err(debug)?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        counts(h).await?,
        before,
        "literal replay cannot burn or mint again"
    );
    for (n, item) in [31572, 31573, 31574, 31575].into_iter().enumerate() {
        supply(owner, admitted, item, 16010 + u64::try_from(n)?, position).await?;
    }
    assert_eq!(
        wire_reward(owner, admitted, npc, &callback, false).await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        quantity(h, 31576).await?,
        2,
        "a new paid occurrence is allowed without artificial progress"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_quest_tracks")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    Ok(())
}

async fn supply(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: u32,
    n: u64,
    position: Position,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "supply fence")?
        .ok_or("supply fence")?;
    super::super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position,
            item: format!("oteryn:item.tibia.i{item}"),
            count: 1,
        },
        n,
        true,
    )
    .await
}

async fn filled_bucket(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    subtype: u8,
    n: u64,
) -> TestResult {
    use crate::durability::{
        item_mint::TypedDefinitionRef, reward_claim_mint::RewardClaimMintRequest,
    };
    use crate::gameplay_transport::item_ref_admission::generation_item_facts;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fluid fixture fence")?
        .ok_or("fluid fixture fence")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native fluid")?;
    let definition = |id| TypedDefinitionRef {
        family: "Item".into(),
        production_key: format!("oteryn:item.tibia.i{id}"),
        revision_ref: "definition-r1".into(),
    };
    let request = RewardClaimMintRequest {
        backpack_outputs: false,
        command: CommandRef::new(admitted.game_session_id, CommandId::new(n).map_err(debug)?),
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: format!("fixture:l2-alliance-fluid.{n}"),
            revision_ref: "definition-r1".into(),
        },
        source_placement: "fixture:l2-alliance-ordinary-provisions".into(),
        item: generation_item_facts(native, &definition(2873)).map_err(debug)?,
        quantity: 1,
        contents: Vec::new(),
        root_text: None,
        root_fluid_subtype: Some(subtype),
        root_charges_remaining: None,
        backpack: generation_item_facts(native, &definition(2854)).map_err(debug)?,
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
        achievement: None,
        quest_transition: None,
        quest_requirements: Vec::new(),
        key_number: None,
    };
    let mut candidate = owner
        .root
        .freeze_reward_claim_mint(owner.character, owner.holder, fence.item_fence(), request)
        .await
        .map_err(debug)?;
    owner
        .root
        .commit_reward_claim_mint(
            owner.character,
            owner.holder,
            fence.item_fence(),
            &mut candidate,
        )
        .await
        .map_err(debug)?;
    Ok(())
}

async fn alliance(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use super::questl2_journeys::{quantity, use_at};
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let find = |name: &str| {
        callbacks
            .iter()
            .find(|c| c.key.ends_with(name))
            .ok_or("Alliance callback")
    };
    let start = find("/l2-alliance-start")?;
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6, 17, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("Alliance capabilities")?;
    let [npc] =
        super::super::classic_quests::install_nearby(owner, admitted, ["oteryn:npc.curos"]).await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_quest_tracks")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    // Only another quest's source prerequisite is arranged; Alliance starts untouched.
    let canonical = Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?);
    let mut prerequisite = start.clone();
    prerequisite.quest = "oteryn:quest.the_new_frontier_quest".into();
    prerequisite.storage = vec![oteryn_gameplay_quests::quest::world_callbacks::Storage {
        track: "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission06".into(),
        minimum: 5,
        maximum: 5,
        set: 5,
        set_if: None,
        preserve: false,
        read_only: false,
    }];
    seed(owner, admitted, &prerequisite, 17000).await?;
    owner.quest_catalogue = Some(canonical);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let position = Position {
        x: 33153,
        y: 31297,
        floor: -3,
    };
    assert_eq!(
        wire_reward(owner, admitted, npc, start, false).await?.0,
        NpcIntentDisposition::Rejected
    );
    supply(owner, admitted, 10219, 17001, position).await?;
    supply(owner, admitted, 2948, 17002, position).await?;
    filled_bucket(owner, admitted, 1, 17003).await?;
    assert_eq!(
        wire_reward(owner, admitted, npc, start, false).await?.0,
        NpcIntentDisposition::Rejected,
        "water cannot pay the milk requirement"
    );
    assert_eq!(quantity(h, 10219).await?, 1);
    assert_eq!(quantity(h, 2948).await?, 1);
    assert_eq!(quest_stage(h, "questline").await?, 0);
    filled_bucket(owner, admitted, 6, 17004).await?;
    assert_eq!(
        wire_reward(owner, admitted, npc, start, false).await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(quest_stage(h, "questline").await?, 1);
    assert_eq!(quest_stage(h, "quest_door").await?, 0);
    assert_eq!(quantity(h, 10219).await?, 0);
    assert_eq!(quantity(h, 2948).await?, 0);
    let active_fluids:Vec<i16>=sqlx::query_scalar("SELECT coalesce(f.fluid_subtype,0::smallint) FROM game_item_instances i LEFT JOIN game_item_fluid_states f USING(item_instance_id) WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i2873' ORDER BY 1").fetch_all(&h.pool).await?;
    assert_eq!(
        active_fluids,
        vec![0, 1],
        "milk is spent, water remains and Curos returns an empty bucket"
    );
    let crystal = Position {
        x: 33154,
        y: 31298,
        floor: -3,
    };
    use_at(owner, admitted, crystal, UseDisposition::Rejected).await?;
    kill_rebel(owner, admitted).await?;
    assert_eq!(
        quest_stage(h, "quest_door").await?,
        1,
        "native damaging-player death advances the source door track"
    );
    assert_eq!(
        wire_reward(
            owner,
            admitted,
            npc,
            find("/l2-alliance-rebel-report")?,
            false
        )
        .await?
        .0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        wire_reward(
            owner,
            admitted,
            npc,
            find("/l2-alliance-spy-mission")?,
            false
        )
        .await?
        .0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(quest_stage(h, "questline").await?, 3);
    use_at(owner, admitted, crystal, UseDisposition::Committed).await?;
    assert_eq!(quest_stage(h, "questline").await?, 4);
    let map = owner.bundle_map().ok_or("crystal map")?;
    let crystal_key =
        oteryn_world_bundle::bundle::placement_key(crystal.floor, crystal.x, crystal.y, 1)
            .ok_or("crystal placement")?;
    assert_eq!(
        map.facts
            .quest_callback_entry(map.overlay.base(), crystal_key)
            .ok_or("broken crystal")?
            .1
            .appearance_id,
        10187
    );
    let due = map
        .facts
        .next_movement_decay_time()
        .ok_or("native ten-second restoration")?;
    assert_eq!(
        map.facts.expire_movement_items(due - 1),
        0,
        "the crystal does not restore early"
    );
    let _ = map;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(11))
        .ok_or("crystal clock")?;
    let map = owner.bundle_map().ok_or("crystal map")?;
    assert_eq!(map.facts.expire_movement_items(owner.owner_now().get()), 1);
    assert_eq!(
        map.facts
            .quest_callback_entry(map.overlay.base(), crystal_key)
            .ok_or("restored crystal")?
            .1
            .appearance_id,
        10186
    );
    assert_eq!(
        quest_stage(h, "questline").await?,
        4,
        "restoration never resets mission progress"
    );

    use_at(
        owner,
        admitted,
        Position {
            x: 33154,
            y: 31297,
            floor: -3,
        },
        UseDisposition::Committed,
    )
    .await?;
    assert_eq!(quantity(h, 10217).await?, 1);
    assert_eq!(
        wire_reward(owner, admitted, npc, find("/l2-alliance-finish")?, false)
            .await?
            .0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(quest_stage(h, "questline").await?, 5);
    assert_eq!(quantity(h, 10217).await?, 2);
    use_at(
        owner,
        admitted,
        Position {
            x: 33154,
            y: 31297,
            floor: -3,
        },
        UseDisposition::Rejected,
    )
    .await?;
    assert_eq!(
        wire_reward(owner, admitted, npc, find("/l2-alliance-finish")?, false)
            .await?
            .0,
        NpcIntentDisposition::Rejected
    );
    assert_eq!(quantity(h, 10217).await?, 2);
    Ok(())
}

async fn quest_stage(h: &Harness, name: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce((SELECT value::bigint FROM game_character_quest_tracks WHERE track_key=$1),0)").bind(format!("oteryn:quest-progress/quest/u8_54/an_uneasy_alliance/{name}")).fetch_one(&h.pool).await?)
}

async fn kill_rebel(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    use oteryn_protocol_oteryn::attack::{
        AttackIntentDisposition, decode_attack_intent_result, encode_attack_target_intent,
    };
    let target = {
        let mut runtime = owner.runtime.lock().await;
        let before: Vec<_> = runtime
            .visible_entities()
            .creatures
            .iter()
            .map(|c| c.actor)
            .collect();
        let pin = runtime.content_pin().clone();
        runtime
            .realize_native_qualification_spawn(
                pin,
                "oteryn:creature.renegade_orc",
                crate::foundation::MovementLocalPosition {
                    x: 33153,
                    y: 31298,
                    floor: 3,
                },
            )
            .map_err(debug)?;
        let creature = runtime
            .visible_entities()
            .creatures
            .into_iter()
            .find(|c| !before.contains(&c.actor))
            .ok_or("native rebel")?;
        // Shorten unrelated combat; the native attack, captured damage and death settlement remain real.
        runtime
            .set_test_creature_health(creature.actor, 1)
            .map_err(debug)?;
        EntityRef {
            identity: creature.actor.placement_identity(),
            generation: creature.generation,
        }
    };
    let command = admitted.continuity.next_command_id;
    let result = super::super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        11,
        encode_attack_target_intent(Some(&target)).map_err(debug)?,
    )
    .await?;
    assert_eq!(
        decode_attack_intent_result(&result).map_err(debug)?,
        AttackIntentDisposition::Ok
    );
    for _ in 0..64 {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(3))
            .ok_or("combat clock")?;
        owner
            .refresh_attack_weapon(
                admitted.runtime_actor.ok_or("actor")?,
                admitted.game_session_id,
            )
            .await;
        owner.drain_auto_attacks().await;
        if owner
            .states
            .attack
            .lock()
            .await
            .kills()
            .has_session(admitted.game_session_id)
        {
            break;
        }
    }
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        crate::gameplay_transport::kill_reward::DrainOutcome::Drained
    );
    Ok(())
}
