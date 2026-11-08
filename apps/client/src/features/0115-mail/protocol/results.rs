use super::*;
impl Outbox {
    pub(in crate::play) fn properties_result(
        &self,
        command: &FeatureCommand,
        result: &GatedCommandResult,
    ) {
        let mut shared = lock(&self.0);
        if command.capability == 447 {
            oteryn_client_ui::imbue::details::apply(&mut shared.imbue, command, result);
            return;
        }
        match command.capability {
            49 => shared.properties.house.apply(command, result),
            51 | 330 => crate::play::feature_0051_staff::protocol::apply(
                &mut shared.properties.staff,
                command,
                result,
            ),
            25 => crate::play::feature_0025_appearance::protocol::apply(
                &mut shared.properties.appearance,
                command,
                result,
            ),
            39 => crate::play::feature_0039_timed_item::protocol::apply(
                &mut shared.properties.timed_item,
                command,
                result,
            ),
            54 => crate::play::feature_0054_training::protocol::apply(
                &mut shared.properties.training,
                command,
                result,
            ),
            55 => crate::play::feature_0055_guild_bank::protocol::apply(
                &mut shared.properties.guild_bank,
                command,
                result,
            ),
            57 => crate::play::feature_0057_cyclopedia_map::protocol::apply(
                &mut shared.properties.cyclopedia_map,
                command,
                result,
            ),
            59 => crate::play::feature_0059_house_move_out::protocol::apply(
                &mut shared.properties.house_move_out,
                command,
                result,
            ),
            67 => crate::play::feature_0067_bestiary_charm_progression::protocol::apply(
                &mut shared.properties.charm_progression,
                command,
                result,
            ),
            76 => crate::play::feature_0076_player_report::protocol::apply(
                &mut shared.properties.player_report,
                command,
                result,
            ),
            86 => crate::play::feature_0086_house_transfer::protocol::apply(
                &mut shared.properties.house_transfer,
                command,
                result,
            ),
            87 => crate::play::feature_0087_pvp_expert::protocol::apply(
                &mut shared.properties.pvp_expert,
                command,
                result,
            ),
            97 => crate::play::feature_0097_character_info::protocol::apply(
                &mut shared.properties.character_info,
                command,
                result,
            ),
            99 => crate::play::feature_0099_guildhall::protocol::apply(
                &mut shared.properties.guildhall,
                command,
                result,
            ),
            103 => crate::play::feature_0103_guild_war::protocol::apply(
                &mut shared.properties.guild_war,
                command,
                result,
            ),
            445 => crate::play::feature_0445_house_furniture_rotation::protocol::apply(
                &mut shared.properties.house_furniture_rotation,
                command,
                result,
            ),
            207 => crate::play::feature_0207_house_furniture_wrap::protocol::apply(
                &mut shared.properties.house_furniture_wrap,
                command,
                result,
            ),
            115 => apply(&mut shared.properties.mail, command, result),
            134 => crate::play::feature_0134_party_skull::protocol::apply(
                &mut shared.properties.party_skull,
                command,
                result,
            ),
            146 => crate::play::feature_0146_container_sort::protocol::apply(
                &mut shared.properties.container_sort,
                command,
                result,
            ),
            149 => crate::play::feature_0149_raid_schedule::protocol::apply(
                &mut shared.properties.raid_schedule,
                command,
                result,
            ),
            150 => crate::play::feature_0150_house_podium::protocol::apply(
                &mut shared.properties.house_podium,
                command,
                result,
            ),
            _ => {}
        }
    }
}
