//! Crystal 201189f8 Ferumbras Ascension phase rules.
use oteryn_gameplay_entities::MovementLocalPosition;

pub const CRYSTALS: [(i32, i32); 8] = [
    (33390, 31468),
    (33394, 31468),
    (33397, 31471),
    (33397, 31475),
    (33394, 31478),
    (33390, 31478),
    (33387, 31475),
    (33387, 31471),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CrystalCharge {
    Absorbed,
    Lit { all_lit: bool },
}

#[derive(Debug, Default)]
pub struct Crystals {
    charges: [u8; CRYSTALS.len()],
}

impl Crystals {
    /// A kill in the source 3×3 square lights a crystal after nine previous kills.
    /// Once lit, later deaths will not publish the same phase transition twice.
    pub fn absorb(&mut self, death: MovementLocalPosition) -> Option<(usize, CrystalCharge)> {
        if death.floor != 14 {
            return None;
        }
        let index = CRYSTALS
            .iter()
            .position(|&(x, y)| death.x.abs_diff(x) <= 1 && death.y.abs_diff(y) <= 1)?;
        let charge = self.charges.get_mut(index)?;
        if *charge >= 10 {
            return None;
        }
        *charge += 1;
        let result = if *charge == 10 {
            CrystalCharge::Lit {
                all_lit: self.charges.iter().all(|&charge| charge == 10),
            }
        } else {
            CrystalCharge::Absorbed
        };
        Some((index, result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ferumbras_crystals_light_once_and_break_the_eighth_seal() {
        let mut crystals = Crystals::default();
        assert_eq!(
            crystals.absorb(MovementLocalPosition {
                x: 33390,
                y: 31468,
                floor: 13
            }),
            None
        );
        for (index, (x, y)) in CRYSTALS.into_iter().enumerate() {
            let at = MovementLocalPosition {
                x: x + 1,
                y,
                floor: 14,
            };
            for _ in 0..9 {
                assert_eq!(crystals.absorb(at), Some((index, CrystalCharge::Absorbed)));
            }
            assert_eq!(
                crystals.absorb(at),
                Some((
                    index,
                    CrystalCharge::Lit {
                        all_lit: index == 7
                    }
                ))
            );
            assert_eq!(crystals.absorb(at), None);
        }
    }
}
