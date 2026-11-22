//! Live Skills details from the existing character, inventory and boost observations.
use super::*;
use oteryn_client_runtime::cyclopedia_character::{CharacterTab, combat::CombatStats};
pub(crate) const GROUPS: [(CharacterTab, &str); 3] = [
    (CharacterTab::Offence, "Offence"),
    (CharacterTab::Defence, "Defence"),
    (CharacterTab::Misc, "Misc"),
];

fn combat_command(tab: CharacterTab) -> oteryn_client_runtime::raw_feature::FeatureCommand {
    let mut command = stamina_command();
    command.payload[0] = tab as u8;
    command
}
impl Outbox {
    pub(in crate::play) async fn hud_character_query<S: crate::play::Stepper>(
        &self,
        session: &mut S,
        command: PlayCommand,
    ) -> Result<(), oteryn_session::SessionError> {
        if let PlayCommand::SkillsCombat(tab) = command {
            let result = session.feature(&combat_command(tab)).await?;
            self.skills_combat_result(tab, &result);
        } else {
            let result = session.feature(&stamina_command()).await?;
            self.hud_stamina_result(&result);
        }
        Ok(())
    }

    pub(crate) fn skills_combat_result(
        &self,
        tab: CharacterTab,
        result: &oteryn_session::GatedCommandResult,
    ) {
        let mut mailbox = lock(&self.0);
        let Some(index) = GROUPS.iter().position(|(t, _)| *t == tab) else {
            return;
        };
        let state = &mut mailbox.hud_resource;
        state.combat_pending[index] = false;
        state.combat[index] = None;
        if matches!(
            result.status,
            oteryn_session::CommandStatus::Accepted
                | oteryn_session::CommandStatus::DuplicateReplay
        ) && let Ok(stats) = CombatStats::decode(&result.payload)
            && stats.tab() == tab
        {
            let panel = oteryn_client_ui::cyclopedia_character::CyclopediaCharacterPanel {
                tab,
                combat: Some(stats),
                ..Default::default()
            };
            state.combat[index] = Some(
                oteryn_client_ui::cyclopedia_character::combat_rows(&panel)
                    .into_iter()
                    .map(|row| format!("{}: {}", row.label, row.value))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }
    }
}
impl PlayLink {
    pub(crate) fn skills_group(&self, index: usize, refresh: bool) -> Option<String> {
        let (tab, _) = *GROUPS.get(index)?;
        let mut m = lock(&self.mailbox);
        if !m.cyclopedia_character.selected {
            return None;
        }
        let state = &mut m.hud_resource;
        if refresh && !state.combat_pending[index] {
            state.combat_pending[index] =
                self.commands.send(PlayCommand::SkillsCombat(tab)).is_ok();
        }
        state.combat[index].clone()
    }
    pub(crate) fn weapon_filter_character(&self) -> (Option<u32>, Option<&'static str>) {
        let m = lock(&self.mailbox);
        (
            m.skill_progress
                .complete
                .as_ref()
                .map(|v| v.level)
                .or(m.hud_resource.level),
            m.cyclopedia_character
                .vocation_id
                .or(m.hud_resource.vocation)
                .and_then(oteryn_client_ui::cyclopedia_character::vocation_title),
        )
    }
    pub(crate) fn skills_xp_details(&self, level: u32) -> String {
        let m = lock(&self.mailbox);
        let low = 1200 - 25 * (level.clamp(2, 50) - 2);
        let mut text = format!(
            "Base XP rate: 100%\nLow-level bonus: {:.2}%",
            f64::from(low) / 12.
        );
        let potions = &m.potion_durations;
        let boosts = potions
            .snapshot
            .as_ref()
            .zip(potions.received_at)
            .map(|(snapshot, at)| {
                let elapsed = u64::try_from(at.elapsed().as_millis()).unwrap_or(u64::MAX);
                let lines =
                    oteryn_client_ui::daily_boost::xp_boost_lines(&snapshot.active, elapsed);
                if !lines.is_empty() {
                    text.push('\n');
                    text.push_str(lines.trim_end());
                }
                snapshot
                    .active
                    .iter()
                    .filter(|p| matches!(p.source, 3 | 4) && p.remaining_ms > elapsed)
                    .map(|p| u32::from(p.percent))
                    .sum::<u32>()
            });
        let stamina = m
            .hud_resource
            .stamina
            .or_else(|| m.cyclopedia_character.general.map(|g| g.stamina_minutes));
        let multiplier = stamina.and_then(|minutes| match minutes {
            0 => Some(0),
            1..=840 => Some(50),
            841..=2340 => Some(100),
            _ => m
                .premium_game
                .view
                .current_premium()
                .map(|p| if p { 150 } else { 100 }),
        });
        if let Some(rate) = multiplier {
            text.push_str(&format!("\nStamina rate: {rate}%"));
            if let Some(boost) = boosts {
                text.push_str(&format!(
                    "\nXP gain rate: {:.2}%",
                    (100. + f64::from(low) / 12. + f64::from(boost)) * f64::from(rate) / 100.
                ));
            }
        }
        if let Some(cap) = m.inventory.capacity {
            text.push_str(&format!(
                "\nCapacity: {:.2} / {:.2} oz",
                f64::from(cap.free) / 100.,
                f64::from(cap.capacity) / 100.
            ));
        }
        text
    }
}
