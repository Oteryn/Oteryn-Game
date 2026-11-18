//! Shared fixture for invoking the real NPC talk dispatcher.
use super::fixture::debug;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

pub(super) async fn install<const N: usize>(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    keys: [&str; N],
) -> TestResult<[EntityRef; N]> {
    install_with_positions(owner, admitted, keys, None).await
}

pub(super) async fn install_positions<const N: usize>(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    keys: [&str; N],
    positions: [Position; N],
) -> TestResult<[EntityRef; N]> {
    install_with_positions(owner, admitted, keys, Some(positions)).await
}

async fn install_with_positions<const N: usize>(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    keys: [&str; N],
    positions: Option<[Position; N]>,
) -> TestResult<[EntityRef; N]> {
    install_offsets(owner, admitted, keys, positions, |i| {
        if N > 5 {
            (i % 5 - 2, 1 + i / 5)
        } else {
            (i, 1)
        }
    })
    .await
}

pub(super) async fn install_with_offsets<const N: usize>(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    keys: [&str; N],
    offsets: Option<[(i32, i32); N]>,
) -> TestResult<[EntityRef; N]> {
    install_offsets(owner, admitted, keys, None, |i| {
        offsets
            .as_ref()
            .and_then(|offsets| usize::try_from(i).ok().and_then(|i| offsets.get(i)))
            .copied()
            .unwrap_or((i, 1))
    })
    .await
}

pub(super) async fn install_above<const N: usize>(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    keys: [&str; N],
) -> TestResult<[EntityRef; N]> {
    install_offsets(owner, admitted, keys, None, |i| (i, -1)).await
}

async fn install_offsets<const N: usize>(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    keys: [&str; N],
    positions: Option<[Position; N]>,
    offset: impl Fn(i32) -> (i32, i32),
) -> TestResult<[EntityRef; N]> {
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    let source =
        oteryn_gameplay_quests::quest::npc_rules::NpcRuleCatalog::runtime().map_err(debug)?;
    let native_npcs = source
        .programs()
        .iter()
        .flat_map(|program| program.document.rules.iter().map(|rule| rule.npc.as_str()))
        .chain(std::iter::once("oteryn:npc.viola"))
        .chain(keys.iter().copied())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    owner.states.chat.install_npc_talk(std::sync::Arc::new(
        NpcTalkBook::from_model_with_native_quests(&model, &native_npcs),
    ));
    owner
        .states
        .chat
        .admit(admitted.game_session_id, "Quest witness", owner.owner_now());
    owner.states.chat.set_npc_service(admitted.game_session_id);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut runtime = owner.runtime.lock().await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    runtime
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let at = runtime
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    let mut placements = keys
        .into_iter()
        .enumerate()
        .map(|(i, key)| {
            let p = if let Some(positions) = positions.as_ref() {
                positions[i]
            } else {
                let (dx, dy) = offset(i32::try_from(i)?);
                Position {
                    x: u16::try_from(at.x + dx)?,
                    y: u16::try_from(at.y + dy)?,
                    floor: -i8::try_from(at.floor)?,
                }
            };
            Ok(NpcPlacements {
                key: key.into(),
                placements: vec![NpcPlacement {
                    x: p.x,
                    y: p.y,
                    floor: p.floor,
                    direction: MovementFacing::South,
                }],
            })
        })
        .collect::<TestResult<Vec<_>>>()?;
    placements.sort_by(|left, right| left.key.cmp(&right.key));
    runtime
        .place_npcs(&NpcPlacementTable { npcs: placements }, |_| true, true)
        .map_err(debug)?;
    let mut found = Vec::new();
    for key in keys {
        found.push(
            runtime
                .visible_entities()
                .npcs
                .into_iter()
                .find_map(|npc| {
                    (runtime.npc_key(npc.actor) == Some(key)).then_some(EntityRef {
                        identity: npc.actor.placement_identity(),
                        generation: npc.generation,
                    })
                })
                .ok_or("placed source NPC")?,
        );
    }
    found.try_into().map_err(|_| "exact NPC refs".into())
}

pub(super) async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc: EntityRef,
    command_id: u64,
    text: &str,
) -> TestResult<Vec<String>> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let result = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            },
        )
        .await;
    assert_eq!(
        result.disposition,
        NpcIntentDisposition::Ok,
        "{command_id}: {text}"
    );
    assert!(!result.unknown);
    Ok(result.conversation.map(|c| c.lines).unwrap_or_default())
}
