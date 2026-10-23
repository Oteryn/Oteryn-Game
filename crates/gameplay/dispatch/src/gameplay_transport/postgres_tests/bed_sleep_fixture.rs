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
