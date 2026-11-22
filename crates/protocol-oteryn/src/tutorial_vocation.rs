//! Newhaven tutorial offer observation and one vocation submission.
pub const CAPABILITY_TUTORIAL_VOCATION_V1: u32 = 504;
pub const COMMAND_TYPE_TUTORIAL_VOCATION: u32 = 6243;
/// Card order observed in the accepted Newhaven tutorial audit.
pub const VOCATIONS: [&str; 5] = ["knight", "sorcerer", "druid", "paladin", "monk"];
pub const TITLES: [&str; 5] = ["Knight", "Sorcerer", "Druid", "Paladin", "Monk"];
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Observation {
    pub offered: bool,
    /// 0 when no vocation is selected, otherwise the card index plus one.
    pub chosen: u8,
}
impl Observation {
    pub fn encode(self) -> Option<Vec<u8>> {
        if self.chosen > 5 || (self.offered && self.chosen != 0) {
            return None;
        }
        Some(vec![if self.offered { 31 } else { 0 }, self.chosen])
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let [mask, chosen] = *bytes else {
            return None;
        };
        if mask != 0 && mask != 31 {
            return None;
        }
        let value = Self {
            offered: mask == 31,
            chosen,
        };
        value.encode()?;
        Some(value)
    }
}
/// Empty queries the offer; a single card id submits that vocation.
pub fn decode_request(bytes: &[u8]) -> Option<Option<u8>> {
    match bytes {
        [] => Some(None),
        [id @ 1..=5] => Some(Some(*id)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tutorial_vocation_codec_bounds_offer_and_actions() {
        assert_eq!(decode_request(&[]), Some(None));
        for id in 1..=5 {
            assert_eq!(decode_request(&[id]), Some(Some(id)));
        }
        for bytes in [&[0][..], &[6], &[1, 0]] {
            assert!(decode_request(bytes).is_none());
        }
        assert_eq!(
            Observation::decode(&[31, 0]),
            Some(Observation {
                offered: true,
                chosen: 0
            })
        );
        for id in 0..=5 {
            assert_eq!(
                Observation::decode(&[0, id]),
                Some(Observation {
                    offered: false,
                    chosen: id
                })
            );
        }
        for bytes in [&[][..], &[31], &[1, 0], &[31, 1], &[0, 6], &[0, 0, 0]] {
            assert!(Observation::decode(bytes).is_none());
        }
    }
}
