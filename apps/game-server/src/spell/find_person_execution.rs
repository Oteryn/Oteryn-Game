//! Find Person against actual Character names, current admission, privacy and actor slots.
//! All DB reads share the caster owner's sealed transaction. No name/position registry or
//! default public privacy is constructed. Cross-Channel physical owners must be resolved by
//! the common owner compositor before such a target can be disclosed.
use super::native::CompiledNativeSpell;
use super::native_house_movement::HouseMovementPlan;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::spell_item_transaction::{
    SpellItemAuthority, SpellItemError, check_transaction,
};
use crate::durability::spell_privacy::{
    ExivaDecision, read_exiva_decision_with_current_groups_in_transaction,
};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, GameSessionState, MovementPositionSnapshot,
    RuntimeScopeRefV1,
};
use sqlx::{Postgres, Row, Transaction};
#[derive(Debug)]
pub(crate) enum Error {
    Items(SpellItemError),
    Admission(crate::durability::DurabilityError),
    Actor(crate::foundation::CarrierError),
    InvalidProfile,
    CurrentCasterMismatch,
    CrossScopeTarget,
    PrivacyUnavailable(ExivaDecision),
    InvalidStoredName,
    Overflow,
}
fn name_query(input: &str) -> Option<(String, bool)> {
    if input.is_empty() || input.len() > 29 {
        return None;
    }
    let prefix = input.ends_with('~');
    let raw = if prefix {
        input.strip_suffix('~')?
    } else {
        input
    };
    if raw.is_empty()
        || !raw.bytes().all(|b| b.is_ascii_alphabetic() || b == b' ')
        || raw.split(' ').any(str::is_empty)
    {
        return None;
    }
    if !prefix && crate::domain::character_name::CharacterName::parse(raw).is_err() {
        return None;
    }
    Some((
        raw.bytes()
            .filter(|b| *b != b' ')
            .map(|b| char::from(b.to_ascii_lowercase()))
            .collect(),
        prefix,
    ))
}
fn absent(reason: &'static str) -> HouseMovementPlan {
    HouseMovementPlan::Refused {
        reason,
        effect: Some("poff"),
        cast_succeeds: false,
        start_cooldown: true,
    }
}
fn supports_player_location(spell: &CompiledNativeSpell) -> bool {
    let behavior = &spell.spell()["execution"]["native_behavior"];
    behavior["key"].as_str() == Some("locate_message")
        && behavior["parameters"]["source"].as_str() == Some("online_player")
}
#[allow(clippy::too_many_arguments)]
pub(crate) async fn plan_find_person_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    admissions: &FreshAdmissionStore,
    runtime: &ChannelRuntimeV1,
    spell: &CompiledNativeSpell,
    actor: ExactActorRef,
    expected: MovementPositionSnapshot,
    name: &str,
) -> Result<HouseMovementPlan, Error> {
    check_transaction(tx, authority)
        .await
        .map_err(Error::Items)?;
    if !supports_player_location(spell) {
        return Err(Error::InvalidProfile);
    }
    let scope = RuntimeScopeRefV1::Channel {
        world_id: runtime.binding().world_id(),
        channel_id: runtime.binding().channel_id(),
    };
    if authority.runtime_scope() != scope
        || authority.scope_generation() != runtime.binding().scope_generation().get()
        || authority.compatible_content_digest() != runtime.content_pin().server_artifact_digest()
        || runtime
            .positioned_player_for_session(authority.game_session_id())
            .map_err(Error::Actor)?
            != Some((actor, expected))
        || expected.context() != runtime.pinned_movement_context()
    {
        return Err(Error::CurrentCasterMismatch);
    }
    let Some((key, prefix)) = name_query(name) else {
        return Ok(absent("player_with_this_name_is_not_online"));
    };
    // Match only actual active admissions; row locks preserve source name/guard/session
    // continuity until the owner's cast decision. Two prefix matches are ambiguous before
    // staff/privacy filtering, matching the source wildcard resolver's ordering.
    let candidates=sqlx::query("SELECT uuid_send(r.character_id) AS character,r.name,r.name_key,uuid_send(s.game_session_id) AS session FROM game_character_roots r JOIN game_durability_admission_character_guards g USING(character_id) JOIN game_durability_reconnect_sessions s ON s.game_session_id=g.holder_game_session_id WHERE r.world_id=encode($1,'hex')::uuid AND r.lifecycle=1 AND g.eligible AND g.world_id=r.world_id AND s.character_id=r.character_id AND s.world_id=r.world_id AND s.session_state=2 AND s.current_transport_ref IS NOT NULL AND s.control_loss_epoch IS NULL AND (($3 AND left(r.name_key,length($2))=$2) OR (NOT $3 AND r.name_key=$2)) ORDER BY r.name_key LIMIT 2 FOR SHARE OF r,g,s")
        .bind(runtime.binding().world_id().as_bytes().as_slice()).bind(&key).bind(prefix).fetch_all(&mut **tx).await
        .map_err(SpellItemError::from).map_err(Error::Items)?;
    if candidates.len() > 1 {
        return Ok(absent("name_is_ambiguous"));
    }
    let Some(target) = candidates.first() else {
        return Ok(absent("player_with_this_name_is_not_online"));
    };
    let character: Vec<u8> = target
        .try_get("character")
        .map_err(SpellItemError::from)
        .map_err(Error::Items)?;
    let character: [u8; 16] = character.try_into().map_err(|_| Error::InvalidStoredName)?;
    let target_name = crate::domain::character_name::CharacterName::parse(
        &target
            .try_get::<String, _>("name")
            .map_err(SpellItemError::from)
            .map_err(Error::Items)?,
    )
    .map_err(|_| Error::InvalidStoredName)?;
    let stored_key: String = target
        .try_get("name_key")
        .map_err(SpellItemError::from)
        .map_err(Error::Items)?;
    if target_name.comparison_key() != stored_key
        || if prefix {
            !stored_key.starts_with(&key)
        } else {
            stored_key != key
        }
    {
        return Err(Error::InvalidStoredName);
    }
    let session: Vec<u8> = target
        .try_get("session")
        .map_err(SpellItemError::from)
        .map_err(Error::Items)?;
    let session = GameSessionId::decode(&session).map_err(|_| Error::InvalidStoredName)?;
    let current = admissions
        .current_session_in_transaction(tx, session)
        .await
        .map_err(Error::Admission)?;
    if current.session_state() != GameSessionState::Active
        || current.current_transport().is_none()
        || current.current_control_loss_epoch().is_some()
        || current.current_character_lease().character_id().as_bytes() != &character
    {
        return Ok(absent("player_with_this_name_is_not_online"));
    }
    if current.current_runtime_scope() != scope
        || current.current_scope_generation() != runtime.binding().scope_generation()
    {
        return Err(Error::CrossScopeTarget);
    }
    let Some((_, target_position)) = runtime
        .positioned_player_for_session(session)
        .map_err(Error::Actor)?
    else {
        return Ok(absent("player_with_this_name_is_not_online"));
    };
    match read_exiva_decision_with_current_groups_in_transaction(
        tx, authority, character, admissions, session,
    )
    .await
    .map_err(Error::Items)?
    {
        ExivaDecision::Allowed => (),
        ExivaDecision::HiddenStaff => {
            return Ok(HouseMovementPlan::Refused {
                reason: "player_with_this_name_is_not_online",
                effect: Some("poff"),
                cast_succeeds: false,
                start_cooldown: false,
            });
        }
        ExivaDecision::Denied => {
            return Ok(HouseMovementPlan::Refused {
                reason: "exiva_protected",
                effect: None,
                cast_succeeds: false,
                start_cooldown: false,
            });
        }
        unknown => return Err(Error::PrivacyUnavailable(unknown)),
    }
    let source = expected.position();
    let target = target_position.position();
    let located = super::locate::locate(
        source.x.checked_sub(target.x).ok_or(Error::Overflow)?,
        source.y.checked_sub(target.y).ok_or(Error::Overflow)?,
        i32::from(target.floor) - i32::from(source.floor),
    );
    check_transaction(tx, authority)
        .await
        .map_err(Error::Items)?;
    Ok(HouseMovementPlan::Message {
        text: format!("{} {}.", target_name.as_str(), located.phrase()),
        effect: "magic_blue",
    })
}
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    #[test]
    fn actual_canonical_find_person_profile_has_online_player_source() {
        let document: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
        ))
        .unwrap();
        let profiles = document["profiles"].as_array().unwrap();
        for row in profiles {
            let native = crate::spell::native::spell_from_bundle(
                &serde_json::json!({"spell":row["spell"]}),
                &row["dependencies"],
            );
            if row["name"] == "Find Person" {
                assert!(supports_player_location(&native.unwrap()));
            } else if row["name"] == "Find Fiend" {
                assert!(!supports_player_location(&native.unwrap()));
            }
        }
    }
    #[test]
    fn exact_names_and_unique_prefix_are_distinct_resolutions() {
        assert_eq!(name_query("Al Dric"), Some(("aldric".into(), false)));
        assert_eq!(name_query("Al~"), Some(("al".into(), true)));
        assert_eq!(name_query("A~"), Some(("a".into(), true)));
        for invalid in ["", "~", "A", "Al*", " Al", "Al  Dric", "Al~x"] {
            assert!(name_query(invalid).is_none());
        }
    }
}
