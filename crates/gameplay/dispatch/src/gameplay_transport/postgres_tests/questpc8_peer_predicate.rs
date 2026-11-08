//! Real PostgreSQL authority and locked peer-state checks for carried quest USE.
use super::TestResult;
use crate::bestiary_postgres_harness::{self as fixture, Harness, debug, id};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::reward_claim_mint::{
    QuestCarriedUse,
    quest_peer::{QuestPeerUseProof, holds_in_transaction},
};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, QuestTrack, reward_callbacks::TrackRequirement,
};
use oteryn_protocol_oteryn::{
    world_object::{UseIntent, UseTarget},
    world_spatial_entities::EntityRef,
};

const TRACK: &str = "oteryn:quest-progress/chosen/colours/colour";
const QUEST: &str = "oteryn:quest.authored.the_colours_of_magic";

#[test]
fn questpc8_peer_predicate_postgres_fences_locked_state_and_no_peer_mutation() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            fixture::runtime().map_err(debug)?.block_on(async move {
                let h = Harness::create(admin, "questpc8_peer", true)
                    .await
                    .map_err(debug)?;
                let result = exercise(&h).await.map_err(|e| e.to_string());
                h.cleanup().await.map_err(debug)?;
                result
            })
        })?
        .join()
        .map_err(|_| "peer predicate thread panicked")?;
    eprintln!(
        "POSTGRES_CASE_FINISHED questpc8_peer_predicate duration={:?} result={result:?}",
        started.elapsed()
    );
    result.map_err(Into::into)
}

async fn exercise(h: &Harness) -> TestResult {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let current =
        crate::world_party_runtime_postgres_cases::committed_fresh_player(h, 60, 63, 70, now)
            .await?;
    let item = crate::gameplay_transport::item_fence_of(current).ok_or("peer fence")?;
    let mut peer = QuestPeerUseProof {
        fence: CurrentCharacterGameplayFence {
            character_id: item.character_id,
            game_session_id: item.game_session_id,
            connection_generation: item.connection_generation,
            character_lease_generation: item.character_lease_generation,
            runtime_scope: item.runtime_scope,
            scope_ownership_generation: item.scope_ownership_generation,
            expected_character_revision: crate::domain::CharacterRevision::new(1).map_err(debug)?,
        },
        target: EntityRef {
            identity: id(80),
            generation: 1,
        },
    };
    let actor = fixture::fence(1)?;
    let catalogue = QuestStateCatalogue::new(
        "content-1",
        vec![QuestTrack {
            key: TRACK.into(),
            quest: QUEST.into(),
            initial: 0,
            min: 0,
            max: 3,
        }],
        vec![],
    )
    .map_err(debug)?;
    let requirement = TrackRequirement {
        track: TRACK.into(),
        minimum: 2,
        maximum: 2,
    };
    let carried = QuestCarriedUse {
        source: id(90),
            recipe_target: None,
        intent: oteryn_protocol_oteryn::world_object::encode_item_use_intent(&UseIntent {
            target: UseTarget::ItemByDefinition(
                std::num::NonZeroU32::new(1).ok_or("nonzero native item index")?,
            ),
            use_with: Some(peer.target),
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(debug)?,
    };
    assert!(
        !check(h, actor, &peer, &requirement, &catalogue, &carried).await?,
        "absent peer state uses declared default"
    );
    let mut setup = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *setup)
        .await?;
    sqlx::query("INSERT INTO game_character_quest_tracks(character_id,quest_key,track_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,2,2)")
        .bind(item.character_id.as_bytes().as_slice()).bind(QUEST).bind(TRACK).execute(&mut *setup).await?;
    sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,'content-1',$3,2,2)")
        .bind(item.character_id.as_bytes().as_slice()).bind(QUEST).bind(catalogue.definition_hash(QUEST).ok_or("definition")?.as_slice())
        .execute(&mut *setup).await?;
    sqlx::query("UPDATE game_character_roots SET character_revision=2 WHERE character_id=encode($1,'hex')::uuid")
        .bind(item.character_id.as_bytes().as_slice()).execute(&mut *setup).await?;
    setup.commit().await?;
    peer.fence.expected_character_revision =
        crate::domain::CharacterRevision::new(2).map_err(debug)?;
    assert!(
        check(h, actor, &peer, &requirement, &catalogue, &carried).await?,
        "current yellow peer satisfies predicate"
    );
    let before: serde_json::Value = sqlx::query_scalar("SELECT to_jsonb(t) FROM game_character_quest_tracks t WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(item.character_id.as_bytes().as_slice()).bind(TRACK).fetch_one(&h.pool).await?;
    let mut stale = peer;
    stale.fence.connection_generation =
        crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
    assert!(!check(h, actor, &stale, &requirement, &catalogue, &carried).await?);
    stale = peer;
    stale.fence.character_lease_generation += 1;
    assert!(!check(h, actor, &stale, &requirement, &catalogue, &carried).await?);
    stale = peer;
    stale.fence.expected_character_revision =
        crate::domain::CharacterRevision::new(3).map_err(debug)?;
    assert!(!check(h, actor, &stale, &requirement, &catalogue, &carried).await?);
    stale = peer;
    stale.fence = actor;
    assert!(
        !check(h, actor, &stale, &requirement, &catalogue, &carried).await?,
        "self never qualifies"
    );
    stale = peer;
    stale.target.generation += 1;
    assert!(
        !check(h, actor, &stale, &requirement, &catalogue, &carried).await?,
        "ingress binds exact target generation"
    );
    stale = peer;
    stale.fence.runtime_scope = crate::foundation::RuntimeScopeRefV1::Channel {
        world_id: actor.runtime_scope.world_id(),
        channel_id: crate::foundation::ChannelId::decode(&id(99)).map_err(debug)?,
    };
    assert!(
        !check(h, actor, &stale, &requirement, &catalogue, &carried).await?,
        "cross-scope peer refuses"
    );
    let mut update = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *update)
        .await?;
    sqlx::query("UPDATE game_character_quest_tracks SET value=1 WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(item.character_id.as_bytes().as_slice()).bind(TRACK).execute(&mut *update).await?;
    update.commit().await?;
    assert!(
        !check(h, actor, &peer, &requirement, &catalogue, &carried).await?,
        "fresh durable colour overrides earlier eligibility"
    );
    let mut update = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *update)
        .await?;
    sqlx::query("UPDATE game_character_quest_tracks SET value=2 WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(item.character_id.as_bytes().as_slice()).bind(TRACK).execute(&mut *update).await?;
    update.commit().await?;
    let mut update = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *update)
        .await?;
    sqlx::query("UPDATE game_character_quest_states SET definition_hash=$2 WHERE character_id=encode($1,'hex')::uuid AND quest_key=$3")
        .bind(item.character_id.as_bytes().as_slice()).bind([9_u8; 32].as_slice()).bind(QUEST).execute(&mut *update).await?;
    update.commit().await?;
    assert!(
        !check(h, actor, &peer, &requirement, &catalogue, &carried).await?,
        "a different quest-definition pin never supplies eligible peer state"
    );
    let mut update = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *update)
        .await?;
    sqlx::query("UPDATE game_character_quest_states SET definition_hash=$2 WHERE character_id=encode($1,'hex')::uuid AND quest_key=$3")
        .bind(item.character_id.as_bytes().as_slice()).bind(catalogue.definition_hash(QUEST).ok_or("definition")?.as_slice()).bind(QUEST).execute(&mut *update).await?;
    update.commit().await?;
    let mut busy = h.pool.begin().await?;
    sqlx::query(
        "SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE",
    )
    .bind(item.character_id.as_bytes().as_slice())
    .execute(&mut *busy)
    .await?;
    assert!(
        !tokio::time::timeout(
            std::time::Duration::from_secs(5),
            check(h, actor, &peer, &requirement, &catalogue, &carried)
        )
        .await??,
        "busy peer refuses without an opposite-order wait"
    );
    busy.rollback().await?;
    let after: serde_json::Value = sqlx::query_scalar("SELECT to_jsonb(t) FROM game_character_quest_tracks t WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(item.character_id.as_bytes().as_slice()).bind(TRACK).fetch_one(&h.pool).await?;
    assert_eq!(before, after, "predicate does not mutate peer progress");
    eprintln!("POSTGRES_PEER_CURRENT_YELLOW_SELF_STALE_INGRESS_BUSY_AND_READ_ONLY_PASS");
    Ok(())
}

async fn check(
    h: &Harness,
    actor: CurrentCharacterGameplayFence,
    peer: &QuestPeerUseProof,
    requirement: &TrackRequirement,
    catalogue: &QuestStateCatalogue,
    carried: &QuestCarriedUse,
) -> TestResult<bool> {
    let mut tx = h.pool.begin().await?;
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
    .map_err(debug)?;
    tx.rollback().await?;
    Ok(result)
}
