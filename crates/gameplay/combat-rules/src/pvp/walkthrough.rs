//! Global player-on-player collision policy.
use super::legality::{PvpRelation, WorldPvpType};

pub fn player_walkthrough_allowed(
    world: WorldPvpType,
    destination_protection_zone: bool,
    relation: &PvpRelation,
) -> bool {
    if destination_protection_zone {
        return true;
    }
    match world {
        WorldPvpType::Optional => !relation.war_between,
        WorldPvpType::Open => {
            (relation.same_party_or_guild == Some(true) && !relation.friendly_fire)
                || !(relation.war_between
                    || relation.target_aggressor_of_actor
                    || relation.target_aggressor_of_actor_group
                    || relation.friendly_fire)
        }
        WorldPvpType::Retro | WorldPvpType::Hardcore => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn relation() -> PvpRelation {
        PvpRelation {
            war_between: false,
            same_party_or_guild: Some(false),
            friendly_fire: false,
            target_aggressor_of_actor: false,
            target_aggressor_of_actor_group: false,
        }
    }
    #[test]
    fn matrix() {
        for world in [
            WorldPvpType::Optional,
            WorldPvpType::Open,
            WorldPvpType::Retro,
            WorldPvpType::Hardcore,
        ] {
            assert!(player_walkthrough_allowed(world, true, &relation()));
        }
        assert!(player_walkthrough_allowed(
            WorldPvpType::Optional,
            false,
            &relation()
        ));
        assert!(player_walkthrough_allowed(
            WorldPvpType::Open,
            false,
            &relation()
        ));
        assert!(!player_walkthrough_allowed(
            WorldPvpType::Retro,
            false,
            &relation()
        ));
        assert!(!player_walkthrough_allowed(
            WorldPvpType::Hardcore,
            false,
            &relation()
        ));
        let mut hostile = relation();
        hostile.war_between = true;
        assert!(!player_walkthrough_allowed(
            WorldPvpType::Optional,
            false,
            &hostile
        ));
        assert!(!player_walkthrough_allowed(
            WorldPvpType::Open,
            false,
            &hostile
        ));
    }
}
