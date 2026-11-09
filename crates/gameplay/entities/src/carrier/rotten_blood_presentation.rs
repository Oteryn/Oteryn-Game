//! Source-keyed Creature icons on the existing boxed Player payload.
//! Crystal201189f creature.hpp:273-302 and rotten area-icons.lua:108-149,214-216.
use super::*;

/// Source key, icon category, icon identifier and count before the shared limit.
pub type RottenBloodKeyedIcons = [Option<(&'static str, u8, u8, u16)>; 4];
/// Icon category, icon identifier and count in the current first-three projection.
pub type RottenBloodVisibleIcons = [Option<(u8, u8, u16)>; 3];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RottenBloodIconKey {
    AgonyStacks,
    Area,
    Hazard,
    WhiteCross,
}
impl RottenBloodIconKey {
    /// Literal donor std::map key, used before the shared first-three limit.
    pub const fn source_key(self) -> &'static str {
        match self {
            Self::AgonyStacks => "agony-stacks",
            Self::Area => "rotten-area",
            Self::Hazard => "rotten-hazard",
            Self::WhiteCross => "white-cross",
        }
    }
    const fn index(self) -> usize {
        match self {
            Self::AgonyStacks => 0,
            Self::Area => 1,
            Self::Hazard => 2,
            Self::WhiteCross => 3,
        }
    }
    const fn icon(self) -> u8 {
        match self {
            Self::AgonyStacks | Self::WhiteCross => 2,
            Self::Area => 17,
            Self::Hazard => 22,
        }
    }
    fn accepts(self, count: u16) -> bool {
        match self {
            Self::AgonyStacks => count <= 21,
            Self::Area => (1..=100).contains(&count),
            Self::Hazard => (1..=4).contains(&count),
            Self::WhiteCross => count <= 111,
        }
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct RottenBloodPresentation {
    counts: [Option<u16>; 4],
}
impl RottenBloodPresentation {
    pub(super) fn set(
        &mut self,
        key: RottenBloodIconKey,
        count: Option<u16>,
    ) -> Result<(), CarrierError> {
        if count.is_some_and(|value| !key.accepts(value)) {
            return Err(CarrierError::InvalidCreatureTarget);
        }
        self.counts[key.index()] = count;
        Ok(())
    }
    fn keyed(&self) -> RottenBloodKeyedIcons {
        [
            RottenBloodIconKey::AgonyStacks,
            RottenBloodIconKey::Area,
            RottenBloodIconKey::Hazard,
            RottenBloodIconKey::WhiteCross,
        ]
        .map(|key| self.counts[key.index()].map(|count| (key.source_key(), 0, key.icon(), count)))
    }
    pub(super) fn visible(&self) -> RottenBloodVisibleIcons {
        let mut result = [None; 3];
        let mut count = 0;
        // std::map<string,CreatureIcon> order in Crystal creature.hpp:273-282.
        // The source keys are agony-stacks, rotten-area, rotten-hazard, white-cross.
        // Zero counts remain present and duplicate enum pairs are not deduplicated.
        for key in [
            RottenBloodIconKey::AgonyStacks,
            RottenBloodIconKey::Area,
            RottenBloodIconKey::Hazard,
            RottenBloodIconKey::WhiteCross,
        ] {
            if let Some(value) = self.counts[key.index()] {
                if count == result.len() {
                    break;
                }
                result[count] = Some((0, key.icon(), value));
                count += 1;
            }
        }
        result
    }

    pub(super) fn leave_area(&mut self) {
        for key in [
            RottenBloodIconKey::Area,
            RottenBloodIconKey::Hazard,
            RottenBloodIconKey::WhiteCross,
        ] {
            self.counts[key.index()] = None;
        }
    }
}

impl ChannelRuntimeV1 {
    /// All source keys, before the Creature-wide first-three projection. Other
    /// source owners can merge their keys without a second presentation store.
    pub fn rotten_blood_keyed_icons(
        &self,
        actor: ExactActorRef,
    ) -> Result<RottenBloodKeyedIcons, CarrierError> {
        self.owner_fence()?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        match &self.carrier.slots[index] {
            Slot::Occupied {
                lifecycle,
                committed: true,
                game_session_id: Some(_),
                generation,
                ..
            } if *generation == actor.0.actor_local_generation.0 => {
                Ok(lifecycle.rotten_presentation.keyed())
            }
            _ => Err(CarrierError::PlayerReservationMismatch),
        }
    }

    /// Read-only presentation on the current physical Player payload. This is
    /// neither damage/status state nor authority to set a source icon.
    pub fn rotten_blood_visible_icons(
        &self,
        actor: ExactActorRef,
    ) -> Result<RottenBloodVisibleIcons, CarrierError> {
        self.owner_fence()?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        match &self.carrier.slots[index] {
            Slot::Occupied {
                lifecycle,
                committed: true,
                game_session_id: Some(_),
                generation,
                ..
            } if *generation == actor.0.actor_local_generation.0 => {
                Ok(lifecycle.rotten_presentation.visible())
            }
            _ => Err(CarrierError::PlayerReservationMismatch),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "source presentation test assertions")]
mod tests {
    use super::*;
    #[test]
    fn srx3_rotten_player_icons_preserve_lexical_keys_duplicates_zero_and_first_three() {
        let mut state = RottenBloodPresentation::default();
        state
            .set(RottenBloodIconKey::WhiteCross, Some(0))
            .expect("bounded source icon");
        state
            .set(RottenBloodIconKey::Hazard, Some(4))
            .expect("bounded source icon");
        state
            .set(RottenBloodIconKey::Area, Some(100))
            .expect("bounded source icon");
        state
            .set(RottenBloodIconKey::AgonyStacks, Some(0))
            .expect("bounded source icon");
        assert_eq!(
            state.visible(),
            [Some((0, 2, 0)), Some((0, 17, 100)), Some((0, 22, 4))]
        );
        state
            .set(RottenBloodIconKey::Area, None)
            .expect("bounded source icon");
        assert_eq!(
            state.visible(),
            [Some((0, 2, 0)), Some((0, 22, 4)), Some((0, 2, 0))]
        );
    }
    #[test]
    fn srx3_rotten_source_leave_clears_only_three_keys_and_preserves_agony() {
        let mut state = RottenBloodPresentation::default();
        for key in [
            RottenBloodIconKey::AgonyStacks,
            RottenBloodIconKey::Area,
            RottenBloodIconKey::Hazard,
            RottenBloodIconKey::WhiteCross,
        ] {
            state.set(key, Some(1)).expect("bounded source icon");
        }
        state.leave_area();
        assert_eq!(state.visible(), [Some((0, 2, 1)), None, None]);
    }
    #[test]
    fn srx3_rotten_source_icon_bounds_reject_before_payload_changes() {
        let mut state = RottenBloodPresentation::default();
        for (key, count) in [
            (RottenBloodIconKey::AgonyStacks, 22),
            (RottenBloodIconKey::Area, 0),
            (RottenBloodIconKey::Area, 101),
            (RottenBloodIconKey::Hazard, 0),
            (RottenBloodIconKey::Hazard, 5),
            (RottenBloodIconKey::WhiteCross, 112),
        ] {
            let before = state.clone();
            assert_eq!(
                state.set(key, Some(count)),
                Err(CarrierError::InvalidCreatureTarget)
            );
            assert_eq!(state, before);
        }
    }
}
