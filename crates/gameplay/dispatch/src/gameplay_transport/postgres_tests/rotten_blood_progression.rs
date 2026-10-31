//! Real native damage/capture proves the current corpse-content refusal. The
//! captured identity then exercises the existing fenced party-XP/Quest consumer
//! seam, followed by real NPC mutations and immutable reconciliation. Per
//! ANSWER6548332205, this is consumer integration, not physical loot-drain E2E;
//! native corpse qualification and instance admission remain owner dependencies.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::character_progression::{CharacterProgressionError, CurrentCharacterGameplayFence};
use crate::durability::quest_state::rotten_blood::RottenBloodOutcome;
use crate::foundation::{CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, TakeOutcome};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::rotten_blood::{
    ACCESS_TRACK, CLEANSE_COOLDOWN_SECONDS, CleanseMode, RottenBloodBoss,
    RottenBloodRefusal, RottenBloodState,
};
use std::time::Duration;

#[test]
fn rotten_blood_postgres_native_chain_final_reward_replay_and_cleanse() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("rotten-blood-progression", scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "rotten_blood_progression", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = crate::content::native_gameplay::test_support::activated_with_loot_tables(
            world,
            Some(include_bytes!("../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json")),
        );
        let active = controller.active().ok_or("activated content")?;
        let native = active.native_gameplay().ok_or("native content")?;
        let room = pin.qualified_room().clone();
        // Reuse the mounted altar fixture's bundle frame: map/NPC placement
        // floors are native -z, while every carrier actor uses legacy +z.
        // The Thalom qualification room uses a different coordinate convention.
        let callback=oteryn_gameplay_quests::quest::world_callbacks::load()?
            .into_iter()
            .find(|callback| callback.key==oteryn_gameplay_quests::quest::rotten_blood::SACRIFICE)
            .ok_or("Rotten sacrifice source callback")?;
        let bundle=super::quest_world_callbacks::world(&callback,
            oteryn_gameplay_quests::quest::world_callbacks::Position { x:3,y:11,floor:-7 })?;
        let cells=bundle.movement_cells(room.movement_cells())
            .map_err(|error|format!("Rotten progression bundle movement cells: {error:?}"))?;
        let frame=bundle.frame_binding_digest(world);
        let channel_pin = ChannelContentPin::from_activation(
            world, pin.activation_sequence(), pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(), frame,
            bundle.map_revision_digest(), (2, 11, 7),
        );
        let deps = Dependencies::with_content_and_capacity(
            // Eleven retained native deaths, one player and one NPC share this
            // qualification carrier; keep its fixture capacity explicitly bounded.
            channel_pin.clone(), room.clone(), crate::spell::cast::v1_spell_book().map_err(debug)?, 16,
        )?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,NULL,$4,$5,$6)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.activation_sequence().to_string())
            .bind(pin.identity().server_artifact_digest().as_slice())
            .bind(pin.identity().client_artifact_digest().as_slice()).bind(frame.as_slice())
            .execute(&h.pool).await?;
        {
            let mut runtime = deps.runtime.lock().await;
            native.install_companion_policies(&mut runtime).map_err(debug)?;
            runtime.bind_attacker_lease(deps.actor, deps.admitted.game_session_id,
                crate::foundation::CharacterLease::new(crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?, 2).map_err(debug)?)
                .map_err(debug)?;
            runtime.initialize_first_entry_position(deps.actor).map_err(debug)?;
        }
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells=&cells;
        owner.quest_catalogue=Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?.catalogue().clone(),
        ));
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
        owner.clock_origin = std::time::Instant::now().checked_sub(Duration::from_secs(60)).ok_or("clock")?;
        deps.initialize(&owner).await;
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        assert!(owner.progression_binding(admitted.game_session_id).is_some());

        // This captured Primal death first proves the physical refusal, then
        // the accepted consumer seam creates a real Hazard CharacterRevision.
        // The imported access prerequisite belongs to that fixture snapshot.
        kill_key(&owner,&h,&deps,native.reward_table(),"oteryn:creature.the_primal_menace",90).await?;
        let primal:(i16,i16)=sqlx::query_as("SELECT unlocked,selected FROM game_character_hazard WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(primal,(2,1));
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_primal_hazard_receipts")
            .fetch_one(&h.pool).await?,1);

        // Only the previously earned access prerequisite is fixture data. Every
        // taint, flag, completion, outfit entitlement and cleanse below goes
        // through production writers with all receipt/state guards enabled.
        let mut seed = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *seed).await?;
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,'oteryn:quest/rotten-blood',4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(ACCESS_TRACK).execute(&mut *seed).await?;
        seed.commit().await?;
        let initial = read(&owner, &h, &authority, admitted.game_session_id).await?;
        assert!(initial.access());
        assert!(!initial.chain_ready());
        assert!(!initial.completed());
        owner.admit_quest_session(&admitted).await;
        let npc=place_npc(&owner,deps.actor,admitted.game_session_id,"oteryn:npc.bloodshade_rotten").await?;
        for (number,text) in [(80,"hi"),(81,"quest"),(82,"quest")] {
            talk(&owner,deps.actor,admitted.game_session_id,npc,number,text).await?;
        }
        assert_eq!(read(&owner,&h,&authority,admitted.game_session_id).await?.access_stage(),5);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1")
            .bind(oteryn_gameplay_quests::quest::rotten_blood::ACKNOWLEDGE_CHAIN).fetch_one(&h.pool).await?,1);
        for (index, boss) in RottenBloodBoss::CHILDREN.into_iter().enumerate() {
            kill(&owner, &h, &deps, native.reward_table(), boss, 100 + u64::try_from(index)?).await?;
            let state = read(&owner, &h, &authority, admitted.game_session_id).await?;
            assert_eq!(state.regular_taints(), u8::try_from(index + 1)?);
            assert!(state.holds_child(boss));
        }
        assert!(read(&owner, &h, &authority, admitted.game_session_id).await?.chain_ready());
        // A different native death of an already held child freezes a refusal.
        kill(&owner, &h, &deps, native.reward_table(), RottenBloodBoss::Ichgahal, 104).await?;
        assert_eq!(read(&owner, &h, &authority, admitted.game_session_id).await?.regular_taints(), 4);
        kill(&owner, &h, &deps, native.reward_table(), RottenBloodBoss::Bakragore, 105).await?;
        let final_state = read(&owner, &h, &authority, admitted.game_session_id).await?;
        assert_eq!((final_state.regular_taints(), final_state.child_flags()), (0, 0));
        assert!(final_state.final_taint());
        assert!(final_state.completed());
        let outfits: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key='oteryn:appearance-grant/quest/rotten-blood/decaying-defender-base'")
            .fetch_one(&h.pool).await?;
        assert_eq!(outfits, 1);
        let addons: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key LIKE '%rotten-blood%addon%'")
            .fetch_one(&h.pool).await?;
        assert_eq!(addons, 0);

        // A later failed final attempt is decided once by the shared owner.
        // Earn a fresh chain through real native kills so the reset must retain
        // the previously earned final taint and the one account entitlement.
        for (index,boss) in RottenBloodBoss::CHILDREN.into_iter().enumerate() {
            kill(&owner,&h,&deps,native.reward_table(),boss,110+u64::try_from(index)?).await?;
        }
        failed_final(&owner,&h,&authority,admitted.game_session_id).await?;
        // First cleanse is immediately available. Regular-only clears the two
        // retained child flags/taints while preserving the final taint.
        let before = current(&owner, admitted.game_session_id).await?;
        for (number,text) in [(197,"hi"),(198,"taints"),(199,"regular"),(200,"yes")] {
            talk(&owner,deps.actor,admitted.game_session_id,npc,number,text).await?;
        }
        let cleaned=h.root.reconcile_rotten_blood_cleanse(&authority,before,200,CleanseMode::Regular).await.map_err(debug)?
            .ok_or("NPC cleanse receipt")?;
        let RottenBloodOutcome::AlreadyCommitted(cleaned) = cleaned else { return Err("first NPC cleanse was not committed".into()) };
        assert_eq!((cleaned.state.regular_taints(), cleaned.state.child_flags()), (0, 0));
        assert!(cleaned.state.final_taint());
        let first_cleanse = cleaned.state.last_cleanse_at().ok_or("cleanse timestamp")?;
        let replay = h.root.commit_rotten_blood_cleanse(&authority, &h.node, before, 200, CleanseMode::Regular).await.map_err(debug)?;
        assert!(matches!(replay, RottenBloodOutcome::AlreadyCommitted(_)));
        let retained = h.root.reconcile_rotten_blood_cleanse(&authority, before, 200, CleanseMode::Regular).await.map_err(debug)?;
        assert_eq!(retained, Some(replay));
        assert!(matches!(h.root.commit_rotten_blood_cleanse(&authority, &h.node, before, 200, CleanseMode::All).await,
            Err(CharacterProgressionError::ConflictingOccurrence)));
        let after = current(&owner, admitted.game_session_id).await?;
        for (number,text) in [(201,"taints"),(202,"all"),(203,"yes")] {
            talk(&owner,deps.actor,admitted.game_session_id,npc,number,text).await?;
        }
        assert_eq!(h.root.reconcile_rotten_blood_cleanse(&authority,after,203,CleanseMode::All).await.map_err(debug)?,
            Some(RottenBloodOutcome::Refused(RottenBloodRefusal::CleanseCooldown { available_at: first_cleanse + CLEANSE_COOLDOWN_SECONDS })));
        assert_eq!(h.root.reconcile_rotten_blood_cleanse(&authority, after, 202, CleanseMode::All).await.map_err(debug)?, None);
        assert_eq!(read(&owner, &h, &authority, admitted.game_session_id).await?.last_cleanse_at(), Some(first_cleanse));

        // Change exactly one current authority fact. Historical reconciliation
        // remains readable, but first mutation cannot use a stale lease.
        let mut stale = after;
        stale.character_lease_generation += 1;
        assert!(h.root.commit_rotten_blood_cleanse(&authority, &h.node, stale, 202, CleanseMode::All).await.is_err());
        let mut stale = after;
        stale.connection_generation = crate::foundation::ConnectionGeneration::new(after.connection_generation.get()+1).map_err(debug)?;
        assert!(h.root.commit_rotten_blood_cleanse(&authority, &h.node, stale, 202, CleanseMode::All).await.is_err());
        let mut stale = after;
        stale.scope_ownership_generation = crate::foundation::ScopeOwnershipGeneration::new(after.scope_ownership_generation.get()+1).map_err(debug)?;
        assert!(h.root.commit_rotten_blood_cleanse(&authority, &h.node, stale, 202, CleanseMode::All).await.is_err());

        // Move only the fixture clock predecessor, following existing timer
        // tests. The subsequent production writer still uses PostgreSQL time
        // and all transition guards; a failed earlier request stays refused.
        let mut age = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *age).await?;
        sqlx::query("UPDATE game_character_rotten_blood SET last_cleanse_at=last_cleanse_at-244801 WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).execute(&mut *age).await?;
        age.commit().await?;
        assert!(matches!(h.root.commit_rotten_blood_cleanse(&authority, &h.node, after, 203, CleanseMode::All).await.map_err(debug)?,
            RottenBloodOutcome::Refused(RottenBloodRefusal::CleanseCooldown { .. })));
        for (number,text) in [(210,"taints"),(211,"all"),(212,"yes")] {
            talk(&owner,deps.actor,admitted.game_session_id,npc,number,text).await?;
        }
        let full = h.root.reconcile_rotten_blood_cleanse(&authority,after,212,CleanseMode::All).await.map_err(debug)?
            .ok_or("full NPC cleanse receipt")?;
        let RottenBloodOutcome::AlreadyCommitted(full) = full else { return Err("full NPC cleanse was not committed".into()) };
        assert!(!full.state.final_taint());
        assert!(full.state.completed());
        assert!(full.state.last_cleanse_at().is_some_and(|at| at>=first_cleanse));
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key='oteryn:appearance-grant/quest/rotten-blood/decaying-defender-base'")
            .fetch_one(&h.pool).await?, 1);
        for sql in [
            "UPDATE game_character_rotten_blood SET regular_taints=4",
            "DELETE FROM game_character_rotten_blood_receipts",
            "UPDATE game_character_rotten_blood_receipts SET request_binding=decode(repeat('00',32),'hex')",
        ] {
            assert!(sqlx::query(sql).execute(&h.pool).await.is_err(), "unproven mutation: {sql}");
        }
        eprintln!("ROTTEN_BLOOD POSTGRES_CASE_COMPLETED physical_corpse_refusals captured_death_consumer_chain final_reward shared_terminal_failure replay npc_command7_cleanse fences guards");
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        Ok(())
    })
}

async fn failed_final(
    owner: &ComposedFreshAdmission<'_, '_, '_>, h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    session: crate::foundation::GameSessionId,
) -> TestResult {
    use crate::durability::endgame_terminal::{
        EndgameFailureRequest, EndgameTerminalError, FinalEncounter,
        TestOwnerState, TestSharedTerminalOwner, UnavailableSharedTerminalOwner,
    };
    let before=read(owner,h,authority,session).await?;
    assert_eq!((before.regular_taints(),before.child_flags()),(4,15));
    assert!(before.final_taint());
    let fence=current(owner,session).await?;
    let mut slot=owner.revision_sequencer.acquire(fence.character_id).await;
    let request=EndgameFailureRequest {
        attempt:[0x71;32],encounter:FinalEncounter::Bakragore,participants:vec![fence],
    };
    assert!(matches!(h.root.apply_endgame_failure(authority,&h.node,
        &UnavailableSharedTerminalOwner,request.clone()).await,
        Err(EndgameTerminalError::ContentUnavailable)));
    for state in [TestOwnerState::Active,TestOwnerState::IndividualDeath,
        TestOwnerState::Disconnected,TestOwnerState::Victory] {
        assert!(matches!(h.root.apply_endgame_failure(authority,&h.node,
            &TestSharedTerminalOwner::new(request.clone(),state),request.clone()).await,
            Err(EndgameTerminalError::NotTerminalUnsuccessful)));
    }
    assert_eq!(read(owner,h,authority,session).await?,before);
    let result=h.root.apply_endgame_failure(authority,&h.node,
        &TestSharedTerminalOwner::new(request.clone(),TestOwnerState::TerminalUnsuccessful),
        request.clone()).await.map_err(debug)?;
    assert!(!result.replayed);
    let pair=result.removed_flags.ok_or("failed-final pair")?;
    assert_eq!(pair.count_ones(),2);
    slot.observe_endgame_progression();
    drop(slot);
    let after=read(owner,h,authority,session).await?;
    assert_eq!((after.regular_taints(),after.child_flags()),(2,15 & !pair));
    assert!(after.final_taint());
    assert!(after.completed());
    // A retained result is readable with the old revision and with W02 now
    // unavailable; it must neither choose another pair nor consume more flags.
    let replay=h.root.apply_endgame_failure(authority,&h.node,
        &UnavailableSharedTerminalOwner,request).await.map_err(debug)?;
    assert!(replay.replayed);
    assert_eq!(replay.removed_flags,Some(pair));
    assert_eq!(read(owner,h,authority,session).await?,after);
    Ok(())
}

async fn current(owner: &ComposedFreshAdmission<'_, '_, '_>, session: crate::foundation::GameSessionId) -> TestResult<CurrentCharacterGameplayFence> {
    owner.current_quest_fence(session).await.map_err(|_| "current Quest fence unavailable")?
        .ok_or_else(|| "current Quest fence missing".into())
}

async fn read(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    session: crate::foundation::GameSessionId,
) -> TestResult<RottenBloodState> {
    h.root.read_rotten_blood_state(authority, &h.node, current(owner, session).await?).await.map_err(debug)
}

async fn kill(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    deps: &Dependencies,
    rewards: &crate::content::creature_reward::CreatureRewardTable,
    boss: RottenBloodBoss,
    command_id: u64,
) -> TestResult {
    kill_key(owner,h,deps,rewards,boss.creature_key(),command_id).await
}

async fn kill_key(
    owner: &ComposedFreshAdmission<'_, '_, '_>, h: &Harness, deps: &Dependencies,
    rewards: &crate::content::creature_reward::CreatureRewardTable,
    creature_key: &str, command_id: u64,
) -> TestResult {
    use crate::content::creature_reward::NoSettlementReason;
    use crate::content::ProjectV2AuthoringProfileData;
    use super::soul_war::{CapturedEndgameDeath, settle_captured_endgame_death};

    // The six actual source corpses have no admitted bounded container facts.
    // Do not invent a corpse, substitute another Item, or alter the reward row.
    assert!(matches!(rewards.row(creature_key), Err(NoSettlementReason::CorpseItemInadmissible)),
        "{creature_key}: retain physical corpse qualification refusal");
    let native=owner.active_generation.and_then(|active|active.native_gameplay()).ok_or("native content")?;
    let record=native.creature_profiles().records.iter()
        .find(|record|record.profile.target.key==creature_key).ok_or("source boss profile")?;
    let ProjectV2AuthoringProfileData::Creature(profile)=&record.profile.data else {
        return Err("source boss profile kind".into());
    };
    let raw_xp=i64::try_from(profile.experience.ok_or("source boss experience")?)?;
    if creature_key=="oteryn:creature.the_primal_menace" { assert_eq!(raw_xp,0); }
    let queue = owner.states.attack.lock().await.kills().clone();
    let session = deps.admitted.game_session_id;
    let at = owner.owner_now().get() / 1000;
    let observed = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(at.checked_mul(1000).ok_or("death clock")?);
    let before_physical=progression_receipts(h).await?;
    let source={
        let mut runtime = owner.runtime.lock().await;
        let pin = runtime.content_pin().clone();
        runtime.realize_native_qualification_spawn_for_source(u16::try_from(command_id)?,pin,creature_key,
            MovementLocalPosition { x:1, y:11, floor:7 })
            .map_err(|error|format!("Rotten native spawn {creature_key} command {command_id}: {error:?}"))?;
        let target = runtime.visible_entities().creatures.last().ok_or("native boss")?.actor;
        runtime.borrow_exact_actor_commit_at(at).commit_damage_for_bound_attacker(
            target, deps.actor, CommandRef::new(session, CommandId::new(command_id).map_err(debug)?), 0,
            crate::foundation::OwnerDamageCommand { target:creature_key.as_bytes(), occurrence:&command_id.to_be_bytes(), binding:b"rotten-blood-native-test", damage:20_000_000 },
        ).map_err(debug)?;
        crate::gameplay_transport::kill_reward::record_creature_kill(
            &mut runtime, KillSink { prey:None, activity:&[], queue:&queue, rewards, charms:None }, target,
            crate::combat::CapturedRewardPrincipal {
                character:crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                lease_generation:2, session, actor:deps.actor,
            }, at, Some(observed),
        );
        // The production death owner captured the real event before the corpse
        // refusal. Read its retained identity and actual damage contribution.
        let mut death_owner=runtime.borrow_combat_death();
        let top=death_owner.top_damage_contributor(target).map_err(debug)?;
        let winner=top.present().ok_or("native damage contributor")?;
        let principal=crate::combat::CapturedRewardPrincipal {
            character:winner.character,lease_generation:winner.lease_generation,
            session:winner.session,actor:winner.actor,
        };
        assert_eq!(principal.character.as_bytes(),&id(41));
        assert_eq!(principal.session,session);
        assert_eq!(principal.lease_generation,2);
        let facts=crate::combat::capture_projected_death_facts(&mut death_owner,target,principal,
            winner.last_damage_at_ms,at).map_err(debug)?;
        let shares=death_owner.reward_damage_shares(target).map_err(debug)?;
        let damaged=shares.iter().any(|share|share.total>0
            && share.character==principal.character && share.session==Some(session)
            && share.lease_generation==Some(principal.lease_generation)
            && share.last_damage_at_ms.is_some_and(|time|facts.death_at_ms.checked_sub(time)
                .is_some_and(|age|age<=60_000)));
        assert!(damaged,"real native damage must qualify the captured contributor");
        let (occurrence,_)=death_owner.party_reward_occurrence(target,principal.character).map_err(debug)?;
        let death=facts.death;
        let mut death_key=[0;52];
        death_key[..16].copy_from_slice(death.world_id().as_bytes());
        death_key[16..32].copy_from_slice(death.channel_id().as_bytes());
        death_key[32..40].copy_from_slice(&death.scope_ownership_generation().get().to_be_bytes());
        death_key[40..44].copy_from_slice(&death.actor_local_id().to_be_bytes());
        death_key[44..52].copy_from_slice(&death.actor_local_generation().to_be_bytes());
        CapturedEndgameDeath { death_key,occurrence,creature:creature_key.into(),raw_xp,
            death_at_ms:facts.death_at_ms,death_at_us:observed.get(),damaged }
    };
    match queue.take(session, 2) {
        TakeOutcome::Empty => assert_eq!(creature_key,"oteryn:creature.the_primal_menace"),
        TakeOutcome::Taken(held) => {
            let entry = held.entry().ok_or("native Bosstiary fallback")?;
            assert!(entry.row.is_none(),"unqualified corpse cannot enter the physical reward path");
            assert!(entry.bosstiary_only.is_some(),"only the existing Bosstiary fallback is admitted");
            held.requeue();
        }
        TakeOutcome::CapacityWait => return Err("native refusal left a blocked loot queue".into()),
    }
    drain(owner, session).await?;
    assert_eq!(progression_receipts(h).await?,before_physical,
        "physical corpse refusal must not create a native XP, Hazard or Rotten receipt");
    eprintln!("ROTTEN_BLOOD PHYSICAL_CORPSE_REFUSED creature={creature_key} reason=corpse_item_inadmissible");

    settle_captured_endgame_death(owner,&h.pool,session,&source).await
        .map_err(|error|format!("Rotten captured death settlement {creature_key} command {command_id}: {error}"))?;
    let before_replay=progression_receipts(h).await?;
    settle_captured_endgame_death(owner,&h.pool,session,&source).await
        .map_err(|error|format!("Rotten captured death replay {creature_key} command {command_id}: {error}"))?;
    assert_eq!(progression_receipts(h).await?,before_replay,
        "same captured native death must never grant twice through the consumer seam");
    Ok(())
}

async fn progression_receipts(h:&Harness) -> TestResult<(i64,i64,i64,i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_rotten_blood_receipts),(SELECT count(*) FROM game_character_xp_receipts),(SELECT count(*) FROM game_character_primal_hazard_receipts),(SELECT count(*) FROM game_party_kill_xp_receipts)")
        .fetch_one(&h.pool).await?)
}

async fn drain(owner: &ComposedFreshAdmission<'_, '_, '_>, session: crate::foundation::GameSessionId) -> TestResult {
    for _ in 0..8 {
        if owner.drain_kill_rewards(session).await==DrainOutcome::Drained {
            return Ok(());
        }
        owner.root.maintain_ready_once().await.map_err(debug)?;
    }
    Err("native boss kill remained unsettled".into())
}

pub(super) async fn place_npc(
    owner: &ComposedFreshAdmission<'_, '_, '_>, actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId, key: &str,
) -> TestResult<oteryn_protocol_oteryn::world_spatial_entities::EntityRef> {
    use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
    let catalogue = crate::content::NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"), crate::content::npc_catalogue_preproduction_limits(),
    ).map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    owner.states.chat.install_npc_talk(std::sync::Arc::new(
        crate::gameplay_transport::npc_talk::NpcTalkBook::from_model(&model),
    ));
    owner.states.chat.admit(session, "Rotten Blood Hero", owner.owner_now());
    owner.states.chat.set_npc_service(session);
    let mut runtime = owner.runtime.lock().await;
    let spot = runtime.read_actor_position(actor)
        .map_err(|error|format!("Rotten NPC {key} actor position: {error:?}"))?.position();
    runtime.place_npcs(&NpcPlacementTable { npcs: vec![NpcPlacements {
        key: key.into(), placements: vec![NpcPlacement {
            floor: -i8::try_from(spot.floor)?, x: u16::try_from(spot.x + 1)?,
            y: u16::try_from(spot.y)?, direction: MovementFacing::South,
        }],
    }] }, |_| true, true)
        .map_err(|error|format!("Rotten NPC {key} placement beside carrier position {spot:?}: {error:?}"))?;
    let placed = runtime.visible_entities().npcs.first().copied().ok_or("Bloodshade placement")?;
    Ok(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
        identity: placed.actor.placement_identity(), generation: placed.generation,
    })
}

pub(super) async fn talk(
    owner: &ComposedFreshAdmission<'_, '_, '_>, actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef, number: u64, text: &str,
) -> TestResult {
    use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
    use oteryn_protocol_oteryn::npc_service::{NpcTalkIntent, NpcIntentDisposition};
    let fence = current(owner, session).await?;
    let result = owner.npc_talk(actor, UseCommand {
        game_session_id: session, command_id: number, item_fence: Some(fence.item_fence()),
    }, NpcTalkIntent { npc_actor: npc, text: text.into() }).await;
    assert!(!result.unknown, "command {number}, NPC {npc:?}, text {text:?}: NPC outcome must be retained");
    assert_eq!(result.disposition, NpcIntentDisposition::Ok, "command {number}, NPC {npc:?}, text {text:?}");
    Ok(())
}
