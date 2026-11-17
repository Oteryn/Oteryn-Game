//! Combat reactions for the first Lord Retro phase.

#[derive(Clone, Copy)]
pub(super) enum Source {
    Distortion,
    BlissfulTimes,
}

#[derive(Clone, Copy)]
pub(super) enum Target {
    HardTimes,
    ParcelCastle,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Reaction {
    Transform,
    Damage(u64),
    None,
}

pub(super) fn reaction(source: Source, target: Target) -> Reaction {
    match (source, target) {
        (_, Target::HardTimes) => Reaction::Transform,
        (Source::BlissfulTimes, Target::ParcelCastle) => Reaction::Damage(500),
        (Source::Distortion, Target::ParcelCastle) => Reaction::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distortion_transforms_hard_times_without_damaging_castles() {
        assert_eq!(
            reaction(Source::Distortion, Target::HardTimes),
            Reaction::Transform
        );
        assert_eq!(
            reaction(Source::Distortion, Target::ParcelCastle),
            Reaction::None
        );
    }

    #[test]
    fn blissful_blast_propagates_and_deals_fixed_castle_damage() {
        assert_eq!(
            reaction(Source::BlissfulTimes, Target::HardTimes),
            Reaction::Transform
        );
        assert_eq!(
            reaction(Source::BlissfulTimes, Target::ParcelCastle),
            Reaction::Damage(500)
        );
    }
}
