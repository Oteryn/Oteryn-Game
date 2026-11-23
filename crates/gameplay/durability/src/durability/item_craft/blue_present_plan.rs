#[derive(Debug, PartialEq, Eq)]
pub(super) enum Admission {
    Carried,
    NoRoom,
    TooHeavy,
}

pub(super) fn admit(
    remaining: u32,
    room_for_root: bool,
    carried: u64,
    gift_weight: u64,
    capacity: u64,
) -> Admission {
    if carried.checked_add(gift_weight)
        .is_none_or(|weight| weight > capacity)
    {
        return Admission::TooHeavy;
    }
    if remaining != 0 && !room_for_root {
        return Admission::NoRoom;
    }
    Admission::Carried
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blue_present_can_fill_existing_stacks_in_a_full_backpack() {
        assert_eq!(admit(0, false, 900, 100, 1000), Admission::Carried);
        assert_eq!(admit(1, false, 900, 100, 1000), Admission::NoRoom);
    }

    #[test]
    fn blue_present_checks_gift_weight_before_crediting_the_debit() {
        assert_eq!(admit(1, true, 950, 100, 1000), Admission::TooHeavy);
        assert_eq!(admit(1, true, 900, 100, 1000), Admission::Carried);
    }
}
