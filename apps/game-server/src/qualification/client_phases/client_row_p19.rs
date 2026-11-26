//! Verify the first visible Forge binding for each exact fixture creature.
use super::*;
use oteryn_dev_client::SessionEvent;
use oteryn_dev_client::native_session::AppearanceUpdate;
use oteryn_protocol_oteryn::appearance::CAPABILITY_FORGE_CREATURE_V1;

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    let result = verdict::indexed_timeout(
        Duration::from_secs(180),
        verdict::observe(prove(clients, world)),
    )
    .await;
    match result {
        Ok(Ok(())) => evidence(
            "e2e_all scenario=P19.client status=PASS evidence=first-appearance-influenced-3-and-fiendish-15",
        ),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P19.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P19.client status=FAIL evidence=timeout"),
    }
}

async fn prove(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult {
    let capture = required("E2E_ALL_CAPTURE_DIR")?;
    let mut client = connect_character(clients, 0x19, 0).await?;
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_FORGE_CREATURE_V1)
    {
        return Err("P19 Forge appearance capability absent".into());
    }
    use crate::qualification_wild_spawn::adjacent_cells;
    let cells = |at: TilePos| {
        adjacent_cells(crate::foundation::MovementLocalPosition {
            x: i32::from(at.x),
            y: i32::from(at.y),
            floor: -i16::from(at.floor),
        })
        .into_iter()
        .filter(|candidate| {
            let (Ok(x), Ok(y)) = (u16::try_from(candidate.x), u16::try_from(candidate.y)) else {
                return false;
            };
            let target = TilePos {
                x,
                y,
                floor: at.floor,
            };
            world
                .base()
                .tile(x, y, at.floor)
                .is_some_and(|tile| tile.walkable() && !tile.protection_zone())
                && world.enterable(target)
                && !world.floors().contains(target)
                && world.floors().teleport(target).is_none()
        })
        .take(2)
        .collect::<Vec<_>>()
    };
    let start = tile(client.world_spatial().actor_position)?;
    let path = route(world, start, |at| cells(at).len() == 2)
        .ok_or("P19 no approach with two open spawn tiles")?;
    for direction in path {
        let origin = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, origin, direction).ok_or("P19 route has no landing")?;
        let movement = client.step(direction).await?;
        let actual = tile(client.world_spatial().actor_position)?;
        if !confirmed_step(
            movement.disposition,
            movement.world_spatial_delta.is_some(),
            actual,
            expected,
        ) {
            return Err(format!(
                "P19 first movement {direction:?} from {origin:?}: disposition={:?} expected={expected:?} actual={actual:?}",
                movement.disposition,
            ).into());
        }
        if movement.step_wait_ms != 0 {
            let wait = Duration::from_millis(u64::from(movement.step_wait_ms));
            let until = tokio::time::Instant::now() + wait;
            while let Some(remaining) = until.checked_duration_since(tokio::time::Instant::now()) {
                client.service_liveness(remaining).await?;
            }
        }
    }
    let at = client.world_spatial().actor_position;
    let positions = cells(tile(at)?);
    if positions.len() != 2 {
        return Err("P19 reached an approach without two open spawn tiles".into());
    }
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let refused = request_fixture_control(
        session.clone(),
        "--spawn-forge",
        format!(
            "oteryn:creature.druid_familiar {} {} {} fiendish",
            positions[0].x, positions[0].y, positions[0].floor
        ),
    )
    .await;
    if refused.as_ref().err().map(String::as_str) != Some("ineligible_forge_creature") {
        return Err(format!("P19 expected ineligible_forge_creature, got {refused:?}").into());
    }
    let mut observed = Vec::new();
    for ((classification, stacks, folder), position) in
        [("influenced 3", 3, ""), ("fiendish", 15, "fiendish")]
            .into_iter()
            .zip(positions)
    {
        let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
        client.take_events();
        let reply = request_fixture_control(
            session.clone(),
            "--spawn-forge",
            format!(
                "oteryn:creature.rat {} {} {} {classification}",
                position.x, position.y, position.floor
            ),
        )
        .await?;
        let exact = parse_spawned_entity(&reply)?;
        let first: u64 = tokio::time::timeout(Duration::from_secs(15), async {
            let mut first_binding = None;
            loop {
                for event in client.wait_for_events(Duration::from_secs(15)).await? {
                    match event {
                        SessionEvent::Appearance(delta) => {
                            if let AppearanceUpdate::Mappings(mappings) = delta.value
                                && let Some(binding) = mappings.upserts.iter().find(|entry| entry.entity == exact)
                            {
                                if binding.forge_stacks != Some(stacks) {
                                    return Err(format!("P19 {classification} wire binding was {:?}", binding.forge_stacks).into());
                                }
                                first_binding.get_or_insert(delta.server_sequence);
                            }
                        }
                        SessionEvent::WorldSpatialEntities(delta)
                            if delta.value.enter.iter().any(|entry| entry.entity == exact) =>
                        {
                            return TestResult::Ok(first_binding.ok_or_else(|| format!(
                                "P19 {classification} entity {exact:?} entered at sequence {} before its marker",
                                delta.server_sequence,
                            ))?);
                        }
                        _ => {}
                    }
                }
            }
        })
        .await??;
        let files = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(files) = client.session_mut().p19_capture_files(exact) {
                    return TestResult::Ok(files);
                }
                client.service_liveness(Duration::from_millis(50)).await?;
            }
        })
        .await
        .map_err(|_| format!("P19 {classification} exact creature replay capture missing"))??;
        let directory = Path::new(&capture).join(folder);
        std::fs::create_dir_all(&directory)?;
        for (name, bytes) in files {
            std::fs::write(directory.join(name), bytes)?;
        }
        observed.push((exact, stacks));
        evidence(&format!(
            "P19_FORGE entity={exact:?} forge_stacks={stacks} sequence={}",
            first
        ));
        let previous = client.join_snapshot().game_session_id;
        let reply = client.session_mut().logout().await?;
        if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err(format!("P19 logout refused: {reply:?}").into());
        }
        drop(client);
        client = connect_character(clients, 0x19, 0).await?;
        if client.join_snapshot().game_session_id == previous
            || client.world_spatial().actor_position != at
        {
            return Err("P19 fresh admission did not resume the saved observation position".into());
        }
        let appearances = client.session_mut().visible_appearances();
        for &(entity, stacks) in &observed {
            if !appearances
                .iter()
                .any(|entry| entry.entity.entity == entity && entry.forge_stacks == Some(stacks))
            {
                return Err(
                    format!("P19 fresh admission lacks {entity:?} with stacks={stacks}").into(),
                );
            }
        }
    }
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("P19 final logout refused: {reply:?}").into());
    }
    Err("BLOCKED-SERVER P19 fixture sets the Forge classification directly; native selection, live marker removal and durable reward outcomes are unproved".into())
}

fn confirmed_step(
    disposition: StepDisposition,
    has_delta: bool,
    actual: TilePos,
    expected: TilePos,
) -> bool {
    disposition == StepDisposition::Moved && has_delta && actual == expected
}

#[test]
fn first_step_refusals_and_unconfirmed_landings_fail() {
    let expected = TilePos {
        x: 1,
        y: 1,
        floor: 7,
    };
    let wrong = TilePos { x: 2, ..expected };
    for refused in [
        StepDisposition::Blocked,
        StepDisposition::Rejected,
        StepDisposition::TooEarly,
    ] {
        assert!(!confirmed_step(refused, true, expected, expected));
    }
    assert!(!confirmed_step(
        StepDisposition::Moved,
        false,
        expected,
        expected
    ));
    assert!(!confirmed_step(
        StepDisposition::Moved,
        true,
        wrong,
        expected
    ));
    assert!(confirmed_step(
        StepDisposition::Moved,
        true,
        expected,
        expected
    ));
}
