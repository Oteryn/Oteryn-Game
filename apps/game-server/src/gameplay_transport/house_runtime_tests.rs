use super::{activate_house_for_login, exit_house_scope, prepare_house_entry};
use crate::foundation::{ChannelId, CharacterId, GameSessionId, RuntimeScopeRefV1, WorldId};
use crate::house_scope::{
    HouseEntryPrecheck, HouseEntryRefusal, HouseEntryRequest, HouseHandoffDirection,
    HouseHandoffError, HouseId, HousePrepareOutcome,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}

fn request() -> TestResult<HouseEntryRequest> {
    let world = WorldId::decode(&id(42)).map_err(|error| format!("{error:?}"))?;
    Ok(HouseEntryRequest {
        direction: HouseHandoffDirection::Entry,
        handoff_id: id(60),
        character_id: CharacterId::decode(&id(41)).map_err(|error| format!("{error:?}"))?,
        source_game_session_id: GameSessionId::decode(&id(50))
            .map_err(|error| format!("{error:?}"))?,
        source_connection_generation: 1,
        source_character_lease_generation: 1,
        source_scope: RuntimeScopeRefV1::channel(
            world,
            ChannelId::decode(&id(43)).map_err(|error| format!("{error:?}"))?,
        ),
        source_scope_ownership_generation: 1,
        house: HouseId::new(world, "oteryn:content.house.thais_1").ok_or("house key")?,
        house_scope_ownership_generation: 1,
        acl_revision: 1,
        guild_revisions: None,
        // Opaque handoff input only; this fixture does not qualify a physical tile.
        reserved_tile: vec![1, 2, 3],
        prepared_at: 100,
    })
}

#[test]
fn permissive_precheck_cannot_prepare_a_runtime_entry() -> TestResult {
    let precheck = HouseEntryPrecheck {
        has_access: true,
        house_open: true,
        combat_locked: false,
        operation_pending: false,
    };
    assert_eq!(precheck.evaluate(), None);
    let request = request()?;
    let before = request.clone();
    assert_eq!(
        prepare_house_entry(&request),
        HousePrepareOutcome::Refused(HouseEntryRefusal::HouseClosed)
    );
    assert_eq!(
        request, before,
        "refusal preserves the exact source request"
    );
    Ok(())
}

#[test]
fn stale_or_invalid_request_cannot_reach_load_or_prepare() -> TestResult {
    let valid = request()?;
    let mut cases = Vec::new();
    let mut stale = valid.clone();
    stale.source_character_lease_generation = 0;
    cases.push(stale);
    let mut no_assignment = valid.clone();
    no_assignment.house_scope_ownership_generation = 0;
    cases.push(no_assignment);
    let mut no_tile = valid;
    no_tile.reserved_tile.clear();
    cases.push(no_tile);
    for request in cases {
        let before = request.clone();
        assert_eq!(
            prepare_house_entry(&request),
            HousePrepareOutcome::Refused(HouseEntryRefusal::HouseClosed)
        );
        assert_eq!(request, before);
    }
    Ok(())
}

#[test]
fn saved_house_identity_never_activates_an_owner() -> TestResult {
    let request = request()?;
    let before = request.house.clone();
    assert_eq!(
        activate_house_for_login(&request.house),
        Err(HouseEntryRefusal::HouseClosed)
    );
    assert_eq!(request.house, before);
    assert_eq!(HouseEntryRefusal::HouseClosed.as_str(), "HOUSE_CLOSED");
    Ok(())
}

#[test]
fn runtime_exit_preserves_the_actual_native_refusal() {
    assert!(matches!(
        crate::house_scope::exit_into_channel(),
        Err(HouseHandoffError::ExitNotAdmitted)
    ));
    assert!(matches!(
        exit_house_scope(),
        Err(HouseHandoffError::ExitNotAdmitted)
    ));
}
