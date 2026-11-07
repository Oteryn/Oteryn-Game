//! Actual NativeGameplay policies and native lethal callbacks for Spider/Bored.
use super::world_change_kills::{activate, capture_creature, seed_epoch};
use super::*;
use crate::foundation::{CharacterLease, MovementLocalPosition};
use crate::gameplay_transport::kill_reward::DrainOutcome;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_change_kills as rules;
use std::sync::Arc;

struct Allow;
impl crate::durability::npc_travel::NpcTravelGate for Allow {
    async fn travel_permitted(
        &self,
        _tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
        _account: [u8; 16],
    ) -> Result<bool, crate::durability::DurabilityError> {
        Ok(true)
    }
}

#[test]
fn wvar_spider_bored_native_inventory_kills_progression_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("WVAR_SPIDER_BORED_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Spider/Bored native PG thread")??;
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        let started = std::time::Instant::now();
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "wvar_spider_bored", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let world = deps.admitted.world_id;
        let epoch = (1..1_000_000)
            .find(|e| {
                crate::durability::world_event::changes::roll(world, *e, 0).is_ok_and(|d| {
                    d.variant("spider_nest") == Some(0)
                        && d.variant("bored").is_some()
                        && d.variant("warpath").is_some()
                })
            })
            .ok_or("joint Spider/Bored epoch")?;
        seed_epoch(
            &h,
            world,
            epoch,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
        )
        .await?;
        let definitions = oteryn_gameplay_world_change::variants::definitions()?;
        let spider = definitions
            .iter()
            .find(|d| d.key == "spider_nest")
            .ok_or("Spider recipe")?;
        let variant = spider.variants.first().ok_or("Spider variant")?;
        let entry = variant
            .portals
            .iter()
            .find(|p| !p.exit)
            .ok_or("Spider entry")?;
        let exit = variant
            .portals
            .iter()
            .find(|p| p.exit)
            .ok_or("Spider return")?;
        let spawn = variant.spawns.first().ok_or("Mamma source spawn")?.position;
        let origin = MovementLocalPosition {
            x: i32::from(spawn.0),
            y: i32::from(spawn.1),
            floor: i16::from(spawn.2),
        };
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            [8; 32],
            (origin.x, origin.y, origin.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            pin,
        )
        .map_err(debug)?;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        inventory(&mut runtime, origin, native.reward_table())?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        deps.runtime = Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.reset_epoch = epoch;
        owner.quest_catalogue = Some(Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.runtime_actor = Some(deps.actor);
        let session = admitted.game_session_id;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        admitted.item_fence = Some(fence.item_fence());
        owner
            .runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                session,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        activate(
            &h,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
        )
        .await?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence.item_fence(),
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;
        owner
            .states
            .chat
            .admit(session, "WVAR native tester", owner.owner_now());
        neutral_level(&owner, session).await?;
        let initial_xp = xp(&h).await?;
        assert_eq!(
            passage(&owner, session, epoch, entry.position).await?,
            Some(entry.destination)
        );
        for n in 0..3 {
            let (target, proof) = capture_creature(
                &owner,
                deps.actor,
                session,
                700 + n,
                rules::SPIDER_NEST.creature,
            )
            .await?;
            if n == 0 {
                fences(&owner, session, proof.clone()).await?;
            }
            assert_eq!(
                owner.drain_kill_rewards(session).await,
                DrainOutcome::Drained
            );
            assert_eq!(
                track(&h, rules::SPIDER_NEST.track).await?,
                i64::try_from(n + 1)?
            );
            owner
                .runtime
                .lock()
                .await
                .remove_encounter_actor(target)
                .map_err(debug)?;
        }
        assert_eq!(
            xp(&h).await? - initial_xp,
            6600,
            "three actual native Mamma rewards"
        );
        assert_eq!(achievement(&h, rules::SPIDER_NEST.achievement).await?, 1);
        assert_eq!(passage(&owner, session, epoch, entry.position).await?, None);
        assert_eq!(
            passage(&owner, session, epoch, exit.position).await?,
            Some(exit.destination)
        );
        let before_wyda_xp = xp(&h).await?;
        let before_wyda_corpses = minted_corpses(&h, "oteryn:item.tibia.i5977").await?;
        let native_rewards = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or("native reward generation")?
            .reward_table();
        assert_eq!(
            native_rewards
                .row(rules::BORED.creature)
                .map_err(|error| format!("Wyda native reward row: {error:?}"))?
                .xp_amount
                .get(),
            12
        );
        assert_eq!(
            native_rewards
                .row(rules::BORED.creature)
                .map_err(|error| format!("Wyda native reward row: {error:?}"))?
                .loot_table
                .entries
                .len(),
            2
        );
        assert_eq!(
            native_rewards
                .row(rules::BIBBY.creature)
                .map_err(|error| format!("Bibby native reward row: {error:?}"))?
                .xp_amount
                .get(),
            1500
        );
        let target = capture_wyda_top_damage(&owner, deps.actor, session).await?;
        assert_eq!(
            owner.drain_kill_rewards(session).await,
            DrainOutcome::Drained
        );
        assert_eq!(
            xp(&h).await? - before_wyda_xp,
            9,
            "source Wyda XP12 apportioned by actual15of20 top damage"
        );
        assert_eq!(
            minted_corpses(&h, "oteryn:item.tibia.i5977").await? - before_wyda_corpses,
            1,
            "actual Wyda corpse/loot materialization"
        );
        owner
            .runtime
            .lock()
            .await
            .remove_encounter_actor(target)
            .map_err(debug)?;
        assert_eq!(track(&h, rules::BORED.track).await?, 1);
        assert_eq!(achievement(&h, rules::BORED.achievement).await?, 1);
        let notices = owner.states.chat.drain(session);
        assert_eq!(notices.iter().filter(|line|matches!(line,oteryn_protocol_oteryn::chat::ChatLine::Local{mode:oteryn_protocol_oteryn::chat::ChatSpeechMode::MonsterSay,text,..} if text=="It seems this was just an illusion.")).count(),1,"one source illusion speech despite duplicate capture");
        assert_eq!(
            owner.drain_kill_rewards(session).await,
            DrainOutcome::Drained
        );
        assert!(
            owner.states.chat.drain(session).is_empty(),
            "settlement replay cannot repeat notice"
        );
        // A separate fully credited death proves the source's entire 12 XP.
        // The earlier15/20 case preserves D52: a departed foreign contributor's
        // quarter is discarded, rather than reassigned to the top-damage winner.
        let before_solo_xp = xp(&h).await?;
        let before_solo_corpses = minted_corpses(&h, "oteryn:item.tibia.i5977").await?;
        let (solo_wyda, _) =
            capture_creature(&owner, deps.actor, session, 713, rules::BORED.creature).await?;
        assert_eq!(
            owner.drain_kill_rewards(session).await,
            DrainOutcome::Drained
        );
        assert_eq!(
            xp(&h).await? - before_solo_xp,
            12,
            "source full solo Wyda XP"
        );
        assert_eq!(
            minted_corpses(&h, "oteryn:item.tibia.i5977").await? - before_solo_corpses,
            1
        );
        replay_native_source(&owner, deps.actor, session, solo_wyda).await?;
        assert_eq!(
            owner.drain_kill_rewards(session).await,
            DrainOutcome::Drained
        );
        assert_eq!(
            xp(&h).await? - before_solo_xp,
            12,
            "settled native source replay adds no XP"
        );
        assert_eq!(
            minted_corpses(&h, "oteryn:item.tibia.i5977").await? - before_solo_corpses,
            1,
            "settled native source replay adds no corpse"
        );
        let solo_notices = owner.states.chat.drain(session);
        assert_eq!(solo_notices.iter().filter(|line|matches!(line,oteryn_protocol_oteryn::chat::ChatLine::Local{mode:oteryn_protocol_oteryn::chat::ChatSpeechMode::MonsterSay,text,..} if text=="It seems this was just an illusion.")).count(),1,"one notice for solo death including settled source replay");
        assert_eq!(track(&h, rules::BORED.track).await?, 1);
        assert_eq!(achievement(&h, rules::BORED.achievement).await?, 1);
        owner
            .runtime
            .lock()
            .await
            .remove_encounter_actor(solo_wyda)
            .map_err(debug)?;
        let before_bibby_xp = xp(&h).await?;
        let before_bibby_corpses = minted_corpses(&h, "oteryn:item.tibia.i6008").await?;
        let (bibby, _) =
            capture_creature(&owner, deps.actor, session, 712, rules::BIBBY.creature).await?;
        assert_eq!(
            owner.drain_kill_rewards(session).await,
            DrainOutcome::Drained
        );
        owner
            .runtime
            .lock()
            .await
            .remove_encounter_actor(bibby)
            .map_err(debug)?;
        assert_eq!(
            xp(&h).await? - before_bibby_xp,
            1500,
            "source Bibby XP via native reward owner"
        );
        assert_eq!(
            minted_corpses(&h, "oteryn:item.tibia.i6008").await? - before_bibby_corpses,
            1,
            "actual Bibby corpse/loot materialization"
        );
        assert_eq!(track(&h, rules::BIBBY.track).await?, 1);
        assert_eq!(achievement(&h, rules::BIBBY.achievement).await?, 1);

        let (ordinary, ordinary_proof) = super::world_change_kills::capture_native(
            &owner,
            deps.actor,
            session,
            711,
            "oteryn:creature.giant_spider",
        )
        .await?;
        assert!(
            ordinary_proof.is_none(),
            "ordinary Giant Spider has no WVAR credit"
        );
        assert_eq!(
            owner.drain_kill_rewards(session).await,
            DrainOutcome::Drained
        );
        owner
            .runtime
            .lock()
            .await
            .remove_encounter_actor(ordinary)
            .map_err(debug)?;
        assert_eq!(track(&h, rules::BORED.track).await?, 1);
        assert_eq!(achievement(&h, rules::BORED.achievement).await?, 1);
        // End the selected episode through another real reset decision. The
        // permanent return remains even when the entry is sealed again.
        let ended = (epoch + 1..epoch + 100_000)
            .find(|e| {
                crate::durability::world_event::changes::roll(world, *e, 0)
                    .is_ok_and(|d| d.variant("spider_nest") == Some(1))
            })
            .ok_or("sealed epoch")?;
        seed_epoch(
            &h,
            world,
            ended,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
        )
        .await?;
        assert_eq!(passage(&owner, session, ended, entry.position).await?, None);
        assert_eq!(
            passage(&owner, session, ended, exit.position).await?,
            Some(exit.destination)
        );
        eprintln!(
            "WVAR_SPIDER_BORED POSTGRES_CASE_COMPLETED native42policy-health-reward-rows,three1800hp,6600xp,nestling,entrylimit,permanentescape,wyda20hp,wyda12xp,bibby1500xp,native-corpse-materialization,achievement,current-fences duration={:?}",
            started.elapsed()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

fn inventory(
    runtime: &mut ChannelRuntimeV1,
    origin: MovementLocalPosition,
    rewards: &crate::content::creature_reward::CreatureRewardTable,
) -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let mut keys = std::collections::BTreeSet::new();
    for name in ["variants.json", "variants-extra.json"] {
        let doc: serde_json::Value = serde_json::from_slice(&std::fs::read(
            root.join("content/encounters/world-changes").join(name),
        )?)?;
        for definition in doc.as_array().ok_or("definitions")? {
            for variant in definition["variants"].as_array().ok_or("variants")? {
                for spawn in variant["spawns"].as_array().ok_or("spawns")? {
                    keys.insert(spawn["creature"].as_str().ok_or("creature")?.to_owned());
                }
            }
        }
    }
    // Dynamic source populations use these same native identities through the
    // generic overlay owner; they must meet the same admission as recipe spawns.
    for key in [
        "elf",
        "elf_scout",
        "elf_arcanist",
        "elf_overseer",
        "thornfire_wolf",
    ] {
        keys.insert(format!("oteryn:creature.{key}"));
    }
    let closure: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("tools/content-schema/native-gameplay/wvar-reward-closure.json"),
    )?)?;
    let source_keys = closure["recipe_creature_keys"]
        .as_array()
        .ok_or("native source closure keys")?
        .iter()
        .map(|key| {
            key.as_str()
                .map(str::to_owned)
                .ok_or("native source closure identity")
        })
        .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
    assert_eq!(
        keys, source_keys,
        "every authored/dynamic population has a source-bound closure row"
    );
    assert_eq!(keys.len(), 42);
    for key in keys {
        let policy = runtime
            .companion_policy(&key)
            .map_err(|error| format!("native policy admission {key}: {error:?}"))?;
        let hp = policy.maximum_health;
        assert!(hp > 0);
        // Real Elf HP is100; use the installed native policy directly instead
        // of a fixture health constant, then read the committed actor back.
        rewards
            .row(&key)
            .map_err(|error| format!("native reward admission {key}: {error:?}"))?;
        let actor = runtime
            .admit_encounter_actor(
                &key,
                hp,
                MovementLocalPosition {
                    x: origin.x + 2,
                    ..origin
                },
            )
            .map_err(debug)?;
        runtime
            .install_creature_policy(actor, &key)
            .map_err(debug)?;
        let live = runtime.companion_snapshot(actor).map_err(debug)?;
        assert_eq!(live.health, hp);
        assert_eq!(live.state.policy.definition_key, key);
        if key == rules::SPIDER_NEST.creature {
            assert_eq!(hp, 1800);
        }
        if key == rules::BORED.creature {
            assert_eq!(hp, 20);
        }
        runtime.remove_encounter_actor(actor).map_err(debug)?;
    }
    eprintln!(
        "WVAR_NATIVE_ADMISSION_INVENTORY_EXECUTED creatures=42 actual-native-policy-health-and-reward-rows"
    );
    Ok(())
}
async fn fences(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    proof: crate::durability::quest_state::world_change_kills::NativeWorldChangeKill,
) -> TestResult {
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let permit = owner.spell_lane_permit().await.ok_or("permit")?;
    for mode in 0..3 {
        let mut stale = fence;
        if mode == 0 {
            stale.connection_generation =
                crate::foundation::ConnectionGeneration::new(stale.connection_generation.get() + 1)
                    .map_err(debug)?;
        }
        if mode == 1 {
            stale.character_lease_generation += 1;
        }
        if mode == 2 {
            stale.scope_ownership_generation = crate::foundation::ScopeOwnershipGeneration::new(
                stale.scope_ownership_generation.get() + 1,
            )
            .map_err(debug)?;
        }
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        assert!(
            slot.commit_world_change_kill(
                owner.root,
                owner.character,
                owner.holder,
                stale,
                Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?),
                &permit,
                proof.clone()
            )
            .await
            .is_err(),
            "stale fence{mode}"
        );
    }
    Ok(())
}
async fn passage(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    epoch: u64,
    position: oteryn_gameplay_world_change::variants::Position,
) -> TestResult<Option<oteryn_gameplay_world_change::variants::Position>> {
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    owner
        .root
        .world_change_passage(
            owner.character,
            owner.holder,
            fence,
            epoch,
            position,
            &Allow,
        )
        .await
        .map_err(debug)
}
async fn track(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),0)").bind(id(41).as_slice()).bind(key).fetch_one(&h.pool).await?)
}
async fn achievement(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1",
    )
    .bind(key)
    .fetch_one(&h.pool)
    .await?)
}
async fn xp(h: &Harness) -> TestResult<i64> {
    let value:String=sqlx::query_scalar("SELECT total_experience::text FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    Ok(value.parse()?)
}

async fn replay_native_source(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    target: ExactActorRef,
) -> TestResult {
    let queue = owner.states.attack.lock().await.kills().clone();
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let mut runtime = owner.runtime.lock().await;
    // A replay preserves the original event clock. A new wall-clock observation
    // would describe a different party-XP census and correctly fail its receipt.
    let death_ms = runtime
        .borrow_combat_death()
        .projected_death_at_ms(target)
        .map_err(debug)?
        .ok_or("retained native death clock")?;
    let observed = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(
        death_ms
            .checked_mul(1000)
            .ok_or("native death clock overflow")?,
    );
    let lease = runtime
        .bound_attacker_lease(actor, session)
        .map_err(debug)?;
    crate::gameplay_transport::kill_reward::record_creature_kill(
        &mut runtime,
        crate::gameplay_transport::kill_reward::KillSink {
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
        observed.get() / 1000,
        Some(observed),
    );
    assert!(
        !queue.has_session(session),
        "finally settled exact native death cannot requeue on source replay"
    );
    Ok(())
}

// The second physical attacker is only a lower-damage lethal contributor. It
// receives no durable authority; the real current principal must win the credit.
async fn capture_wyda_top_damage(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult<ExactActorRef> {
    owner.refresh_quest_session(session, false).await;
    let queue = owner.states.attack.lock().await.kills().clone();
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    let other_session = GameSessionId::decode(&id(90)).map_err(debug)?;
    let target;
    {
        let mut runtime = owner.runtime.lock().await;
        let at = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        let reservation = runtime
            .reserve_fresh_session(other_session)
            .map_err(debug)?;
        let other = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(other, MovementLocalPosition { x: at.x - 1, ..at })
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                other,
                other_session,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(91)).map_err(debug)?,
                    1,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        let key = rules::BORED.creature;
        let policy = runtime.companion_policy(key).map_err(debug)?;
        assert_eq!(policy.maximum_health, 20);
        target = runtime
            .admit_encounter_actor(
                key,
                policy.maximum_health,
                MovementLocalPosition { x: at.x + 1, ..at },
            )
            .map_err(debug)?;
        runtime
            .install_creature_policy(target, key)
            .map_err(debug)?;
        for (attacker, acting_session, damage, command) in
            [(actor, session, 15, 710), (other, other_session, 5, 711)]
        {
            runtime
                .borrow_exact_actor_commit_at(now)
                .commit_damage_for_bound_attacker(
                    target,
                    attacker,
                    crate::foundation::CommandRef::new(
                        acting_session,
                        crate::foundation::CommandId::new(command).map_err(debug)?,
                    ),
                    0,
                    crate::foundation::OwnerDamageCommand {
                        target: key.as_bytes(),
                        occurrence: acting_session.as_bytes(),
                        binding: b"wvar-top-damage-only",
                        damage,
                    },
                )
                .map_err(debug)?;
        }
        let last = runtime
            .bound_attacker_lease(other, other_session)
            .map_err(debug)?;
        for _ in 0..2 {
            crate::gameplay_transport::kill_reward::record_creature_kill(
                &mut runtime,
                crate::gameplay_transport::kill_reward::KillSink {
                    prey: None,
                    activity: &[],
                    queue: &queue,
                    rewards: native.reward_table(),
                    charms: None,
                },
                target,
                crate::combat::CapturedRewardPrincipal {
                    character: last.character_id(),
                    lease_generation: last.generation(),
                    session: other_session,
                    actor: other,
                },
                now,
                Some(observed),
            );
        }
        // This secondary physical attacker deliberately has no durable admission.
        // Depart it before settlement: D52 excludes gone contributors, while the
        // retained sealed damage still proves that the current winner was15/20.
        runtime
            .remove_terminal_session(other_session, other)
            .map_err(debug)?;
    }
    assert!(
        queue.has_session(session),
        "actual most-damage principal owns the queued credit"
    );
    assert!(
        !queue.has_session(other_session),
        "lower-damage lethal attacker cannot take the credit"
    );
    Ok(target)
}

// Native XP owner establishes level50 before measurement, where the existing
// Global low-level hunting bonus is zero. No raw progression or Quest writes.
async fn neutral_level(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
) -> TestResult {
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
    };
    let binding = owner
        .progression_binding(session)
        .ok_or("native progression binding")?;
    let target = binding
        .policy
        .thresholds
        .iter()
        .find(|t| t.level == 50)
        .ok_or("level50 native threshold")?
        .minimum_experience
        .get();
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let before = owner
        .root
        .read_character_progression(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("progression")?;
    let amount = target
        .checked_sub(before.total_experience.get())
        .ok_or("level50 delta")?;
    if amount > 0 {
        let outcome = owner
            .revision_sequencer
            .acquire(fence.character_id)
            .await
            .commit_experience(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                ExperienceAwardRequest {
                    occurrence: ExperienceRewardOccurrence::from_bytes(id(230)).map_err(debug)?,
                    amount: oteryn_simulation_determinism::ExactI64::new(amount),
                    context: binding.context.clone(),
                    policy_revision: binding.policy_revision.clone(),
                    reward_revision: binding.reward_revision.clone(),
                    policy: binding.policy.clone(),
                },
                None,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, ExperienceCommitOutcome::Committed(_)));
    }
    Ok(())
}

async fn minted_corpses(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_item_mint_receipts r JOIN game_item_instances i USING(item_instance_id) WHERE r.loot_purpose_key='CORPSE_MATERIALIZATION' AND i.definition_production_key=$1 AND r.materialized_at IS NOT NULL").bind(key).fetch_one(&h.pool).await?)
}
