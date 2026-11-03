//! Bed-sleep dispatch fixtures only; durable PG cases live in game-server.
#[path = "../../../../../../apps/game-server/tests/support/house_scope_handoff_fixture.rs"]
mod house_fixture;
use crate::bestiary_postgres_harness::{CHARACTER, Harness, TestResult, WORLD, debug, id};
use crate::content::LogicalCell;
use crate::domain::appearance::Sex;
use crate::durability::bed_sleep::BedSleepRequest;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{AuthenticatedTransportRefV1, GameSessionId};

/// Fixed server-owner fixture facts, independently of each request under test.
pub(super) struct FixturePresence {
    world: crate::foundation::WorldId,
    session: GameSessionId,
}
pub(super) fn presence() -> TestResult<FixturePresence> {
    Ok(FixturePresence {
        world: crate::foundation::WorldId::decode(&id(WORLD)).map_err(debug)?,
        session: GameSessionId::decode(&id(61)).map_err(debug)?,
    })
}
impl crate::durability::spell_house_abi::house_presence_seal::Sealed for FixturePresence {}
impl crate::durability::spell_house_abi::HousePresenceProof for FixturePresence {
    fn world(&self) -> crate::foundation::WorldId {
        self.world
    }
    fn actor_session(&self) -> GameSessionId {
        self.session
    }
    fn scope_generation(&self) -> u64 {
        1
    }
    fn content_digest(&self) -> [u8; 32] {
        [7; 32]
    }
    fn house_key(&self) -> &str {
        HOUSE_KEY
    }
    fn has_door(&self, _: u32) -> bool {
        false
    }
}
use house_fixture::{HOUSE_KEY, admit, house, install_test_house, seed};

pub(super) async fn fixture(tag: &str, admin: &str) -> TestResult<Harness> {
    let h = Harness::create(admin.to_owned(), tag, false).await?;
    seed(&h).await?;
    install_test_house(&h).await?;
    admit(&h).await?;
    sqlx::query(include_str!(
        "../../../../../../apps/game-server/tests/support/owned_house_fixture.sql"
    ))
        .bind(id(WORLD).as_slice()).bind(HOUSE_KEY).bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
    sqlx::query("UPDATE game_durability_reconnect_sessions SET current_transport_ref=$1,session_state=1 WHERE game_session_id=encode($2,'hex')::uuid")
        .bind(id(64).as_slice()).bind(id(61).as_slice()).execute(&h.pool).await?;
    Ok(h)
}

pub(super) fn request() -> TestResult<BedSleepRequest> {
    let current = crate::bestiary_postgres_harness::fence(1)?;
    Ok(BedSleepRequest {
        final_position: None,
        fence: CurrentCharacterItemFence {
            character_id: current.character_id,
            game_session_id: GameSessionId::decode(&id(61)).map_err(debug)?,
            connection_generation: current.connection_generation,
            character_lease_generation: 2,
            runtime_scope: house()?.runtime_scope().map_err(debug)?,
            scope_ownership_generation: current.scope_ownership_generation,
        },
        transport: AuthenticatedTransportRefV1::decode(&id(64)).map_err(debug)?,
        command_id: 1,
        house: house()?,
        head: LogicalCell {
            x: 100,
            y: 200,
            z: -7,
        },
        sex: Sex::Female,
        skill: 6,
    })
}


/// Bounded authored bed pair; the active native qualifier validates these records.
/// Crystal items.xml:1671-1686 gives pillow/blanket and reciprocal east/west directions.
pub(super) fn bed_profiles(input: &mut crate::content::native_gameplay::NativeGameplayInput) -> TestResult {
    use sha2::Digest as _;
    let profiles = input.item_profiles.as_mut().ok_or("item profiles")?;
    let mut document: serde_json::Value = serde_json::from_slice(&profiles.bytes)?;
    let records = document["records"].as_array_mut().ok_or("records")?;
    let template = records.iter().find(|r| r["production_definition"]["production_key"] == "oteryn:item.tibia.i1629")
        .ok_or("fixture bed template")?.clone();
    for (id, part, direction) in [(696,"HEAD","EAST"),(697,"FOOT","WEST")] {
        let mut record = template.clone();
        let key = format!("oteryn:item.tibia.i{id}");
        record["authoring"]["item"]["key"] = format!("candidate:item/{id}").into();
        record["production_definition"]["production_key"] = key.clone().into();
        record["production_binding"]["external_id"] = id.to_string().into();
        record["production_binding"]["target"]["key"] = key.clone().into();
        record["semantics"]["classification"]["value"]["item_type"]["value"] = "BED".into();
        record["semantics"]["presentation"]["value"]["name"]["value"] = "bed".into();
        record["semantics"]["bed"] = serde_json::json!({"state":"KNOWN","value":{
            "part":part,"partner_direction":direction,
            "occupied_male":{"key":key,"revision":"definition-r1"},
            "occupied_female":{"key":key,"revision":"definition-r1"}
        }});
        record["attributes"]["blocks_movement"] = false.into();
        record["attributes"]["blocks_projectile"] = false.into();
        record["attributes"]["immovable_block_solid"] = false.into();
        records.push(record);
    }
    profiles.bytes = serde_json::to_vec(&document)?;
    profiles.sha256 = sha2::Sha256::digest(&profiles.bytes).iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(())
}
