//! Peer projection gaps must not turn a departing player into a quest USE target.
use super::*;
use crate::bestiary_postgres_harness as f;
use oteryn_gameplay_quests::quest::QuestTrack;
use oteryn_protocol_oteryn::world_object::{UseIntent, UseTarget};

#[test]
fn m43_peer_postgres_rejects_pending_arrival_before_reward() -> f::TestResult {
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
                    let h = f::Harness::create(admin, "questpc8_peer_transit", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M43_POSTGRES_PEER_PENDING_ARRIVAL_REJECTED duration={:?}",
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

async fn exercise(h: &f::Harness) -> f::TestResult {
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
    let actor = f::fence(1)?;
    let mut peer = QuestPeerUseProof {
        fence: actor,
        target: EntityRef {
            identity: f::id(94),
            generation: 1,
        },
    };
    peer.fence.character_id =
        crate::domain::CharacterId::from_bytes(f::id(91)).map_err(f::debug)?;
    peer.fence.game_session_id =
        crate::foundation::GameSessionId::decode(&f::id(92)).map_err(f::debug)?;
    let quest = "oteryn:quest.authored.the_colours_of_magic";
    let track = "oteryn:quest-progress/chosen/colours/colour";
    let catalogue = QuestStateCatalogue::new(
        "content-1",
        vec![QuestTrack {
            key: track.into(),
            quest: quest.into(),
            initial: 0,
            min: 0,
            max: 3,
        }],
        vec![],
    )
    .map_err(f::debug)?;
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
    let mut seed = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *seed)
        .await?;
    sqlx::query("INSERT INTO game_character_quest_tracks(character_id,quest_key,track_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,2,2)").bind(f::id(91).as_slice()).bind(quest).bind(track).execute(&mut *seed).await?;
    sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,'content-1',$3,2,2)").bind(f::id(91).as_slice()).bind(quest).bind(catalogue.definition_hash(quest).ok_or("quest hash")?.as_slice()).execute(&mut *seed).await?;
    sqlx::query("UPDATE game_character_roots SET character_revision=2 WHERE character_id=encode($1,'hex')::uuid").bind(f::id(91).as_slice()).execute(&mut *seed).await?;
    seed.commit().await?;
    peer.fence.expected_character_revision =
        crate::domain::CharacterRevision::new(2).map_err(f::debug)?;
    assert!(
        check(h, actor, &peer, &requirement, &catalogue, &carried).await?,
        "fresh yellow peer qualifies"
    );
    sqlx::query("INSERT INTO game_character_pending_arrivals(character_id,occurrence_id,world_id,tile_x,tile_y,tile_floor) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,32859,31302,7)")
        .bind(f::id(91).as_slice()).bind(f::id(96).as_slice()).bind(f::id(f::WORLD).as_slice()).execute(&h.pool).await?;
    assert!(
        !check(h, actor, &peer, &requirement, &catalogue, &carried).await?,
        "committed departure is not a live peer even before runtime placement catches up"
    );
    sqlx::query(
        "DELETE FROM game_character_pending_arrivals WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(f::id(91).as_slice())
    .execute(&h.pool)
    .await?;
    assert!(
        check(h, actor, &peer, &requirement, &catalogue, &carried).await?,
        "consumed arrival restores eligibility"
    );
    assert_eq!(h.root_revision().await?, "1");
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2").bind(f::id(91).as_slice()).bind(track).fetch_one(&h.pool).await?,2);
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
