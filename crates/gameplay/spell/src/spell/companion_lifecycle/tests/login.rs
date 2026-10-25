#[test]
fn foreign_open_login_discards_crash_remaining_and_preserves_writer_and_cooldown()
-> Result<(), CompanionExecutionError> {
    let old_writer = FamiliarLifecycleWriter {
        game_session_id: "0199aabb-1234-7000-8000-000000000001".into(),
        connection_generation: 1,
        world_id: "0199aabb-1234-7000-8000-000000000002".into(),
        channel_id: "0199aabb-1234-7000-8000-000000000003".into(),
        scope_ownership_generation: 4,
    };
    let current_writer = FamiliarLifecycleWriter {
        game_session_id: "0199aabb-1234-7000-8000-000000000004".into(),
        ..old_writer.clone()
    };
    let mut state = DurableFamiliarState {
        lifecycle: Some(FamiliarLifecycleV2 {
            version: 2,
            familiar_remaining_ms: 90_000,
            open: true,
            writer: old_writer.clone(),
            clean_end_game_session_id: None,
        }),
        cooldowns: vec![FamiliarCooldownSnapshot {
            reference_spell_id: 197,
            remaining_micros: 1_234,
        }],
        ..DurableFamiliarState::default()
    };
    let before = state.clone();
    let lifecycle = state
        .lifecycle
        .as_mut()
        .ok_or(CompanionExecutionError::InvalidOwnerFacts)?;
    prepare_familiar_login_lifecycle(lifecycle, &current_writer)?;
    assert_eq!(lifecycle.familiar_remaining_ms, 0);
    assert!(!lifecycle.open);
    assert_eq!(lifecycle.writer, old_writer);
    assert_eq!(lifecycle.clean_end_game_session_id, None);
    assert_eq!(state.cooldowns, before.cooldowns);
    let mut expected = before;
    expected
        .lifecycle
        .as_mut()
        .ok_or(CompanionExecutionError::InvalidOwnerFacts)?
        .familiar_remaining_ms = 0;
    expected
        .lifecycle
        .as_mut()
        .ok_or(CompanionExecutionError::InvalidOwnerFacts)?
        .open = false;
    assert_eq!(state, expected);
    Ok(())
}

/// #2199 Codex P1 4302326064: the owner's current session scope resolves lever boss room
/// membership. A Channel scope is outside (the fenced write re-proves the Channel scope);
/// an instance scope stays unresolved without the BOSS-ROOM-1 owner and is never guessed.
#[test]
fn lever_boss_room_membership_comes_from_the_current_scope()
-> Result<(), CompanionExecutionError> {
    let uuid = |seed: u8| {
        let mut bytes = [0_u8; 16];
        bytes[0] = 1;
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = seed;
        bytes
    };
    let world = crate::foundation::WorldId::decode(&uuid(1))
        .map_err(|_| CompanionExecutionError::InvalidOwnerFacts)?;
    let channel = crate::foundation::ChannelId::decode(&uuid(2))
        .map_err(|_| CompanionExecutionError::InvalidOwnerFacts)?;
    let channel_scope = crate::foundation::RuntimeScopeRefV1::channel(world, channel);
    let instance_scope = crate::foundation::RuntimeScopeRefV1::instance(world, uuid(3))
        .map_err(|_| CompanionExecutionError::InvalidOwnerFacts)?;
    assert_eq!(
        LeverBossRoomMembership::of_current_scope(&channel_scope),
        LeverBossRoomMembership::Outside
    );
    assert_eq!(
        LeverBossRoomMembership::of_current_scope(&instance_scope),
        LeverBossRoomMembership::Unresolved
    );
    Ok(())
}

/// #2199 Codex P1 4302326064 (FAMILIARS0 §7): a waiting return inside a lever boss room
/// ends the stored familiar (time 0, row closed, writer and marker kept); outside it is
/// untouched, and an unresolved room refuses so the row stays waiting. A row that does not
/// wait is never changed by the room check.
#[test]
fn lever_boss_room_return_ends_the_stored_familiar() -> Result<(), CompanionExecutionError> {
    let (old_writer, _) = writers();
    let waiting = lifecycle(&old_writer, 120_000, false, Some(&old_writer));

    let mut inside = waiting.clone();
    assert!(end_familiar_return_in_lever_boss_room(
        &mut inside,
        LeverBossRoomMembership::Inside
    )?);
    let mut ended = waiting.clone();
    ended.familiar_remaining_ms = 0;
    assert_eq!(inside, ended);
    assert!(!familiar_waits_to_return(&inside));

    let mut outside = waiting.clone();
    assert!(!end_familiar_return_in_lever_boss_room(
        &mut outside,
        LeverBossRoomMembership::Outside
    )?);
    assert_eq!(outside, waiting);

    let mut unresolved = waiting.clone();
    assert_eq!(
        end_familiar_return_in_lever_boss_room(
            &mut unresolved,
            LeverBossRoomMembership::Unresolved
        ),
        Err(CompanionExecutionError::InvalidOwnerFacts)
    );
    assert_eq!(unresolved, waiting);

    for not_waiting in [
        lifecycle(&old_writer, 120_000, true, None),
        lifecycle(&old_writer, 0, false, Some(&old_writer)),
    ] {
        for room in [
            LeverBossRoomMembership::Outside,
            LeverBossRoomMembership::Inside,
            LeverBossRoomMembership::Unresolved,
        ] {
            let mut row = not_waiting.clone();
            assert!(!end_familiar_return_in_lever_boss_room(&mut row, room)?);
            assert_eq!(row, not_waiting);
        }
    }
    Ok(())
}

/// #2199 Codex P1 4302326065 (CREATURE-AI-0 RL-19): a stored return checks the complete
/// owned-summon census against the aggregate cap of two; at the cap it keeps waiting.
#[test]
fn familiar_return_respects_the_aggregate_summon_cap() {
    assert!(familiar_return_within_summon_cap(0));
    assert!(familiar_return_within_summon_cap(1));
    assert!(!familiar_return_within_summon_cap(
        CREATUREAI0_PLAYER_SUMMONS_PER_CHARACTER_MAX
    ));
    assert!(!familiar_return_within_summon_cap(3));
}

fn knight_familiar_profile() -> Result<CompiledNativeSpell, CompanionExecutionError> {
    let document: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
    ))
    .map_err(|_| CompanionExecutionError::UnqualifiedProfile)?;
    let row = document["profiles"]
        .as_array()
        .and_then(|rows| {
            rows.iter().find(|r| {
                let behavior = &r["spell"]["execution"]["native_behavior"];
                behavior["key"] == "familiar_summon"
                    && behavior["parameters"]["vocation"] == "knight"
                    && behavior["parameters"]["login"]["lifetime_basis"] == "current_unix_time"
            })
        })
        .ok_or(CompanionExecutionError::UnqualifiedProfile)?;
    super::super::native::spell_from_bundle(
        &serde_json::json!({"spell": row["spell"]}),
        &row["dependencies"],
    )
    .map_err(|_| CompanionExecutionError::UnqualifiedProfile)
}

/// #2199 Codex P2 4302542011 (FAMILIARS0 §5, §7): a clean stored familiar returns at login
/// after Premium lapsed or the level dropped, because a return passes only the lever boss
/// room, the summon cap and placement. Without a free tile it reports the placement refusal
/// that keeps the row waiting for the in-session retry. The source Login look handling
/// (removing the vocation look without Premium) is kept, no cooldown is restarted, and a
/// login without stored time still follows the source Premium gate.
#[test]
fn stored_familiar_returns_at_login_without_fresh_cast_eligibility()
-> Result<(), CompanionExecutionError> {
    let spell = knight_familiar_profile()?;
    let p = parameters(&spell, "familiar_summon")?;
    let look = p["default_look_type"]
        .as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .ok_or(CompanionExecutionError::UnqualifiedProfile)?;
    let lapsed = FamiliarFacts {
        event: FamiliarEvent::Login,
        vocation: "knight".into(),
        premium: false,
        level: 250,
        account_at_least_god: false,
        owned_summons: 0,
        spawn_room: true,
        chosen_look: 0,
        has_vocation_look: true,
        owner_current_speed: 220,
        familiar_base_speed: 200,
        familiar_minutes: 30,
        vip: false,
        vip_reduction_minutes: 0,
        cooldown_rate: 1.0,
        now_unix: 10_000,
        saved_expiry_unix: 10_090,
        last_logout_unix: 10_000,
        owner_present: true,
        creature_present: false,
        creature_name: p["creature_name"]
            .as_str()
            .ok_or(CompanionExecutionError::UnqualifiedProfile)?
            .to_owned(),
        matching_summon_ids: Vec::new(),
        dx: 0,
        dy: 0,
        dz: 0,
        owner_tile_is_teleport: false,
    };
    let creates = |actions: &[FamiliarAction]| {
        actions
            .iter()
            .filter(|a| matches!(a, FamiliarAction::Create { .. }))
            .count()
    };

    // The source Login alone drops the creation without Premium: the stuck row.
    let source = plan_familiar_event_actions(&spell, &lapsed, false)?;
    assert_eq!(creates(&source), 0);
    assert!(source.contains(&FamiliarAction::RemoveVocationLook(look)));

    // A stored return comes back with its stored lifetime and keeps the source looks.
    let restored = plan_familiar_event_actions(&spell, &lapsed, true)?;
    assert_eq!(creates(&restored), 1);
    assert!(restored.iter().any(|a| matches!(
        a,
        FamiliarAction::Create { name, look: created, owned: true, .. }
            if *name == lapsed.creature_name && *created == look
    )));
    assert!(restored.contains(&FamiliarAction::StoreExpiry(10_090)));
    assert!(
        restored
            .iter()
            .any(|a| matches!(a, FamiliarAction::ScheduleExpiry { .. }))
    );
    assert!(restored.contains(&FamiliarAction::RemoveVocationLook(look)));
    assert!(!restored.iter().any(|a| matches!(
        a,
        FamiliarAction::SelectLook(_)
            | FamiliarAction::GrantVocationLook(_)
            | FamiliarAction::Cooldown { .. }
    )));
    assert_eq!(
        restored
            .iter()
            .filter(|a| **a == FamiliarAction::RegisterAdvance)
            .count(),
        1
    );

    // Below the source login level the stored familiar returns the same way.
    let low_level = FamiliarFacts {
        premium: true,
        level: 120,
        ..lapsed.clone()
    };
    assert_eq!(
        creates(&plan_familiar_event_actions(&spell, &low_level, false)?),
        0
    );
    assert_eq!(
        creates(&plan_familiar_event_actions(&spell, &low_level, true)?),
        1
    );

    // No free tile: the placement refusal keeps the row waiting for the in-session retry.
    let no_tile = FamiliarFacts {
        spawn_room: false,
        ..lapsed.clone()
    };
    let waiting = plan_familiar_event_actions(&spell, &no_tile, true)?;
    assert_eq!(creates(&waiting), 0);
    assert!(login_left_familiar_waiting(None, &waiting));

    // An eligible owner's plan is exactly the source Login's.
    let eligible = FamiliarFacts {
        premium: true,
        has_vocation_look: false,
        ..lapsed.clone()
    };
    assert_eq!(
        plan_familiar_event_actions(&spell, &eligible, true)?,
        plan_familiar_event_actions(&spell, &eligible, false)?
    );
    Ok(())
}
