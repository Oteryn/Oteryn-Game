//! PODIUM-1. House custody and current cosmetic entitlement are resolved in one transaction.
use super::character_appearance::{AppearanceReadError, earned_unlocks};
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::appearance::{
    AppearanceContent, AppearanceSelection, EarnedCosmetics, Sex, validate_selection,
};
use crate::foundation::{CommandRef, LocalPosition};
use oteryn_gameplay_content::content::native_gameplay::NativeCosmeticsView;
use oteryn_protocol_oteryn::podium::{self as wire, Display};
use serde_json::{Value, json};

pub struct PodiumCommand {
    pub command: CommandRef,
    pub request: wire::Request,
    /// Independently read under the House runtime lock, never from the client.
    pub position: LocalPosition,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn unhex(value: &Value, name: &str) -> Result<Vec<u8>, DurabilityError> {
    let text = value[name]
        .as_str()
        .ok_or(DurabilityError::InvalidStoredState)?;
    if text.len() % 2 != 0 || text.len() > 256 {
        return Err(DurabilityError::InvalidStoredState);
    }
    (0..text.len())
        .step_by(2)
        .map(|i| {
            text.get(i..i + 2)
                .and_then(|v| u8::from_str_radix(v, 16).ok())
                .ok_or(DurabilityError::InvalidStoredState)
        })
        .collect()
}
fn state(value: &Value) -> Result<(u64, Display), DurabilityError> {
    let revision = value["revision"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .ok_or(DurabilityError::InvalidStoredState)?;
    let display = wire::decode_display(&unhex(value, "display")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok((revision, display))
}
fn reply(code: u16, revision: u64, display: Display) -> Result<Vec<u8>, DurabilityError> {
    wire::encode_result(code, revision, display).map_err(|_| DurabilityError::InvalidStoredState)
}
fn refused(code: u16) -> Result<Vec<u8>, DurabilityError> {
    reply(code, 0, Display::default())
}
fn sql_refusal(error: sqlx::Error) -> Result<Vec<u8>, DurabilityError> {
    if let sqlx::Error::Database(e) = &error
        && let Some(code) = e
            .message()
            .strip_prefix("PODIUM_")
            .and_then(|s| s.parse::<u16>().ok())
        && (1630..=1639).contains(&code)
    {
        return refused(code);
    }
    Err(error.into())
}
fn authorized(
    display: Display,
    sex: Sex,
    content: &NativeCosmeticsView,
    facts: &[(String, String)],
) -> bool {
    let outfit_key = if display.look_type == 0 {
        content.fallback(sex).outfit_key
    } else {
        match content
            .outfit_keys()
            .find(|key| content.look_type(key, sex) == Some(u32::from(display.look_type)))
        {
            Some(key) => key.to_owned(),
            None => return false,
        }
    };
    let mount_key = if display.mount == 0 {
        None
    } else {
        match content.mount_keys().find(|key| {
            content
                .mount_metadata(key)
                .is_some_and(|(look, _, _)| look == u32::from(display.mount))
        }) {
            Some(key) => Some(key.to_owned()),
            None => return false,
        }
    };
    let selection = AppearanceSelection {
        outfit_key,
        mount_key,
        colours: display.colours.map(u16::from),
        addons: display.addons,
    };
    validate_selection(
        &selection,
        sex,
        content,
        &EarnedCosmetics { content, facts },
    )
    .is_ok()
}

impl DurabilityRoot {
    /// A current House actor, native recovery and same-transaction Item/ACL/earned checks.
    /// Display metadata follows the Item across custody changes; this writer moves no value.
    pub async fn podium_command(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        content: NativeCosmeticsView,
        command: PodiumCommand,
    ) -> Result<Vec<u8>, DurabilityError> {
        if wire::encode_request(&command.request).is_err()
            || command.command.game_session_id() != fence.game_session_id
        {
            return refused(1630);
        }
        let house = super::house_scope_handoff::HouseId::new(
            fence.runtime_scope.world_id(),
            &command.request.house,
        );
        if !house.is_some_and(|house| house.runtime_scope().ok() == Some(fence.runtime_scope)) {
            return refused(1631);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let request = &command.request;
            let uuid = hex;
            let p = json!({"session":uuid(fence.game_session_id.as_bytes()),"command":command.command.command_id().get().to_string(),
                "character":uuid(fence.character_id.as_bytes()),"world":uuid(fence.runtime_scope.world_id().as_bytes()),
                "house":request.house,"item":uuid(&request.item),"connection":fence.connection_generation.get().to_string(),
                "lease":fence.character_lease_generation.to_string(),"generation":fence.scope_ownership_generation.get().to_string(),
                "content":content.content_revision(),"digest":hex(&content.source_digest())});
            let snapshot: Value = match sqlx::query_scalar("SELECT game_podium_access($1,encode($2,'hex')::uuid,$3::text::numeric,$4)")
                .bind(&p).bind(node.fact().node_id().as_bytes().as_slice())
                .bind(node.fact().registration_revision().to_string()).bind(node.sql_possession_secret())
                .fetch_one(&mut *tx).await { Ok(v)=>v,Err(e)=>return sql_refusal(e) };
            let (revision,display) = state(&snapshot)?;
            let position = unhex(&snapshot,"position")?;
            if position.len() != 12 { return Err(DurabilityError::InvalidStoredState); }
            let coords: [i32; 3] = std::array::from_fn(|i| i32::from_be_bytes(position.as_chunks::<4>().0[i]));
            if i64::from(coords[0]).abs_diff(i64::from(command.position.x)) > 1
                || i64::from(coords[1]).abs_diff(i64::from(command.position.y)) > 1
                || coords[2] != i32::from(command.position.floor) { return refused(1639); }
            if request.action == 0 {
                commit_semantic_transaction(tx,deadline).await?;
                return reply(0,revision,display);
            }
            let selected = if request.action == 2 { display } else { request.display };
            let sex = match snapshot["sex"].as_i64() { Some(1)=>Sex::Female,Some(2)=>Sex::Male,_=>return Err(DurabilityError::InvalidStoredState) };
            let facts = earned_unlocks::read(&mut tx,fence.character_id,&content.earned_grant_keys()).await.map_err(|e| match e {
                AppearanceReadError::InvalidStoredState=>DurabilityError::InvalidStoredState, AppearanceReadError::Storage(e)=>e.into() })?;
            if !authorized(selected,sex,&content,&facts) { return refused(1635); }
            let after: Value = match sqlx::query_scalar("SELECT game_podium_apply($1,encode($2,'hex')::uuid,$3::text::numeric,$4,$5,$6::text::numeric,$7)")
                .bind(p).bind(node.fact().node_id().as_bytes().as_slice()).bind(node.fact().registration_revision().to_string())
                .bind(node.sql_possession_secret()).bind(i16::from(request.action)).bind(request.expected_revision.to_string())
                .bind(wire::encode_display(request.display).map_err(|_| DurabilityError::InvalidStoredState)?.as_slice())
                .fetch_one(&mut *tx).await { Ok(v)=>v,Err(e)=>return sql_refusal(e) };
            let (revision,display) = state(&after)?;
            commit_semantic_transaction(tx,deadline).await?;
            reply(0,revision,display)
        })).await
    }
}
