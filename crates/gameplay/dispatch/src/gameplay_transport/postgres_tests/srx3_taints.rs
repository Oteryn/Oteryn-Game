//! SRX-3: command 133 reads private taints through the real admitted connection.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::item_fence_of;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1;

#[test]
fn srx3_taints_postgres_native_query_private_expiry_and_session_fence() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("srx3-taints", || {
        run(async |admin| {
            reset();
            let h = Harness::create_with_admission_seed(admin, "srx3_taints", true, false).await?;
            prepare_current_session(&h).await?;
            // Historical state setup follows soul_war.rs's expiry fixture. Only the
            // admin setup transaction bypasses receipt triggers; the dispatched read
            // runs with normal runtime privileges, guards and constraints.
            // A different retained Character must never leak into this view.
            use crate::durability::guild_chat_fixture as characters;
            let other =
                crate::domain::CharacterId::from_bytes(characters::sid(90, 1)).map_err(debug)?;
            characters::historical_character(
                &h.pool,
                90,
                other,
                characters::sid(90, 2),
                super::fixture::id(42),
                "Tainted Other",
            )
            .await?;
            let mut historical = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *historical)
                .await?;
            sqlx::query("INSERT INTO game_character_soul_war(character_id,teleport_access,defeated_bosses,first_taint_at,megalomania_completed,final_reward_claimed) VALUES(encode($1,'hex')::uuid,true,31,floor(extract(epoch FROM clock_timestamp()))::bigint,false,false)")
                .bind(other.as_bytes().as_slice()).execute(&mut *historical).await?;
            historical.commit().await?;
            let deps = Dependencies::new()?;
            let mut admitted = deps.admitted;
            admitted.first_entry = FirstEntryOutcome::Positioned;
            let current =
                crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                    .current_session_at(admitted.game_session_id)
                    .await?
                    .0;
            admitted.item_fence = item_fence_of(current);
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let owner = deps.owner(&h, &authority);
            deps.initialize(&owner).await;
            owner
                .runtime
                .lock()
                .await
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[CAPABILITY_BOSS_ENCOUNTER_V1],
            )
            .ok_or("boss capability")?;
            let character = admitted
                .item_fence
                .ok_or("current item fence")?
                .character_id;
            let (status, bytes) =
                super::boss_encounter::exchange_kind(&owner, &mut admitted, 1, 133, &[16, 1])
                    .await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert!(bytes.ends_with(&[58, 8, 8, 0, 16, 0, 24, 0, 32, 0]));
            let mut historical = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *historical)
                .await?;
            sqlx::query("INSERT INTO game_character_soul_war(character_id,teleport_access,defeated_bosses,first_taint_at,megalomania_completed,final_reward_claimed) VALUES(encode($1,'hex')::uuid,true,31,floor(extract(epoch FROM clock_timestamp()))::bigint,false,false)")
                .bind(character.as_bytes().as_slice()).execute(&mut *historical).await?;
            sqlx::query("INSERT INTO game_character_rotten_blood(character_id,regular_taints,child_flags,final_taint,completed,last_cleanse_at) VALUES(encode($1,'hex')::uuid,3,15,true,false,NULL)")
                .bind(character.as_bytes().as_slice()).execute(&mut *historical).await?;
            historical.commit().await?;
            let (status, bytes) =
                super::boss_encounter::exchange_kind(&owner, &mut admitted, 2, 133, &[16, 1])
                    .await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert!(bytes.ends_with(&[58, 8, 8, 5, 16, 3, 24, 15, 32, 1]));
            // Expiry is projected using server time without rewriting the Quest state.
            let mut historical = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *historical)
                .await?;
            sqlx::query("UPDATE game_character_soul_war SET first_taint_at=first_taint_at-1209600 WHERE character_id=encode($1,'hex')::uuid")
                .bind(character.as_bytes().as_slice()).execute(&mut *historical).await?;
            historical.commit().await?;
            let (_, bytes) =
                super::boss_encounter::exchange_kind(&owner, &mut admitted, 3, 133, &[16, 1])
                    .await?;
            assert!(bytes.ends_with(&[58, 8, 8, 0, 16, 3, 24, 15, 32, 1]));
            let (_, legacy) =
                super::boss_encounter::exchange_kind(&owner, &mut admitted, 4, 133, &[]).await?;
            assert!(!legacy.ends_with(&[58, 8, 8, 0, 16, 3, 24, 15, 32, 1]));
            let mut stale = admitted.item_fence.ok_or("item fence")?;
            stale.game_session_id =
                crate::foundation::GameSessionId::decode(&characters::sid(99, 1)).map_err(debug)?;
            if !owner.root.is_ready() {
                owner.root.request_ready();
            }
            if owner.root.has_ready_demand() {
                owner.root.maintain_ready_once().await.map_err(debug)?;
            }
            assert!(
                owner
                    .root
                    .read_boss_cooldown_list(owner.character, owner.holder, stale, &[16, 1])
                    .await?
                    .is_none()
            );
            eprintln!("SRX3_TAINTS POSTGRES_CASE_COMPLETED");
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}
