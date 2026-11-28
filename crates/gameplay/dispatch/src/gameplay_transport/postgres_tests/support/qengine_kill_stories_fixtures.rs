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
    queue_death_at(owner, admitted, creature, command, departed_top_damager, None).await
}

pub(super) async fn queue_death_at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    creature: &str,
    command: u64,
    departed_top_damager: Option<
        crate::durability::character_progression::CurrentCharacterGameplayFence,
    >,
    position: Option<crate::foundation::MovementLocalPosition>,
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
                position.unwrap_or(MovementLocalPosition { x: at.x + 1, ..at }),
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
        "cave-explorer" | "cave-explorer-inherited-one" | "cave-explorer-inherited-two" => {
            let inherited = match label {
                "cave-explorer-inherited-one" => Some(1_i64),
                "cave-explorer-inherited-two" => Some(2_i64),
                _ => None,
            };
            return Box::pin(spike(owner, admitted, h, authority, inherited)).await;
        }
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

async fn spike(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    inherited: Option<i64>,
) -> TestResult {
    use super::qengine_rewards_fixtures::talk;
    use super::{classic_quests_fixtures as npc, qengine_voucher_stories_fixtures as items};
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
    const FAME: &str = "oteryn:quest-progress/quest/u10_20/spike_task_quest/constants/spike_fame_points";
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
    if inherited.is_some() {
        // Drive the purchase route with the transferred Character's level and
        // fame already in place. The hunting story below has its own coverage.
        super::qengine_outfit_stories_fixtures::earn_level(owner, admitted, authority, 80).await?;
        super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
            h, owner, admitted, &[(FAME.into(), 100)], 124,
        )
        .await?;
        assert_eq!(
            talk(owner, admitted, commander, 15, "hi").await?,
            NpcIntentDisposition::Ok
        );
    } else {
        Box::pin(spike_hunting(
            owner, admitted, h, authority, [argery, commander, milly, bold],
        ))
        .await?;
        // Bring the balance to the qualification threshold after proving how fame is earned.
        arrange_track(h, FAME, 100).await?;
    }
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    items::mint(owner, admitted, 3043, 1, &mut 30_000).await?;
    if let Some(progress) = inherited {
        // Bazaar transfers the Character's counter, while unlock facts stay with the seller.
        sqlx::query("INSERT INTO game_character_achievement_progress(character_id,achievement_key,progress)
            SELECT character_id,'oteryn:achievement/cave_completionist',$1 FROM game_character_roots")
            .bind(progress).execute(&h.pool).await?;
    }
    for (index, (keyword, remaining, mask)) in
        [("basic", 100, 0), ("second", 100, 2), ("first", 100, 3)]
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
        if mask == 2 && inherited.is_none() {
            // Main granted the cosmetic before these purchases tracked achievement progress.
            let erased = sqlx::query("DELETE FROM game_character_achievement_progress WHERE achievement_key='oteryn:achievement/cave_completionist'")
                .execute(&h.pool)
                .await?;
            assert_eq!(erased.rows_affected(), 1);
        }
    }
    if inherited.is_some() {
        return Ok(());
    }
    let migration = include_str!(
        "../../../../../../../apps/game-server/migrations/3339_cave_completionist_backfill.sql"
    );
    // Bazaar-locked roots still prove the account grant, with no Character
    // progress row to clear before the next lifecycle case.
    for (lifecycle, previous_progress) in [(2_i16, 1), (3, 0), (4, 1)] {
        // Reconstruct a pre-achievement purchase with one retained character.
        let mut fixture = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *fixture)
            .await?;
        for (delete, rows) in [
            ("DELETE FROM game_account_achievements WHERE achievement_key=$1", 1),
            ("DELETE FROM game_account_achievement_grant_requests WHERE achievement_key=$1", 1),
            (
                "DELETE FROM game_character_achievement_progress WHERE achievement_key=$1",
                previous_progress,
            ),
        ] {
            let erased = sqlx::query(delete)
                .bind("oteryn:achievement/cave_completionist")
                .execute(&mut *fixture)
                .await?;
            assert_eq!(erased.rows_affected(), rows, "{lifecycle}: {delete}");
        }
        let retained = sqlx::query("UPDATE game_character_roots SET lifecycle=$1")
            .bind(lifecycle)
            .execute(&mut *fixture)
            .await?;
        assert_eq!(retained.rows_affected(), 1);
        fixture.commit().await?;
        for _ in 0..2 {
            let mut upgrade = h.pool.begin().await?;
            sqlx::raw_sql(migration).execute(&mut *upgrade).await?;
            upgrade.commit().await?;
        }
        let (facts, progress): (i64, Option<i64>) = sqlx::query_as(
            "SELECT count(*), max(p.progress) FROM game_account_achievements a
             JOIN game_account_achievement_grant_requests r
               USING (source_kind, source_event_id, achievement_key, account_id)
             LEFT JOIN game_character_achievement_progress p USING (character_id, achievement_key)
             WHERE a.achievement_key='oteryn:achievement/cave_completionist'",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!((facts, progress), (1, (lifecycle != 2).then_some(2)), "lifecycle {lifecycle}");
    }
    // Replay a pre-achievement purchase after a sale with an active seller alt.
    // Keep this reconstruction in one transaction so the live story can resume.
    let mut sale = h.pool.begin().await?;
    sqlx::raw_sql(
        "SET LOCAL session_replication_role=replica;
         DELETE FROM game_account_achievements WHERE achievement_key='oteryn:achievement/cave_completionist';
         DELETE FROM game_account_achievement_grant_requests WHERE achievement_key='oteryn:achievement/cave_completionist';
         DELETE FROM game_character_achievement_progress WHERE achievement_key='oteryn:achievement/cave_completionist';
         INSERT INTO game_character_roots
             (character_id, account_id, world_id, lifecycle, character_revision, profile_revision,
              ruleset_revision, content_revision, starter_template_revision, name)
         SELECT uuidv7(), account_id, world_id, 1, 1, profile_revision,
                ruleset_revision, content_revision, starter_template_revision, 'Retained Alt'
           FROM game_character_roots;
         UPDATE game_character_roots SET account_id=uuidv7(), lifecycle=1 WHERE lifecycle=4;
         SET LOCAL session_replication_role=origin;"
    ).execute(&mut *sale).await?;
    sqlx::raw_sql(migration).execute(&mut *sale).await?;
    let (purchaser, progress): (bool, i64) = sqlx::query_as(
        "SELECT r.character_id = u.granting_character_id,
                (SELECT count(*) FROM game_character_achievement_progress WHERE achievement_key=r.achievement_key)
           FROM game_account_achievement_grant_requests r
           JOIN game_account_appearance_unlock_receipts u USING (account_id)
          WHERE r.achievement_key='oteryn:achievement/cave_completionist'
            AND u.unlock_key='oteryn:outfit.cave_explorer/quest-grant/2'"
    ).fetch_one(&mut *sale).await?;
    assert_eq!((purchaser, progress), (true, 0));
    // Reuse the two historical purchases. Move the completing purchase to the
    // seller's retained alt and leave the earlier purchaser on the buyer's account.
    for (addon, suffix) in [(1, "first"), (2, "second")] {
        let mut split = sqlx::Acquire::begin(&mut *sale).await?;
        sqlx::raw_sql("SET LOCAL session_replication_role=replica;
            DELETE FROM game_account_achievements WHERE achievement_key='oteryn:achievement/cave_completionist';
            DELETE FROM game_account_achievement_grant_requests WHERE achievement_key='oteryn:achievement/cave_completionist'")
            .execute(&mut *split).await?;
        let key = format!("oteryn:quest-reward/l12-cave-explorer-{suffix}");
        let alt: sqlx::types::Uuid = sqlx::query_scalar("SELECT character_id FROM game_character_roots WHERE name='Retained Alt'")
            .fetch_one(&mut *split).await?;
        let changed = sqlx::query("UPDATE game_character_quest_receipts SET character_id=$1,
                committed_at=(SELECT max(committed_at)+1 FROM game_character_quest_receipts) WHERE transition_key=$2")
            .bind(alt).bind(&key).execute(&mut *split).await?;
        assert_eq!(changed.rows_affected(), 1);
        sqlx::query("UPDATE game_quest_reward_sources SET character_id=$1 WHERE transition_key=$2")
            .bind(alt).bind(&key).execute(&mut *split).await?;
        sqlx::query("UPDATE game_account_appearance_unlock_receipts SET granting_character_id=$1,
                source_event_id=overlay(source_event_id placing uuid_send($1) from length('oteryn:quest-callback-outfit:v1')+2 for 16)
                WHERE unlock_key=$2")
            .bind(alt).bind(format!("oteryn:outfit.cave_explorer/quest-grant/{addon}"))
            .execute(&mut *split).await?;
        sqlx::raw_sql("SET LOCAL session_replication_role=origin; DROP TABLE cave_completionist_backfill_eligible")
            .execute(&mut *split).await?;
        sqlx::raw_sql(migration).execute(&mut *split).await?;
        let earned: (sqlx::types::Uuid, i64) = sqlx::query_as(
            "SELECT r.character_id, p.progress FROM game_account_achievement_grant_requests r
             JOIN game_character_achievement_progress p USING (character_id,achievement_key)
             WHERE r.achievement_key='oteryn:achievement/cave_completionist'",
        ).fetch_one(&mut *split).await?;
        assert_eq!(earned, (alt, 2), "addon {addon} completed the pair");
        // A timestamp tie proves the account grant, but cannot identify the finisher.
        sqlx::raw_sql("SET LOCAL session_replication_role=replica;
            DELETE FROM game_account_achievements WHERE achievement_key='oteryn:achievement/cave_completionist';
            DELETE FROM game_account_achievement_grant_requests WHERE achievement_key='oteryn:achievement/cave_completionist';
            DELETE FROM game_character_achievement_progress WHERE achievement_key='oteryn:achievement/cave_completionist';
            UPDATE game_character_quest_receipts SET committed_at=1 WHERE transition_key IN
                ('oteryn:quest-reward/l12-cave-explorer-first','oteryn:quest-reward/l12-cave-explorer-second');
            SET LOCAL session_replication_role=origin; DROP TABLE cave_completionist_backfill_eligible")
            .execute(&mut *split).await?;
        sqlx::raw_sql(migration).execute(&mut *split).await?;
        let tied: (i64, i64) = sqlx::query_as("SELECT
            (SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/cave_completionist'),
            (SELECT count(*) FROM game_character_achievement_progress WHERE achievement_key='oteryn:achievement/cave_completionist')")
            .fetch_one(&mut *split).await?;
        assert_eq!(tied, (1, 0));
        split.rollback().await?;
    }
    sale.rollback().await?;
    let mut fixture = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *fixture)
        .await?;
    let restored = sqlx::query("UPDATE game_character_roots SET lifecycle=1 WHERE lifecycle=4")
        .execute(&mut *fixture)
        .await?;
    assert_eq!(restored.rows_affected(), 1);
    fixture.commit().await?;
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
    Box::pin(spike_surveys(owner, admitted, h)).await
}

pub(super) async fn recover_nest(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    trigger: oteryn_gameplay_quests::quest::world_callbacks::Position,
) -> TestResult {
    use super::questpc8_authored_programmes_fixtures::{object_target, use_target};
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    const PROGRESS: &str = "oteryn:quest-progress/spike/nests/progress";
    const STAMP: &str = "oteryn:quest-progress/spike/nests/transformed_at";
    let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
        .fetch_one(&h.pool).await?;
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
        h, owner, admitted, &[(PROGRESS.into(), 1), (STAMP.into(), now)], 217,
    ).await?;
    let session = admitted.game_session_id;
    owner.refresh_quest_session(session, false).await;
    owner.refresh_quest_disguise(session).await;
    let (draw, creature) = match trigger.x {
        32162 => (61, "oteryn:creature.spider"),
        32221 => (71, "oteryn:creature.larva"),
        32112 => (81, "oteryn:creature.scarab"),
        32180 => (91, "oteryn:creature.tarantula"),
        _ => return Err("unexpected nest".into()),
    };
    let tile = crate::map::overlay::TilePos { x: trigger.x, y: trigger.y, floor: trigger.floor };
    let before = owner.runtime.lock().await.current_live_creature_census().map_err(debug)?;
    owner.states.quest_callbacks.nest_draw.store(draw, std::sync::atomic::Ordering::Release);
    let target = object_target(owner, tile, 1)?;
    owner.states.quest_callbacks.interrupt_after_commit.store(true, std::sync::atomic::Ordering::Release);
    let command = admitted.continuity.next_command_id;
    assert!(use_target(owner, admitted, command, &target).await.is_err());
    assert_eq!(count(h, PROGRESS).await?, 2);
    assert!(owner.has_pending_quest_callback(session));
    owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(121)).ok_or("clock")?;
    if draw == 61 {
        let mut blockers = Vec::new();
        {
            let mut runtime = owner.runtime.lock().await;
            let at = runtime.read_actor_position(admitted.runtime_actor.ok_or("actor")?).map_err(debug)?.position();
            loop {
                match runtime.admit_encounter_actor("oteryn:creature.rat", 1, at) {
                    Ok(actor) => blockers.push(actor),
                    Err(crate::domain::carrier_values::CarrierError::CapacityExceeded) => break,
                    Err(error) => return Err(debug(error).into()),
                }
            }
        }
        assert!(!owner.reconcile_quest_callbacks(session).await);
        assert!(owner.has_pending_quest_callback(session));
        assert_eq!(object_target(owner, tile, 1)?.expected_revision, target.expected_revision);
        let mut runtime = owner.runtime.lock().await;
        for actor in blockers {
            runtime.remove_encounter_actor(actor).map_err(debug)?;
        }
    }
    assert!(owner.reconcile_quest_callbacks(session).await);
    assert!(owner.reconcile_quest_callbacks(session).await);
    assert!(!owner.has_pending_quest_callback(session));
    assert_eq!(use_target(owner, admitted, command, &target).await?, UseDisposition::Committed);
    assert_eq!(count(h, PROGRESS).await?, 2);
    assert!(owner.reconcile_quest_callbacks(session).await);
    let runtime = owner.runtime.lock().await;
    assert_eq!(object_target(owner, tile, 1)?.expected_revision, target.expected_revision + 1);
    let after = runtime.current_live_creature_census().map_err(debug)?;
    assert_eq!(after.len(), before.len() + 1);
    let spawned: Vec<_> = after.iter().filter(|a| !before.contains(a)).collect();
    assert_eq!(spawned.len(), 1);
    assert_eq!(runtime.current_live_creature_identity(*spawned[0]).map_err(debug)?, creature.as_bytes());
    Ok(())
}

async fn spike_surveys(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use super::qengine_voucher_stories_fixtures as items;
    use crate::foundation::MovementLocalPosition;
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    const TRACK: &str = "oteryn:quest-progress/spike/tracking/progress";
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
        h, owner, admitted, &[(TRACK.into(), 1)], 216,
    ).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    items::mint(owner, admitted, 19205, 1, &mut 30_100).await?;
    let detector: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT item_instance_id FROM game_item_instances
         WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i19205'",
    ).fetch_one(&h.pool).await?;
    let character = admitted.item_fence.ok_or("fence")?.character_id;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let rows = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    for phase in 1..=3 {
        let key = format!("oteryn:quest-reward/l12-spike-tracking-survey-{phase}");
        let survey = rows.iter().find(|row| row.key == key)
            .and_then(|row| row.survey.as_ref()).ok_or("survey")?;
        let areas = serde_json::to_value(survey)?;
        let at = areas["areas"].as_array().ok_or("survey areas")?.iter().find_map(|area| {
            let p: oteryn_gameplay_quests::quest::world_callbacks::Position =
                serde_json::from_value(area["minimum"].clone()).ok()?;
            let at = MovementLocalPosition { x: p.x.into(), y: p.y.into(), floor: -i16::from(p.floor) };
            survey.clue(character.as_bytes(), owner.reset_epoch, at.x, at.y, at.floor)
                .is_none().then_some(at)
        }).ok_or("selected survey area")?;
        {
            let mut runtime = owner.runtime.lock().await;
            let before = runtime.read_actor_position(actor).map_err(debug)?;
            runtime.relocate_arena_player(actor, admitted.game_session_id, before, at).map_err(debug)?;
        }
        items::use_native(owner, admitted, 19205, UseDisposition::Committed).await?;
        assert_eq!(count(h, TRACK).await?, phase + 1);
        assert_eq!(items::quantity(h, 19205).await?, i64::from(phase < 3));
    }
    let retained: Vec<sqlx::types::Uuid> = sqlx::query_scalar(
        "SELECT retained_detector FROM game_quest_reward_sources WHERE retained_detector IS NOT NULL",
    ).fetch_all(&h.pool).await?;
    assert_eq!(retained, [detector, detector]);
    let consumed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_quest_callback_consumptions WHERE item_instance_id=$1",
    ).bind(detector).fetch_one(&h.pool).await?;
    assert_eq!(consumed, 1);
    Ok(())
}

async fn spike_hunting(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    [argery, commander, milly, bold]: [oteryn_protocol_oteryn::world_spatial_entities::EntityRef; 4],
) -> TestResult {
    use super::qengine_rewards_fixtures::talk;
    use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
    const FAME: &str = "oteryn:quest-progress/quest/u10_20/spike_task_quest/constants/spike_fame_points";
    const DRILLWORM: &str = "oteryn:creature.drillworm";
    const REPORTED_AT: &str = "oteryn:quest-progress/spike/lower/reported_at";
    const PHASE: &str = "oteryn:quest-progress/spike/lower/phase";
    const KILLS: &str = "oteryn:quest-progress/spike/lower/kills";
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

pub(super) fn pad_world(
    native: &crate::content::native_gameplay::NativeGameplayState,
    start_y: u16,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups_fixtures::{ExpectedTeleport, NativeCrop, native_scene};
    let crop = |bounds| NativeCrop {
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z11-x125-y127.b3"
        )),
        region: (11, 125, 127),
        bounds,
        start: (32242, start_y),
        expected: Vec::new(),
    };
    // The south pad also carries a native teleport. Keep its destination inside the
    // extent but without landable ground, so movement settles on the pad for StepIn.
    native_scene(
        native,
        vec![crop((32242, 32609, 32243, 32612)), crop((32241, 32608, 32242, 32609))],
        &[ExpectedTeleport {
            position: (32242, 32611, 11),
            ordinal: 1,
            destination: (32242, 32608, 11),
        }],
    )
}

pub(super) async fn transform_pad(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    trigger: oteryn_gameplay_quests::quest::world_callbacks::Position,
) -> TestResult {
    use super::quest_world_callbacks_fixtures::local;
    use super::questpc8_authored_programmes_fixtures::step;
    use crate::domain::condition::{ConditionValues, ConflictKey};
    use crate::foundation::{ApplicationFacts, ConditionDefinition, ConditionSourceKind};
    use oteryn_gameplay_entities::{ActorConditionTransition, ConditionSource};
    use oteryn_protocol_oteryn::world_spatial::StepDirection;
    const STAMP: &str = "oteryn:quest-progress/spike/nests/transformed_at";
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
        h,
        owner,
        admitted,
        &[
            ("oteryn:quest-progress/spike/nests/progress".into(), 1),
            (STAMP.into(), 1),
        ],
        217,
    )
    .await?;
    let session = admitted.game_session_id;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    owner.refresh_quest_session(session, false).await;
    let origin = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let member = native
        .spell_appearances()
        .and_then(|a| a.for_look_type(1593))
        .ok_or("outfit")?;
    let item = ConditionDefinition::new(
        "spell.chameleon",
        0,
        ConditionValues::ItemOutfit {
            duration_ms: 60_000,
        },
    )
    .and_then(|d| {
        d.with_item_appearance(
            "oteryn:item.tibia.i3031",
            "definition-r1",
            native.source_digest(),
        )
    })
    .ok_or("chameleon")?;
    let ordinary = ConditionDefinition::new(
        "fixture.outfit",
        1,
        ConditionValues::Outfit {
            duration_ms: 60_000,
            look_type: member.look_type(),
        },
    )
    .and_then(|d| d.with_appearance(member.selection()))
    .ok_or("ordinary outfit")?;
    for (spell, definition) in [(true, item), (false, ordinary)] {
        let now = owner.owner_now();
        let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes([7; 32]);
        let facts = ApplicationFacts {
            now: now.get(),
            base_speed: 220,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes([8; 16]),
        };
        {
            let mut runtime = owner.runtime.lock().await;
            if spell {
                let mut states = owner.states.spell_states.lock().await;
                let state = states.get(&runtime, actor, session).ok_or("player")?;
                let next = state
                    .stage_owned_test_conditions(std::slice::from_ref(&definition), &facts)
                    .ok_or("spell conditions")?;
                assert!(states.commit(&runtime, actor, session, next));
            } else {
                let plan = runtime
                    .prepare_actor_condition(
                        actor,
                        Some(session),
                        ActorConditionTransition::Apply {
                            definition: &definition,
                            source: ConditionSource {
                                actor,
                                session: Some(session),
                                kind: ConditionSourceKind::SelfUse,
                            },
                            immunities: &[],
                            facts,
                        },
                        now,
                    )
                    .map_err(debug)?;
                assert!(runtime.commit_actor_condition(&plan, now).map_err(debug)?);
            }
        }
        let command = admitted.continuity.next_command_id;
        step(owner, admitted, command, StepDirection::South).await?;
        assert_eq!(count(h, STAMP).await?, 1);
        {
            let runtime = owner.runtime.lock().await;
            assert_eq!(
                runtime
                    .read_actor_position(actor)
                    .map_err(debug)?
                    .position(),
                origin
            );
            let now = owner.owner_now().get();
            let states = owner.states.spell_states.lock().await;
            let retained = if spell {
                states
                    .get(&runtime, actor, session)
                    .ok_or("player")?
                    .owned_conditions()
                    .active_at(ConflictKey::Outfit, now)
                    .map(|c| c.definition())
            } else {
                runtime
                    .actor_conditions_at(actor, Some(session), now)
                    .map_err(debug)?
                    .active_at(ConflictKey::Outfit, now)
                    .map(|c| c.definition())
            };
            assert_eq!(retained, Some(&definition));
        }
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(61))
            .ok_or("clock")?;
    }
    let command = admitted.continuity.next_command_id;
    step(owner, admitted, command, StepDirection::South).await?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        local(trigger)
    );
    assert!(count(h, STAMP).await? > 1);
    Ok(())
}
