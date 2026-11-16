//! GUILD-EMBLEM-1: Tibia's guild emblem of each positioned player, relative to the session's
//! character, from the durable guild membership and guild-war owner.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::guild_emblem::{CAPABILITY_GUILD_EMBLEM_V1, GuildEmblem};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn observe_own_guild_emblems(
        &self,
        admitted: AdmittedSession,
    ) -> Option<Vec<GuildEmblem>> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_GUILD_EMBLEM_V1)
        {
            return None;
        }
        let fence = admitted.item_fence?;
        if fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return None;
        }
        // The Channel's positioned players and their bound characters, under the runtime lock.
        let players: Vec<(EntityRef, [u8; 16])> = {
            let runtime = self.runtime.lock().await;
            let visible = runtime.visible_entities();
            let mut players = Vec::new();
            for (actor, _, session) in runtime.positioned_actor_census().ok()? {
                let Some(session) = session else { continue };
                let Ok(lease) = runtime.bound_attacker_lease(actor, session) else {
                    continue;
                };
                let Some(entity) = visible.players.iter().find(|p| p.actor == actor) else {
                    continue;
                };
                players.push((
                    EntityRef {
                        identity: actor.placement_identity(),
                        generation: entity.generation,
                    },
                    *lease.character_id().as_bytes(),
                ));
            }
            players
        };
        let viewed = players.iter().map(|(_, character)| *character).collect();
        let emblems = self
            .root
            .read_guild_emblems(self.character, self.holder, fence, viewed)
            .await
            .ok()??;
        Some(
            players
                .iter()
                .filter_map(|(entity, character)| {
                    let (_, emblem) = emblems.iter().find(|(id, _)| id == character)?;
                    Some(GuildEmblem {
                        entity: *entity,
                        emblem: *emblem,
                    })
                })
                .collect(),
        )
    }
}
