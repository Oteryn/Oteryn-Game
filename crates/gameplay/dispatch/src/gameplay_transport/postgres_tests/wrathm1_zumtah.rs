use super::fixture::debug;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::Harness};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

async fn install(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<EntityRef> {
    const NPC: &str = quest::NPC;
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    owner.states.chat.install_npc_talk(std::sync::Arc::new(
        NpcTalkBook::from_model_with_native_quests(&model, &[NPC]),
    ));
    owner
        .states
        .chat
        .admit(admitted.game_session_id, "Prisoner", owner.owner_now());
    owner.states.chat.set_npc_service(admitted.game_session_id);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut runtime = owner.runtime.lock().await;
    runtime
        .bind_attacker_lease(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    runtime
        .place_npcs(
            &NpcPlacementTable {
                npcs: vec![NpcPlacements {
                    key: NPC.into(),
                    placements: vec![NpcPlacement {
                        x: 33367,
                        y: 31205,
                        floor: -8,
                        direction: MovementFacing::North,
                    }],
                }],
            },
            |_| true,
            true,
        )
        .map_err(debug)?;
    runtime
        .visible_entities()
        .npcs
        .into_iter()
        .find_map(|npc| {
            (runtime.npc_key(npc.actor) == Some(NPC)).then_some(EntityRef {
                identity: npc.actor.placement_identity(),
                generation: npc.generation,
            })
        })
        .ok_or_else(|| "source Zumtah placement".into())
}

use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::{world_callbacks::Position, wrath_zumtah as quest};

pub(super) const NAME: &str = "qengine-rewards-wrathm1-zumtah";
pub(super) const START: Position = Position {
    x: 33366,
    y: 31205,
    floor: -8,
};

#[test]
fn wrathm1_zumtah_dialogue_arms_release_and_projects_beetle() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, START, false)
    })
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    super::questpc8_native_slice::world(
        native,
        &[(33365, 31204, 33368, 31206, -8)],
        &[],
        TilePos {
            x: START.x,
            y: START.y,
            floor: START.floor,
        },
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let npc = install(owner, admitted).await?;
    let path = [
        "hi", "exit", "no", "yes", "exit", "no", "yes", "exit", "no", "yes", "278", "exit", "exit",
        "exit", "exit", "exit",
    ];
    for (index, text) in path.into_iter().enumerate() {
        super::classic_quests::talk(owner, admitted, npc, 7200 + index as u64, text).await?;
    }
    for key in [quest::STATUS, quest::RELEASE] {
        let value: i64 =
            sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
                .bind(key)
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(value, 1);
    }
    let actor = admitted.runtime_actor.ok_or("actor")?;
    {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let player = states
            .get(&runtime, actor, admitted.game_session_id)
            .ok_or("spell state")?;
        assert_eq!(
            player.owned_conditions().outfit_at(owner.owner_now().get()),
            Some(348)
        );
    }
    super::classic_quests::talk(owner, admitted, npc, 7300, "exit").await?;
    let repeats: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(quest::RETURN)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(repeats, 1);
    Ok(())
}
