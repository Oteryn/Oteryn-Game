use crate::domain::appearance::{AccountUnlocks, UnlockFact};
use crate::foundation::CharacterId;

const MOUNTS: [(i16, &str); 3] = [
    (22, "oteryn:content.mount.rented_horse"),
    (25, "oteryn:content.mount.rented_horse_gray"),
    (26, "oteryn:content.mount.rented_horse_brown"),
];

pub(super) async fn current_mount(
    connection: &mut sqlx::PgConnection,
    character: CharacterId,
) -> Result<Option<&'static str>, super::DurabilityError> {
    let current: Option<i16> = sqlx::query_scalar(
        "SELECT mount_id FROM game_horse_rental_receipts
         WHERE character_id=encode($1,'hex')::uuid
           AND expires_at_ms>floor(extract(epoch FROM clock_timestamp())*1000)::bigint
         ORDER BY expires_at_ms DESC LIMIT 1",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_optional(connection)
    .await?;
    current.map(|id| MOUNTS.iter().find(|(mount, _)| *mount == id).map(|(_, key)| *key)
        .ok_or(super::DurabilityError::InvalidStoredState)).transpose()
}

pub fn is_rental_mount(key: &str) -> bool {
    MOUNTS.iter().any(|(_, mount)| *mount == key)
}

/// Extend the normal appearance entitlement with a character's current rental.
pub struct RentalMountUnlocks<T> {
    pub earned: T,
    pub rental: Option<&'static str>,
}

impl<T: AccountUnlocks<String>> AccountUnlocks<String> for RentalMountUnlocks<T> {
    fn outfit(&self, key: &String) -> UnlockFact {
        self.earned.outfit(key)
    }

    fn addon(&self, key: &String, addon: u8) -> UnlockFact {
        self.earned.addon(key, addon)
    }

    fn mount(&self, key: &String) -> UnlockFact {
        if self.rental == Some(key.as_str()) {
            UnlockFact::Compatible
        } else {
            self.earned.mount(key)
        }
    }
}
