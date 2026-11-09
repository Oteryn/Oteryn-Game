use super::*;

type TestResult<T = ()> = Result<T, String>;

fn sample() -> TestResult<OneItemDecayRetireV1> {
    let mut value = super::super::tests::world_reset_root_fixture();
    let id = |seed| super::super::tests::world_reset_uuid_fixture(seed).to_vec();
    let ground = value.ground.as_mut().ok_or("ground")?;
    ground.spatial_position = [
        32205_i32.to_be_bytes().as_slice(),
        31094_i32.to_be_bytes().as_slice(),
        (-7_i16).to_be_bytes().as_slice(),
    ]
    .concat();
    ground.runtime_scope_ownership_generation = value.runtime_scope_ownership_generation;
    value.arena_reset = Some(OneItemArenaResetV1 {
        world_id: ground.world_id.clone(),
        channel_id: ground.channel_id.clone(),
        character_id: id(10),
        game_session_id: id(11),
        run_id: id(12),
        item_instance_id: value
            .before
            .as_ref()
            .ok_or("before")?
            .item_instance_id
            .clone(),
        rank: 1,
        pit: 1,
        deadline_unix_ms: 1_790_000_600_000,
    });
    value.cause = None;
    Ok(value)
}

#[test]
fn m51_arena_reset_audit_round_trips_truthful_exact_channel_cause() -> TestResult {
    let value = sample()?;
    assert_eq!(check_decay_retire(&value), Ok(DecayRetireShape::Ground));
    let wire = encode_decay_retire_event(
        DecayRetireEventIdentity {
            event_id: super::super::tests::world_reset_uuid_fixture(30),
            transaction_id: super::super::tests::world_reset_uuid_fixture(40),
            occurred_at_unix_ms: 1_790_000_000_000,
            server_build_id: "oteryn-game-server/test",
        },
        value.clone(),
        mint::Type2EventTuple::V1,
    )
    .map_err(|e| format!("{e:?}"))?;
    let (_, decoded) = decode_decay_retire_envelope(&wire).map_err(|e| format!("{e:?}"))?;
    assert_eq!(decoded, value);
    Ok(())
}

#[test]
fn m51_arena_reset_audit_refuses_scope_value_cause_and_geometry_substitution() -> TestResult {
    let mut candidates = Vec::new();
    for pit in [0, 11, 257] {
        let mut value = sample()?;
        value.arena_reset.as_mut().ok_or("arena")?.pit = pit;
        candidates.push(value);
    }
    for rank in [0, 4] {
        let mut value = sample()?;
        value.arena_reset.as_mut().ok_or("arena")?.rank = rank;
        candidates.push(value);
    }
    for deadline in [0, u64::MAX] {
        let mut value = sample()?;
        value.arena_reset.as_mut().ok_or("arena")?.deadline_unix_ms = deadline;
        candidates.push(value);
    }
    let mut value = sample()?;
    value.arena_reset.as_mut().ok_or("arena")?.channel_id =
        super::super::tests::world_reset_uuid_fixture(99).to_vec();
    candidates.push(value);
    let mut value = sample()?;
    value.arena_reset.as_mut().ok_or("arena")?.run_id.clear();
    candidates.push(value);
    let mut value = sample()?;
    value.ground.as_mut().ok_or("ground")?.instance_id =
        Some(super::super::tests::world_reset_uuid_fixture(99).to_vec());
    candidates.push(value);
    let mut value = sample()?;
    value
        .ground
        .as_mut()
        .ok_or("ground")?
        .runtime_scope_ownership_generation += 1;
    candidates.push(value);
    let mut value = sample()?;
    value.ground.as_mut().ok_or("ground")?.spatial_position[0..4]
        .copy_from_slice(&32204_i32.to_be_bytes());
    candidates.push(value);
    let mut value = sample()?;
    value
        .ground
        .as_mut()
        .ok_or("ground")?
        .spatial_position
        .clear();
    candidates.push(value);
    let mut value = sample()?;
    value.after.as_mut().ok_or("after")?.quantity = 1;
    candidates.push(value);
    let mut value = sample()?;
    value.after.as_mut().ok_or("after")?.key_number += 1;
    candidates.push(value);
    let mut value = sample()?;
    value.cause = super::super::tests::world_reset_root_fixture().cause;
    candidates.push(value);
    for value in candidates {
        assert!(
            check_decay_retire(&value).is_err(),
            "substituted reset must be refused"
        );
    }
    Ok(())
}

#[test]
fn m51_arena_reset_audit_envelope_refuses_the_original_deadline_and_later() -> TestResult {
    let value = sample()?;
    let deadline = i64::try_from(value.arena_reset.as_ref().ok_or("arena")?.deadline_unix_ms)
        .map_err(|e| e.to_string())?;
    for at in [deadline - 1, deadline, deadline + 1] {
        let result = encode_decay_retire_event(
            DecayRetireEventIdentity {
                event_id: super::super::tests::world_reset_uuid_fixture(30),
                transaction_id: super::super::tests::world_reset_uuid_fixture(40),
                occurred_at_unix_ms: at,
                server_build_id: "oteryn-game-server/test",
            },
            value.clone(),
            mint::Type2EventTuple::V1,
        );
        assert_eq!(result.is_ok(), at < deadline);
    }
    Ok(())
}
