//! A real native early-exit gesture must retire the actual private fight.
use super::*;
use crate::gameplay_transport::connection::AdmittedSession;
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

pub(super) async fn scenario<'a, 'f, 's>(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'a, 'f, 's>,
    entry: &capture::NativeCapture,
    mut player: AdmittedSession,
    registered: crate::gameplay_transport::activity_runtime::RegisteredRuntime<'a>,
) -> TestResult {
    let started = std::time::Instant::now();
    owner
        .drive_shared_soulpits(owner.owner_now().get(), false)
        .await
        .map_err(|_| "prepare real exit-case wave")?;
    let mut instance = owner.runtime_view(&registered);
    let fence = player.item_fence.ok_or("actual preparing Instance fence")?;
    let bound = h
        .root
        .read_soulpit_scope_binding(
            owner.character,
            owner.holder,
            fence.runtime_scope,
            fence.scope_ownership_generation,
        )
        .await?
        .ok_or("actual early-exit handoff")?;
    let all_at_first_exit_cell = {
        let mut runtime = registered.runtime.lock().await;
        let (carrier, continuity, _) = runtime.soulpit_kernel_parts().map_err(debug)?;
        let plan = carrier
            .soulpit_wave_plan(continuity, bound.handoff_id, 1)
            .map_err(debug)?
            .ok_or("actual retained first-wave plan")?;
        let points = plan.positions.iter().flatten().collect::<Vec<_>>();
        assert!(
            !points.is_empty(),
            "real Source arena has a selected creation cell"
        );
        points.iter().all(|at| (at.x, at.y) == (32374, 31152))
    };
    // Walk before creation. If every retained attempt selected the first exit
    // neighbour, use the second real neighbour: no spawn selection is redrawn.
    // Thus at least one attempt remains unoccupied for the cleanup proof.
    let exit_x = if all_at_first_exit_cell { 32375 } else { 32374 };
    let actor = player.runtime_actor.ok_or("actual exiting actor")?;
    let mut route = vec![StepDirection::East; usize::try_from(exit_x - 32373)?];
    route.push(StepDirection::South);
    for direction in route {
        instance.clock_origin = instance
            .clock_origin
            .checked_sub(std::time::Duration::from_millis(500))
            .ok_or("actual exit movement clock")?;
        let (step, _) = instance
            .paced_step(actor, player.game_session_id, direction)
            .await;
        assert_eq!(
            step.disposition,
            StepDisposition::Moved,
            "walk to the real USE-only exit"
        );
    }
    owner.clock_origin = instance
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(4))
        .ok_or("actual Source spawn deadline")?;
    owner
        .drive_shared_soulpits(owner.owner_now().get(), false)
        .await
        .map_err(|_| "spawn actual early-exit opponents")?;
    let before = registered
        .runtime
        .lock()
        .await
        .positioned_actor_census()
        .map_err(debug)?
        .iter()
        .filter(|(_, _, session)| session.is_none())
        .count();
    assert!(
        before >= 1,
        "early exit must have real living opponents to retire"
    );
    assert!(
        h.root
            .read_soulpit_terminal_cleanup(
                owner.character,
                owner.holder,
                fence.runtime_scope,
                fence.scope_ownership_generation,
            )
            .await?
            .is_none(),
        "a living encounter is not terminal cleanup evidence"
    );
    instance = owner.runtime_view(&registered);
    let map = instance.bundle_map().ok_or("actual private source map")?;
    let pos = crate::map::overlay::TilePos {
        x: 32375,
        y: 31153,
        floor: -8,
    };
    let tile = crate::map::view::compose_tile(&map.overlay, &map.facts, pos)
        .map_err(debug)?
        .ok_or("actual source exit tile")?;
    let target = tile.entries.last().ok_or("actual source exit object")?;
    let crate::map::view::EntrySource::Base { placement_key, .. } = target.source else {
        return Err("source exit must retain pinned base placement".into());
    };
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&placement_key.to_be_bytes());
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::WorldObject(WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(placement_key),
        }),
        use_with: None,
        use_with_position: None,
        use_with_map_item: None,
    })
    .map_err(debug)?;
    let admission = player;
    let mut frames = Vec::new();
    let command = player.continuity.next_command_id;
    let (status, result, scope, _) = super::super::guild_wire::exchange_capture(
        &instance,
        &mut player,
        command,
        oteryn_protocol_oteryn::world_object::COMMAND_TYPE_USE_INTENT,
        |_| Ok(payload.clone()),
        true,
        &mut frames,
    )
    .await?;
    assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
        UseDisposition::Committed
    );
    assert!(matches!(
        scope.ok_or("actual early-return barrier")?.scope,
        oteryn_protocol_oteryn::activity_scope::ActivityScope::Channel { .. }
    ));
    assert_ne!(player.game_session_id, admission.game_session_id);
    let fence = admission.item_fence.ok_or("actual former Instance fence")?;
    assert!(
        h.root
            .read_soulpit_scope_binding(
                owner.character,
                owner.holder,
                fence.runtime_scope,
                fence.scope_ownership_generation
            )
            .await?
            .is_none(),
        "all returned members leave no live Soulpit binding"
    );
    let cleanup = h
        .root
        .read_soulpit_terminal_cleanup(
            owner.character,
            owner.holder,
            fence.runtime_scope,
            fence.scope_ownership_generation,
        )
        .await?
        .ok_or("exact current terminal cleanup evidence")?;
    assert_eq!(cleanup.handoff_id(), bound.handoff_id);
    assert!(cleanup.matches(fence.runtime_scope, fence.scope_ownership_generation));
    let stale = crate::foundation::ScopeOwnershipGeneration::new(
        fence
            .scope_ownership_generation
            .get()
            .checked_add(1)
            .ok_or("next cleanup generation")?,
    )
    .map_err(debug)?;
    assert!(!cleanup.matches(fence.runtime_scope, stale));
    assert!(
        matches!(
            h.root
                .read_soulpit_terminal_cleanup(
                    owner.character,
                    owner.holder,
                    fence.runtime_scope,
                    stale,
                )
                .await,
            Err(crate::durability::soulpit::SoulpitWriteError::AuthorityRejected)
        ),
        "a stale ownership generation cannot read cleanup for this Instance"
    );
    owner
        .drive_shared_soulpits(owner.owner_now().get(), false)
        .await
        .map_err(|_| "actual terminal Source cleanup pass")?;
    let remaining = registered
        .runtime
        .lock()
        .await
        .positioned_actor_census()
        .map_err(debug)?;
    assert!(
        remaining.is_empty(),
        "a real last native exit retires private monsters instead of stranding the Instance"
    );
    assert!(
        registered
            .runtime
            .lock()
            .await
            .activity_retirement_empty()
            .map_err(debug)?
    );
    let private = instance.bundle_map().ok_or("actual reset private map")?;
    let glyph = crate::map::view::compose_tile(
        &private.overlay,
        &private.facts,
        crate::map::overlay::TilePos {
            x: 32375,
            y: 31157,
            floor: -8,
        },
    )
    .map_err(debug)?
    .ok_or("actual reset private obelisk")?;
    assert!(
        glyph
            .entries
            .iter()
            .any(|entry| entry.facts.appearance_id == 47367)
            && glyph
                .entries
                .iter()
                .all(|entry| entry.facts.appearance_id != 47379),
        "last native exit resets its independently owned Source glyph"
    );
    let mastery: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_animus_masteries WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(mastery, 0, "leaving a living fight grants no Animus reward");
    let xp: i64 = sqlx::query_scalar(
        "SELECT COALESCE(sum(base_xp),0)::bigint FROM game_party_kill_xp_receipts",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(xp, 0, "retiring living opponents is not a rewarded kill");
    if let Ok(path) = std::env::var("SRX3_SOULPIT_EXIT_CAPTURE_PATH") {
        if !std::path::Path::new(&path).starts_with("/tmp") {
            return Err("actual native capture must target /tmp".into());
        }
        let value = serde_json::json!({"schema":"OTERYN_NATIVE_SOULPIT_EXIT_REPLAY/v1",
            "character_id":hex(fence.character_id.as_bytes()),"entry":epoch(entry.source,&entry.entry_command,&entry.entry_frames)?,
            "exit":epoch(admission,&payload,&frames)?});
        std::fs::write(path, serde_json::to_vec_pretty(&value)?)?;
    }
    eprintln!(
        "SRX3_SOULPIT NATIVE_EARLY_EXIT_COMPLETED retired={before} duration={:?}",
        started.elapsed()
    );
    Ok(())
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn epoch(
    admission: AdmittedSession,
    payload: &[u8],
    frames: &[Vec<u8>],
) -> TestResult<serde_json::Value> {
    let boundary = frames
        .iter()
        .position(|raw| {
            oteryn_protocol_oteryn::decode_wire_envelope(raw).is_ok_and(|e| {
                e.message_type() == oteryn_protocol_oteryn::MessageType::SnapshotCommit
            })
        })
        .ok_or("real native initial boundary")?
        + 1;
    Ok(
        serde_json::json!({"admission":{"game_session_id":hex(admission.game_session_id.as_bytes()),
        "world_id":hex(admission.world_id.as_bytes()),"channel_id":hex(admission.channel_id.as_bytes()),
        "connection_generation":admission.continuity.connection_generation,"current_server_sequence":admission.continuity.server_sequence,
        "next_command_id":admission.continuity.next_command_id,"schema_revision":crate::gameplay_transport::connection::SERVER_SCHEMA_REVISION,
        "selected_capabilities":admission.continuity.selected_capabilities.as_slice()},
        "initial_frames":frames[..boundary].iter().map(|raw|hex(raw)).collect::<Vec<_>>(),
        "command":{"kind":oteryn_protocol_oteryn::world_object::COMMAND_TYPE_USE_INTENT,"payload":hex(payload)},
        "subsequent_frames":frames[boundary..].iter().map(|raw|hex(raw)).collect::<Vec<_>>()}),
    )
}
