use super::cyclopedia_fixtures::support::exchange;
use super::fixture::{Harness, debug, id};
use super::kill_reward_e2e_fixtures::scenario_thread::run_async;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::domain::CharacterId;
use crate::durability::guild_chat_fixture::historical_character;
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::cyclopedia_character::{
    Badge, CAPABILITY_CYCLOPEDIA_CHARACTER_V1, COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB, profile,
};

#[test]
fn cyclopedia_profile_recognizes_accepted_friends_without_vip_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run_async(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "friend_profile", true, false).await?;
        let result: TestResult = async {
            let peer = id(60);
            let account = id(59);
            let world = id(42);
            let peer_name = "Friend Profile";
            historical_character(
                &h.pool,
                60,
                CharacterId::from_bytes(peer).map_err(debug)?,
                account,
                world,
                peer_name,
            )
            .await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let deps = Dependencies::new()?;
            deps.runtime
                .lock()
                .await
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let owner = deps.owner(&h, &authority);
            deps.initialize(&owner).await;
            let mut admitted = deps.admitted;
            admitted.first_entry = FirstEntryOutcome::Positioned;
            admitted.item_fence = Some(
                super::death_ml_loss_composed_fixtures::current_fence(
                    &owner,
                    admitted.game_session_id,
                )
                .await?
                .item_fence(),
            );
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &[CAPABILITY_CYCLOPEDIA_CHARACTER_V1],
            )
            .ok_or("Cyclopedia character selection")?;
            let viewer = id(40);
            let relation: i64 = sqlx::query_scalar(
                "INSERT INTO game_account_friends
                 (world_id,left_account,right_account,requester,requester_name,target_name)
                 SELECT encode($1,'hex')::uuid,LEAST(v.account_id,t.account_id),
                        GREATEST(v.account_id,t.account_id),v.account_id,v.name,t.name
                 FROM game_character_roots v,game_character_roots t
                 WHERE v.character_id=encode($2,'hex')::uuid AND t.character_id=encode($3,'hex')::uuid
                 RETURNING id",
            )
            .bind(world.as_slice())
            .bind(id(41).as_slice())
            .bind(peer.as_slice())
            .fetch_one(&h.pool)
            .await?;
            let badge = Badge::FledgelingHero;
            sqlx::query(
                "INSERT INTO game_account_badges(account_id,badge,earned_at_unix_ms)
                 VALUES(encode($1,'hex')::uuid,$2,0)",
            )
            .bind(account.as_slice())
            .bind(i16::from(badge.id()))
            .execute(&h.pool)
            .await?;
            let has_vip_consent: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM game_vip_consents WHERE account_id=encode($1,'hex')::uuid
                 AND world_id=encode($2,'hex')::uuid AND character_id=encode($3,'hex')::uuid)",
            )
            .bind(viewer.as_slice())
            .bind(world.as_slice())
            .bind(peer.as_slice())
            .fetch_one(&h.pool)
            .await?;
            assert!(!has_vip_consent);

            let request = profile::request(Some(peer));
            let command = admitted.continuity.next_command_id;
            let (status, _) = exchange(&owner, &mut admitted, command, COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB, &request).await?;
            assert_eq!(status, CommandStatus::Rejected, "pending friendship");
            sqlx::query("UPDATE game_account_friends SET accepted=true WHERE id=$1")
                .bind(relation)
                .execute(&h.pool)
                .await?;
            let command = admitted.continuity.next_command_id;
            let (status, reply) = exchange(&owner, &mut admitted, command, COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB, &request).await?;
            assert_eq!(status, CommandStatus::Accepted, "accepted friendship");
            let reply = profile::Profile::decode(&reply).map_err(debug)?;
            assert_eq!(reply.character, peer);
            assert_eq!(reply.name, peer_name);
            assert_eq!(reply.badges, [badge]);

            sqlx::query(
                "INSERT INTO game_account_badge_visibility(account_id,badge,is_public)
                 VALUES(encode($1,'hex')::uuid,$2,false)",
            )
            .bind(account.as_slice())
            .bind(i16::from(badge.id()))
            .execute(&h.pool)
            .await?;
            let command = admitted.continuity.next_command_id;
            let (status, reply) = exchange(&owner, &mut admitted, command, COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB, &request).await?;
            assert_eq!(status, CommandStatus::Accepted, "hidden badge leaves profile visible");
            assert!(profile::Profile::decode(&reply).map_err(debug)?.badges.is_empty());

            sqlx::query("DELETE FROM game_account_friends WHERE id=$1")
                .bind(relation)
                .execute(&h.pool)
                .await?;
            let command = admitted.continuity.next_command_id;
            let (status, _) = exchange(&owner, &mut admitted, command, COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB, &request).await?;
            assert_eq!(status, CommandStatus::Rejected, "revoked friendship");
            eprintln!("cyclopedia_profile_recognizes_accepted_friends_without_vip_postgres POSTGRES_CASE_COMPLETED");
            Ok(())
        }
        .await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}
