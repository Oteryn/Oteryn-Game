//! Presentation names from the accepted Game Task Board catalogue; prices always come from the reply.
use super::task_board::full::ShopOffer;
type Label = (u16, u8, u32, u32, u8, &'static str);
const LABELS: &[Label] = &[
    (0, 2, 1907, 0, 0, "Feral Trapper (Base Outfit)"),
    (1, 2, 1907, 0, 1, "Feral Trapper (Addon 1)"),
    (2, 2, 1907, 0, 2, "Feral Trapper (Addon 2)"),
    (3, 2, 1282, 0, 0, "Falconer (Base Outfit)"),
    (4, 2, 1282, 0, 1, "Falconer (Addon 1)"),
    (5, 2, 1282, 0, 2, "Falconer (Addon 2)"),
    (6, 1, 1908, 0, 0, "Tidal Seawater Predator (Mount)"),
    (7, 1, 1909, 0, 0, "Ashen Coast Predator (Mount)"),
    (8, 1, 1910, 0, 0, "Crimson Bay Predator (Mount)"),
    (9, 1, 1281, 0, 0, "Antelope (Mount)"),
    (10, 0, 36753, 0, 0, "Falcon Pet"),
    (11, 0, 31465, 0, 0, "Cerberus Champion Puppy"),
    (12, 0, 32753, 0, 0, "Hellflayer Trophy"),
    (13, 0, 32756, 0, 0, "Golden Hunter Trophy"),
    (14, 0, 36748, 0, 0, "Brachiodemon Trophy"),
    (15, 0, 36749, 0, 0, "Many Faces Trophy"),
    (16, 0, 32752, 0, 0, "Sea Serpent Trophy"),
    (17, 0, 32755, 0, 0, "Silver Hunter Trophy"),
    (18, 0, 32754, 0, 0, "Bronze Hunter Trophy"),
    (19, 0, 32751, 0, 0, "Gozzler Trophy"),
    (20, 0, 32918, 0, 0, "Demon Doll"),
    (21, 0, 32943, 0, 0, "Vexclaw Doll"),
    (22, 3, 32795, 32796, 0, "Bone Bed"),
    (23, 4, 0, 0, 0, "Bonus Promotion"),
];
pub(super) fn name(offer: &ShopOffer) -> String {
    LABELS
        .iter()
        .find(|(id, kind, primary, secondary, addon, _)| {
            (*id, *kind, *primary, *secondary, *addon)
                == (
                    offer.index,
                    offer.kind,
                    offer.primary,
                    offer.secondary,
                    offer.addon,
                )
        })
        .map(|(_, _, _, _, _, name)| (*name).to_owned())
        .unwrap_or_else(|| {
            format!(
                "{} #{}",
                match offer.kind {
                    0 => "Item",
                    1 => "Mount",
                    2 => "Outfit",
                    3 => "Item pair",
                    4 => "Promotion",
                    _ => "Reward",
                },
                offer.primary
            )
        })
}
