//! Peer projection gaps must not turn a departing player into a quest USE target.
use super::*;
use crate::bestiary_postgres_harness as f;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestTrack, QuestTransition,
};
use oteryn_protocol_oteryn::world_object::{UseIntent, UseTarget};
use std::sync::Arc;

#[test]
fn m43_peer_postgres_rejects_pending_arrival_before_reward() -> f::TestResult {
    peer_case(false)
}

#[test]
fn m43_peer_postgres_rejects_pending_respawn_before_reward() -> f::TestResult {
    peer_case(true)
}

fn peer_case(respawn: bool) -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let tag = if respawn {
                        "questpc8_peer_respawn"
                    } else {
                        "questpc8_peer_transit"
                    };
                    let h = f::Harness::create(admin, tag, true).await?;
                    let result = exercise(&h, respawn).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M43_POSTGRES_PEER_LIVENESS_REJECTED respawn={respawn} duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M43 peer transit PG thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness, respawn: bool) -> f::TestResult {
    // Admit native authority on the reconciled store, before the read-only actor fixture.
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(|e| format!("initial native authority: {e:?}"))?;
    // A second independently fenced Character, preserving all native fixture columns.
    sqlx::query("INSERT INTO game_character_account_guards VALUES(encode($1,'hex')::uuid)")
        .bind(f::id(90).as_slice())
        .execute(&h.pool)
        .await?;
    for query in [
        "INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name,sex) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,'Peer Fixture',sex FROM game_character_roots WHERE character_id=encode($4,'hex')::uuid AND octet_length($3::bytea)=16",
        "INSERT INTO game_durability_reconnect_sessions OVERRIDING USER VALUE SELECT (jsonb_populate_record(NULL::game_durability_reconnect_sessions,to_jsonb(r)||jsonb_build_object('character_id',encode($1,'hex')::uuid,'account_id',encode($2,'hex')::uuid,'game_session_id',encode($3,'hex')::uuid,'current_transport_ref',decode(repeat('5d',16),'hex')))).* FROM game_durability_reconnect_sessions r WHERE character_id=encode($4,'hex')::uuid",
        "INSERT INTO game_durability_admission_account_guards SELECT (jsonb_populate_record(NULL::game_durability_admission_account_guards,to_jsonb(r)||jsonb_build_object('presence_character_id',encode($1,'hex')::uuid,'account_id',encode($2,'hex')::uuid,'holder_game_session_id',encode($3,'hex')::uuid))).* FROM game_durability_admission_account_guards r WHERE presence_character_id=encode($4,'hex')::uuid",
        "INSERT INTO game_durability_admission_character_guards SELECT (jsonb_populate_record(NULL::game_durability_admission_character_guards,to_jsonb(r)||jsonb_build_object('character_id',encode($1,'hex')::uuid,'account_id',encode($2,'hex')::uuid,'holder_game_session_id',encode($3,'hex')::uuid))).* FROM game_durability_admission_character_guards r WHERE character_id=encode($4,'hex')::uuid",
    ] {
        sqlx::query(query)
            .bind(f::id(91).as_slice())
            .bind(f::id(90).as_slice())
            .bind(f::id(92).as_slice())
            .bind(f::id(f::CHARACTER).as_slice())
            .execute(&h.pool)
            .await?;
    }
    sqlx::query("INSERT INTO game_character_progression_state SELECT (jsonb_populate_record(NULL::game_character_progression_state,to_jsonb(p)||jsonb_build_object('character_id',encode($1,'hex')::uuid))).* FROM game_character_progression_state p WHERE character_id=encode($2,'hex')::uuid")
        .bind(f::id(91).as_slice()).bind(f::id(f::CHARACTER).as_slice()).execute(&h.pool).await?;
    // Native quest/death writes use the harness's fully bootstrapped Character.
    // The second fenced Character is the read-only predicate's locked actor.
    let peer_fence = f::fence(1)?;
    let mut actor = peer_fence;
    actor.character_id = crate::domain::CharacterId::from_bytes(f::id(91)).map_err(f::debug)?;
    actor.game_session_id =
        crate::foundation::GameSessionId::decode(&f::id(92)).map_err(f::debug)?;
    let mut peer = QuestPeerUseProof {
        fence: peer_fence,
        target: EntityRef {
            identity: f::id(94),
            generation: 1,
        },
    };
    let quest = "oteryn:quest.authored.the_colours_of_magic";
    let track = "oteryn:quest-progress/chosen/colours/colour";
    let setup_key = "oteryn:quest-test/peer-yellow/setup";
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: track.into(),
                quest: quest.into(),
                initial: 0,
                min: 0,
                max: 3,
            }],
            vec![QuestTransition {
                key: setup_key.into(),
                quest: quest.into(),
                effects: vec![QuestEffect {
                    track: track.into(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(2),
                }],
                completes: false,
                experience: None,
                world_change: None,
            }],
        )
        .map_err(f::debug)?,
    );
    let requirement = TrackRequirement {
        track: track.into(),
        minimum: 2,
        maximum: 2,
    };
    let carried = super::super::QuestCarriedUse {
        source: f::id(95),
        intent: oteryn_protocol_oteryn::world_object::encode_item_use_intent(&UseIntent {
            target: UseTarget::ItemByDefinition(
                std::num::NonZeroU32::new(1).ok_or("native item index")?,
            ),
            use_with: Some(peer.target),
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(f::debug)?,
    };
    let seed = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            peer.fence,
            crate::durability::quest_state::QuestTransitionRequest {
                transition_key: setup_key.into(),
                cause: crate::durability::quest_state::QuestCause::Command(
                    crate::foundation::CommandRef::new(
                        peer.fence.game_session_id,
                        crate::foundation::CommandId::new(9700).map_err(f::debug)?,
                    ),
                ),
            },
            catalogue.clone(),
        )
        .await
        .map_err(|e| format!("native yellow quest setup: {e:?}"))?;
    assert!(matches!(
        seed,
        crate::durability::quest_state::QuestTransitionOutcome::Committed(_)
    ));
    peer.fence.expected_character_revision =
        crate::domain::CharacterRevision::new(2).map_err(f::debug)?;
    assert!(
        check(h, actor, &peer, &requirement, &catalogue, &carried).await?,
        "fresh yellow peer qualifies"
    );
    sqlx::query("INSERT INTO game_character_pending_arrivals(character_id,occurrence_id,world_id,tile_x,tile_y,tile_floor) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,32859,31302,7)")
        .bind(peer.fence.character_id.as_bytes().as_slice()).bind(f::id(96).as_slice()).bind(f::id(f::WORLD).as_slice()).execute(&h.pool).await?;
    assert!(
        !check(h, actor, &peer, &requirement, &catalogue, &carried).await?,
        "committed departure is not a live peer even before runtime placement catches up"
    );
    sqlx::query(
        "DELETE FROM game_character_pending_arrivals WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(peer.fence.character_id.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    assert!(
        check(h, actor, &peer, &requirement, &catalogue, &carried).await?,
        "consumed arrival restores eligibility"
    );
    if respawn {
        peer.fence.expected_character_revision =
            commit_peer_death(h, &authority, peer.fence).await?;
        assert!(
            !check(h, actor, &peer, &requirement, &catalogue, &carried).await?,
            "a durably dead peer is ineligible before runtime death projection catches up"
        );
    }
    let actor_revision: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(actor.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(actor_revision, "1");
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2").bind(peer.fence.character_id.as_bytes().as_slice()).bind(track).fetch_one(&h.pool).await?,2);
    Ok(())
}

async fn check(
    h: &f::Harness,
    actor: CurrentCharacterGameplayFence,
    peer: &QuestPeerUseProof,
    requirement: &TrackRequirement,
    catalogue: &QuestStateCatalogue,
    carried: &super::super::QuestCarriedUse,
) -> f::TestResult<bool> {
    let mut tx = h.pool.begin().await?;
    sqlx::query(
        "SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE",
    )
    .bind(actor.character_id.as_bytes().as_slice())
    .fetch_one(&mut *tx)
    .await?;
    let result = holds_in_transaction(
        &mut tx,
        &h.node,
        actor,
        peer,
        requirement,
        catalogue,
        carried,
    )
    .await
    .map_err(f::debug)?;
    tx.rollback().await?;
    Ok(result)
}

async fn commit_peer_death(
    h: &f::Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    fence: CurrentCharacterGameplayFence,
) -> f::TestResult<crate::domain::CharacterRevision> {
    use crate::domain::progression::{FiniteProgressionPolicy, LevelThreshold};
    use crate::durability::character_death::{
        CharacterDeathOutcome, CharacterDeathRequest, DeathCell, PlayerDeathOccurrence,
    };
    use oteryn_simulation_determinism::{ExactI64, RoundingMode};
    let context = f::context();
    let request = CharacterDeathRequest {
        public_cause: None,
        occurrence: PlayerDeathOccurrence::from_bytes(f::id(97)).map_err(f::debug)?,
        context: context.clone(),
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
        policy: FiniteProgressionPolicy {
            context,
            policy_revision: "policy-1".into(),
            reward_revision: "reward-1".into(),
            death_policy_revision: "death-1".into(),
            declared_difference_revision: "declaration-1".into(),
            // Existing native level50/1000XP fixture policy, not a Global progression table.
            thresholds: std::array::from_fn::<_, 50, _>(|index| LevelThreshold {
                level: u32::try_from(index + 1).unwrap_or(u32::MAX),
                minimum_experience: ExactI64::new(i64::try_from(index).unwrap_or(i64::MAX) * 20),
            }),
            terminal_exclusive_experience: ExactI64::new(1_000_000),
            death_loss_numerator: 1,
            death_loss_denominator: 1,
            death_loss_rounding: RoundingMode::Floor,
        },
        held_blessings: vec![],
        pvp_contributors: vec![],
        death_cell: DeathCell {
            runtime_scope: fence.runtime_scope,
            spatial_position: vec![100, 200, 7],
            map_revision: "map-1".into(),
        },
        respawn_position: vec![10, 20, 7],
    };
    let result = h
        .root
        .commit_character_death(authority, &h.node, fence, request)
        .await
        .map_err(|e| format!("native peer death: {e:?}"))?;
    let CharacterDeathOutcome::Committed(death) = result else {
        return Err("first peer death replayed".into());
    };
    let pending:bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid)")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert!(
        pending,
        "native death created its durable respawn obligation"
    );
    Ok(death.committed_character_revision)
}
