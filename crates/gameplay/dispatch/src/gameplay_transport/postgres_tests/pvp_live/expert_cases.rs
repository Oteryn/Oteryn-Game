//! Command 103 through the actual admitted serving loop, fenced SQL facts and combat owners.
use super::*;
use crate::foundation::ExactActorRef;
use attack::AttackIntentDisposition::{Ok as Accepted, Rejected};
use oteryn_protocol_oteryn::pvp_expert::{
    COMMAND_TYPE_EXPERT_FIGHT_MODES_INTENT, ExpertFightModes, ExpertMode,
    decode_expert_fight_modes, encode_expert_fight_modes,
};

fn modes(expert: ExpertMode) -> ExpertFightModes {
    ExpertFightModes {
        modes: attack::FightModes {
            secure: false,
            ..attack::FightModes::DEFAULT
        },
        expert,
    }
}

async fn set(client: &mut DuplexStream, id: u64, state: ExpertFightModes) -> TestResult {
    let result = command(
        client,
        id,
        COMMAND_TYPE_EXPERT_FIGHT_MODES_INTENT,
        &encode_expert_fight_modes(&state),
    )
    .await?;
    assert_eq!(
        attack::decode_attack_intent_result(&result).map_err(debug)?,
        Accepted
    );
    Ok(())
}

async fn delta(client: &mut DuplexStream, expected: ExpertFightModes) -> TestResult {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let f = super::super::item_look::read_frame(client).await?;
            let env = decode_wire_envelope(&f).map_err(debug)?;
            if env.message_type() == MessageType::LivenessProbe {
                ack(client, env.payload()).await?;
            }
            if env.message_type() == MessageType::StateDelta {
                let d = oteryn_protocol_oteryn::decode_state_delta(env.payload()).map_err(debug)?;
                if d.domain_id == 52 {
                    assert_eq!(d.new_revision, d.base_revision + 1);
                    assert_eq!(
                        decode_expert_fight_modes(d.payload).map_err(debug)?,
                        expected
                    );
                    return Ok::<_, Box<dyn std::error::Error>>(());
                }
            }
        }
    })
    .await??;
    Ok(())
}

async fn target(
    client: &mut DuplexStream,
    id: u64,
    entity: Option<&EntityRef>,
    expected: attack::AttackIntentDisposition,
) -> TestResult {
    let result = command(
        client,
        id,
        attack::COMMAND_TYPE_ATTACK_TARGET_INTENT,
        &attack::encode_attack_target_intent(entity).map_err(debug)?,
    )
    .await?;
    assert_eq!(
        attack::decode_attack_intent_result(&result).map_err(debug)?,
        expected
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn drive(
    client: &mut DuplexStream,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    pool: &sqlx::PgPool,
    actor: ExactActorRef,
    victim: ExactActorRef,
    session: GameSessionId,
    entity: EntityRef,
    scenario: i16,
) -> TestResult {
    if scenario == 9 {
        let result = command(
            client,
            1,
            COMMAND_TYPE_EXPERT_FIGHT_MODES_INTENT,
            &encode_expert_fight_modes(&modes(ExpertMode::RedFist)),
        )
        .await?;
        assert!(result.is_empty());
        assert_eq!(
            owner
                .states
                .attack
                .lock()
                .await
                .expert_modes(actor, session),
            ExpertFightModes::default()
        );
        // A legacy fight-mode command must not enable expert state emission.
        command(
            client,
            2,
            attack::COMMAND_TYPE_FIGHT_MODES_INTENT,
            &attack::encode_fight_modes_intent(&modes(ExpertMode::Dove).modes),
        )
        .await?;
        let until = tokio::time::Instant::now() + Duration::from_millis(500);
        while let Ok(frame) =
            tokio::time::timeout_at(until, super::super::item_look::read_frame(client)).await
        {
            let frame = frame?;
            let env = decode_wire_envelope(&frame).map_err(debug)?;
            if env.message_type() == MessageType::StateDelta {
                assert_ne!(
                    oteryn_protocol_oteryn::decode_state_delta(env.payload())
                        .map_err(debug)?
                        .domain_id,
                    52
                );
            }
        }
        return Ok(());
    }
    if scenario == 8 || scenario == 12 {
        let mut requested = modes(ExpertMode::WhiteHand);
        requested.modes.fight_mode = attack::FightMode::Defensive;
        set(client, 1, requested).await?;
        let expected = ExpertFightModes {
            expert: if scenario == 8 {
                ExpertMode::RedFist
            } else {
                ExpertMode::Dove
            },
            ..requested
        };
        delta(client, expected).await?;
        assert_eq!(
            owner
                .states
                .attack
                .lock()
                .await
                .expert_modes(actor, session),
            expected
        );
        if scenario == 12 {
            target(client, 2, Some(&entity), Rejected).await?;
        }
        return Ok(());
    }
    if scenario == 10 {
        set(client, 1, modes(ExpertMode::RedFist)).await?;
        delta(client, modes(ExpertMode::RedFist)).await?;
        sqlx::query("INSERT INTO game_character_pvp_state(character_id,world_id,skull,skull_until) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,2,clock_timestamp()+interval '1 day') ON CONFLICT(character_id) DO UPDATE SET skull=2,skull_until=EXCLUDED.skull_until,revision=game_character_pvp_state.revision+1")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).execute(pool).await?;
        delta(client, modes(ExpertMode::Dove)).await?;
        let result = command(
            client,
            2,
            COMMAND_TYPE_EXPERT_FIGHT_MODES_INTENT,
            &encode_expert_fight_modes(&modes(ExpertMode::RedFist)),
        )
        .await?;
        assert_eq!(
            attack::decode_attack_intent_result(&result).map_err(debug)?,
            Rejected
        );
        // Black skull and in-fight blocks do not prevent a peaceful mode switch.
        set(client, 3, modes(ExpertMode::WhiteHand)).await?;
        delta(client, modes(ExpertMode::WhiteHand)).await?;
        assert_eq!(
            owner
                .expert_fight_modes(
                    actor,
                    GameSessionId::decode(&id(111)).map_err(debug)?,
                    4,
                    modes(ExpertMode::YellowHand)
                )
                .await,
            Rejected
        );
        assert_eq!(
            owner
                .states
                .attack
                .lock()
                .await
                .expert_modes(actor, session)
                .expert,
            ExpertMode::WhiteHand
        );
        // A new logical session has its own default, even with the same actor address.
        assert_eq!(
            owner
                .states
                .attack
                .lock()
                .await
                .expert_modes(actor, GameSessionId::decode(&id(111)).map_err(debug)?),
            ExpertFightModes::default()
        );
        return Ok(());
    }
    if scenario == 13 {
        super::super::fixture::pvp_source_death_fixture::seed_root(pool, 41, 60, 42).await?;
        let ally = {
            let mut runtime = owner.runtime.lock().await;
            let reservation = runtime
                .reserve_fresh_session(GameSessionId::decode(&id(112)).map_err(debug)?)
                .map_err(debug)?;
            let ally = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_first_entry_position(ally)
                .map_err(debug)?;
            ally
        };
        let mut tx = pool.begin().await?;
        // Seed a prior admitted combat situation; all decisions below use the wire dispatcher.
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_parties(party_id,world_id,leader_character_id,revision,next_seq) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,3)")
            .bind(id(80).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_party_members(character_id,party_id,seq,presence_state,presence_channel_id) VALUES(encode($1,'hex')::uuid,encode($3,'hex')::uuid,1,1,encode($4,'hex')::uuid),(encode($2,'hex')::uuid,encode($3,'hex')::uuid,2,1,encode($4,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(60).as_slice()).bind(id(80).as_slice()).bind(id(43).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_pvp_aggression(attacker,target,until_at,scope_generation,attacker_placement,target_placement) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,clock_timestamp()+interval '60 seconds',1,$3,$4)")
            .bind(id(101).as_slice()).bind(id(60).as_slice()).bind(victim.placement_identity().as_slice()).bind(ally.placement_identity().as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        wait_live(client, Duration::from_millis(10_100)).await?;
        set(client, 1, modes(ExpertMode::Dove)).await?;
        delta(client, modes(ExpertMode::Dove)).await?;
        let before = health(owner, victim).await?;
        target(client, 2, Some(&entity), Rejected).await?;
        assert_eq!(health(owner, victim).await?, before);
        set(client, 3, modes(ExpertMode::WhiteHand)).await?;
        delta(client, modes(ExpertMode::WhiteHand)).await?;
        target(client, 4, Some(&entity), Accepted).await?;
        wait_live(client, Duration::from_secs(3)).await?;
        assert!(health(owner, victim).await? < before);
        target(client, 5, None, Accepted).await?;
        let mut tx = pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM game_party_members")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM game_parties")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_guilds(guild_id,world_id,name,state,formation_deadline,revision,created_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Expert Guild',2,9999999999,1,1)")
            .bind(id(81).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_guild_ranks(guild_id,level,name) VALUES(encode($1,'hex')::uuid,1,'Leader'),(encode($1,'hex')::uuid,3,'Member')").bind(id(81).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_guild_members(character_id,guild_id,rank_level,joined_at) VALUES(encode($1,'hex')::uuid,encode($3,'hex')::uuid,1,1),(encode($2,'hex')::uuid,encode($3,'hex')::uuid,3,1)")
            .bind(id(41).as_slice()).bind(id(60).as_slice()).bind(id(81).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        target(client, 6, Some(&entity), Accepted).await?;
        target(client, 7, None, Accepted).await?;
        // Stale placement cannot carry a protected group's aggression into a new incarnation.
        sqlx::query("UPDATE game_character_pvp_aggression SET target_placement=$1")
            .bind(id(113).as_slice())
            .execute(pool)
            .await?;
        target(client, 8, Some(&entity), Rejected).await?;
        // A direct aggressor remains attackable in Dove, independently of group membership.
        sqlx::query("INSERT INTO game_character_pvp_aggression(attacker,target,until_at,scope_generation,attacker_placement,target_placement) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,clock_timestamp()+interval '60 seconds',1,$3,$4)")
            .bind(id(101).as_slice()).bind(id(41).as_slice()).bind(victim.placement_identity().as_slice()).bind(actor.placement_identity().as_slice()).execute(pool).await?;
        set(client, 9, modes(ExpertMode::Dove)).await?;
        delta(client, modes(ExpertMode::Dove)).await?;
        target(client, 10, Some(&entity), Accepted).await?;
        target(client, 11, None, Accepted).await?;
        return Ok(());
    }
    if scenario == 11 {
        let mut tx = pool.begin().await?;
        // Fixture-only custody; the real ranged owner performs every shot and payment.
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        for (seed, item, count, slot) in [(70, 3350, 1, 6_i16), (71, 3447, 100, 10)] {
            sqlx::raw_sql(sqlx::AssertSqlSafe(
                crate::durability::item_fee_burn::test_support::item(
                    seed,
                    &format!("oteryn:item.tibia.i{item}"),
                    "definition-r1",
                    count,
                ),
            ))
            .execute(&mut *tx)
            .await?;
            sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($4,'hex')::uuid)")
                .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(slot).bind(id(seed).as_slice()).execute(&mut *tx).await?;
        }
        tx.commit().await?;
    }
    // Malformed command is refused by dispatch without modifying authoritative modes.
    let result = command(client, 1, COMMAND_TYPE_EXPERT_FIGHT_MODES_INTENT, &[8, 1]).await?;
    assert!(result.is_empty());
    assert_eq!(
        owner
            .states
            .attack
            .lock()
            .await
            .expert_modes(actor, session),
        ExpertFightModes::default()
    );
    wait_live(client, Duration::from_millis(10_100)).await?;
    let before = health(owner, victim).await?;
    let mut next = 2;
    for expert in [
        ExpertMode::Dove,
        ExpertMode::WhiteHand,
        ExpertMode::YellowHand,
    ] {
        set(client, next, modes(expert)).await?;
        delta(client, modes(expert)).await?;
        next += 1;
        target(client, next, Some(&entity), Rejected).await?;
        next += 1;
    }
    assert_eq!(health(owner, victim).await?, before);
    let harms: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_pvp_harm")
        .fetch_one(pool)
        .await?;
    assert_eq!(harms, 0);
    if scenario == 7 {
        let spell = (1..=owner.spells.source_len())
            .find_map(|i| {
                std::num::NonZeroU32::new(u32::try_from(i).ok()?).filter(|index| {
                    owner
                        .spells
                        .source_indexed(*index)
                        .is_some_and(|(s, active)| {
                            active && s.key == "candidate:spell/flame_strike"
                        })
                })
            })
            .ok_or("flame strike")?;
        let cast = actor_spell::SpellCastIntent {
            spell,
            target: actor_spell::SpellTarget::Position(actor_spell::SpellTargetPosition {
                x: 1,
                y: 11,
                floor: -7,
            }),
            aim_at_target: true,
        };
        let result = command(
            client,
            next,
            actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT,
            &actor_spell::encode_spell_cast_intent(&cast),
        )
        .await?;
        assert_eq!(
            actor_spell::decode_spell_cast_result(&result).map_err(debug)?,
            // The ordinary spell owner removes illegal players before its target preview.
            actor_spell::SpellCastDisposition::Rejected
        );
        assert_eq!(health(owner, victim).await?, before);
        let harms: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_pvp_harm")
            .fetch_one(pool)
            .await?;
        assert_eq!(harms, 0);
        next += 1;
        set(client, next, modes(ExpertMode::RedFist)).await?;
        delta(client, modes(ExpertMode::RedFist)).await?;
        next += 1;
        wait_live(client, Duration::from_secs(2)).await?;
        let result = command(
            client,
            next,
            actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT,
            &actor_spell::encode_spell_cast_intent(&cast),
        )
        .await?;
        assert_eq!(
            actor_spell::decode_spell_cast_result(&result).map_err(debug)?,
            actor_spell::SpellCastDisposition::Cast
        );
        assert!(
            health(owner, victim).await? < before,
            "chosen Red Fist reaches the spell checker"
        );
        next += 1;
    } else {
        set(client, next, modes(ExpertMode::RedFist)).await?;
        delta(client, modes(ExpertMode::RedFist)).await?;
        next += 1;
    }
    let before = health(owner, victim).await?;
    target(client, next, Some(&entity), Accepted).await?;
    next += 1;
    wait_live(client, Duration::from_secs(3)).await?;
    assert!(
        health(owner, victim).await? < before,
        "chosen Red Fist reaches the swing checker"
    );
    target(client, next, None, Accepted).await?;
    next += 1;
    // The character is already in combat; a peaceful switch still succeeds.
    set(client, next, modes(ExpertMode::Dove)).await?;
    delta(client, modes(ExpertMode::Dove)).await?;
    next += 1;
    target(client, next, Some(&entity), Rejected).await?;
    Ok(())
}
