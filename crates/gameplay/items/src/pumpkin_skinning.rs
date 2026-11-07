//! Crystal201 skinning.lua:89–104,272–284 supplies13uniform entries;
//! current Wiki source SHA c5fbb7ddf71779e795806b92038a209770324cb06e6dd610b5335708765ed742 corrects quantities.
//! Duplicate chocolate entries keep donor entry weight; ranges use independent uniform count draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PumpkinReward {
    pub item_id: u32,
    pub count: u32,
}
pub const KNIFE: u32 = 5908;
pub const CORPSE: u32 = 12816;
pub const COOLDOWN_SECONDS: i64 = 14_400;
pub const DRAW_DOMAIN: u8 = 213;
pub const COUNT_DOMAIN: u8 = 214;
pub const REWARDS: [PumpkinReward; 13] = [
    PumpkinReward {
        item_id: 8032,
        count: 1,
    },
    PumpkinReward {
        item_id: 8178,
        count: 1,
    },
    PumpkinReward {
        item_id: 6491,
        count: 1,
    },
    PumpkinReward {
        item_id: 6525,
        count: 1,
    },
    PumpkinReward {
        item_id: 8177,
        count: 20,
    },
    PumpkinReward {
        item_id: 6571,
        count: 1,
    },
    PumpkinReward {
        item_id: 6570,
        count: 1,
    },
    PumpkinReward {
        item_id: 6574,
        count: 50,
    },
    PumpkinReward {
        item_id: 2977,
        count: 1,
    },
    PumpkinReward {
        item_id: 3594,
        count: 50,
    },
    PumpkinReward {
        item_id: 3599,
        count: 50,
    },
    PumpkinReward {
        item_id: 6569,
        count: 50,
    },
    PumpkinReward {
        item_id: 6574,
        count: 50,
    },
];
/// The caller draws a uniform entry index, not an item-ID weighted table value.
pub fn reward(tool: u32, corpse: u32, index: usize) -> Option<PumpkinReward> {
    (tool == KNIFE && corpse == CORPSE)
        .then(|| REWARDS.get(index).copied())
        .flatten()
}
pub fn select(
    tool: u32,
    corpse: u32,
    mut draw: impl FnMut(u8, u64) -> Option<u64>,
) -> Option<PumpkinReward> {
    if tool != KNIFE || corpse != CORPSE {
        return None;
    }
    let mut chosen = reward(tool, corpse, usize::try_from(draw(DRAW_DOMAIN, 13)?).ok()?)?;
    if matches!(chosen.item_id, 3594 | 6574) {
        chosen.count = u32::try_from(draw(COUNT_DOMAIN, 50)?)
            .ok()?
            .checked_add(1)?;
        if chosen.count > 50 {
            return None;
        }
    }
    Some(chosen)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pumpkin_special_source_preserves_thirteen_entries_with_current_wiki_counts() {
        assert_eq!(
            reward(KNIFE, CORPSE, 4),
            Some(PumpkinReward {
                item_id: 8177,
                count: 20
            })
        );
        assert_eq!(
            reward(KNIFE, CORPSE, 7),
            Some(PumpkinReward {
                item_id: 6574,
                count: 50
            })
        );
        assert_eq!(
            reward(KNIFE, CORPSE, 12),
            Some(PumpkinReward {
                item_id: 6574,
                count: 50
            })
        );
        assert_eq!(reward(KNIFE, CORPSE, 13), None);
        assert_eq!(reward(5942, CORPSE, 0), None);
        assert_eq!(reward(KNIFE, 5969, 0), None);
        assert_eq!(COOLDOWN_SECONDS, 4 * 3600);
    }
    #[test]
    fn pumpkin_current_wiki_ranges_use_independent_count_domain_and_retain_duplicate_weight()
    -> Result<(), &'static str> {
        for entry in [7_u64, 9, 12] {
            for (raw, count) in [(0, 1), (49, 50)] {
                let value = select(KNIFE, CORPSE, |domain, upper| match domain {
                    213 => {
                        assert_eq!(upper, 13);
                        Some(entry)
                    }
                    214 => {
                        assert_eq!(upper, 50);
                        Some(raw)
                    }
                    _ => None,
                })
                .ok_or("closed draws")?;
                assert_eq!(value.count, count);
            }
        }
        assert_eq!(REWARDS.iter().filter(|v| v.item_id == 6574).count(), 2);
        assert_eq!(
            select(KNIFE, CORPSE, |domain, _| if domain == 213 {
                Some(4)
            } else {
                None
            }),
            Some(PumpkinReward {
                item_id: 8177,
                count: 20
            })
        );
        Ok(())
    }
    #[test]
    fn pumpkin_every_source_entry_is_reachable_with_exact_current_quantity_policy() {
        let expected = [
            (8032, 1),
            (8178, 1),
            (6491, 1),
            (6525, 1),
            (8177, 20),
            (6571, 1),
            (6570, 1),
            (6574, 17),
            (2977, 1),
            (3594, 17),
            (3599, 50),
            (6569, 50),
            (6574, 17),
        ];
        for (index, (id, count)) in expected.into_iter().enumerate() {
            let selected = select(KNIFE, CORPSE, |domain, _| match domain {
                213 => Some(index as u64),
                214 => Some(16),
                _ => None,
            });
            assert_eq!(selected, Some(PumpkinReward { item_id: id, count }));
        }
    }
    #[test]
    fn current_wiki_counts_follow_exact_pinned_xml_item_names() {
        assert_eq!(
            select(KNIFE, CORPSE, |d, _| if d == 213 { Some(4) } else { None }),
            Some(PumpkinReward {
                item_id: 8177,
                count: 20
            }),
            "8177 is yummy gummy worm"
        );
        assert_eq!(
            select(KNIFE, CORPSE, |d, _| if d == 213 { Some(8) } else { None }),
            Some(PumpkinReward {
                item_id: 2977,
                count: 1
            }),
            "2977 is pumpkinhead"
        );
        for (raw, count) in [(0, 1), (49, 50)] {
            assert_eq!(
                select(KNIFE, CORPSE, |d, _| match d {
                    213 => Some(9),
                    214 => Some(raw),
                    _ => None,
                }),
                Some(PumpkinReward {
                    item_id: 3594,
                    count
                }),
                "3594 is pumpkin"
            );
        }
        assert_eq!(
            reward(KNIFE, CORPSE, 0),
            Some(PumpkinReward {
                item_id: 8032,
                count: 1
            }),
            "8032 is spiderwebs"
        );
    }
}
