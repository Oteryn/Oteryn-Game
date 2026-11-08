//! Physical custody for the exact source root; daily route qualification is separate.
use super::fixture::{Harness, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

#[test]
fn questpc5_oramond_five_exact_juicy_roots_enter_native_backpack() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-oramond-roots")
}

#[test]
fn questpc5_cook_fryclops_axe_exact_item_enters_native_backpack() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-cook-axe")
}
#[test]
fn questpc5_cook_ratha_empty_exact_item_enters_native_backpack() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-cook-empty")
}
#[test]
fn questpc5_cook_ratha_full_exact_item_enters_native_backpack() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-cook-full")
}

pub(super) async fn exercise_cook(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    if name == "qengine-rewards-cook-contacts" {
        return exercise_contacts(owner, admitted, h).await;
    }
    let item = match name {
        "qengine-rewards-cook-axe" => 44525,
        "qengine-rewards-cook-empty" => 44527,
        "qengine-rewards-cook-full" => 44528,
        _ => return Err("unknown cook material case".into()),
    };
    exercise_item(owner, admitted, h, item, 1, "cook_material_custody").await
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    exercise_item(owner, admitted, h, 21291, 5, "oramond_root_custody").await
}

async fn exercise_item(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    item: u32,
    quantity: u16,
    marker: &str,
) -> TestResult {
    let started = std::time::Instant::now();
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_CASE_STARTED {marker} item={item} server_version={version}");
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            item: format!("oteryn:item.tibia.i{item}"),
            count: quantity,
        },
        91,
        true,
    )
    .await?;
    let amount: i64 = sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key=$2 AND i.lifecycle=1 AND e.character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?;
    assert_eq!(
        amount,
        i64::from(quantity),
        "exact source quantity survives native backpack custody"
    );
    let quests: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        quests, 0,
        "physical item admission does not complete a quest"
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED {marker} item={item} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn questpc5_twenty_years_contacts_issue_one_exact_empty_ratha_container() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-cook-contacts")
}

pub(super) fn cook_contact_world() -> TestResult<crate::map::boot::BundleWorld> {
    super::questpc5_serpentine_route::source_world(
        &[(128, 125)],
        Position {
            x: 32844,
            y: 32138,
            floor: -6,
        },
    )
}

#[test]
fn questpc5_cook_trashcan_survives_actual_bundle_compilation() -> TestResult {
    use crate::map::view::MapFacts;
    let world = cook_contact_world()?;
    let pos = crate::map::overlay::TilePos { x: 32974, y: 32110, floor: -7 };
    let tile = world.base().tile(pos.x, pos.y, pos.floor).ok_or("trashcan tile")?;
    let (id, _) = tile.ids().iter().zip(tile.depths())
        .filter(|(_, depth)| **depth == 0).nth(1).ok_or("live trashcan ordinal")?;
    assert_eq!(world.facts().base_entry(pos, 1, *id).ok_or("trashcan facts")?.appearance_id, 44563);
    Ok(())
}

async fn exercise_contacts(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let started = std::time::Instant::now();
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_CASE_STARTED cook_native_contacts server_version={version}");
    let [cheaty, draccoon] = super::classic_quests::install_source(
        owner,
        admitted,
        ["oteryn:npc.cheaty", "oteryn:npc.the_draccoon_npc"],
    )
    .await?;
    let stage = || {
        sqlx::query_scalar::<_, i64>("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/cook-twenty-years/stage'").bind(id(41).to_vec())
    };
    assert_eq!(stage().fetch_optional(&h.pool).await?.unwrap_or(-1), -1);
    super::quest_catalogue_chests::relocate_source_actor(
        owner,
        admitted,
        admitted.runtime_actor.ok_or("actor")?,
        Position {
            x: 32973,
            y: 32111,
            floor: -7,
        },
    )
    .await?;
    assert_ne!(
        super::questpc5_banshees::use_at(
            owner,
            admitted,
            Position {
                x: 32974,
                y: 32110,
                floor: -7
            }
        )
        .await?,
        oteryn_protocol_oteryn::world_object::UseDisposition::Committed,
        "the lair entry requires Cheaty's assignment"
    );
    assert_eq!(stage().fetch_optional(&h.pool).await?.unwrap_or(-1), -1);
    for (index, (npc, at, word)) in [
        (
            cheaty,
            Position {
                x: 32844,
                y: 32138,
                floor: -6,
            },
            "mission",
        ),
        (
            draccoon,
            Position {
                x: 32973,
                y: 32111,
                floor: -7,
            },
            "mission",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        super::quest_catalogue_chests::relocate_source_actor(
            owner,
            admitted,
            admitted.runtime_actor.ok_or("actor")?,
            at,
        )
        .await?;
        if index == 1 {
            assert_eq!(
                super::questpc5_banshees::use_at(
                    owner,
                    admitted,
                    Position {
                        x: 32974,
                        y: 32110,
                        floor: -7,
                    }
                )
                .await?,
                oteryn_protocol_oteryn::world_object::UseDisposition::Committed,
                "the retained ominous trashcan reaches the Draccoon lair"
            );
            assert_eq!(stage().fetch_one(&h.pool).await?, 0);
        }
        assert_eq!(
            super::qengine_rewards::talk(owner, admitted, npc, 5001 + index as u64 * 2, "hi")
                .await?,
            oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok
        );
        assert_eq!(
            super::qengine_rewards::talk(owner, admitted, npc, 5002 + index as u64 * 2, word)
                .await?,
            oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok
        );
        assert_eq!(stage().fetch_one(&h.pool).await?, index as i64);
    }
    let _ = super::qengine_rewards::talk(owner, admitted, draccoon, 5005, "mission").await?;
    assert_eq!(
        stage().fetch_one(&h.pool).await?,
        1,
        "replay cannot advance the rescue"
    );
    let containers: i64 = sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i44527' AND i.lifecycle=1 AND e.character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(containers, 1);
    let complete: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.authored.20_years_a_cook_quest' AND completed_character_revision IS NOT NULL")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        complete, 0,
        "contacts alone do not qualify or complete the campaign"
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED cook_native_contacts duration={:?}",
        started.elapsed()
    );
    Ok(())
}
