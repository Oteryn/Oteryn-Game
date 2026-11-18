use super::fixture::{Harness, id};
use super::questpc8_authored_programmes::{object_target, step, use_target};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

#[test]
fn summit_bones_repeat_after_twenty_hours() -> TestResult {
    run("authored-tinder-box-qcx-repeat")
}

#[test]
fn summit_bones_do_not_require_step_credit() -> TestResult {
    run("authored-tinder-box-qcx-direct")
}

fn run(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(name, || {
        super::quest_world_callbacks::scenario(
            name,
            Position {
                x: 32084,
                y: 31025,
                floor: -1,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

async fn quantity(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances \
         WHERE definition_production_key='oteryn:item.tibia.i20357' AND lifecycle=1",
    )
    .fetch_one(&h.pool)
    .await?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    repeat: bool,
) -> TestResult {
    // The source describes a 20-hour repeat within the April season.
    // TibiaWiki curid 76272, oldid 800663.
    h.root.set_test_quest_clock(1_713_139_200)?;
    let bones = TilePos {
        x: 32085,
        y: 31025,
        floor: -1,
    };
    if repeat {
        owner.admit_premium(id(40), admitted.game_session_id);
        step(owner, admitted, 1, StepDirection::East).await?;
    } else {
        let premium = owner.premium_coordinator.take();
        step(owner, admitted, 1, StepDirection::East).await?;
        owner.premium_coordinator = premium;
        owner.admit_premium(id(40), admitted.game_session_id);
    }

    let target = object_target(owner, bones, 1)?;
    assert_eq!(
        use_target(owner, admitted, 2, &target).await?,
        UseDisposition::Committed
    );
    assert_eq!(quantity(h).await?, 1);
    if repeat {
        let target = object_target(owner, bones, 1)?;
        h.root.advance_test_quest_clock(20 * 60 * 60 - 1)?;
        assert_eq!(
            use_target(owner, admitted, 3, &target).await?,
            UseDisposition::Rejected
        );
        assert_eq!(quantity(h).await?, 1);
        h.root.advance_test_quest_clock(1)?;
        assert_eq!(
            use_target(owner, admitted, 4, &target).await?,
            UseDisposition::Committed
        );
        assert_eq!(quantity(h).await?, 2);
        let catalogue = owner.quest_catalogue.as_ref().ok_or("quest catalogue")?;
        assert!(
            oteryn_gameplay_quests::quest::npc_rules::legacy_defaults(
                catalogue,
                "oteryn:quest.tinder_box_quest_chyllfroest",
                &[
                    0x67, 0xa5, 0x3f, 0xc8, 0x46, 0x7f, 0x4a, 0x8f, 0x22, 0x9c, 0x96, 0x1c, 0x58,
                    0x81, 0x0c, 0x82, 0x47, 0xe3, 0xfa, 0xa5, 0x36, 0x7f, 0x48, 0x62, 0x14, 0xe0,
                    0xfe, 0x08, 0x5f, 0xa6, 0xfa, 0x37
                ],
            )
            .is_some()
        );
    }
    Ok(())
}
