//! Pure house-podium admission rules. Persistence and transport must supply the
//! current house/session fences; this module deliberately grants no authority.

pub const RENOWN_ITEM: &str = "oteryn:content.item.podium_of_renown";
pub const VIGOUR_ITEM: &str = "oteryn:content.item.podium_of_vigour";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PodiumKind {
    Renown,
    Vigour,
    Boss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayKind {
    Outfit,
    Mount,
    Boss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    WrongItem,
    WrongDisplayKind,
    UnknownContent,
    InvalidVariant,
}

/// Content-owned lookup used instead of guessing Tibia/Canary numeric ids.
pub trait PodiumContent {
    fn has_outfit(&self, key: &str) -> bool;
    fn has_mount(&self, key: &str) -> bool;
    fn has_boss(&self, key: &str) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Display<'a> {
    pub kind: DisplayKind,
    pub key: &'a str,
    pub direction: u8,
}

pub fn admit<C: PodiumContent>(
    item: &str,
    display: Display<'_>,
    content: &C,
) -> Result<PodiumKind, Refusal> {
    if display.direction > 3 {
        return Err(Refusal::InvalidVariant);
    }
    let podium = match item {
        RENOWN_ITEM => PodiumKind::Renown,
        VIGOUR_ITEM => PodiumKind::Vigour,
        // No boss-display item is admitted until its production key exists in
        // Oteryn content. This is an explicit parity gap, not an invented key.
        _ => return Err(Refusal::WrongItem),
    };
    let allowed = matches!(
        (podium, display.kind),
        (PodiumKind::Renown, DisplayKind::Outfit) | (PodiumKind::Vigour, DisplayKind::Mount)
    );
    if !allowed {
        return Err(Refusal::WrongDisplayKind);
    }
    let known = match display.kind {
        DisplayKind::Outfit => content.has_outfit(display.key),
        DisplayKind::Mount => content.has_mount(display.key),
        DisplayKind::Boss => content.has_boss(display.key),
    };
    known.then_some(podium).ok_or(Refusal::UnknownContent)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Known;
    impl PodiumContent for Known {
        fn has_outfit(&self, k: &str) -> bool {
            k == "oteryn:outfit.citizen"
        }
        fn has_mount(&self, k: &str) -> bool {
            k == "oteryn:mount.widow_queen"
        }
        fn has_boss(&self, _: &str) -> bool {
            false
        }
    }
    #[test]
    fn podium_matrix_and_content_are_closed() {
        let d = |kind, key| Display {
            kind,
            key,
            direction: 2,
        };
        assert_eq!(
            admit(
                RENOWN_ITEM,
                d(DisplayKind::Outfit, "oteryn:outfit.citizen"),
                &Known
            ),
            Ok(PodiumKind::Renown)
        );
        assert_eq!(
            admit(
                VIGOUR_ITEM,
                d(DisplayKind::Mount, "oteryn:mount.widow_queen"),
                &Known
            ),
            Ok(PodiumKind::Vigour)
        );
        assert_eq!(
            admit(
                RENOWN_ITEM,
                d(DisplayKind::Mount, "oteryn:mount.widow_queen"),
                &Known
            ),
            Err(Refusal::WrongDisplayKind)
        );
        assert_eq!(
            admit(
                RENOWN_ITEM,
                d(DisplayKind::Outfit, "oteryn:outfit.invented"),
                &Known
            ),
            Err(Refusal::UnknownContent)
        );
        assert_eq!(
            admit(
                "oteryn:content.item.boss_display",
                d(DisplayKind::Boss, "oteryn:boss.ferumbras"),
                &Known
            ),
            Err(Refusal::WrongItem)
        );
        assert_eq!(
            admit(
                RENOWN_ITEM,
                Display {
                    direction: 4,
                    ..d(DisplayKind::Outfit, "oteryn:outfit.citizen")
                },
                &Known
            ),
            Err(Refusal::InvalidVariant)
        );
    }
}
