use oteryn_platform_client::native_login::premium::*;
pub(crate) fn snapshot(state: PremiumState) -> PremiumStatus {
    PremiumStatus {
        version: 1,
        authority_revision: 1,
        generated_at_unix: 1_800_000_000,
        valid_for_seconds: 60,
        state,
        expires_at_unix: match state {
            PremiumState::None => None,
            PremiumState::Active => Some(1_800_086_400),
            _ => Some(1_799_999_000),
        },
        features: vec![PremiumFeature {
            id: "private_channels".into(),
            title: "Private channels".into(),
            description: "Create a private conversation channel where this world permits it."
                .into(),
            enabled: state == PremiumState::Active,
        }],
    }
}
