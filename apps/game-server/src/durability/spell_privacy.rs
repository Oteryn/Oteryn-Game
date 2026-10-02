//! Actual target-owned Exiva preferences. No caller boolean grants permission and
//! neither absent preferences nor absent staff roles mean public/ordinary access.
use super::spell_item_transaction::{SpellItemAuthority, SpellItemError, check_transaction};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Row, Transaction};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExivaPreferences {
    pub(crate) allow_all: bool,
    pub(crate) allow_own_guild: bool,
    pub(crate) allow_own_party: bool,
    pub(crate) allow_vip_list: bool,
    pub(crate) allow_player_whitelist: bool,
    pub(crate) allow_guild_whitelist: bool,
    pub(crate) player_whitelist: Vec<[u8; 16]>,
    pub(crate) guild_whitelist: Vec<u32>,
}
impl ExivaPreferences {
    /// Exact Canary Player::ExivaRestrictions constructor at pinned
    /// 99902524e052f37574194466c2949c576e4ab269, player.hpp SHA256
    /// 08b540c7667b8ec7b0fa54e7e196d9587ecb62406fdc0a5661b7ac8e91adb0f2.
    /// These target preferences grant no staff role or relationship facts.
    pub(crate) fn source_initial_preferences() -> Self {
        Self {
            allow_all: false,
            allow_own_guild: true,
            allow_own_party: true,
            allow_vip_list: true,
            allow_player_whitelist: true,
            allow_guild_whitelist: true,
            player_whitelist: Vec::new(),
            guild_whitelist: Vec::new(),
        }
    }
    fn validate(&self) -> Result<(), SpellItemError> {
        if self.player_whitelist.len() > 4096
            || self.guild_whitelist.len() > 4096
            || self.player_whitelist.windows(2).any(|w| w[0] >= w[1])
            || self.guild_whitelist.windows(2).any(|w| w[0] >= w[1])
            || self.guild_whitelist.contains(&0)
            || self
                .player_whitelist
                .iter()
                .any(|id| crate::domain::CharacterId::from_bytes(*id).is_err())
        {
            return Err(SpellItemError::Rejected("invalid Exiva preferences"));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExivaDecision {
    Allowed,
    Denied,
    HiddenStaff,
    UnknownPreferences,
    UnknownStaffRole,
    UnknownRelationships,
}
// Read-only semantic helper; its booleans are not accepted by the production reader.
fn decision(
    preferences: &ExivaPreferences,
    caster: [u8; 16],
    caster_staff: Option<bool>,
    target_staff: Option<bool>,
) -> ExivaDecision {
    match target_staff {
        Some(true) if caster_staff == Some(false) => return ExivaDecision::HiddenStaff,
        Some(true) if caster_staff.is_none() => return ExivaDecision::UnknownStaffRole,
        None => return ExivaDecision::UnknownStaffRole,
        _ => (),
    }
    if preferences.allow_all
        || preferences.allow_player_whitelist && preferences.player_whitelist.contains(&caster)
    {
        return ExivaDecision::Allowed;
    }
    // These relations have no admitted owner in the current native generation.
    // Their enabled branches cannot be evaluated with invented empty groups/lists.
    if preferences.allow_own_guild
        || preferences.allow_own_party
        || preferences.allow_vip_list
        || preferences.allow_guild_whitelist && !preferences.guild_whitelist.is_empty()
    {
        return ExivaDecision::UnknownRelationships;
    }
    ExivaDecision::Denied
}

pub(crate) async fn read_exiva_decision_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    target: [u8; 16],
) -> Result<ExivaDecision, SpellItemError> {
    read_exiva_decision_with_roles_in_transaction(tx, authority, target, None).await
}

/// Missing control projection can use only the genuine source Game group observed
/// from current admission in this same physical transaction. Preferences cannot
/// create this ordinary role or replace an explicit staff observation.
pub(crate) async fn read_exiva_decision_with_current_groups_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    target: [u8; 16],
    admissions: &super::fresh_admission::FreshAdmissionStore,
    target_session: crate::foundation::GameSessionId,
) -> Result<ExivaDecision, SpellItemError> {
    let caster = super::spell_familiar_group::read_current_group_in_transaction(
        tx,
        authority,
        admissions,
        authority.game_session_id(),
    )
    .await?;
    let target_group = super::spell_familiar_group::read_current_group_in_transaction(
        tx,
        authority,
        admissions,
        target_session,
    )
    .await?;
    if caster.character().as_bytes() != &authority.character_id_bytes()
        || target_group.character().as_bytes() != &target
    {
        return Err(SpellItemError::Rejected(
            "current Exiva group character mismatch",
        ));
    }
    read_exiva_decision_with_roles_in_transaction(
        tx,
        authority,
        target,
        Some((caster.access(), target_group.access())),
    )
    .await
}
async fn read_exiva_decision_with_roles_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    target: [u8; 16],
    current_group_roles: Option<(bool, bool)>,
) -> Result<ExivaDecision, SpellItemError> {
    check_transaction(tx, authority).await?;
    let source = authority.character_id_bytes();
    // Source and target security rows are read in UUID order, sharing actual
    // admission's transaction locks. The preference writer never changes roles.
    let rows=sqlx::query("SELECT uuid_send(character_id) AS character,staff_access FROM game_character_spell_access WHERE character_id=ANY(ARRAY[encode($1,'hex')::uuid,encode($2,'hex')::uuid]) ORDER BY character_id FOR SHARE")
        .bind(source.as_slice()).bind(target.as_slice()).fetch_all(&mut **tx).await?;
    let mut caster_staff = None;
    let mut target_staff = None;
    for row in rows {
        let id: Vec<u8> = row.try_get("character")?;
        let value: bool = row.try_get("staff_access")?;
        if id == source {
            caster_staff = Some(value);
        }
        if id == target {
            target_staff = Some(value);
        }
    }
    if let Some((caster_group_access, target_group_access)) = current_group_roles {
        caster_staff = caster_staff.or(Some(caster_group_access));
        target_staff = target_staff.or(Some(target_group_access));
    }
    let row=sqlx::query("SELECT preferences FROM game_character_spell_privacy WHERE character_id=encode($1,'hex')::uuid FOR SHARE")
        .bind(target.as_slice()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(ExivaDecision::UnknownPreferences);
    };
    let preferences: ExivaPreferences = serde_json::from_value(row.try_get("preferences")?)
        .map_err(|_| SpellItemError::Rejected("stored Exiva preference shape"))?;
    preferences.validate()?;
    Ok(decision(&preferences, source, caster_staff, target_staff))
}

/// Initialize only the actual current Character under its existing owner seal.
/// A stored target choice survives reconnect and retries; no role projection is
/// created here because the current admission material contains no staff role.
pub(crate) async fn initialize_exiva_preferences_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
) -> Result<u64, SpellItemError> {
    check_transaction(tx, authority).await?;
    let character = authority.character_id_bytes();
    sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(character.as_slice()).fetch_one(&mut **tx).await?;
    let row = sqlx::query("SELECT revision::text,preferences FROM game_character_spell_privacy WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(character.as_slice()).fetch_optional(&mut **tx).await?;
    if let Some(row) = row {
        let preferences: ExivaPreferences = serde_json::from_value(row.try_get("preferences")?)
            .map_err(|_| SpellItemError::Rejected("stored Exiva preference shape"))?;
        preferences.validate()?;
        let revision: String = row.try_get("revision")?;
        return revision
            .parse()
            .map_err(|_| SpellItemError::Rejected("stored privacy revision"));
    }
    write_exiva_preferences_in_transaction(
        tx,
        authority,
        None,
        &ExivaPreferences::source_initial_preferences(),
    )
    .await
}

/// The existing current Character item/admission seal binds this writer to the
/// target's own session and command. It grants no security role and commits no TX.
pub(crate) async fn write_exiva_preferences_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    expected_revision: Option<u64>,
    preferences: &ExivaPreferences,
) -> Result<u64, SpellItemError> {
    check_transaction(tx, authority).await?;
    preferences.validate()?;
    let character = authority.character_id_bytes();
    sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(character.as_slice()).fetch_one(&mut **tx).await?;
    let row=sqlx::query("SELECT revision::text FROM game_character_spell_privacy WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(character.as_slice()).fetch_optional(&mut **tx).await?;
    let revision = row
        .as_ref()
        .map(|row| row.try_get::<String, _>("revision"))
        .transpose()?
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|_| SpellItemError::Rejected("stored privacy revision"))
        })
        .transpose()?;
    if revision != expected_revision {
        return Err(SpellItemError::Rejected("stale Exiva preference revision"));
    }
    let before = revision.unwrap_or(0);
    let next = before
        .checked_add(1)
        .ok_or(SpellItemError::Rejected("privacy revision exhausted"))?;
    let preferences = serde_json::to_value(preferences)
        .map_err(|_| SpellItemError::Rejected("privacy encoding"))?;
    let command = authority.command();
    let receipt:Vec<u8>=sqlx::query_scalar("INSERT INTO game_character_spell_privacy_receipts(receipt_id,character_id,game_session_id,command_id,revision_before,revision_after,preferences) VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric(20,0),$4::text::numeric(20,0),$5::text::numeric(20,0),$6) RETURNING uuid_send(receipt_id)")
        .bind(character.as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
        .bind(before.to_string()).bind(next.to_string()).bind(&preferences).fetch_one(&mut **tx).await?;
    sqlx::query("INSERT INTO game_character_spell_privacy(character_id,revision,preferences,last_receipt) VALUES(encode($1,'hex')::uuid,$2::text::numeric(20,0),$3,encode($4,'hex')::uuid) ON CONFLICT(character_id) DO UPDATE SET revision=EXCLUDED.revision,preferences=EXCLUDED.preferences,last_receipt=EXCLUDED.last_receipt")
        .bind(character.as_slice()).bind(next.to_string()).bind(&preferences).bind(&receipt).execute(&mut **tx).await?;
    Ok(next)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn restrictive() -> ExivaPreferences {
        ExivaPreferences {
            allow_all: false,
            allow_own_guild: false,
            allow_own_party: false,
            allow_vip_list: false,
            allow_player_whitelist: true,
            allow_guild_whitelist: false,
            player_whitelist: vec![],
            guild_whitelist: vec![],
        }
    }
    #[test]
    fn source_constructor_is_restrictive_and_preserves_unknown_relations() {
        let preferences = ExivaPreferences::source_initial_preferences();
        assert!(!preferences.allow_all);
        assert!(preferences.player_whitelist.is_empty() && preferences.guild_whitelist.is_empty());
        assert_eq!(
            decision(&preferences, [1; 16], Some(false), Some(false)),
            ExivaDecision::UnknownRelationships
        );
    }
    #[test]
    fn unknown_role_and_relations_do_not_become_permission() {
        let mut p = restrictive();
        assert_eq!(
            decision(&p, [1; 16], Some(false), None),
            ExivaDecision::UnknownStaffRole
        );
        assert_eq!(
            decision(&p, [1; 16], Some(false), Some(false)),
            ExivaDecision::Denied
        );
        p.allow_own_party = true;
        assert_eq!(
            decision(&p, [1; 16], Some(false), Some(false)),
            ExivaDecision::UnknownRelationships
        );
        p.allow_all = true;
        assert_eq!(
            decision(&p, [1; 16], Some(false), Some(false)),
            ExivaDecision::Allowed
        );
        assert_eq!(
            decision(&p, [1; 16], Some(false), Some(true)),
            ExivaDecision::HiddenStaff
        );
    }
    #[test]
    fn target_whitelist_proves_allowed_without_guessing_other_relations() {
        let mut p = restrictive();
        p.allow_own_party = true;
        p.player_whitelist.push([1; 16]);
        assert_eq!(
            decision(&p, [1; 16], Some(false), Some(false)),
            ExivaDecision::Allowed
        );
        assert_eq!(
            decision(&p, [2; 16], Some(false), Some(false)),
            ExivaDecision::UnknownRelationships
        );
    }
}
