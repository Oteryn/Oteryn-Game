//! Visible-player filter facts; no Account IDs, other Guild IDs, ranks or private roster data.
use super::{
    ComposedFreshAdmission,
    connection::{AdmissionRefusal, AdmittedSession},
    item_look::{RuntimeLook, runtime_look_creature},
};
use crate::spell::Vocation;
use oteryn_protocol_oteryn::{
    battle_metadata::{PlayerMetadata, encode_reply},
    world_spatial_entities::EntityRef,
};
fn vocation_family(vocation: Vocation) -> u8 {
    match vocation {
        Vocation::None => 0,
        Vocation::Sorcerer | Vocation::MasterSorcerer => 1,
        Vocation::Druid | Vocation::ElderDruid => 2,
        Vocation::Paladin | Vocation::RoyalPaladin => 3,
        Vocation::Knight | Vocation::EliteKnight => 4,
        Vocation::Monk | Vocation::ExaltedMonk => 5,
    }
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn battle_metadata_page(
        &self,
        admitted: AdmittedSession,
        refs: &[EntityRef],
    ) -> Option<Vec<u8>> {
        let actor = admitted.runtime_actor?;
        let fence = admitted.item_fence?;
        let mut rows = refs
            .iter()
            .map(|entity| PlayerMetadata {
                entity: *entity,
                vocation: None,
                same_guild: None,
            })
            .collect::<Vec<_>>();
        let observations = {
            let mut runtime = self.runtime.lock().await;
            if !super::battle_instinct_status::actor_is_current(&runtime, admitted) {
                return None;
            }
            let mut observations = Vec::new();
            for (index, reference) in refs.iter().enumerate() {
                if let Ok(RuntimeLook::Player {
                    actor: target,
                    session,
                    ..
                }) = runtime_look_creature(
                    &mut runtime,
                    self.states.chat.npc_talk(),
                    actor,
                    *reference,
                ) {
                    rows[index].vocation =
                        self.states.spell_states.try_lock().ok().and_then(|states| {
                            states
                                .get(&runtime, target, session)
                                .map(|state| vocation_family(state.character_facts().vocation))
                        });
                    let character = runtime
                        .bound_attacker_lease(target, session)
                        .ok()
                        .map(|lease| *lease.character_id().as_bytes());
                    observations.push((index, target, session, character));
                }
            }
            observations
        };
        let characters = observations
            .iter()
            .filter_map(|(index, _, _, character)| character.map(|character| (*index, character)))
            .collect::<Vec<_>>();
        if !characters.is_empty() {
            let ids = characters
                .iter()
                .map(|(_, character)| *character)
                .collect::<Vec<_>>();
            let retry = super::AdmissionRetryBudget::for_command(self.root);
            let allies = retry
                .retry_unavailable(async || {
                    self.root
                        .read_battle_guild_allies(self.character, self.holder, fence, &ids)
                        .await
                        .map_err(|error| match error {
                            crate::durability::guild::GuildError::Unavailable(
                                crate::durability::DurabilityError::RootUnavailable,
                            ) => AdmissionRefusal::Unavailable,
                            _ => AdmissionRefusal::Rejected,
                        })
                })
                .await
                .ok();
            if let Some(allies) = allies {
                for ((index, _), same) in characters.iter().zip(allies) {
                    rows[*index].same_guild = Some(same);
                }
            }
        }
        let mut runtime = self.runtime.lock().await;
        if !super::battle_instinct_status::actor_is_current(&runtime, admitted) {
            return None;
        }
        for (index, target, session, character) in observations {
            let current = matches!(runtime_look_creature(&mut runtime,self.states.chat.npc_talk(),actor,refs[index]),Ok(RuntimeLook::Player {actor:current,session:current_session,..}) if current==target && current_session==session);
            let same_character = character.is_none_or(|character| {
                runtime
                    .bound_attacker_lease(target, session)
                    .is_ok_and(|lease| lease.character_id().as_bytes() == &character)
            });
            if !current || !same_character {
                rows[index].vocation = None;
                rows[index].same_guild = None;
            }
        }
        encode_reply(&rows)
    }
}
macro_rules! authority_methods {
    () => {
        fn battle_metadata_query(
            &self,
            _admitted: $crate::gameplay_transport::AdmittedSession,
            _refs: &[oteryn_protocol_oteryn::world_spatial_entities::EntityRef],
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn battle_metadata_query(
            &self,
            admitted: connection::AdmittedSession,
            refs: &[oteryn_protocol_oteryn::world_spatial_entities::EntityRef],
        ) -> Option<Vec<u8>> {
            self.battle_metadata_page(admitted, refs).await
        }
    };
}
pub(super) use composed_methods;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn battle_metadata_preserves_promoted_vocation_families() -> Result<(), &'static str> {
        for (vocation, family) in [
            (Vocation::None, 0),
            (Vocation::Sorcerer, 1),
            (Vocation::MasterSorcerer, 1),
            (Vocation::ElderDruid, 2),
            (Vocation::RoyalPaladin, 3),
            (Vocation::EliteKnight, 4),
            (Vocation::ExaltedMonk, 5),
        ] {
            assert_eq!(vocation_family(vocation), family);
        }
        Ok(())
    }
}
