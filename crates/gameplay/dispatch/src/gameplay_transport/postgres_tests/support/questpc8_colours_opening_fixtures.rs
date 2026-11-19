use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use super::fixture::{Harness, debug, id};

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

use oteryn_protocol_oteryn::world_object::UseDisposition;

const STAGE: &str = "oteryn:quest-progress/chosen/colours/stage";

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    eprintln!(
        "POSTGRES_CASE_STARTED questpc8_colours_opening after PostgreSQL fixture initialization"
    );
    assert!(
        oteryn_gameplay_quests::quest::reward_callbacks::load()?
            .iter()
            .any(|c| c.key == "oteryn:quest-reward/chosen/colours/envelope"),
        "source envelope binding missing"
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("native item capabilities")?;
    super::questpc8_mystery_geometry_fixtures::arrive(
        owner,
        admitted,
        crate::map::overlay::TilePos {
            x: 33353,
            y: 31410,
            floor: -8,
        },
    )
    .await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: 33353,
                y: 31410,
                floor: -8,
            },
            item: "oteryn:item.tibia.i12204".into(),
            count: 1,
        },
        875,
        true,
    )
    .await?;
    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: 33353,
                y: 31410,
                floor: -8,
            },
            item: "oteryn:item.tibia.i12204".into(),
            count: 1,
        },
        876,
        true,
    )
    .await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    super::qengine_voucher_stories_fixtures::use_native(owner, admitted, 12204, UseDisposition::Committed)
        .await?;
    assert_eq!(
        quantity(h, 12204).await?,
        1,
        "only the selected envelope is consumed"
    );
    assert_eq!(
        quantity(h, 12205).await?,
        1,
        "source letter accompanies powder"
    );
    assert_eq!(quantity(h, 12208).await?, 1);
    assert_eq!(stage(h).await?, 1);
    super::qengine_voucher_stories_fixtures::use_native(owner, admitted, 12208, UseDisposition::Committed)
        .await?;
    assert_eq!(quantity(h, 12208).await?, 0);
    assert_eq!(
        quantity(h, 12228).await?,
        1,
        "remaining blue powder is retained for peer pairing"
    );
    assert_eq!(stage(h).await?, 2);
    owner
        .states
        .quest_sessions
        .lock()
        .map_err(debug)?
        .remove(&admitted.game_session_id);
    owner.admit_quest_session(admitted).await;
    assert_eq!(stage(h).await?, 2, "progress survives cold readmission");
    super::qengine_voucher_stories_fixtures::use_native(owner, admitted, 12204, UseDisposition::Rejected)
        .await?;
    assert_eq!(
        quantity(h, 12204).await?,
        1,
        "another envelope cannot rearm the once-only opening"
    );
    assert_eq!(quantity(h, 12228).await?, 1);
    let complete: bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(id(41).as_slice()).bind("oteryn:quest.authored.the_colours_of_magic").fetch_one(&h.pool).await?;
    assert!(
        !complete,
        "self marking never fabricates a successful live-player pairing"
    );
    eprintln!("POSTGRES_COLOURS_ENVELOPE_SELF_MARK_AND_REPLAY_PASS");
    Ok(())
}

async fn quantity(h: &Harness, item: u16) -> TestResult<i64> {
    super::qengine_voucher_stories_fixtures::quantity(h, item).await
}

async fn stage(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),-1)").bind(id(41).as_slice()).bind(STAGE).fetch_one(&h.pool).await?)
}
