//! BOOSTED-1: the committed World/server-save choice shared by queries, kills and home spawns.
use super::{ComposedFreshAdmission, connection::AdmittedSession, guild_intent::GuildReply};
use crate::content::ProjectV2AuthoringProfileData;
use crate::durability::daily_boost::DailyBoostBinding;
use crate::foundation::RuntimeScopeRefV1;
use oteryn_protocol_oteryn::daily_boost as wire;

pub(super) fn offered(
    base: &[super::capabilities::OfferedCapability],
    available: bool,
) -> Vec<super::capabilities::OfferedCapability> {
    let mut offers: Vec<_> = base
        .iter()
        .copied()
        .filter(|c| c.id != wire::CAPABILITY_DAILY_BOOST_V1)
        .collect();
    if available {
        let at = offers.partition_point(|c| c.id < wire::CAPABILITY_DAILY_BOOST_V1);
        offers.insert(
            at,
            super::capabilities::OfferedCapability {
                id: wire::CAPABILITY_DAILY_BOOST_V1,
                requires: &[],
            },
        );
    }
    offers
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn daily_boost_pools(&self) -> (Vec<String>, Vec<String>) {
        let mut creatures = Vec::new();
        let mut bosses = Vec::new();
        if let Some(native) = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        {
            for record in &native.creature_profiles().records {
                let ProjectV2AuthoringProfileData::Creature(profile) = &record.profile.data else {
                    continue;
                };
                if profile.bestiary.is_some()
                    && profile.bosstiary.is_none()
                    && profile.familiar.is_none()
                    && !profile.details.as_ref().is_some_and(|d| {
                        d.bosstiary.is_some()
                            || d.system_eligibility.reward_boss
                            || d.summoning.is_familiar
                    })
                {
                    creatures.push(record.profile.target.key.clone());
                }
                if profile
                    .bosstiary
                    .as_ref()
                    .map(|b| b.category.as_str())
                    .or_else(|| {
                        profile
                            .details
                            .as_ref()?
                            .bosstiary
                            .as_ref()
                            .map(|b| b.category.as_str())
                    })
                    .is_some_and(|category| category.eq_ignore_ascii_case("archfoe"))
                {
                    bosses.push(record.profile.target.key.clone());
                }
            }
        }
        (creatures, bosses)
    }

    pub(super) async fn initialize_daily_boost(&self) -> Result<(), ()> {
        self.load_daily_boost(false).await
    }

    async fn load_daily_boost(&self, refresh: bool) -> Result<(), ()> {
        let Some(map) = self.bundle_map() else {
            return Ok(());
        };
        let mut runtime = self.runtime.lock().await;
        runtime.owner_fence().map_err(|_| ())?;
        if self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
            .is_some_and(|native| {
                native.source_digest() != runtime.content_pin().server_artifact_digest()
            })
        {
            return Err(());
        }

        if !refresh
            && runtime
                .daily_boost()
                .is_some_and(|choice| choice.reset_epoch == self.reset_epoch)
        {
            return Ok(());
        }
        // Small qualification worlds with no eligible pools have no boosted mechanic.
        let pools = self.daily_boost_pools();
        if pools.0.is_empty() || pools.1.is_empty() {
            return Ok(());
        }
        drop(runtime);
        let permit = self.spell_lane_permit().await.ok_or(())?;
        runtime = self.runtime.lock().await;
        let binding = runtime.binding();
        let selection = self
            .root
            .prepare_daily_boost(
                self.character,
                self.holder,
                &permit,
                DailyBoostBinding {
                    scope: RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
                    generation: binding.scope_generation().get(),
                    bundle: map.overlay.base().digest(),
                    epoch: self.reset_epoch,
                },
                pools,
            )
            .await
            .map_err(|_| ())?;
        runtime.install_daily_boost(selection).map_err(|_| ())
    }

    pub(super) async fn daily_boost_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        kind: u32,
        payload: &[u8],
    ) -> GuildReply {
        let Some(fence) = admitted.item_fence else {
            return GuildReply::Rejected;
        };
        if fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
            || id == 0
            || kind != wire::COMMAND_TYPE_DAILY_BOOST_QUERY
            || !admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_DAILY_BOOST_V1)
            || wire::decode_query(payload).is_err()
        {
            return GuildReply::Rejected;
        }
        if self.load_daily_boost(true).await.is_err() {
            return GuildReply::Unavailable;
        }
        let runtime = self.runtime.lock().await;
        if runtime.owner_fence().is_err()
            || fence.runtime_scope
                != RuntimeScopeRefV1::channel(admitted.world_id, admitted.channel_id)
            || fence.scope_ownership_generation != runtime.binding().scope_generation()
            || runtime.binding().world_id() != admitted.world_id
            || runtime.binding().channel_id() != admitted.channel_id
            || !admitted.runtime_actor.is_some_and(|actor| {
                runtime
                    .player_control_facts(actor, admitted.game_session_id)
                    .is_ok_and(|f| f.control_loss.is_none())
            })
        {
            return GuildReply::Rejected;
        }
        let Some(choice) = runtime.daily_boost() else {
            return GuildReply::Rejected;
        };
        match wire::encode_view(&wire::DailyBoostView {
            reset_epoch: choice.reset_epoch,
            creature_key: choice.creature.clone(),
            boss_key: choice.boss.clone(),
        }) {
            Ok(bytes) => GuildReply::Payload(bytes),
            Err(_) => GuildReply::Unavailable,
        }
    }
}

macro_rules! authority_methods {
    () => {
        fn daily_boost_available(&self) -> bool {
            false
        }
        fn daily_boost_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _id: u64,
            _kind: u32,
            _payload: &[u8],
        ) -> impl std::future::Future<Output = $crate::gameplay_transport::guild_intent::GuildReply>
        {
            async { $crate::gameplay_transport::guild_intent::GuildReply::Rejected }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn daily_boost_available(&self) -> bool {
            if self.bundle_map().is_none() {
                return false;
            }
            let pools = self.daily_boost_pools();
            !pools.0.is_empty() && !pools.1.is_empty()
        }
        async fn daily_boost_command(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            kind: u32,
            payload: &[u8],
        ) -> crate::gameplay_transport::guild_intent::GuildReply {
            self.daily_boost_wire(admitted, id, kind, payload).await
        }
    };
}
pub(super) use composed_methods;
