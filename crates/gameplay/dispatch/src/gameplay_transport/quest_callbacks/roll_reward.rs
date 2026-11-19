//! Crystal 201189f actions_sacrophagus_use.lua:31-44.
use super::*;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use sha2::{Digest, Sha256};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn prepare_roll_failure<'a>(
        &self,
        runtime: &mut ChannelRuntimeV1,
        map: &BundleMap,
        position: TilePos,
        creature: &'a str,
        command: CommandRef,
    ) -> Result<Option<(&'a str, i64, MovementLocalPosition)>, UseDisposition> {
        let native = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
            .ok_or(UseDisposition::Rejected)?;
        native
            .install_companion_policies(runtime)
            .map_err(|_| UseDisposition::Rejected)?;
        let health = runtime
            .companion_policy(creature)
            .map_err(|_| UseDisposition::Rejected)?
            .maximum_health;
        let seed = Sha256::new()
            .chain_update(b"oteryn:quest-roll-failure-placement:v1")
            .chain_update(command.game_session_id().as_bytes())
            .chain_update(command.command_id().get().to_be_bytes())
            .finalize();
        let landing = self.quest_creature_landing(
            runtime,
            map,
            MovementLocalPosition {
                x: i32::from(position.x),
                y: i32::from(position.y),
                floor: -i16::from(position.floor),
            },
            &seed,
        )?;
        Ok(landing.map(|to| (creature, health, to)))
    }
}
