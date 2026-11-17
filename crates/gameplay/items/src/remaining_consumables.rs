//! Keg filling rules from Crystal 201189f871d23597b82caf88fd611454a2ddefb8,
//! `data/scripts/actions/objects/cask_and_kegs.lua:2-23,33-54`.

/// The atomic inventory changes required by one flask-on-keg interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KegFill {
    pub potion_id: u32,
    pub filled: u16,
    pub empty_flasks_remaining: u16,
    pub charges_remaining: u16,
}

impl KegFill {
    /// Crystal removes the target when its final charge is spent (lines 52-54).
    pub const fn removes_keg(self) -> bool {
        self.charges_remaining == 0
    }
}

/// Dispensers spend charges only through flask filling, never through plain use.
pub fn is_dispenser(item: u32) -> bool {
    [283, 284, 285]
        .into_iter()
        .any(|flask| keg_fill(item, flask, 1, 1).is_some())
}

/// Match the flask to the dispenser, then fill at most the available charges.
/// The caller must commit both inventory changes together; this plan grants no custody.
pub fn keg_fill(keg: u32, flask: u32, count: u16, charges: u16) -> Option<KegFill> {
    let (required, potion_id) = match keg {
        25879 | 25903 => (285, 266),
        25880 | 25904 => (283, 236),
        25881 | 25905 => (284, 239),
        25882 | 25906 => (284, 7643),
        25883 | 25907 => (284, 23375),
        25889 | 25908 => (285, 268),
        25890 | 25909 => (283, 237),
        25891 | 25910 => (284, 238),
        25892 | 25911 => (284, 23373),
        25899 | 25913 => (284, 7642),
        25900 | 25914 => (284, 23374),
        _ => return None,
    };
    if flask != required || count == 0 || charges == 0 {
        return None;
    }
    let filled = count.min(charges);
    Some(KegFill {
        potion_id,
        filled,
        empty_flasks_remaining: count - filled,
        charges_remaining: charges - filled,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reaudit_consumable_variants() {
        for (cask, keg, flask, potion) in [
            (25879, 25903, 285, 266),
            (25880, 25904, 283, 236),
            (25881, 25905, 284, 239),
            (25882, 25906, 284, 7643),
            (25883, 25907, 284, 23375),
            (25889, 25908, 285, 268),
            (25890, 25909, 283, 237),
            (25891, 25910, 284, 238),
            (25892, 25911, 284, 23373),
            (25899, 25913, 284, 7642),
            (25900, 25914, 284, 23374),
        ] {
            for dispenser in [cask, keg] {
                assert!(is_dispenser(dispenser));
                let last = keg_fill(dispenser, flask, 10, 3);
                assert_eq!(
                    last,
                    Some(KegFill {
                        potion_id: potion,
                        filled: 3,
                        empty_flasks_remaining: 7,
                        charges_remaining: 0,
                    })
                );
                assert!(last.is_some_and(KegFill::removes_keg));
                assert_eq!(
                    keg_fill(dispenser, flask, 3, 10),
                    Some(KegFill {
                        potion_id: potion,
                        filled: 3,
                        empty_flasks_remaining: 0,
                        charges_remaining: 7,
                    })
                );
                assert_eq!(keg_fill(dispenser, 999, 1, 1), None);
                assert_eq!(keg_fill(dispenser, flask, 1, 0), None);
                assert_eq!(keg_fill(dispenser, flask, 0, 1), None);
            }
        }
        assert!(!is_dispenser(0));
        assert_eq!(keg_fill(0, 285, 1, 1), None);
    }
}
