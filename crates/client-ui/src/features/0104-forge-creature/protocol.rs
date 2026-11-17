use super::ForgeMarker;

/// Replace, including removal, from the current exact-incarnation session projection.
pub fn replace(state: &mut Option<ForgeMarker>, creature: bool, stacks: Option<u8>) {
    *state = if creature {
        match stacks {
            Some(15) => Some(ForgeMarker::Fiendish),
            Some(stacks @ 1..=5) => Some(ForgeMarker::Influenced(stacks)),
            _ => None,
        }
    } else {
        None
    };
}
