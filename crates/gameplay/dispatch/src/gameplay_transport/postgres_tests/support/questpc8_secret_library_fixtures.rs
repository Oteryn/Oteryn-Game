use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use super::fixture::{Harness, debug, id};

use super::questpc8_authored_programmes_fixtures::{object_target, step, use_target};

use super::{TestResult, quest_world_callbacks_fixtures as quest_world_callbacks};

use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, FreshAdmissionAuthority},
};

use crate::map::overlay::TilePos;

use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

use oteryn_protocol_oteryn::world_object::UseDisposition;

use oteryn_protocol_oteryn::world_spatial::StepDirection;

use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

#[path = "questpc8_secret_library/world.rs"]
mod fixture_world;

pub(super) fn run_library_journey() -> TestResult {
    library_journey(false)
}

pub(super) fn library_journey(past_oberon_objective: bool) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-secret-library", move || {
        quest_world_callbacks::scenario(
            if past_oberon_objective {
                "authored-secret-library-completed-oberon"
            } else {
                "authored-secret-library"
            },
            Position {
                x: 32031,
                y: 32463,
                floor: -7,
            },
            false,
        )
    })
}

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    fixture_world::world()
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    past_oberon_objective: bool,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[3],
        )
        .ok_or("native NPC capability")?;
    assert_eq!(admitted.continuity.selected_capabilities.as_slice(), &[3]);
    let premium = owner.premium_coordinator.take();
    let mut command = 1;
    raise_level(owner, admitted, 249, 201).await?;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    assert_eq!(progress(h, "stage").await?, -1);
    raise_level(owner, admitted, 250, 202).await?;
    step(owner, admitted, command, StepDirection::West).await?;
    command += 1;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    assert_eq!(
        progress(h, "stage").await?,
        -1,
        "premium remains required at level250"
    );
    owner.premium_coordinator = premium;
    owner.admit_premium(id(40), admitted.game_session_id);
    step(owner, admitted, command, StepDirection::West).await?;
    command += 1;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    assert_eq!(progress(h, "stage").await?, 0);
    let [charles, spectulus, gail] = install_npcs(owner, admitted).await?;
    travel(owner, admitted, 32529, 32782, 6).await?;
    say(owner, admitted, charles, &mut command, &["hi", "raft"]).await?;
    assert_eq!(progress(h, "stage").await?, 1);
    travel(owner, admitted, 32030, 32463, 7).await?;
    assert_eq!(
        claim(owner, admitted, &mut command, 32031, 32463, -7, 0).await?,
        UseDisposition::Rejected,
        "hawser is required"
    );
    carry(owner, admitted, 28707, 501).await?;
    assert_eq!(
        claim(owner, admitted, &mut command, 32031, 32463, -7, 0).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        quantity(h, 28707).await?,
        0,
        "raft binds and consumes the owned hawser"
    );
    assert_eq!(progress(h, "stage").await?, 2);
    travel(owner, admitted, 33268, 31834, 1).await?;
    say(owner, admitted, spectulus, &mut command, &["hi", "rumours"]).await?;
    assert_eq!(progress(h, "stage").await?, 3);
    travel(owner, admitted, 33524, 31464, 14).await?;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    assert_eq!(progress(h, "stage").await?, 4);
    travel(owner, admitted, 32807, 32764, 10).await?;
    assert_eq!(
        claim(owner, admitted, &mut command, 32807, 32765, -10, 0).await?,
        UseDisposition::Rejected,
        "missing ebony wood cannot assemble an instrument"
    );
    carry(owner, admitted, 28491, 502).await?;
    assert_eq!(
        claim(owner, admitted, &mut command, 32808, 32765, -10, 1).await?,
        UseDisposition::Rejected,
        "hair station cannot precede fingerboard"
    );
    assert_eq!(
        claim(owner, admitted, &mut command, 32807, 32765, -10, 0).await?,
        UseDisposition::Committed
    );
    assert_eq!(quantity(h, 28491).await?, 0);
    assert_eq!(quantity(h, 28492).await?, 1);
    refresh(owner, admitted).await;
    let target = object_target(
        owner,
        TilePos {
            x: 32808,
            y: 32765,
            floor: -10,
        },
        1,
    )?;
    assert_eq!(
        use_target(owner, admitted, command, &target).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        use_target(owner, admitted, command, &target).await?,
        UseDisposition::Committed,
        "same source conversion replays once"
    );
    command += 1;
    assert_eq!(quantity(h, 28492).await?, 0);
    assert_eq!(quantity(h, 28493).await?, 1);
    assert_eq!(progress(h, "stage").await?, 6);
    travel(owner, admitted, 32671, 32736, 11).await?;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    let denied = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    assert_eq!(
        (denied.x, denied.y, denied.floor),
        (32671, 32736, 11),
        "unearned native entrance returns to the source approach"
    );
    assert_eq!(
        progress(h, "access").await?,
        -1,
        "instrument alone does not grant library access"
    );
    travel(owner, admitted, 32621, 32737, 6).await?;
    refresh(owner, admitted).await;
    super::qcx_the_secret_library_fixtures::assert_gail_reply(owner, admitted, gail, command).await?;
    command += 2;
    assert_eq!(progress(h, "stage").await?, 7);
    travel(owner, admitted, 32671, 32736, 11).await?;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    let denied = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    assert_eq!(
        (denied.x, denied.y, denied.floor),
        (32671, 32736, 11),
        "unearned native entrance returns to the source approach"
    );
    assert_eq!(
        progress(h, "access").await?,
        -1,
        "Gail report does not replace Oberon death"
    );
    assert!(oberon_achievements(h, 41).await?.is_empty());
    travel(owner, admitted, 33364, 31318, 9).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let creature = "oteryn:creature.grand_master_oberon";
    let actors = owner
        .quest_guardians_for_test("oteryn:quest-death/authored-secret-library-oberon")
        .await;
    assert_eq!(actors.len(), 1);
    let target = actors[0];
    exercise_oberon_debates(owner, admitted, target).await?;
    use crate::gameplay_transport::kill_reward::DrainOutcome;
    if past_oberon_objective {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,quest_key,track_key,value,committed_character_revision)
            SELECT character_id,'oteryn:quest.the_secret_library_quest','oteryn:quest-progress/chosen/secret-library/kills',1,character_revision
            FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid
            ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value")
            .bind(id(41).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let source = super::house_transfer_fixtures::RecoverySource::start(h).await?;
        let peers = &source.evidence;
        let admit = super::boss_rewards_fixtures::privacy::admit_gameplay_peer;
        let peer = admit(h, peers, 1).await?;
        let departing = admit(h, peers, 2).await?;
        let peer_actor = oberon_peer_damage(owner, admitted, target, command, creature, peer).await?;
        owner
            .runtime
            .lock()
            .await
            .record_control_loss(
                peer_actor,
                peer,
                crate::foundation::ControlLossMark {
                    epoch: 1,
                    grace_deadline: i64::try_from(owner.owner_now().get())? + 30_000,
                },
            )
            .map_err(debug)?;
        oberon_peer_damage(owner, admitted, target, command, creature, departing).await?;
        // Fail the first write for a different recipient. The first recipient's
        // two requests must be invisible until the entire roster commits.
        sqlx::raw_sql(
            "CREATE FUNCTION reject_second_oberon_recipient() RETURNS trigger LANGUAGE plpgsql AS $$
             BEGIN
                 IF NEW.source_kind = 'oteryn:oberon-death' AND EXISTS (
                     SELECT 1 FROM game_account_achievement_grant_requests
                     WHERE source_kind = NEW.source_kind AND character_id <> NEW.character_id
                 ) THEN RAISE EXCEPTION 'injected second Oberon recipient failure'; END IF;
                 RETURN NEW;
             END $$;
             CREATE TRIGGER reject_second_oberon_recipient BEFORE INSERT ON
                 game_account_achievement_grant_requests FOR EACH ROW
                 EXECUTE FUNCTION reject_second_oberon_recipient()",
        )
        .execute(&h.pool)
        .await?;
        super::grizzly_npc_fixtures::questpc8_campaign::queue_lethal(
            owner, admitted, target, creature, command, 1,
        )
        .await?;
        owner
            .runtime
            .lock()
            .await
            .restore_control(peer_actor, peer, 1)
            .map_err(debug)?;
        assert_eq!(
            owner.drain_kill_rewards(admitted.game_session_id).await,
            DrainOutcome::Deferred
        );
        assert!(oberon_achievements(h, 41).await?.is_empty());
        assert!(oberon_achievements(h, 101).await?.is_empty());
        sqlx::query("DROP TRIGGER reject_second_oberon_recipient ON game_account_achievement_grant_requests")
            .execute(&h.pool).await?;
        terminalize_oberon_peer(h, departing).await?;
        assert!(
            owner
                .current_quest_fence(departing)
                .await
                .map_err(|()| "peer fence")?
                .is_none()
        );
        assert_eq!(
            owner.drain_kill_rewards(admitted.game_session_id).await,
            DrainOutcome::Drained
        );
        assert!(oberon_achievements(h, 102).await?.is_empty());
    } else {
        // Leave the real quest descendant unresolved at the death's commit boundary.
        sqlx::raw_sql(
            "CREATE FUNCTION reject_oberon_objective() RETURNS trigger LANGUAGE plpgsql AS $$
             BEGIN
                 IF NEW.track_key = 'oteryn:quest-progress/chosen/secret-library/kills'
                    AND NEW.value = 1
                 THEN RAISE EXCEPTION 'injected Oberon objective failure'; END IF;
                 RETURN NEW;
             END $$;
             CREATE TRIGGER reject_oberon_objective BEFORE INSERT OR UPDATE ON
                 game_character_quest_tracks FOR EACH ROW EXECUTE FUNCTION reject_oberon_objective()",
        )
        .execute(&h.pool)
        .await?;
        super::grizzly_npc_fixtures::questpc8_campaign::queue_lethal(
            owner, admitted, target, creature, command, 1,
        )
        .await?;
        assert_eq!(
            owner.drain_kill_rewards(admitted.game_session_id).await,
            DrainOutcome::Deferred
        );
        assert!(progress(h, "kills").await? < 1);
        assert!(
            oberon_achievements(h, 41).await?.is_empty(),
            "a deferred death exposed its achievements"
        );
        sqlx::query("DROP TRIGGER reject_oberon_objective ON game_character_quest_tracks")
            .execute(&h.pool)
            .await?;
        assert_eq!(
            owner.drain_kill_rewards(admitted.game_session_id).await,
            DrainOutcome::Drained
        );
    }
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(progress(h, "kills").await?, 1);
    let expected = [
        "oteryn:achievement/master_debater",
        "oteryn:achievement/millennial_falcon",
    ]
    .map(str::to_owned);
    let principal = oberon_achievements(h, 41).await?;
    if past_oberon_objective {
        assert_eq!(
            (
                principal.as_slice(),
                oberon_achievements(h, 101).await?.as_slice()
            ),
            (expected.as_slice(), expected.as_slice()),
            "principal has a completed objective and peer in reconnect grace at death has no quest progress"
        );
    } else {
        assert_eq!(principal, expected);
    }
    travel(owner, admitted, 32671, 32736, 11).await?;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    let position = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    assert_eq!(
        (position.x, position.y, position.floor),
        (32480, 32597, 15),
        "earned access operates the original native library descent"
    );
    travel(owner, admitted, 32479, 32597, 15).await?;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    assert_eq!(progress(h, "stage").await?, 8);
    assert_eq!(progress(h, "access").await?, 1);
    owner
        .states
        .quest_sessions
        .lock()
        .map_err(|_| "quest cache")?
        .remove(&admitted.game_session_id);
    owner.admit_quest_session(admitted).await;
    step(owner, admitted, command, StepDirection::West).await?;
    command += 1;
    step(owner, admitted, command, StepDirection::East).await?;
    assert_eq!(
        quantity(h, 28493).await?,
        1,
        "final access and cold replay do not duplicate the instrument"
    );
    let completed:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.the_secret_library_quest'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed);
    Ok(())
}

async fn exercise_oberon_debates(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    boss: crate::foundation::ExactActorRef,
) -> TestResult {
    use super::srx3_native_source_cast_fixtures::{
        NativeOwnerClock, next_native_melee_deadline, settle_monster_melee_at_until,
    };
    use crate::foundation::OwnerDamageCommand;
    use oteryn_gameplay_encounters::bosses::source_callbacks::oberon_think::{
        OBERON_KEY, OBERON_LIVES, OBERON_SUMMONS,
    };
    use oteryn_protocol_oteryn::chat::{ChatDisposition, ChatIntent, ChatSpeechMode};

    const OBERON_DEBATE: [(&str, &str); 9] = [
        (
            "You appear like a worm among men!",
            "How appropriate, you look like something worms already got the better of!",
        ),
        (
            "The world will suffer for its idle laziness!",
            "Are you ever going to fight or do you prefer talking!",
        ),
        (
            "People fall at my feet when they see me coming!",
            "Even before they smell your breath?",
        ),
        (
            "This will be the end of mortal men!",
            "Then let me show you the concept of mortality before it!",
        ),
        (
            "I will remove you from this plane of existence!",
            "Too bad you barely exist at all!",
        ),
        (
            "Dragons will soon rule this world, I am their herald!",
            "Excuse me but I still do not get the message!",
        ),
        (
            "The true virtues of chivalry are my belief!",
            "Dare strike up a Minnesang and you will receive your last accolade!",
        ),
        (
            "I lead the most honourable and formidable following of knights!",
            "Then why are we fighting alone right now?",
        ),
        ("ULTAH SALID'AR, ESDO LO!", "SEHWO ASIMO, TOLIDO ESD"),
    ];

    let speaker = admitted.runtime_actor.ok_or("Oberon challenger")?;
    let session = admitted.game_session_id;
    let facts = &owner.bundle_map().ok_or("native Oberon room")?.facts;
    let clock = NativeOwnerClock::new(owner);
    let mut now = owner.owner_now().get();
    owner
        .states
        .chat
        .admit_as(session, "Challenger", Some(id(41)), owner.owner_now());
    let maximum = owner
        .runtime
        .lock()
        .await
        .companion_snapshot(boss)
        .map_err(debug)?
        .maximum_health;
    // Fandom describes approximately 3,000 HP out of 30,000.
    let red = maximum / 10;
    let strike = async |occurrence: &str, damage| -> TestResult<i64> {
        let mut binding = occurrence.as_bytes().to_vec();
        binding.push(0);
        let mut runtime = owner.runtime.lock().await;
        let result = runtime
            .borrow_exact_actor_commit_at(owner.owner_now().get() / 1000)
            .commit_damage(
                boss,
                OwnerDamageCommand {
                    target: OBERON_KEY.as_bytes(),
                    occurrence: occurrence.as_bytes(),
                    binding: &binding,
                    damage,
                },
            )
            .map_err(debug)?;
        Ok(result.health_after)
    };
    let mut taunts = std::collections::BTreeSet::new();
    for phase in 1..=OBERON_LIVES {
        if phase == 1 {
            let mut runtime = owner.runtime.lock().await;
            runtime.set_test_creature_health(boss, red).map_err(debug)?;
        } else {
            assert_eq!(
                strike(&format!("oberon-enter-red-{phase}"), maximum).await?,
                red,
                "lethal burst must stop at red HP until the pending debate starts"
            );
        }
        let cursor = facts
            .movement_speech_since(0)
            .last()
            .map_or(0, |entry| entry.0);
        let mut healed = false;
        for _ in 0..6 {
            now = next_native_melee_deadline(owner).await?.max(now + 1);
            healed = settle_monster_melee_at_until(owner, now, async || {
                owner.runtime.lock().await.live_creature_health(boss) == Some(maximum as u64)
            })
            .await?;
            if healed {
                break;
            }
        }
        assert!(
            healed,
            "reserved native think must heal Oberon from red HP to full"
        );
        let (taunt, answer) = OBERON_DEBATE
            .into_iter()
            .find(|(taunt, _)| {
                facts
                    .movement_speech_since(cursor)
                    .iter()
                    .any(|(_, line)| line.speaker == boss && line.text == *taunt)
            })
            .ok_or("native think must publish Oberon's taunt")?;
        assert!(taunts.insert(taunt), "each phase must ask a new question");
        {
            let runtime = owner.runtime.lock().await;
            let summons = runtime
                .visible_entities()
                .creatures
                .into_iter()
                .filter(|entry| {
                    OBERON_SUMMONS.iter().any(|key| {
                        runtime.matches_live_creature_identity(entry.actor, key.as_bytes())
                    })
                })
                .count();
            assert_eq!(summons, usize::from(phase), "one Falcon per debate");
        }
        assert_eq!(
            strike(&format!("oberon-immune-{phase}"), maximum).await?,
            maximum
        );
        clock.set(now);
        if phase == 1 {
            let wrong = ChatIntent::Say {
                mode: ChatSpeechMode::Say,
                text: "Wrong answer".into(),
            };
            assert_eq!(
                owner
                    .chat_speak(speaker, session, 1, &wrong)
                    .await
                    .disposition,
                ChatDisposition::Ok
            );
            assert!(
                facts
                    .movement_speech_since(cursor)
                    .iter()
                    .any(|(_, line)| line.speaker == boss && line.text == "HAHAHAHA!")
            );
            assert_eq!(strike("oberon-after-wrong-answer", maximum).await?, maximum);
        }
        let correct = ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: answer.into(),
        };
        assert_eq!(
            owner
                .chat_speak(speaker, session, u64::from(phase) + 1, &correct)
                .await
                .disposition,
            ChatDisposition::Ok
        );
        assert!(
            facts
                .movement_speech_since(cursor)
                .iter()
                .any(|(_, line)| line.speaker == boss && line.text == "GRRRAAANNGH!")
        );
        assert_eq!(
            strike(&format!("oberon-vulnerable-{phase}"), 1).await?,
            maximum - 1
        );
    }
    Ok(())
}

async fn oberon_peer_damage(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    target: crate::foundation::ExactActorRef,
    command: u64,
    creature: &str,
    session: crate::foundation::GameSessionId,
) -> TestResult<crate::foundation::ExactActorRef> {
    use crate::foundation::{
        CharacterId, CharacterLease, CommandId, CommandRef, OwnerDamageCommand,
    };
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "peer fence")?
        .ok_or("peer fence")?;
    let now = owner.owner_now().get() / 1000;
    let mut runtime = owner.runtime.lock().await;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    let peer = runtime.commit_fresh_session(reservation).map_err(debug)?;
    let at = runtime
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    runtime
        .initialize_pinned_test_position(peer, at)
        .map_err(debug)?;
    let lease = CharacterLease::new(
        CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
        fence.character_lease_generation,
    )
    .map_err(debug)?;
    runtime
        .bind_attacker_lease(peer, session, lease)
        .map_err(debug)?;
    runtime
        .borrow_exact_actor_commit_at(now)
        .commit_swing_damage_for_bound_attacker(
            target,
            peer,
            CommandRef::new(session, CommandId::new(command).map_err(debug)?),
            1,
            OwnerDamageCommand {
                target: creature.as_bytes(),
                occurrence: session.as_bytes(),
                binding: b"review-oberon",
                damage: 1,
            },
        )
        .map_err(debug)?;
    Ok(peer)
}

async fn terminalize_oberon_peer(
    h: &Harness,
    session: crate::foundation::GameSessionId,
) -> TestResult {
    use crate::durability::fresh_admission::{ExpiredLossReleaseV1, FreshAdmissionStore};

    let store = FreshAdmissionStore::from_root(h.root.clone());
    let current = store.current_session(session).await.map_err(debug)?;
    let transport = current.current_transport().ok_or("peer transport")?;
    let account: String = sqlx::query_scalar(
        "SELECT account_id::text FROM game_durability_reconnect_sessions \
         WHERE game_session_id=encode($1,'hex')::uuid",
    )
    .bind(session.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert!(matches!(
        store
            .release_logout_session(session, &account, transport)
            .await
            .map_err(debug)?,
        ExpiredLossReleaseV1::Released { .. }
    ));
    Ok(())
}

async fn oberon_achievements(h: &Harness, character: u8) -> TestResult<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT achievement_key FROM game_account_achievements
         WHERE account_id=(SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid)
         AND achievement_key IN ('oteryn:achievement/master_debater', 'oteryn:achievement/millennial_falcon')
         ORDER BY achievement_key",
    )
    .bind(id(character).as_slice())
    .fetch_all(&h.pool)
    .await?)
}

async fn carry(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: u16,
    occurrence: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: 32030,
                y: 32463,
                floor: -7,
            },
            item: format!("oteryn:item.tibia.i{item}"),
            count: 1,
        },
        occurrence,
        true,
    )
    .await
}

async fn progress(h: &Harness, suffix: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),-1)").bind(id(41).as_slice()).bind(format!("oteryn:quest-progress/chosen/secret-library/{suffix}")).fetch_one(&h.pool).await?)
}

async fn quantity(h: &Harness, item: u16) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.definition_production_key=$3 AND i.lifecycle=1").bind(id(41).as_slice()).bind(id(42).as_slice()).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}

async fn claim(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: &mut u64,
    x: u16,
    y: u16,
    floor: i8,
    ordinal: u8,
) -> TestResult<UseDisposition> {
    refresh(owner, admitted).await;
    let target = object_target(owner, TilePos { x, y, floor }, ordinal)?;
    let result = use_target(owner, admitted, *command, &target).await?;
    *command += 1;
    Ok(result)
}

async fn say(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    command: &mut u64,
    text: &[&str],
) -> TestResult {
    refresh(owner, admitted).await;
    let commands = text
        .iter()
        .enumerate()
        .map(|(offset, text)| {
            Ok((
                command
                    .checked_add(u64::try_from(offset)?)
                    .ok_or("command")?,
                *text,
            ))
        })
        .collect::<TestResult<Vec<_>>>()?;
    super::grizzly_npc_fixtures::wire::talk(owner, admitted, npc, &commands).await?;
    *command = command
        .checked_add(u64::try_from(text.len())?)
        .ok_or("command")?;
    Ok(())
}

// The live owner cadence reloads scheduled quest state. Explicitly perform that same
// durable read between fixture commands so cross-owner selection never reads an old copy.
async fn refresh(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &AdmittedSession) {
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
}

/// Fixture travel between genuine objective areas uses the existing sealed teleport
/// proof, as classic_quests does; it never seeds or advances any quest track.
async fn travel(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    x: i32,
    y: i32,
    floor: i16,
) -> TestResult {
    refresh(owner, admitted).await;
    let map = owner.bundle_map().ok_or("map")?;
    let mut runtime = owner.runtime.lock().await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let expected = runtime.read_actor_position(actor).map_err(debug)?;
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        &owner.movement_cells,
        actor,
        admitted.game_session_id,
        expected,
        crate::foundation::MovementLocalPosition { x, y, floor },
        false,
        &doors,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}

async fn install_npcs(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<[EntityRef; 3]> {
    use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
    use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
    use crate::gameplay_transport::npc_talk::NpcTalkBook;
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    let keys = [
        "oteryn:npc.charles",
        "oteryn:npc.spectulus",
        "oteryn:npc.gail",
    ];
    owner.states.chat.install_npc_talk(std::sync::Arc::new(
        NpcTalkBook::from_model_with_native_quests(&model, &keys),
    ));
    owner.states.chat.admit(
        admitted.game_session_id,
        "Library researcher",
        owner.owner_now(),
    );
    owner.states.chat.set_npc_service(admitted.game_session_id);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut runtime = owner.runtime.lock().await;
    runtime
        .bind_attacker_lease(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let mut npcs = keys
        .into_iter()
        .zip([(32529, 32783, -6), (33268, 31835, -1), (32621, 32738, -6)])
        .map(|(key, (x, y, floor))| NpcPlacements {
            key: key.into(),
            placements: vec![NpcPlacement {
                x,
                y,
                floor,
                direction: MovementFacing::South,
            }],
        })
        .collect::<Vec<_>>();
    npcs.sort_by(|a, b| a.key.cmp(&b.key));
    runtime
        .place_npcs(&NpcPlacementTable { npcs }, |_| true, true)
        .map_err(debug)?;
    let refs = keys
        .into_iter()
        .map(|key| {
            runtime
                .visible_entities()
                .npcs
                .into_iter()
                .find_map(|npc| {
                    (runtime.npc_key(npc.actor) == Some(key)).then_some(EntityRef {
                        identity: npc.actor.placement_identity(),
                        generation: npc.generation,
                    })
                })
                .ok_or_else(|| "source NPC".into())
        })
        .collect::<TestResult<Vec<_>>>()?;
    refs.try_into().map_err(|_| "exact NPC refs".into())
}

async fn raise_level(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    level: u32,
    occurrence: u8,
) -> TestResult {
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
    };
    let binding = owner
        .progression_binding(admitted.game_session_id)
        .ok_or("progression")?;
    let xp = binding
        .policy
        .thresholds
        .iter()
        .find(|t| t.level == level)
        .ok_or("level threshold")?
        .minimum_experience
        .get();
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let current = owner
        .root
        .read_character_progression(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("progression state")?;
    let amount = xp.checked_sub(current.total_experience.get()).ok_or("XP")?;
    if amount > 0 {
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_experience(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                ExperienceAwardRequest {
                    occurrence: ExperienceRewardOccurrence::from_bytes(id(occurrence))
                        .map_err(debug)?,
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
