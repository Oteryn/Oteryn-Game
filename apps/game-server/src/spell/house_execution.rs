//! Aleta and GUI use the actual qualified map's physical House membership and
//! the same durable World-global ACL. Catalogue ids or detached plan booleans
//! cannot manufacture a current House presence.
use super::native::CompiledNativeSpell;
use super::native_house_movement::HouseList;
use crate::content::{LogicalCell, QualifiedHousePlacement, QualifiedNativeEntryRoom};
use crate::durability::house_spell_acl::{HouseEditor, open_editor_in_transaction};
use crate::durability::spell_item_transaction::{
    SpellItemAuthority, SpellItemError, check_transaction,
};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementFacing, MovementPositionSnapshot,
};
use sqlx::{Postgres, Transaction};
#[derive(Debug)]
pub(crate) enum Error {
    Authority,
    PlacementUnknown,
    NotInHouse,
    InvalidProfile,
    FacingUnknown,
    NoDoor,
    HouseInteriorOwnerUnavailable,
    PrivilegeUnavailable,
    Storage(SpellItemError),
}
#[derive(Debug)]
pub(crate) struct CurrentHousePresence<'owner> {
    house: &'owner QualifiedHousePlacement,
    runtime: &'owner ChannelRuntimeV1,
    session: GameSessionId,
    current: MovementPositionSnapshot,
}
impl CurrentHousePresence<'_> {
    pub(crate) fn world(&self) -> crate::foundation::WorldId {
        self.runtime.binding().world_id()
    }
    pub(crate) fn actor_session(&self) -> GameSessionId {
        self.session
    }
    pub(crate) fn scope_generation(&self) -> u64 {
        self.runtime.binding().scope_generation().get()
    }
    pub(crate) fn content_digest(&self) -> [u8; 32] {
        self.runtime.content_pin().server_artifact_digest()
    }
    pub(crate) fn house_key(&self) -> &str {
        &self.house.house().key
    }
    pub(crate) fn has_door(&self, id: u32) -> bool {
        self.house.has_door(id)
    }
    pub(crate) fn current_position(&self) -> MovementPositionSnapshot {
        self.current
    }
    pub(crate) fn entry(&self) -> LogicalCell {
        self.house.entry()
    }
}
impl crate::durability::spell_house_abi::house_presence_seal::Sealed for CurrentHousePresence<'_> {}
impl crate::durability::spell_house_abi::HousePresenceProof for CurrentHousePresence<'_> {
    fn world(&self) -> crate::foundation::WorldId {
        CurrentHousePresence::world(self)
    }
    fn actor_session(&self) -> GameSessionId {
        CurrentHousePresence::actor_session(self)
    }
    fn scope_generation(&self) -> u64 {
        CurrentHousePresence::scope_generation(self)
    }
    fn content_digest(&self) -> [u8; 32] {
        CurrentHousePresence::content_digest(self)
    }
    fn house_key(&self) -> &str {
        CurrentHousePresence::house_key(self)
    }
    fn has_door(&self, id: u32) -> bool {
        CurrentHousePresence::has_door(self, id)
    }
}
pub(crate) fn current_house_presence<'owner>(
    room: &'owner QualifiedNativeEntryRoom,
    runtime: &'owner ChannelRuntimeV1,
    session: GameSessionId,
    actor: ExactActorRef,
    expected: MovementPositionSnapshot,
) -> Result<CurrentHousePresence<'owner>, Error> {
    let pin = runtime.content_pin();
    let cells = room.movement_cells();
    if cells.scope().world_id != runtime.binding().world_id()
        || room.compiled().server_digest() != pin.server_artifact_digest()
        || cells.scope().generation_digest != pin.server_artifact_digest()
        || room.frame_binding().digest() != pin.frame_binding_digest()
        || room.map_revision_digest() != pin.map_revision_digest()
        || expected.context() != runtime.pinned_movement_context()
        || runtime
            .positioned_player_for_session(session)
            .map_err(|_| Error::Authority)?
            != Some((actor, expected))
    {
        return Err(Error::Authority);
    }
    let p = expected.position();
    let cell = LogicalCell {
        x: p.x,
        y: p.y,
        z: i32::from(p.floor),
    };
    let house = cells
        .house_tiles()
        .house_at(cells.scope(), cell)
        .map_err(|_| Error::PlacementUnknown)?
        .ok_or(Error::NotInHouse)?;
    // The present carrier is explicitly Channel-scoped. A mapped physical
    // address cannot install the accepted World-global House Instance owner.
    // Interior admission/handoff must supply that actual owner before this
    // proof can be constructed; permitting it here would create Channel copies.
    let _ = (house, runtime, session, expected);
    Err(Error::HouseInteriorOwnerUnavailable)
}
/// Native House editor view replaces the legacy numeric house/window id. The
/// matching one-use editor token and current session are checked again on save.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn open_aleta_editor_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    room: &QualifiedNativeEntryRoom,
    runtime: &ChannelRuntimeV1,
    spell: &CompiledNativeSpell,
    actor: ExactActorRef,
    expected: MovementPositionSnapshot,
) -> Result<HouseEditor, Error> {
    check_transaction(tx, authority)
        .await
        .map_err(Error::Storage)?;
    let behavior = &spell.spell()["execution"]["native_behavior"];
    if behavior["key"].as_str() != Some("house_access")
        || behavior["parameters"]["action"].as_str() != Some("edit_list")
    {
        return Err(Error::InvalidProfile);
    }
    let presence =
        current_house_presence(room, runtime, authority.game_session_id(), actor, expected)?;
    let list = match behavior["parameters"]["list"].as_str() {
        Some("guest") => HouseList::Guest,
        Some("subowner") => HouseList::Subowner,
        Some("door") => {
            let position = expected.position();
            let (dx, dy) = match expected.facing().ok_or(Error::FacingUnknown)? {
                MovementFacing::North => (0, -1),
                MovementFacing::East => (1, 0),
                MovementFacing::South => (0, 1),
                MovementFacing::West => (-1, 0),
            };
            let forward = LogicalCell {
                x: position.x.checked_add(dx).ok_or(Error::Authority)?,
                y: position.y.checked_add(dy).ok_or(Error::Authority)?,
                z: i32::from(position.floor),
            };
            let current = LogicalCell {
                x: position.x,
                y: position.y,
                z: i32::from(position.floor),
            };
            let door = presence
                .house
                .door_at(forward)
                .or_else(|| presence.house.door_at(current))
                .ok_or(Error::NoDoor)?;
            HouseList::Door(door)
        }
        _ => return Err(Error::InvalidProfile),
    };
    open_editor_in_transaction(tx, authority, &presence, list)
        .await
        .map_err(Error::Storage)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_room_cannot_invent_house_from_catalogue_or_character() {
        let (mut runtime, actor, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(90);
        let room = crate::content::qualify_native_entry_room(runtime.binding().world_id()).unwrap();
        runtime.initialize_first_entry_position(actor).unwrap();
        let expected = runtime.read_actor_position(actor).unwrap();
        // Fixture helper has a different active pin than the qualified room;
        // neither that mismatch nor missing placement can become a House proof.
        assert!(current_house_presence(&room, &runtime, session, actor, expected).is_err());
    }
}

/// Constructor is private to the actual DB/admission/map resolver below. Its
/// coordinates come from the target's current authored House, never a Move plan.
pub(crate) struct HouseKickAuthorization {
    actor: ExactActorRef,
    expected: MovementPositionSnapshot,
    destination: crate::foundation::MovementLocalPosition,
}
impl HouseKickAuthorization {
    pub(super) fn parts(
        &self,
    ) -> (
        ExactActorRef,
        MovementPositionSnapshot,
        crate::foundation::MovementLocalPosition,
    ) {
        (self.actor, self.expected, self.destination)
    }
}
#[allow(clippy::too_many_arguments)]
pub(crate) async fn authorize_aleta_kick_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    admissions: &crate::durability::fresh_admission::FreshAdmissionStore,
    room: &QualifiedNativeEntryRoom,
    runtime: &ChannelRuntimeV1,
    spell: &CompiledNativeSpell,
    actor: ExactActorRef,
    expected: MovementPositionSnapshot,
    name: &str,
) -> Result<HouseKickAuthorization, Error> {
    use super::native_house_movement::HouseAccess;
    use crate::durability::house_spell_acl::house_access_for_character;
    use sqlx::Row;
    check_transaction(tx, authority)
        .await
        .map_err(Error::Storage)?;
    let behavior = &spell.spell()["execution"]["native_behavior"];
    if behavior["key"].as_str() != Some("house_access")
        || behavior["parameters"]["action"].as_str() != Some("kick")
        || name.is_empty()
        || name.len() > 29
    {
        return Err(Error::InvalidProfile);
    }
    let caster =
        current_house_presence(room, runtime, authority.game_session_id(), actor, expected)?;
    if authority.runtime_scope().world_id() != caster.world()
        || authority.scope_generation() != caster.scope_generation()
        || authority.compatible_content_digest() != caster.content_digest()
    {
        return Err(Error::Authority);
    }
    // Source Player(name) falls back to the caster when an exact online name is
    // absent. The ordinary native naming-policy namespace is the sole name owner.
    let candidate = if let Ok(parsed) = crate::domain::character_name::CharacterName::parse(name) {
        sqlx::query("SELECT uuid_send(r.character_id) AS character,uuid_send(s.game_session_id) AS session FROM game_character_roots r JOIN game_durability_admission_character_guards g USING(character_id) JOIN game_durability_reconnect_sessions s ON s.game_session_id=g.holder_game_session_id WHERE r.world_id=encode($1,'hex')::uuid AND r.lifecycle=1 AND r.name_key=$2 AND g.eligible AND g.world_id=r.world_id AND s.character_id=r.character_id AND s.world_id=r.world_id AND s.session_state=2 AND s.current_transport_ref IS NOT NULL AND s.control_loss_epoch IS NULL FOR SHARE OF r,g,s")
            .bind(caster.world().as_bytes().as_slice()).bind(parsed.comparison_key()).fetch_optional(&mut **tx).await.map_err(SpellItemError::from).map_err(Error::Storage)?
    } else {
        None
    };
    let mut target_actor = actor;
    let mut target_snapshot = expected;
    let mut target_character = authority.character_id_bytes();
    let mut target_session = authority.game_session_id();
    if let Some(row) = candidate {
        let character: Vec<u8> = row
            .try_get("character")
            .map_err(SpellItemError::from)
            .map_err(Error::Storage)?;
        let session: Vec<u8> = row
            .try_get("session")
            .map_err(SpellItemError::from)
            .map_err(Error::Storage)?;
        target_character = character.try_into().map_err(|_| Error::Authority)?;
        target_session = GameSessionId::decode(&session).map_err(|_| Error::Authority)?;
        let current = admissions
            .current_session_in_transaction(tx, target_session)
            .await
            .map_err(|_| Error::Authority)?;
        if current.session_state() != crate::foundation::GameSessionState::Active
            || current.current_transport().is_none()
            || current.current_control_loss_epoch().is_some()
            || current.current_character_lease().character_id().as_bytes() != &target_character
            || current.current_runtime_scope() != authority.runtime_scope()
            || current.current_scope_generation().get() != authority.scope_generation()
        {
            return Err(Error::Authority);
        }
        let (target, snapshot) = runtime
            .positioned_player_for_session(target_session)
            .map_err(|_| Error::Authority)?
            .ok_or(Error::Authority)?;
        target_actor = target;
        target_snapshot = snapshot;
    }
    let target =
        current_house_presence(room, runtime, target_session, target_actor, target_snapshot)?;
    if target_actor != actor {
        let (_, caster_own_access) = house_access_for_character(
            tx,
            caster.world(),
            caster.house_key(),
            authority.character_id_bytes(),
        )
        .await
        .map_err(Error::Storage)?;
        if caster_own_access < HouseAccess::Subowner {
            return Err(Error::Authority);
        }
        let (_, caster_target_access) = house_access_for_character(
            tx,
            caster.world(),
            target.house_key(),
            authority.character_id_bytes(),
        )
        .await
        .map_err(Error::Storage)?;
        let (_, target_access) =
            house_access_for_character(tx, caster.world(), target.house_key(), target_character)
                .await
                .map_err(Error::Storage)?;
        if caster_target_access < target_access {
            return Err(Error::Authority);
        }
        let group = crate::durability::spell_familiar_group::read_current_group_in_transaction(
            tx,
            authority,
            admissions,
            target_session,
        )
        .await
        .map_err(Error::Storage)?;
        match group.flag("canedithouses") {
            Some(false) => (),
            Some(true) => return Err(Error::Authority),
            None => return Err(Error::PrivilegeUnavailable),
        }
    }
    let entry = target.entry();
    let destination = crate::foundation::MovementLocalPosition {
        x: entry.x,
        y: entry.y,
        floor: i16::try_from(entry.z).map_err(|_| Error::Authority)?,
    };
    check_transaction(tx, authority)
        .await
        .map_err(Error::Storage)?;
    Ok(HouseKickAuthorization {
        actor: target_actor,
        expected: target_snapshot,
        destination,
    })
}
