//! Source close-position selection uses a projectile check independently of movement.
//! Crystal 201189f data/libs/functions/tile.lua:39-50 (no-argument isWalkable).
use super::*;

impl MonsterMap {
    /// Current movement admission plus Tile:isWalkable()'s unconditional projectile refusal.
    /// Unknown or stale views withhold the candidate rather than inventing tile facts.
    pub fn admits_close_spawn_candidate(
        &self,
        runtime: &ChannelRuntimeV1,
        at: MovementLocalPosition,
    ) -> Option<bool> {
        if !self.matches(runtime) {
            return None;
        }
        self.tile(at).map(|tile| tile.admits && !tile.projectile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::{ChannelId, NodeId, WorldId};
    use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
    type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

    #[test]
    fn close_spawn_rejects_projectile_only_barriers_without_changing_movement() -> TestResult {
        let id = |seed| {
            let mut bytes = [seed; 16];
            bytes[6] = 0x70;
            bytes[8] = 0x80;
            bytes
        };
        let debug = |error| format!("{error:?}");
        let world = WorldId::decode(&id(1)).map_err(debug)?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).map_err(debug)?,
            NodeId::decode(&id(3)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            ChannelContentPin::test(world),
        )
        .map_err(|error| format!("{error:?}"))?;
        let pin = runtime.content_pin();
        let mut map = MonsterMap {
            valid: true,
            scope: runtime.binding().runtime_scope(),
            generation: runtime.binding().scope_generation(),
            context: runtime.pinned_movement_context(),
            activation: pin.activation_sequence(),
            server: pin.server_artifact_digest(),
            client: pin.client_artifact_digest(),
            frame: pin.frame_binding_digest(),
            map: pin.map_revision_digest(),
            tiles: BTreeMap::new(),
        };
        let at = MovementLocalPosition {
            x: 100,
            y: 100,
            floor: 7,
        };
        assert_eq!(map.admits_close_spawn_candidate(&runtime, at), None);
        map.tiles.insert(
            (at.x, at.y, at.floor),
            Some(Tile {
                admits: true,
                projectile: true,
                speed: 150,
                field: None,
            }),
        );
        assert!(
            map.admits(at),
            "projectile-only barriers remain movement-admissible"
        );
        assert_eq!(map.admits_close_spawn_candidate(&runtime, at), Some(false));
        map.tiles.insert(
            (at.x, at.y, at.floor),
            Some(Tile {
                admits: true,
                projectile: false,
                speed: 150,
                field: None,
            }),
        );
        assert_eq!(map.admits_close_spawn_candidate(&runtime, at), Some(true));
        map.tiles.insert(
            (at.x, at.y, at.floor),
            Some(Tile {
                admits: false,
                projectile: false,
                speed: 0,
                field: None,
            }),
        );
        assert_eq!(map.admits_close_spawn_candidate(&runtime, at), Some(false));
        runtime.retire_owner_cycle();
        assert_eq!(map.admits_close_spawn_candidate(&runtime, at), None);
        Ok(())
    }
}
