//! The three loaded Rotten mechanics handlers share the canonical death cursor.
use super::*;
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::item_mint::BakragoreDeathLoot;
use oteryn_gameplay_entities::carrier::native_source_death::SourceCreatureDeathSeal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RottenDeathAction {
    begin: SourceCreatureDeathSeal,
    index: usize,
    loot: Option<BakragoreDeathLoot>,
}

impl RottenDeathAction {
    pub(super) fn capture(
        runtime: &ChannelRuntimeV1,
        states: &actor_spell::ChannelSpellStates,
        native: &NativeGameplayState,
        begin: &SourceCreatureDeathSeal,
        index: usize,
    ) -> Result<Option<Self>, ()> {
        let event = begin.events().get(index).ok_or(())?;
        let key = match event.name() {
            "VemiathMechanicsDeath" => "oteryn:creature.vemiath",
            "ChagorzMechanicsDeath" => "oteryn:creature.chagorz",
            "BakragoreMechanicsDeath" => "oteryn:creature.bakragore",
            _ => return Ok(None),
        };
        if begin.definition_key() != key
            || index != 0
            || event.registration_index() != 1
            || begin.events().len() != 1
            || !begin.current_for(runtime)
            || runtime.source_creature_death_event_index(begin).ok() != Some(Some(index))
            || !native.qualifies_rotten_blood_callbacks(begin.content_digest())
        {
            return Err(());
        }
        let loot = if key == "oteryn:creature.bakragore" {
            let count = states
                .rotten_blood_callbacks
                .capture_bakragore_death_essences(runtime, begin)?;
            Some(
                BakragoreDeathLoot::from_current_death(runtime, native, begin, count)
                    .map_err(|_| ())?,
            )
        } else {
            None
        };
        Ok(Some(Self {
            begin: begin.clone(),
            index,
            loot,
        }))
    }

    fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.begin.current_for(runtime)
            && runtime.source_creature_death_event_index(&self.begin).ok() == Some(Some(self.index))
    }

    pub(super) async fn execute(
        &self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        action: &PreparedSourceDeathAction,
    ) -> Result<(), ()> {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.try_lock().map_err(|_| ())?;
        let attack = owner.states.attack.try_lock().map_err(|_| ())?;
        if action.begin() != &self.begin
            || action.event() != &self.begin.events()[self.index]
            || !self.current_for(&runtime)
            || states.source_death_action.as_ref() != Some(action)
            || !states.source_death_action_ready
            || states
                .source_death_action_reservation
                .as_ref()
                .is_none_or(|reservation| {
                    runtime
                        .validate_source_death_action_reservation(reservation)
                        .is_err()
                })
        {
            return Err(());
        }
        let native = owner
            .active_generation
            .and_then(|active| active.native_gameplay())
            .ok_or(())?;
        if !native.qualifies_rotten_blood_callbacks(self.begin.content_digest()) {
            return Err(());
        }
        if let Some(source) = &self.loot {
            let credited = attack.kills().retain_bakragore_source_loot(
                &runtime,
                native,
                &self.begin,
                source,
            )?;
            if !credited {
                if self.begin.has_reward_damage_credit()
                    || self.begin.uncredited_corpse_item_key().is_none()
                {
                    return Err(());
                }
                states.rotten_blood_callbacks.retain_bakragore_death_loot(
                    &runtime,
                    native,
                    &self.begin,
                    source,
                )?;
            }
        }
        // No await follows either publication. The existing death task keeps
        // the corpse closed until its same atomic ItemMINT owner reconciles it.
        Ok(())
    }

    pub(super) fn publish(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &mut actor_spell::ChannelSpellStates,
    ) -> Result<(), ()> {
        if !self.current_for(runtime) {
            return Err(());
        }
        states
            .rotten_blood_callbacks
            .publish_mechanics_death(runtime, &self.begin)
    }
}
