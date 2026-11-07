//! The native partial-source overlay has no canonical Encounter parent. Accept
//! its distinct opaque descriptor directly; never manufacture one of the nine
//! ordinary callback parents or an authored Creature/Ability profile.
use super::*;
impl ComposedSource {
    /// Capture the current delayed-handler position only once. A missing current
    /// Map reader can retry this exact event without moving its retained area.
    pub fn from_omrafir_beam_current(
        native: &crate::content::native_gameplay::NativeGameplayState,
        stage: &crate::content::native_omrafir::QualifiedOmrafirBeamStage,
        runtime: &ChannelRuntimeV1,
        parent: &ProfileAbilityProposal,
    ) -> Result<Self, AttackError> {
        let mut source = Self::from_omrafir_beam(native, stage)?;
        source.anchor = Some(SourcePositionAnchor::capture(
            runtime,
            source.primary.as_ref().ok_or(AttackError::InvalidSource)?,
            parent,
        )?);
        Ok(source)
    }
    pub fn from_omrafir_beam(
        native: &crate::content::native_gameplay::NativeGameplayState,
        stage: &crate::content::native_omrafir::QualifiedOmrafirBeamStage,
    ) -> Result<Self, AttackError> {
        if native.source_digest() != stage.content() {
            return Err(AttackError::ContentChanged);
        }
        let record = native
            .creature_profiles()
            .records
            .iter()
            .find(|record| record.profile.target == *stage.creature())
            .ok_or(AttackError::InvalidSource)?;
        let Data::Creature(creature) = &record.profile.data else {
            return Err(AttackError::InvalidSource);
        };
        let critical = crate::source_critical_baseline::SourceCriticalBaseline::qualify(
            &stage.creature().key,
            creature
                .details
                .as_ref()
                .map_or(0, |details| details.critical_chance_ppm),
            false,
        )
        .map_err(|_| AttackError::InvalidSource)?;
        let details = stage.details().clone();
        let primary = SpellSource {
            list: ScheduleList::Attack,
            creature_key: stage.creature().key.clone(),
            ability: stage.ability().clone(),
            entry_index: stage.entry_index(),
            minimum: stage.minimum(),
            maximum: stage.maximum(),
            reference_level: creature
                .details
                .as_ref()
                .and_then(|details| details.reference_level),
            reference_level_overrides: creature.details.as_ref().map_or_else(Vec::new, |details| {
                details.reference_level_overrides.clone()
            }),
            critical,
            conditions: Vec::new(),
            appearances: Vec::new(),
            appearance_cast: false,
            formula_revision: None,
            fingerprint: stage.fingerprint(),
            content_digest: stage.content(),
            element: Some("fire".into()),
            armor: false,
            range: 0,
            magnitude: None,
            area: details.area.clone(),
            chain: None,
            presentation: Some(Box::new(details.clone())),
            source_none_correction: false,
            dispel_invisible: false,
            variants: Vec::new(),
            variant_child: None,
            windup: None,
            needs_target: false,
            needs_direction: false,
        };
        Ok(Self {
            caster: stage.creature().clone(),
            ability: stage.ability().clone(),
            index: stage.entry_index(),
            details: Box::new(details),
            primary: Some(primary),
            callbacks: Vec::new(),
            fingerprint: stage.fingerprint(),
            content: stage.content(),
            range: stage.parent_schedule().range_tiles.unwrap_or(0),
            magnitude: stage.parent_schedule().magnitude,
            callback_stage: Some(0),
            anchor: None,
            omrafir_stage: Some(stage.clone()),
            retained_caster: None,
        })
    }
}
