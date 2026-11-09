//! Trusted server relocation intent; never decoded from a client request or receipt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HomeTownRelocationCause {
    Arrival,
    PremiumExpired,
}

pub(super) fn targuna_citizenship_allowed(
    citizenship: Option<(u32, HomeTownRelocationCause)>,
) -> bool {
    matches!(
        citizenship,
        Some((34, HomeTownRelocationCause::Arrival))
            | Some((8, HomeTownRelocationCause::PremiumExpired))
    )
}

#[cfg(test)]
mod tests {
    use super::{HomeTownRelocationCause as Cause, targuna_citizenship_allowed as allowed};

    #[test]
    fn m38_targuna_preserves_the_existing_p20_thais_override() {
        assert!(allowed(Some((8, Cause::PremiumExpired))));
    }

    #[test]
    fn m38_targuna_refuses_missing_and_unrelated_citizenship_intents() {
        assert!(!allowed(None));
        for town in [0, 6, 8, 11, 33, 34, 100] {
            for cause in [Cause::Arrival, Cause::PremiumExpired] {
                if (town, cause) == (34, Cause::Arrival)
                    || (town, cause) == (8, Cause::PremiumExpired)
                {
                    continue;
                }
                assert!(
                    !allowed(Some((town, cause))),
                    "town={town}, cause={cause:?}"
                );
            }
        }
        assert!(allowed(Some((34, Cause::Arrival))));
    }
}
