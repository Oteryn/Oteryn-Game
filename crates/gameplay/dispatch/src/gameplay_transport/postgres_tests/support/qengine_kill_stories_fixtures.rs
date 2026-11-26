use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use super::fixture::{Harness, debug};

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use crate::durability::character_authority::ReconciledCharacterAuthority;

async fn count(h: &Harness, track: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),0)",
    )
    .bind(track)
    .fetch_one(&h.pool)
    .await?)
}

pub(super) async fn death(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    creature: &str,
    command: u64,
) -> TestResult {
    let target = queue_death(owner, admitted, creature, command).await?;
    drain_deaths(owner, admitted, &[target]).await
}

pub(super) async fn death_actor(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    creature: &str,
    command: u64,
) -> TestResult<crate::foundation::ExactActorRef> {
    let target = queue_death(owner, admitted, creature, command).await?;
    // These callers retain the original actor for subsequent source-death assertions.
    drain_deaths(owner, admitted, &[]).await?;
    Ok(target)
}

pub(super) async fn queue_death(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    creature: &str,
    command: u64,
) -> TestResult<crate::foundation::ExactActorRef> {
    queue_death_with_departure(owner, admitted, creature, command, None).await
}

pub(super) async fn queue_death_with_departure(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    creature: &str,
    command: u64,
    departed_top_damager: Option<
        crate::durability::character_progression::CurrentCharacterGameplayFence,
    >,
) -> TestResult<crate::foundation::ExactActorRef> {
    use crate::foundation::{CommandId, CommandRef, MovementLocalPosition};
    use crate::gameplay_transport::kill_reward::KillSink;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    owner.refresh_quest_session(session, false).await;
    assert!(owner.refresh_source_party_presence(actor, session).await);
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let queue = owner.states.attack.lock().await.kills().clone();
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    let target = {
        let mut runtime = owner.runtime.lock().await;
        let at = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        let policy = runtime.companion_policy(creature).map_err(debug)?;
        let target = runtime
            .admit_encounter_actor(
                creature,
                policy.maximum_health,
                MovementLocalPosition { x: at.x + 1, ..at },
            )
            .map_err(debug)?;
        runtime
            .install_creature_policy(target, creature)
            .map_err(debug)?;
        let lease = runtime
            .bound_attacker_lease(actor, session)
            .map_err(debug)?;
        if let Some(fence) = departed_top_damager {
            use crate::foundation::{CharacterId, CharacterLease};
            let departed = fence.game_session_id;
            let character = CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?;
            let reservation = runtime.reserve_fresh_session(departed).map_err(debug)?;
            let attacker = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(
                    attacker,
                    MovementLocalPosition { x: at.x - 1, ..at },
                )
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    attacker,
                    departed,
                    CharacterLease::new(character, fence.character_lease_generation)
                        .map_err(debug)?,
                )
                .map_err(debug)?;
            let health = runtime.companion_snapshot(target).map_err(debug)?.health;
            runtime
                .borrow_exact_actor_commit_at(now)
                .commit_damage_for_bound_attacker(
                    target,
                    attacker,
                    CommandRef::new(departed, CommandId::new(command).map_err(debug)?),
                    0,
                    crate::foundation::OwnerDamageCommand {
                        target: creature.as_bytes(),
                        occurrence: departed.as_bytes(),
                        binding: b"m25-departed-top-damager",
                        damage: health - 1,
                    },
                )
                .map_err(debug)?;
            runtime.remove_test_actor(attacker).map_err(debug)?;
            assert!(
                runtime
                    .borrow_combat_death()
                    .top_damage_contributor(target)
                    .map_err(debug)?
                    .is_gone()
            );
        }
        let health = runtime.companion_snapshot(target).map_err(debug)?.health;
        runtime
            .borrow_exact_actor_commit_at(now)
            .commit_damage_for_bound_attacker(
                target,
                actor,
                CommandRef::new(session, CommandId::new(command).map_err(debug)?),
                0,
                crate::foundation::OwnerDamageCommand {
                    target: creature.as_bytes(),
                    occurrence: session.as_bytes(),
                    binding: b"qengine-native-trial-death",
                    damage: health,
                },
            )
            .map_err(debug)?;
        for _ in 0..2 {
            crate::gameplay_transport::kill_reward::record_creature_kill(
                &mut runtime,
                KillSink {
                    prey: None,
                    activity: &[],
                    queue: &queue,
                    rewards: native.reward_table(),
                    charms: None,
                },
                target,
                crate::combat::CapturedRewardPrincipal {
                    character: lease.character_id(),
                    lease_generation: lease.generation(),
                    session,
                    actor,
                },
                now,
                Some(observed),
            );
        }
        target
    };
    Ok(target)
}

pub(super) async fn drain_deaths(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    targets: &[crate::foundation::ExactActorRef],
) -> TestResult {
    use crate::gameplay_transport::kill_reward::DrainOutcome;
    let session = admitted.game_session_id;
    assert_eq!(
        owner.drain_kill_rewards(session).await,
        DrainOutcome::Drained
    );
    assert_eq!(
        owner.drain_kill_rewards(session).await,
        DrainOutcome::Drained
    );
    // Both duplicate projections and drains have settled before reclaiming the fixture slot.
    for target in targets {
        owner
            .runtime
            .lock()
            .await
            .remove_encounter_actor(*target)
            .map_err(debug)?;
    }
    Ok(())
}

async fn claim(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    callback: &oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback,
) -> TestResult {
    use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
    let mut quantities = vec![];
    for item in &callback.take {
        let id = item
            .item
            .strip_prefix("oteryn:item.tibia.i")
            .ok_or("item")?
            .parse::<u16>()?;
        quantities.push((
            id,
            item.count,
            super::qengine_voucher_stories_fixtures::quantity(h, id).await?,
        ));
    }
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_sources")
        .fetch_one(&h.pool)
        .await?;
    let (result, _, command) =
        super::qengine_rewards_fixtures::wire_reward(owner, admitted, npc, callback, true).await?;
    assert_eq!(result, NpcIntentDisposition::Ok);
    let replay = super::quest_world_callbacks_fixtures::wire(
        owner,
        admitted,
        command,
        7,
        oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
            npc_actor: npc,
            text: callback.keywords[0].clone(),
        })
        .map_err(debug)?,
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&replay).map_err(debug)?,
        NpcIntentDisposition::Ok
    );
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_sources")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        after,
        before + 1,
        "literal replay cannot create a second reward source"
    );
    for (id, cost, before) in quantities {
        assert_eq!(
            super::qengine_voucher_stories_fixtures::quantity(h, id).await?,
            before - i64::from(cost),
            "exact source recipe, retained surplus, replay cannot charge twice"
        );
    }
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    name: &str,
) -> TestResult {
    use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
    let label = name
        .strip_prefix("qengine-rewards-l12-")
        .and_then(|s| s.strip_suffix("-story"))
        .ok_or("trial")?;
    if label == "demon-wars" {
        return demon_wars_trial(owner, admitted, h).await;
    }
    let (family, npc, level, creatures): (&str, &str, u32, &[(&str, &str, i64)]) = match label {
        "dragon-slayer" => (
            "dragon_slayer",
            "oteryn:npc.a_sweaty_cyclops",
            50,
            &[
                ("dragons", "oteryn:creature.dragon", 20),
                ("dragon_lords", "oteryn:creature.dragon_lord", 20),
            ],
        ),
        "cave-explorer" => (
            "cave_explorer",
            "oteryn:npc.gnommander",
            80,
            &[("kills", "oteryn:creature.crystal_spider", 20)],
        ),
        "fire-fighter" => (
            "fire_fighter",
            "oteryn:npc.gnomadness",
            250,
            &[("kills", "oteryn:creature.lava_lurker", 10)],
        ),
        _ => return Err("trial".into()),
    };
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let find = |stage: &str| {
        callbacks
            .iter()
            .find(|c| c.key == format!("oteryn:quest-reward/l12-{label}-{stage}"))
            .ok_or("trial binding")
    };
    let start = find("start")?;
    let base = find("base")?;
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 4, 6, 11, 15, 18, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    owner
        .states
        .chat
        .admit(admitted.game_session_id, "Trial witness", owner.owner_now());
    owner.admit_premium(super::fixture::id(40), admitted.game_session_id);
    let [npc] = super::classic_quests_fixtures::install(owner, admitted, [npc]).await?;
    super::qengine_outfit_stories_fixtures::earn_level(owner, admitted, authority, level).await?;
    owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native policies")?
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    let root = format!("oteryn:quest-progress/chosen-trial/{family}/");
    death(owner, admitted, creatures[0].1, 20000).await?;
    assert_eq!(
        count(h, &format!("{root}{}", creatures[0].0)).await?,
        0,
        "death before acceptance never counts"
    );
    let premium = owner.premium_coordinator.take();
    assert_eq!(
        super::qengine_rewards_fixtures::wire_reward(owner, admitted, npc, start, false)
            .await?
            .0,
        NpcIntentDisposition::Rejected
    );
    owner.premium_coordinator = premium;
    assert_eq!(
        super::qengine_rewards_fixtures::wire_reward(owner, admitted, npc, start, true)
            .await?
            .0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(count(h, &format!("{root}phase")).await?, 1);
    assert_eq!(
        super::qengine_rewards_fixtures::wire_reward(owner, admitted, npc, base, false)
            .await?
            .0,
        NpcIntentDisposition::Rejected,
        "early report cannot grant"
    );
    let mut command = 20001;
    for (track, creature, goal) in creatures {
        for expected in 1..=*goal {
            death(owner, admitted, creature, command).await?;
            command += 1;
            assert_eq!(
                count(h, &format!("{root}{track}")).await?,
                expected,
                "duplicate lethal projection and drain credit once"
            );
        }
        death(owner, admitted, creature, command).await?;
        command += 1;
        assert_eq!(
            count(h, &format!("{root}{track}")).await?,
            *goal,
            "bounded counter saturation"
        );
    }
    let mut ordinal = 30000;
    let mut bags = vec![];
    for item in &base.take {
        super::qengine_outfit_stories_fixtures::supply(owner, admitted, h, item, &mut bags, &mut ordinal)
            .await?;
    }
    if label == "cave-explorer" {
        super::qengine_voucher_stories_fixtures::mint(owner, admitted, 3043, 1, &mut ordinal).await?;
    }
    claim(owner, admitted, h, npc, base).await?;
    assert_eq!(count(h, &format!("{root}phase")).await?, 2);
    let outfit = format!("oteryn:outfit.{family}");
    super::qengine_voucher_stories_fixtures::mask(owner, admitted, &outfit, 0).await?;
    assert_eq!(
        super::qengine_rewards_fixtures::wire_reward(owner, admitted, npc, base, false)
            .await?
            .0,
        NpcIntentDisposition::Rejected
    );
    if label == "fire-fighter" {
        for (index, item) in [39545, 39544].into_iter().enumerate() {
            super::qengine_voucher_stories_fixtures::mint(owner, admitted, item, 1, &mut ordinal).await?;
            super::questpc7_outfit_items_fixtures::use_token(owner, admitted, u32::from(item)).await?;
            super::qengine_voucher_stories_fixtures::mask(
                owner, admitted, &outfit, if index == 0 { 2 } else { 3 },
            ).await?;
            let awards:i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/friendly_fire'")
                .fetch_one(&h.pool).await?;
            assert_eq!(awards, i64::try_from(index)?, "both token USE receipts earn Friendly Fire");
        }
    } else if label == "cave-explorer" {
        for bit in 1..=2 {
            let callback = find(if bit == 1 { "first" } else { "second" })?;
            for item in &callback.take {
                super::qengine_outfit_stories_fixtures::supply(
                    owner,
                    admitted,
                    h,
                    item,
                    &mut bags,
                    &mut ordinal,
                )
                .await?;
            }
            claim(owner, admitted, h, npc, callback).await?;
            super::qengine_voucher_stories_fixtures::mask(
                owner,
                admitted,
                &outfit,
                if bit == 1 { 1 } else { 3 },
            )
            .await?;
            if label == "cave-explorer" {
                let grants: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM game_account_achievements \
                     WHERE achievement_key = 'oteryn:achievement/cave_completionist'",
                )
                .fetch_one(&h.pool)
                .await?;
                assert_eq!(grants, i64::from(bit == 2), "only the full outfit grants the achievement");
            }
        }
    }
    Ok(())
}

pub(super) async fn queued_deaths(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    creature: &str,
    commands: &[u64],
) -> TestResult<Vec<crate::foundation::ExactActorRef>> {
    use crate::gameplay_transport::kill_reward::DrainOutcome;
    let targets = capture_deaths(owner, admitted, creature, commands).await?;
    for _ in 0..2 {
        assert_eq!(
            owner.drain_kill_rewards(admitted.game_session_id).await,
            DrainOutcome::Drained
        );
    }
    Ok(targets)
}

pub(super) async fn capture_deaths(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    creature: &str,
    commands: &[u64],
) -> TestResult<Vec<crate::foundation::ExactActorRef>> {
    use crate::foundation::{CommandId, CommandRef, MovementLocalPosition};
    use crate::gameplay_transport::kill_reward::KillSink;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    owner.refresh_quest_session(session, false).await;
    assert!(owner.refresh_source_party_presence(actor, session).await);
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let queue = owner.states.attack.lock().await.kills().clone();
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    let mut targets = Vec::with_capacity(commands.len());
    {
        let mut runtime = owner.runtime.lock().await;
        let at = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        let policy = runtime.companion_policy(creature).map_err(debug)?;
        for &command in commands {
            let target = runtime
                .admit_encounter_actor(
                    creature,
                    policy.maximum_health,
                    MovementLocalPosition { x: at.x + 1, ..at },
                )
                .map_err(debug)?;
            runtime
                .install_creature_policy(target, creature)
                .map_err(debug)?;
            let lease = runtime
                .bound_attacker_lease(actor, session)
                .map_err(debug)?;
            let health = runtime.companion_snapshot(target).map_err(debug)?.health;
            runtime
                .borrow_exact_actor_commit_at(now)
                .commit_damage_for_bound_attacker(
                    target,
                    actor,
                    CommandRef::new(session, CommandId::new(command).map_err(debug)?),
                    0,
                    crate::foundation::OwnerDamageCommand {
                        target: creature.as_bytes(),
                        occurrence: session.as_bytes(),
                        binding: b"qengine-native-trial-death",
                        damage: health,
                    },
                )
                .map_err(debug)?;
            for _ in 0..2 {
                crate::gameplay_transport::kill_reward::record_creature_kill(
                    &mut runtime,
                    KillSink {
                        prey: None,
                        activity: &[],
                        queue: &queue,
                        rewards: native.reward_table(),
                        charms: None,
                    },
                    target,
                    crate::combat::CapturedRewardPrincipal {
                        character: lease.character_id(),
                        lease_generation: lease.generation(),
                        session,
                        actor,
                    },
                    now,
                    Some(observed),
                );
            }
            targets.push(target);
        }
    }
    Ok(targets)
}

async fn demon_wars_trial(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    use oteryn_gameplay_quests::quest::world_callbacks::Position;
    use oteryn_protocol_oteryn::npc_service::{CAPABILITY_NPC_SERVICE_V1, NpcIntentDisposition};

    let catalogue = owner.quest_catalogue.as_ref().ok_or("quest catalogue")?;
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load_for(catalogue)?;
    let find = |name| {
        callbacks
            .iter()
            .find(|callback| callback.key == format!("oteryn:quest-reward/demon-wars/{name}"))
            .ok_or("priestess dialogue")
    };
    let briefing = find("briefing")?;
    let report = find("report")?;
    let placements: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../../content/world/npc-placements/placements-00000-00055.json"
    ))?;
    let priestess = Position {
        x: 33258,
        y: 31938,
        floor: -15,
    };
    assert!(
        placements["records"]
            .as_array()
            .ok_or("placements")?
            .iter()
            .any(|row| row["declaration"]["npc"] == briefing.npc
                && row["declaration"]["cell"]
                    == serde_json::json!({"x": priestess.x, "y": priestess.y, "floor": 15}))
    );
    let location = |x, y, floor| crate::foundation::MovementLocalPosition { x, y, floor };
    let actor = admitted.runtime_actor.ok_or("actor")?;
    move_actor(owner, actor, location(33259, 31938, 15)).await?;
    let [npc] = super::classic_quests_fixtures::install_positions(
        owner,
        admitted,
        [briefing.npc.as_str()],
        [priestess],
    )
    .await?;
    let mut supported = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    supported.push(CAPABILITY_NPC_SERVICE_V1);
    supported.sort_unstable();
    supported.dedup();
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &supported,
        )
        .ok_or("NPC capability")?;
    owner.admit_premium(super::fixture::id(40), admitted.game_session_id);
    owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native creatures")?
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;

    let wire = super::qengine_rewards_fixtures::wire_conversation;
    assert_eq!(
        wire(
            owner,
            admitted,
            npc,
            briefing,
            false,
            vec!["hi", &briefing.keywords[0]]
        )
        .await?
        .0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        wire(
            owner,
            admitted,
            npc,
            report,
            false,
            vec!["hi", &report.keywords[0]]
        )
        .await?
        .0,
        NpcIntentDisposition::Rejected
    );
    let prefix = "oteryn:quest-progress/demon-wars/trial/";
    assert_eq!(count(h, &format!("{prefix}phase")).await?, 1);
    move_actor(owner, actor, location(33260, 31930, 13)).await?;
    death(owner, admitted, "oteryn:creature.shaburak_prince", 22000).await?;
    assert_eq!(count(h, &format!("{prefix}prince")).await?, 0);
    let mut command = 22001;
    for (creature, goal) in [("demon", 20), ("lord", 1), ("prince", 1)] {
        let definition = format!("oteryn:creature.shaburak_{creature}");
        for expected in 1..=goal {
            death(owner, admitted, &definition, command).await?;
            command += 1;
            assert_eq!(count(h, &format!("{prefix}{creature}")).await?, expected);
        }
    }
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    move_actor(owner, actor, location(33259, 31938, 15)).await?;
    assert_eq!(
        wire(
            owner,
            admitted,
            npc,
            report,
            false,
            vec!["hi", &report.keywords[0]]
        )
        .await?
        .0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        wire(
            owner,
            admitted,
            npc,
            report,
            false,
            vec!["hi", &report.keywords[0]]
        )
        .await?
        .0,
        NpcIntentDisposition::Rejected
    );
    let completions: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes",
    )
    .bind(&report.quest)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(completions, 1);
    Ok(())
}

pub(super) async fn move_actor(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    position: crate::foundation::MovementLocalPosition,
) -> TestResult {
    let mut runtime = owner.runtime.lock().await;
    let from = runtime
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    let moved = runtime
        .commit_quest_platform_ride(from, position, actor, |_, _, _| true, || true)
        .map_err(debug)?;
    assert_eq!(moved.len(), 1);
    Ok(())
}
