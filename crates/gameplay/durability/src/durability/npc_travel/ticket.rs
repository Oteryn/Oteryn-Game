//! Elapsed ticket windows reuse Quest-owned purchase timestamps and database time.
use crate::domain::CharacterId;
use crate::durability::DurabilityError;
use crate::durability::item_mint_audit::Type2Transaction;
use oteryn_gameplay_content::content::npc_catalogue::travel::TravelTicketRequirement;
use sha2::{Digest, Sha256};

// The same bounded horizon as Quest StateCondition::ElapsedAtLeast.
const MAX_LIFETIME: u32 = 366 * 24 * 60 * 60;

fn valid_at(purchased: i64, now: i64, lifetime: u32) -> bool {
    purchased > 0
        && (1..=MAX_LIFETIME).contains(&lifetime)
        && now
            .checked_sub(purchased)
            .is_some_and(|age| age >= 0 && age < i64::from(lifetime))
}

pub(super) async fn permitted(
    tx: &mut Type2Transaction<'_>,
    character: CharacterId,
    ticket: &TravelTicketRequirement,
) -> Result<bool, DurabilityError> {
    let observed: Option<(i64, i64)> = sqlx::query_as("SELECT value,floor(extract(epoch FROM clock_timestamp()))::bigint FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(character.as_bytes().as_slice()).bind(&ticket.track).fetch_optional(&mut ***tx).await?;
    Ok(observed.is_some_and(|(purchased, now)| valid_at(purchased, now, ticket.valid_for_seconds)))
}

pub(super) fn bind(hash: &mut Sha256, ticket: &TravelTicketRequirement) {
    hash.update(b"quest-travel-ticket:v1");
    hash.update((ticket.track.len() as u64).to_be_bytes());
    hash.update(ticket.track.as_bytes());
    hash.update(ticket.valid_for_seconds.to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m38_travel_ticket_counts_elapsed_seconds_across_midnight() {
        let purchase = 86400 - 1;
        for age in [0, 1, 6 * 3600 + 1, 86399] {
            assert!(valid_at(purchase, purchase + age, 86400));
        }
        assert!(!valid_at(purchase, purchase + 86400, 86400));
        assert!(!valid_at(purchase, purchase + 86401, 86400));
    }

    #[test]
    fn m38_travel_ticket_refuses_future_invalid_and_unbounded_times() {
        for (purchased, now, lifetime) in [
            (0, 100, 86400),
            (-1, 100, 86400),
            (101, 100, 86400),
            (1, 100, 0),
            (1, 100, MAX_LIFETIME + 1),
            (i64::MAX, i64::MIN, 86400),
        ] {
            assert!(!valid_at(purchased, now, lifetime));
        }
        assert!(valid_at(1, 1, MAX_LIFETIME));
    }

    #[test]
    fn m38_travel_ticket_binding_includes_track_and_duration() {
        let digest = |track: &str, seconds: u32| -> [u8; 32] {
            let mut hash = Sha256::new();
            bind(
                &mut hash,
                &TravelTicketRequirement {
                    track: track.into(),
                    valid_for_seconds: seconds,
                },
            );
            hash.finalize().into()
        };
        assert_ne!(
            digest("oteryn:quest-progress/ticket_a", 86400),
            digest("oteryn:quest-progress/ticket_b", 86400)
        );
        assert_ne!(
            digest("oteryn:quest-progress/ticket_a", 86400),
            digest("oteryn:quest-progress/ticket_a", 3600)
        );
    }
}
