//! The current server World policy, query predicate and client selector share one round trip.
use super::fixture::{Harness, debug, id};
use super::kill_reward_e2e_fixtures::scenario_thread::run_async;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::{
    capabilities::{OfferedCapability, SelectedCapabilities},
    connection::FirstEntryOutcome,
};
use oteryn_client_ui::highscores::{self as ui, Action, HighscoresPanel};
use oteryn_protocol_oteryn::highscores_catalogue::{self as wire, WorldType};
use oteryn_session::{CommandStatus, GatedCommandResult};
#[test]
fn highscores_catalogue_and_type_predicate_reach_dispatch_client_state_and_selectors() -> TestResult
{
    for (mode, kind) in [
        (1i16, WorldType::Optional),
        (2, WorldType::Open),
        (3, WorldType::Hardcore),
    ] {
        run_async(async move |admin| {
            let h = Harness::create_with_admission_seed(admin, "highscores_catalogue", true, false)
                .await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            deps.runtime
                .lock()
                .await
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let owner = deps.owner(&h, &authority);
            deps.initialize(&owner).await;
            let current =
                crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                    .current_session_at(deps.admitted.game_session_id)
                    .await?
                    .0;
            let character = current.commit().character_id();
            let mut admitted = deps.admitted;
            admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
            admitted.first_entry = FirstEntryOutcome::Positioned;
            let offers = [
                OfferedCapability {
                    id: 9,
                    requires: &[],
                },
                OfferedCapability {
                    id: 507,
                    requires: &[9],
                },
            ];
            admitted.continuity.selected_capabilities =
                SelectedCapabilities::select(&offers, &[9, 507]).ok_or("select world catalogue")?;
            let world = admitted.world_id;
            sqlx::query("INSERT INTO game_control_field_policy_grants VALUES(session_user,encode($1,'hex')::uuid,'policy-1')")
            .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_spell_field_world_policies(world_id,world_policy_revision,world_type,protection_level,in_fight_ms,source_pin,control_role,decision_identity) VALUES(encode($1,'hex')::uuid,'policy-1',$2,20,60000,'99902524e052f37574194466c2949c576e4ab269',session_user,'catalogue-fixture')")
            .bind(world.as_bytes().as_slice()).bind(mode).execute(&h.pool).await?;
            super::highscores_world_fixtures::snapshot(&h.pool, world, 1, character.as_bytes(), 1).await?;
            let query = oteryn_protocol_oteryn::highscores::HighscoresQuery {
                world: Some(wire::WorldId::decode(world.as_bytes()).map_err(debug)?),
                details: true,
                category: 1,
                vocation: 1,
                page: 1,
                snapshot_id: 0,
            };
            let mut admission_writer = h.pool.begin().await?;
            sqlx::query("LOCK TABLE game_durability_reconnect_sessions IN EXCLUSIVE MODE")
                .execute(&mut *admission_writer)
                .await?;
            let response = crate::durability::highscores_catalogue::answer_session(
                &h.root,
                world,
                world,
                admitted.game_session_id,
                None,
            )
            .await;
            let typed = crate::durability::highscores_catalogue::answer_session(
                &h.root,
                world,
                world,
                admitted.game_session_id,
                Some((query, Some(kind))),
            )
            .await;
            admission_writer.rollback().await?;
            assert!(
                matches!(
                    response,
                    crate::durability::highscore_snapshot::reply::HighscoresReply::Result(_)
                ),
                "admission writer blocked the catalogue request"
            );
            assert!(
                matches!(
                    typed,
                    crate::durability::highscore_snapshot::reply::HighscoresReply::Result(_)
                ),
                "admission writer blocked the typed query"
            );
            let mut panel = HighscoresPanel::default();
            panel.selected = true;
            panel.catalogue_selected = true;
            let command = ui::request(&mut panel, Action::Catalogue).ok_or("catalogue request")?;
            let (status, payload) = super::cyclopedia_fixtures::support::exchange(
                &owner,
                &mut admitted,
                1,
                command.command_type,
                &command.payload,
            )
            .await?;
            assert_eq!(status, CommandStatus::Accepted);
            ui::apply_result(
                &mut panel,
                &GatedCommandResult {
                    command_id: 1,
                    status,
                    server_sequence: 1,
                    payload,
                },
            );
            assert_eq!(panel.catalogue.len(), 1);
            assert_eq!(panel.catalogue[0].world.as_bytes(), world.as_bytes());
            assert_eq!(panel.catalogue[0].world_type, kind);
            assert_eq!(ui::world_choices(&panel).count(), 1);
            let command =
                ui::request(&mut panel, Action::WorldType(Some(kind))).ok_or("typed highscores")?;
            let (status, payload) = super::cyclopedia_fixtures::support::exchange(
                &owner,
                &mut admitted,
                2,
                command.command_type,
                &command.payload,
            )
            .await?;
            assert_eq!(status, CommandStatus::Accepted);
            ui::apply_result(
                &mut panel,
                &GatedCommandResult {
                    command_id: 2,
                    status,
                    server_sequence: 2,
                    payload,
                },
            );
            assert_eq!(
                panel.reply.as_ref().ok_or("filtered highscores")?.category,
                1
            );
            let wrong = wire::encode_query(
                query,
                Some(if kind == WorldType::Optional {
                    WorldType::Open
                } else {
                    WorldType::Optional
                }),
            )
            .map_err(debug)?;
            assert_eq!(
                super::cyclopedia_fixtures::support::exchange(
                    &owner,
                    &mut admitted,
                    3,
                    wire::COMMAND_QUERY,
                    &wrong
                )
                .await?
                .0,
                CommandStatus::Rejected
            );
            let foreign = crate::foundation::WorldId::decode(&id(90)).map_err(debug)?;
            super::highscores_world_fixtures::snapshot(&h.pool, foreign, 1, character.as_bytes(), 1).await?;
            for (command_id, action) in [
                (4, Action::WorldType(None)),
                (
                    5,
                    Action::World(Some(
                        wire::WorldId::decode(foreign.as_bytes()).map_err(debug)?,
                    )),
                ),
            ] {
                let command =
                    ui::request(&mut panel, action).ok_or("untyped public World query")?;
                let (status, payload) = super::cyclopedia_fixtures::support::exchange(
                    &owner,
                    &mut admitted,
                    command_id,
                    command.command_type,
                    &command.payload,
                )
                .await?;
                assert_eq!(status, CommandStatus::Accepted);
                ui::apply_result(
                    &mut panel,
                    &GatedCommandResult {
                        command_id,
                        status,
                        server_sequence: command_id,
                        payload,
                    },
                );
                assert!(panel.reply.is_some());
            }
            assert_eq!(
                panel
                    .reply
                    .as_ref()
                    .ok_or("foreign page")?
                    .world
                    .map(|id| *id.as_bytes()),
                Some(*foreign.as_bytes())
            );
            assert!(
                panel
                    .reply
                    .as_ref()
                    .ok_or("foreign page")?
                    .own_row
                    .is_none()
            );
            assert_eq!(admitted.world_id, world);
            let command =
                ui::request(&mut panel, Action::Refresh).ok_or("selected public query")?;
            admitted.continuity.selected_capabilities =
                SelectedCapabilities::select(&offers, &[9]).ok_or("legacy highscores selection")?;
            assert_eq!(
                super::cyclopedia_fixtures::support::exchange(
                    &owner,
                    &mut admitted,
                    6,
                    command.command_type,
                    &command.payload
                )
                .await?
                .0,
                CommandStatus::Rejected
            );
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })?;
    }
    Ok(())
}
