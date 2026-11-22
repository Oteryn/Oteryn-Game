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
        "cave-explorer" => return Box::pin(spike(owner, admitted, h, authority)).await,
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
    if label != "dragon-slayer" {
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
        }
    }
    Ok(())
}

async fn spike(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    use super::qengine_rewards_fixtures::talk;
    use super::{classic_quests_fixtures as npc, qengine_voucher_stories_fixtures as items};
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
    const FAME: &str = "oteryn:quest-progress/spike/fame";
    const KILLS: &str = "oteryn:quest-progress/spike/lower/kills";
    const PHASE: &str = "oteryn:quest-progress/spike/lower/phase";
    const REPORTED_AT: &str = "oteryn:quest-progress/spike/lower/reported_at";
    const DRILLWORM: &str = "oteryn:creature.drillworm";
    const OUTFIT: &str = "oteryn:outfit.cave_explorer";
    let mut capabilities = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    capabilities.extend([3, 4, 6, 11, 15, 18, 25]);
    capabilities.sort_unstable();
    capabilities.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            &offered,
            &capabilities,
        )
        .ok_or("Spike capabilities")?;
    owner.admit_premium(super::fixture::id(40), admitted.game_session_id);
    owner
        .states
        .chat
        .admit(admitted.game_session_id, "Spike Witness", owner.owner_now());
    let [argery, commander, milly, bold] = npc::install_with_offsets(
        owner,
        admitted,
        [
            "oteryn:npc.gnomargery",
            "oteryn:npc.gnommander",
            "oteryn:npc.gnomilly",
            "oteryn:npc.gnombold",
        ],
        Some([(0, 1), (1, 1), (-1, 1), (0, -1)]),
    )
    .await?;
    for (tier, npc, maximum, id) in [("upper", milly, 49, 101), ("middle", bold, 79, 111)] {
        use super::qengine_outfit_stories_fixtures::earn_level;
        earn_level(owner, admitted, authority, maximum).await?;
        assert_eq!(talk(owner, admitted, npc, id, "hi").await?, NpcIntentDisposition::Ok);
        assert_eq!(talk(owner, admitted, npc, id + 1, "kill").await?, NpcIntentDisposition::Ok);
        earn_level(owner, admitted, authority, maximum + 1).await?;
        assert_eq!(talk(owner, admitted, npc, id + 2, "yes").await?, NpcIntentDisposition::Rejected);
        assert_eq!(count(h, &format!("oteryn:quest-progress/spike/{tier}/phase")).await?, 0);
        assert_eq!(talk(owner, admitted, npc, id + 3, "kill").await?, NpcIntentDisposition::Rejected);
        let _ = talk(owner, admitted, npc, id + 4, "bye").await?;
    }
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native policies")?;
    for creature in ["oteryn:creature.demon_skeleton", "oteryn:creature.crystalcrusher", DRILLWORM] {
        native.reward_table().row(creature).map_err(debug)?;
    }
    native.install_companion_policies(&mut *owner.runtime.lock().await).map_err(debug)?;
    death(owner, admitted, DRILLWORM, 20_000).await?;
    assert_eq!(count(h, KILLS).await?, 0);
    for (id, text) in [(1, "hi"), (2, "kill"), (3, "yes")] {
        assert_eq!(
            talk(owner, admitted, argery, id, text).await?,
            NpcIntentDisposition::Ok
        );
    }
    assert_eq!(count(h, PHASE).await?, 1);
    for (id, text) in [(4, "report"), (5, "kill")] {
        let _ = talk(owner, admitted, argery, id, text).await?;
    }
    assert_eq!(
        count(h, FAME).await?,
        0,
        "an incomplete report earns no fame"
    );
    for kills in 1..=7 {
        death(owner, admitted, DRILLWORM, 20_000 + kills).await?;
        assert_eq!(
            count(h, KILLS).await?,
            i64::try_from(kills)?,
            "one credit per lethal occurrence"
        );
    }
    death(owner, admitted, DRILLWORM, 20_008).await?;
    assert_eq!(count(h, KILLS).await?, 7);
    for (id, text) in [(6, "report"), (7, "kill")] {
        assert_eq!(
            talk(owner, admitted, argery, id, text).await?,
            NpcIntentDisposition::Ok
        );
    }
    assert_eq!(count(h, FAME).await?, 1);
    assert_eq!(count(h, KILLS).await?, 0);
    assert_eq!(count(h, PHASE).await?, 0);
    for (id, text) in [(8, "kill"), (9, "yes")] {
        let _ = talk(owner, admitted, argery, id, text).await?;
    }
    assert_eq!(
        count(h, PHASE).await?,
        0,
        "no new task during the source cooldown"
    );
    let elapsed_start: i64 = sqlx::query_scalar(
        "SELECT floor(extract(epoch FROM clock_timestamp()))::bigint - 72001",
    )
    .fetch_one(&h.pool)
    .await?;
    assert!(count(h, REPORTED_AT).await? > elapsed_start, "the report stamps the transaction clock");
    arrange_track(h, REPORTED_AT, elapsed_start).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    for (id, text) in [(10, "bye"), (11, "hi"), (12, "kill"), (13, "yes")] {
        assert_eq!(
            talk(owner, admitted, argery, id, text).await?,
            NpcIntentDisposition::Ok,
            "{id}: {text}"
        );
    }
    assert_eq!(count(h, PHASE).await?, 1);

    let _ = talk(owner, admitted, argery, 14, "bye").await?;
    assert_eq!(
        talk(owner, admitted, commander, 15, "hi").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        talk(owner, admitted, commander, 16, "worthy").await?,
        NpcIntentDisposition::Rejected
    );
    assert_eq!(count(h, FAME).await?, 1);

    // Bring the balance to the three-purchase boundary after proving how fame is earned.
    arrange_track(h, FAME, 300).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    items::mint(owner, admitted, 3043, 1, &mut 30_000).await?;
    for (index, (keyword, remaining, mask)) in
        [("basic", 200, 0), ("second", 100, 2), ("first", 0, 3)]
            .into_iter()
            .enumerate()
    {
        let first = 17 + u64::try_from(index)? * 3;
        for (id, text) in [(first, "worthy"), (first + 1, keyword), (first + 2, "yes")] {
            assert_eq!(
                talk(owner, admitted, commander, id, text).await?,
                NpcIntentDisposition::Ok
            );
        }
        assert_eq!(count(h, FAME).await?, remaining);
        items::mask(owner, admitted, OUTFIT, mask).await?;
        let achievement: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/cave_completionist'")
            .fetch_one(&h.pool).await?;
        assert_eq!(achievement, i64::from(mask == 3));
        if mask == 3 {
            let progress: i64 = sqlx::query_scalar("SELECT progress FROM game_character_achievement_progress WHERE achievement_key='oteryn:achievement/cave_completionist'")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(progress, 2);
        }
        if mask == 2 {
            // Main granted the cosmetic before these purchases tracked achievement progress.
            let erased = sqlx::query("DELETE FROM game_character_achievement_progress WHERE achievement_key='oteryn:achievement/cave_completionist'")
                .execute(&h.pool)
                .await?;
            assert_eq!(erased.rows_affected(), 1);
        }
    }
    // Reconstruct the persisted state of a character who bought both addons before
    // the achievement was recorded. A repeat purchase must remain free.
    let mut fixture = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *fixture)
        .await?;
    for delete in [
        "DELETE FROM game_account_achievements WHERE achievement_key=$1",
        "DELETE FROM game_account_achievement_grant_requests WHERE achievement_key=$1",
        "DELETE FROM game_character_achievement_progress WHERE achievement_key=$1",
    ] {
        let erased = sqlx::query(delete)
            .bind("oteryn:achievement/cave_completionist")
            .execute(&mut *fixture)
            .await?;
        assert_eq!(erased.rows_affected(), 1);
    }
    fixture.commit().await?;
    let migration = include_str!(
        "../../../../../../../apps/game-server/migrations/3033_cave_completionist_backfill.sql"
    );
    for _ in 0..2 {
        sqlx::raw_sql(migration).execute(&h.pool).await?;
    }
    arrange_track(h, FAME, 100).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    for (id, keyword) in [(26, "worthy"), (27, "first")] {
        assert_eq!(
            talk(owner, admitted, commander, id, keyword).await?,
            NpcIntentDisposition::Ok
        );
    }
    assert_eq!(
        talk(owner, admitted, commander, 28, "yes").await?,
        NpcIntentDisposition::Rejected
    );
    assert_eq!(count(h, FAME).await?, 100);
    let achievement: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/cave_completionist'"
    ).fetch_one(&h.pool).await?;
    assert_eq!(achievement, 1, "both main-era addons earn the achievement");
    Ok(())
}

async fn arrange_track(h: &Harness, track: &str, value: i64) -> TestResult {
    let mut fixture = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *fixture)
        .await?;
    let updated = sqlx::query("UPDATE game_character_quest_tracks SET value=$2 WHERE track_key=$1")
        .bind(track)
        .bind(value)
        .execute(&mut *fixture)
        .await?;
    assert_eq!(updated.rows_affected(), 1);
    fixture.commit().await?;
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
