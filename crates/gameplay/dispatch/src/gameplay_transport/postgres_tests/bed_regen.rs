//! BED-REGEN-1: production first-entry -> runtime-role transaction -> wire vitals.
use super::fixture::{Harness, debug, id, run};
use super::tutorial_vitals_fixture::admit;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::actor_spell::{STATE_DOMAIN_ACTOR_VITALS, decode_actor_vitals};

#[test]
fn bed_regen_admission_wire_replay_and_restart_postgres() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("bed-regen-wire", || scenario(false))
}

#[test]
fn bed_regen_freed_bed_limits_only_sleep_postgres() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("bed-regen-freed", || scenario(true))
}

async fn seed(h: &Harness, freed: bool) -> TestResult {
    let wake: i64 = sqlx::query_scalar("SELECT admitted_at FROM game_bed_admission_times WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(id(50).as_slice()).fetch_one(&h.pool).await?;
    let slept = wake - 900_000;
    sqlx::query("INSERT INTO game_durability_reconnect_sessions(game_session_id,account_id,character_id,world_id,runtime_scope_kind,runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,original_grace_deadline,predecessor_generation,character_lease_generation,scope_ownership_generation,current_generation,session_state) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1,999999,1,1,1,1,3)")
        .bind(id(49).as_slice()).bind(id(40).as_slice()).bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query(include_str!(
        "../../../../../../apps/game-server/tests/support/owned_house_fixture.sql"
    ))
        .bind(id(42).as_slice()).bind("oteryn:content.house.bed_regen").bind(id(41).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_house_bed_sleepers VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'oteryn:content.house.bed_regen',100,200,-7,1,'male',$3,NULL,NULL)")
        .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(slept).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_house_bed_sleep_receipts VALUES(encode($1,'hex')::uuid,1,jsonb_build_array(encode($2,'hex')::uuid,encode($3,'hex')::uuid,'oteryn:content.house.bed_regen',1,1,1,'transport',100,200,-7,'male',6),$4)")
        .bind(id(49).as_slice()).bind(id(41).as_slice()).bind(id(42).as_slice()).bind(slept).execute(&h.pool).await?;
    if freed {
        sqlx::query("UPDATE game_house_bed_sleepers SET freed_at=$1,freed_cause='owner_wake' WHERE character_id=encode($2,'hex')::uuid")
            .bind(slept+29_999).bind(id(41).as_slice()).execute(&h.pool).await?;
    }
    sqlx::query("INSERT INTO game_character_last_positions(character_id,world_id,tile_x,tile_y,tile_floor,coordinate_frame_digest,map_revision_digest,server_artifact_digest,character_lease_generation,write_sequence,health,mana,rest) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,100,200,7,$3,$3,$3,1,1,27,9,ARRAY[5,31567,1234,567])")
        .bind(id(41).as_slice()).bind(id(42).as_slice()).bind([0u8;32].as_slice()).execute(&h.pool).await?;
    Ok(())
}

fn scenario(freed: bool) -> TestResult {
    run(async |admin| {
        let tag = if freed {
            "bed_regen_freed"
        } else {
            "bed_regen_wire"
        };
        let mut h = Harness::create_with_admission_seed(admin, tag, false, false)
            .await
            .map_err(|e| format!("create: {e}"))?;
        let request = prepare_current_session(&h)
            .await
            .map_err(|e| format!("prepare: {e}"))?;
        seed(&h, freed).await.map_err(|e| format!("seed: {e}"))?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(|e| format!("open authority: {e:?}"))?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&h, &authority);
        let fence = owner
            .current_quest_fence(dependencies.admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("session")?;
        // One changed authority invariant: an old connection cannot consume the sleeper.
        let mut stale = fence;
        stale.character_lease_generation += 1;
        assert!(
            h.root
                .settle_bed_regen(&authority, &h.node, stale)
                .await
                .is_err()
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_house_bed_sleepers")
                .fetch_one(&h.pool)
                .await?,
            1
        );
        // Fail after the projection update: deletion's revision trigger must roll
        // the projection and receipt back with the sleeper.
        sqlx::query("UPDATE game_house_bed_revisions SET revision=18446744073709551615")
            .execute(&h.pool)
            .await?;
        assert!(
            h.root
                .settle_bed_regen(&authority, &h.node, fence)
                .await
                .is_err()
        );
        let untouched: (i32,i32,i64) = sqlx::query_as("SELECT health,mana,(SELECT count(*) FROM game_bed_regen_receipts) FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(untouched, (27, 9, 0));
        sqlx::query("UPDATE game_house_bed_revisions SET revision=$1::text::numeric")
            .bind(if freed { "3" } else { "2" })
            .execute(&h.pool)
            .await?;
        admit(&owner, &dependencies, &request)
            .await
            .map_err(|e| format!("admit: {e}"))?;
        let expected = if freed {
            (27, 9, 5, 31567)
        } else {
            (28, 10, 6, 1567)
        };
        let stored: (i32,i32,Vec<i32>) = sqlx::query_as("SELECT health,mana,rest FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!((stored.0, stored.1, stored.2[0], stored.2[1]), expected);
        assert_eq!(&stored.2[2..], &[1234, 567]);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_house_bed_sleepers")
                .fetch_one(&h.pool)
                .await?,
            0
        );
        let revision: String = sqlx::query_scalar("SELECT revision::text FROM game_house_bed_revisions WHERE house_key='oteryn:content.house.bed_regen'").fetch_one(&h.pool).await?;
        assert_eq!(revision, if freed { "4" } else { "3" });
        // First-entry retry leaves both live and durable post-values alone.
        admit(&owner, &dependencies, &request)
            .await
            .map_err(|e| format!("admit: {e}"))?;
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_bed_regen_receipts")
                .fetch_one(&h.pool)
                .await?,
            1
        );
        drop(owner);
        drop(authority);
        // Crash after commit, before actor installation: reopen the real root and actor.
        h.root = super::fixture::tutorial_vitals_fixture::reopen_root(&h)?;
        assert!(
            h.root
                .maintain_ready_once()
                .await
                .map_err(|e| format!("restart ready: {e}"))?
        );
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(|e| format!("open authority: {e:?}"))?;
        let restarted = Dependencies::new()?;
        let owner = restarted.owner(&h, &authority);
        admit(&owner, &restarted, &request).await?;
        {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let state = states
                .get(
                    &runtime,
                    restarted.actor,
                    restarted.admitted.game_session_id,
                )
                .ok_or("restored actor")?;
            let vitals = state.vitals();
            assert_eq!(
                (vitals.health, vitals.mana, vitals.soul),
                (expected.0 as u32, expected.1 as u32, expected.2 as u32)
            );
            assert_eq!(
                state.bed_rest_snapshot(owner.owner_now().get())[2..],
                [1234, 567]
            );
        }
        assert_eq!(owner.write_due_last_positions(true).await, 0);
        let saved_rest: Vec<i32> = sqlx::query_scalar("SELECT rest FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(saved_rest[0], expected.2);
        assert!(saved_rest[1] <= expected.3 && saved_rest[1] >= expected.3.saturating_sub(1000));
        assert_eq!(&saved_rest[2..], &[1234, 567]);
        let mut session = restarted.admitted;
        session.first_entry = FirstEntryOutcome::Positioned;
        session.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[6, 7])
                .ok_or("vitals capabilities")?;
        // A bounded existing USE command traverses the real wire dispatcher.
        let command = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id: 1,
                command_type: 1,
                payload: &[],
            },
        )
        .map_err(debug)?;
        let mut input = (command.len() as u32).to_be_bytes().to_vec();
        input.extend_from_slice(&command);
        let (_, output) =
            super::weather_wire::fixture_access::serve_input(&owner, session, &input).await?;
        let mut cursor = 0;
        let mut found = false;
        while cursor < output.len() {
            let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
            let frame =
                crate::foundation::decode_wire_envelope(&output[cursor + 4..cursor + 4 + length])
                    .map_err(debug)?;
            cursor += 4 + length;
            if frame.message_type() == crate::foundation::MessageType::SnapshotChunk {
                let (_, domains) = oteryn_protocol_oteryn::decode_snapshot_chunk(frame.payload())
                    .map_err(debug)?;
                for domain in domains {
                    if domain.domain_id == STATE_DOMAIN_ACTOR_VITALS {
                        let vitals = decode_actor_vitals(domain.payload).map_err(debug)?;
                        assert_eq!(
                            (vitals.health, vitals.mana, vitals.soul),
                            (expected.0 as u32, expected.1 as u32, expected.2 as u32)
                        );
                        found = true;
                    }
                }
            }
        }
        assert!(
            found,
            "existing vitals snapshot must reach the logged-in connection"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
