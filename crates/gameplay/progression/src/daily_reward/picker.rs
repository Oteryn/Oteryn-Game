//! Canary 99902524 daily_reward.lua:68-75,107-150; DAILY_REWARD_V2 T144 amendment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pick {
    pub item: u32,
    pub count: u8,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Offer {
    pub epoch: u64,
    pub day: u8,
    pub premium: bool,
    pub maximum: u8,
    pub items: [u32; 32],
}
pub fn offer(epoch: u64, day: u8, premium: bool, vocation: &str) -> Option<Offer> {
    if epoch == 0 || !(1..=30).contains(&day) {
        return None;
    }
    let reference_day = (day - 1) % 7 + 1;
    let maximum = match reference_day {
        1 | 2 => 5,
        4 => 10,
        6 => 1,
        _ => 0,
    } * if premium { 2 } else { 1 };
    let items: &[u32] = if reference_day == 6 {
        &[28540, 28541, 28542, 28543, 28544, 28545, 44064, 50292]
    } else if maximum == 0 {
        &[]
    } else {
        match vocation {
            "paladin" | "royal_paladin" | "monk" | "exalted_monk" => &[
                266, 236, 268, 237, 7642, 23374, 3203, 3161, 3178, 3153, 3197, 3149, 3164, 3200,
                3192, 3188, 3190, 3189, 3191, 3158, 3152, 3180, 3173, 3176, 3195, 3175, 3155, 3202,
            ],
            "druid" | "elder_druid" => &[
                266, 268, 237, 238, 23373, 3203, 3161, 3178, 3153, 3197, 3149, 3164, 3200, 3192,
                3188, 3190, 3189, 3156, 3191, 3158, 3152, 3180, 3173, 3176, 3195, 3175, 3155, 3202,
            ],
            "sorcerer" | "master_sorcerer" => &[
                266, 268, 237, 238, 23373, 3203, 3161, 3178, 3153, 3197, 3149, 3164, 3200, 3192,
                3188, 3190, 3189, 3191, 3158, 3152, 3180, 3173, 3176, 3195, 3175, 3155, 3202,
            ],
            "knight" | "elite_knight" => &[
                266, 236, 239, 7643, 23375, 268, 238, 3203, 3161, 3178, 3153, 3197, 3149, 3164,
                3200, 3192, 3188, 3190, 3189, 3191, 3158, 3152, 3180, 3173, 3176, 3195, 3175, 3155,
                3202,
            ],
            _ => &[266, 268],
        }
    };
    let mut result = Offer {
        epoch,
        day,
        premium,
        maximum,
        items: [0; 32],
    };
    result.items[..items.len()].copy_from_slice(items);
    Some(result)
}
pub fn valid(offer: &Offer, picks: &[Pick]) -> bool {
    !picks.is_empty()
        && picks.len() <= 19
        && picks.windows(2).all(|p| p[0].item < p[1].item)
        && picks
            .iter()
            .all(|p| p.count > 0 && p.item != 0 && offer.items.contains(&p.item))
        && picks.iter().map(|p| u16::from(p.count)).sum::<u16>() <= u16::from(offer.maximum)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_picker_vocations_premium_and_thirty_day_overlay() -> Result<(), &'static str> {
        let free = offer(1, 1, false, "none").ok_or("offer")?;
        assert!(valid(
            &free,
            &[
                Pick {
                    item: 266,
                    count: 3
                },
                Pick {
                    item: 268,
                    count: 2
                }
            ]
        ));
        assert!(!valid(
            &free,
            &[Pick {
                item: 266,
                count: 6
            }]
        ));
        assert!(!valid(
            &free,
            &[Pick {
                item: 237,
                count: 1
            }]
        ));
        assert_eq!(
            offer(1, 8, true, "royal_paladin").ok_or("offer")?.maximum,
            10
        );
        assert_eq!(offer(1, 6, true, "knight").ok_or("offer")?.maximum, 2);
        assert_eq!(offer(1, 7, false, "knight").ok_or("offer")?.maximum, 0);
        Ok(())
    }
}
