//! Negotiated shaping state is consumed by the existing proficiency panel.
pub fn request(
    panel: &mut crate::weapon_proficiency::WeaponProficiencyPanel,
) -> Option<oteryn_client_runtime::raw_feature::FeatureCommand> {
    crate::weapon_proficiency::request(panel, crate::weapon_proficiency::Intent::State)
}
