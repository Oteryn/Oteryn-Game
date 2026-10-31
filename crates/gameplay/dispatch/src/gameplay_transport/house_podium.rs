//! W18: house podium display query, set and viewer projection (capability 150).
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use crate::content::creature_reward::CreatureRewardTable;
use crate::content::native_gameplay::NativeCosmeticsView;
use crate::domain::appearance::{AppearanceContent, EarnedCosmetic, Sex, UnlockFact};
use crate::durability::house_podium::{HousePodiumRefusal, HousePodiumSet};
use crate::durability::house_scope_handoff::HouseId;
use oteryn_gameplay_housing::house_podium::PodiumContent;
use oteryn_protocol_oteryn::house_podium as wire;
use std::sync::Arc;

pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    let mut offers: Vec<_> = base
        .iter()
        .copied()
        .filter(|c| c.id != wire::CAPABILITY_HOUSE_PODIUM_V1)
        .collect();
    if available {
        let at = offers.partition_point(|c| c.id < wire::CAPABILITY_HOUSE_PODIUM_V1);
        offers.insert(
            at,
            OfferedCapability {
                id: wire::CAPABILITY_HOUSE_PODIUM_V1,
                requires: &[49],
            },
        );
    }
    offers
}

/// One decoded command of capability 150.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Query([u8; 16]),
    Set(wire::SetRequest),
    Projection([u8; 16]),
}

impl Request {
    pub fn decode(command_type: u32, payload: &[u8]) -> Option<Self> {
        match command_type {
            wire::COMMAND_QUERY => wire::decode_item(payload).map(Self::Query),
            wire::COMMAND_SET => wire::SetRequest::decode(payload).map(Self::Set),
            wire::COMMAND_PROJECTION => wire::decode_item(payload).map(Self::Projection),
            _ => None,
        }
    }
}

macro_rules! authority_methods {
    () => {
        fn house_podium_available(&self) -> bool {
            false
        }
        fn house_podium(
            &self,
            _: $crate::gameplay_transport::connection::AdmittedSession,
            _: $crate::gameplay_transport::house_podium::Request,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn house_podium_available(&self) -> bool {
            true
        }
        async fn house_podium(
            &self,
            admitted: connection::AdmittedSession,
            request: house_podium::Request,
        ) -> Option<Vec<u8>> {
            house_podium::execute(self, admitted, request).await
        }
    };
}
pub(super) use composed_methods;

/// The active generation's cosmetics, bosses and Bestiary, checked by the store before a
/// write (Crystal `game.cpp:4692-4699`, `game.cpp:11149`).
struct PodiumContentView {
    cosmetics: NativeCosmeticsView,
    rewards: Arc<CreatureRewardTable>,
}

impl AppearanceContent<String> for PodiumContentView {
    fn palette_size(&self) -> u16 {
        self.cosmetics.palette_size()
    }
    fn earned_grant_keys(&self) -> Vec<String> {
        self.cosmetics.earned_grant_keys()
    }
    fn earned_unlock(
        &self,
        grant_key: &str,
        provenance: &str,
        target: EarnedCosmetic<'_>,
    ) -> UnlockFact {
        self.cosmetics.earned_unlock(grant_key, provenance, target)
    }
    fn outfit_addon_count(&self, outfit: &String) -> Option<u8> {
        self.cosmetics.outfit_addon_count(outfit)
    }
    fn outfit_is_starter(&self, outfit: &String, sex: Sex) -> bool {
        self.cosmetics.outfit_is_starter(outfit, sex)
    }
    fn addon_is_starter(&self, outfit: &String, sex: Sex, addon: u8) -> bool {
        self.cosmetics.addon_is_starter(outfit, sex, addon)
    }
    fn mount_exists(&self, mount: &String) -> bool {
        self.cosmetics.mount_exists(mount)
    }
}

impl PodiumContent for PodiumContentView {
    fn has_outfit(&self, key: &str) -> bool {
        self.cosmetics.outfit_addon_count(&key.to_owned()).is_some()
    }
    fn has_mount(&self, key: &str) -> bool {
        self.cosmetics.mount_exists(&key.to_owned())
    }
    fn has_creature(&self, key: &str, boss: bool) -> bool {
        if boss {
            self.rewards.boss(key).is_some()
        } else {
            super::charm_progression::bestiary_race_known(key)
        }
    }
}

fn refusal(refusal: HousePodiumRefusal) -> wire::SetRefusal {
    use wire::SetRefusal as R;
    match refusal {
        HousePodiumRefusal::StaleSession => R::StaleSession,
        HousePodiumRefusal::NoAccess => R::NoAccess,
        HousePodiumRefusal::NotHouseItem => R::NotHouseItem,
        HousePodiumRefusal::WrongItem => R::WrongItem,
        HousePodiumRefusal::WrongDisplay => R::WrongDisplay,
        HousePodiumRefusal::UnknownContent => R::UnknownContent,
        HousePodiumRefusal::NotOwned => R::NotOwned,
        HousePodiumRefusal::Changed => R::Changed,
        HousePodiumRefusal::InvalidInput => R::InvalidInput,
    }
}

/// Every reply comes from a committed durable read or write; nothing is shown before its
/// commit. A missing fence or an unavailable store ends the command without a reply.
pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    request: Request,
) -> Option<Vec<u8>> {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(wire::CAPABILITY_HOUSE_PODIUM_V1)
    {
        return None;
    }
    let fence = admitted.item_fence?;
    let actor = admitted.runtime_actor?;
    let house = {
        let runtime = owner.runtime.lock().await;
        runtime
            .house_actor_state(actor, admitted.game_session_id)
            .ok()?
            .house
    };
    let house = HouseId::new(admitted.world_id, &house)?;
    let item = match request {
        Request::Query(item) | Request::Projection(item) => item,
        Request::Set(set) => {
            let gameplay = owner.active_generation?.native_gameplay()?;
            let content = PodiumContentView {
                cosmetics: gameplay.cosmetics()?.clone(),
                rewards: gameplay.reward_table().clone(),
            };
            let request = HousePodiumSet {
                item_instance_id: set.item,
                expected_revision: set.expected_revision,
                display_kind: set.display_kind,
                display_key: set.key,
                direction: set.direction,
            };
            let result = owner
                .root
                .set_house_podium(owner.holder, fence, house, request, content)
                .await
                .ok()?;
            return Some(
                match result {
                    Ok(revision) => wire::SetResult::Accepted { revision },
                    Err(error) => wire::SetResult::Refused(refusal(error)),
                }
                .encode(),
            );
        }
    };
    let displays = owner
        .root
        .load_house_podiums(owner.holder, fence, house)
        .await
        .ok()?
        .ok()?;
    let projection = displays
        .into_iter()
        .find(|display| display.item_instance_id == item)
        .map(|display| wire::Projection {
            podium_kind: display.podium_kind,
            display_kind: display.display_kind,
            direction: display.direction,
            revision: display.revision,
            key: display.display_key,
        });
    wire::Projection::encode(projection.as_ref())
}
