//! Real Soul War boss aliases reach source prerequisites before scope handoff.
use super::TestResult;
use super::fixture::{Harness, debug, id};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom as Room;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_protocol_oteryn::boss_encounter as wire;

#[test]
fn m26_soul_war_postgres_all_six_source_aliases_reach_real_entry_gates() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        // Literal first waiting tiles from the qualified Soul War recipes.
        // Each case owns a fresh source map, Character and ready control allocation.
        for (source, boss, [x, y, floor]) in [
            (Room::GoshnarsSpite, "goshnar_s_spite", [33774, 31634, 14]),
            (Room::GoshnarsMalice, "goshnar_s_malice", [33679, 31599, 14]),
            (Room::GoshnarsGreed, "goshnar_s_greed", [33776, 31665, 14]),
            (Room::GoshnarsHatred, "goshnar_s_hatred", [33773, 31601, 14]),
            (
                Room::GoshnarsCruelty,
                "goshnar_s_cruelty",
                [33854, 31854, 6],
            ),
            (
                Room::GoshnarsMegalomania,
                "goshnar_s_megalomania",
                [33676, 31634, 14],
            ),
        ] {
            super::reset();
            let scene = SourceScene::new(
                source,
                MovementLocalPosition {
                    x,
                    y,
                    floor: i16::try_from(floor)?,
                },
            )?;
            let deps = scene.dependencies()?;
            let h = Harness::create_with_admission_seed(
                admin.clone(),
                "m26_source_aliases",
                false,
                false,
            )
            .await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            install_scene(&mut owner, &scene, source)?;
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[wire::CAPABILITY_BOSS_ENCOUNTER_V1],
            )
            .ok_or("boss capability")?;
            prepare_player(&h, &owner, &mut admitted, 50).await?;
            let level:i64=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert!(level < 250, "fixture exercises the source level gate");
            // The ordinary route requires an independently ready destination
            // before checking its roster. Rejection must commit no handoff.
            super::activity_runtime::allocate_ready_instance(&h, 210).await?;
            let key = format!("oteryn:boss.{boss}");
            assert_eq!(
                super::boss_encounter::exchange_kind(
                    &owner,
                    &mut admitted,
                    1,
                    wire::COMMAND_TYPE_BOSS_LEVER,
                    &wire::encode_boss_key(&key).map_err(debug)?
                )
                .await?,
                (
                    oteryn_protocol_oteryn::CommandStatus::Rejected,
                    wire::encode_boss_activity_reply(&wire::BossActivityReply::Refused(
                        wire::BossActivityRefusal::Level
                    ))
                    .map_err(debug)?
                ),
                "{key}"
            );
            let groups: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_activity_handoff_groups")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(
                groups, 0,
                "source prerequisite refusal cannot allocate a handoff"
            );
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await?;
        }
        Ok(())
    });
    eprintln!(
        "M26_SOURCE_ALIASES POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
