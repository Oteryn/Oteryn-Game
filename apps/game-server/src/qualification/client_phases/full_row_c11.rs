//! C11 mainland qualification: buy the Knight promotion, verify its live benefit, and
//! commit a Wheel allocation that activates Battle Instinct. Dawnport is out of scope.

use super::full_row_c01::admission_error;
use super::full_row_p20::talk;
use super::*;
use oteryn_dev_client::DevClientSession;
use std::collections::BTreeSet;

const KNIGHT_PREPARATION: character_prep::Seeded = character_prep::Seeded {
    profile: "c11-knight",
    reason: "C11 tests Battle Instinct activation after committing a Knight Wheel allocation.",
};

const ACTIONS: &[&str] = &["battle-instinct-snapshot", "malformed-refusal"];

struct Row {
    passed: BTreeSet<&'static str>,
    failures: Vec<String>,
    started: std::time::Instant,
}
impl Row {
    fn new() -> Self {
        Self {
            passed: BTreeSet::new(),
            failures: Vec::new(),
            started: std::time::Instant::now(),
        }
    }
    fn pass(&mut self, action: &'static str, detail: impl std::fmt::Debug) {
        if ACTIONS.contains(&action) && self.passed.insert(action) {
            evidence(&format!("C11_CHECK {action} PASS observed={detail:?}"));
        } else {
            verdict::record_assertion(action);
            self.failures
                .push(format!("step={action} observed=duplicate-or-unknown"));
        }
    }
    fn fail(&mut self, action: &'static str, error: impl std::fmt::Display) {
        verdict::record_assertion(action);
        let error = format!("step={action} observed={error}");
        evidence(&format!("C11_CHECK {action} FAIL {error}"));
        self.failures.push(error);
    }
    fn finish(mut self, terminal: Option<String>) {
        if let Some(detail) = terminal {
            evidence(&format!("C11_GAP {detail}"));
            self.failures.push(detail);
        }
        for action in ACTIONS {
            if !self.passed.contains(action) {
                evidence(&gap(action, &self.failures));
            }
        }
        let pass = self.failures.is_empty() && ACTIONS.iter().all(|a| self.passed.contains(a));
        evidence(&format!(
            "e2e_all scenario=C11.full status={} seconds={:.3} evidence={}",
            if pass { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64(),
            if pass {
                "observed-knight-promotion-and-battle-instinct".into()
            } else {
                self.failures.join("_").replace(' ', "-")
            }
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    evidence("e2e_all scenario=C11.dawnport status=OUT_OF_SCOPE seconds=0 evidence=owner-excluded-Dawnport-Manop-shrine-actions-7101-7104");
    character_prep::declare_seeded("C11", "platform-ledger-premium-knight,wheel-fee-gold", "Knight promotion and Wheel mechanics");
    let mut row = Row::new();
    let terminal = match verdict::indexed_timeout(
        Duration::from_secs(300), verdict::observe(async {
            let result = knight_journey(&mut row, clients).await;
            if let Err(error) = &result {
                verdict::record_error(error.as_ref());
            }
            result
        }),
    ).await {
        Ok(Ok(())) => None,
        Ok(Err(error)) => {
            let detail = error.to_string();
            Some(if detail.starts_with("step=") { detail } else { format!("step=journey observed={detail}") })
        }
        Err(_) => Some("step=journey observed=timeout".into()),
    };
    row.finish(terminal);
}

async fn knight_journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    use oteryn_protocol_oteryn::{battle_instinct_status as wire, wheel};
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let dedicated = character_prep::dedicated_named_character(
        clients,
        "NODE_BOOT_C11_KNIGHT_ACCOUNT",
        Some("Wheel Knight"),
    )
    .await?;
    let clients = &dedicated.clients(clients);
    let mut client = KNIGHT_PREPARATION.connect(clients, 0xca, 0).await?;
    character_prep::grant_item(&client, 3043, 2).await?;
    let prior = client.join_snapshot().game_session_id;
    let (result, detail, _, _) = logout_retrying_busy(&mut client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("Knight preparation logout refused: {detail}").into());
    }
    drop(client);
    let mut client = connect_character(clients, 0xca, 0)
        .await
        .map_err(|error| admission_error("knight-preparation", error))?;
    if client.join_snapshot().game_session_id == prior {
        return Err("Knight preparation reused its original session".into());
    }
    open_castle_entrance(&mut client, world).await?;
    let king = metadata.npcs.npcs.iter()
        .find(|npc| npc.key == "oteryn:npc.king_tibianus")
        .and_then(|npc| npc.placements.first())
        .ok_or("King Tibianus is absent from the served NPC placements")?;
    let king = walk_to_npc(
        &mut client,
        world,
        TilePos { x: king.x, y: king.y, floor: king.floor },
        "King Tibianus",
    ).await?;
    if talk(&mut client, king, "hi").await?.is_empty() {
        return Err("King Tibianus returned an empty promotion greeting".into());
    }
    let unpromoted = promotion_forecast(&mut client).await?;
    if unpromoted.promotion_benefit || unpromoted.level != 625 {
        return Err(format!("expected an unpromoted level-625 Knight before purchase: {unpromoted:?}").into());
    }
    let offer = talk(&mut client, king, "promotion").await?;
    if offer != ["Do you want to be promoted for 20000 gold coins?"] {
        return Err(format!("King Tibianus promotion offer: {offer:?}").into());
    }
    let confirmation = talk(&mut client, king, "yes").await?;
    if confirmation != ["You are now promoted."] {
        return Err(format!("King Tibianus promotion confirmation: {confirmation:?}").into());
    }

    let (vocation, level) = admitted_build(&mut client).await?;
    if !(is_vocation(vocation, "knight") || is_vocation(vocation, "elite_knight")) || level != 625 {
        return Err(format!("Battle Instinct requires the prepared level-625 Knight; observed vocation={vocation} level={level}").into());
    }
    let promoted = promotion_forecast(&mut client).await?;
    if !promoted.promotion_benefit || promoted.level != level {
        return Err(format!("Battle Instinct requires current promotion benefits: {unpromoted:?} -> {promoted:?}").into());
    }
    evidence(&format!("C11_PROMOTION_BENEFIT level={level} before=false after=true"));
    let before = battle_status(&mut client).await?;
    if before.active || before.melee != 0 || before.shielding != 0 {
        return Err(format!("Battle Instinct active before allocation: {before:?}").into());
    }
    let request = wheel::WheelAllocationRequest {
        action: 0, occurrence: [0; 16], expected_revision: 0, points: [0; 36], vessels: None,
    };
    let initial = wheel_command(&mut client, &request).await?;
    if initial.refused || initial.points != [0; 36] {
        return Err(format!("expected clean Knight Wheel: {initial:?}").into());
    }
    // Pinned Knight route from core state slot 15 to Battle Instinct in state slot 1.
    let mut points = [0; 36];
    for (slot, capacity) in [(15, 50), (9, 75), (8, 100), (7, 150), (1, 200)] {
        points[slot - 1] = capacity;
    }
    let saved = wheel_command(&mut client, &wheel::WheelAllocationRequest {
        action: 1, occurrence: v7(0xcb, 1), expected_revision: initial.revision, points, vessels: None,
    }).await?;
    if saved.refused || Some(saved.revision) != initial.revision.checked_add(1) || saved.points != points {
        return Err(format!("Knight Wheel allocation did not commit: initial={initial:?} saved={saved:?}").into());
    }
    let persisted = wheel_command(&mut client, &request).await?;
    if persisted != saved {
        return Err(format!("Knight Wheel readback differs from the committed allocation: saved={saved:?} readback={persisted:?}").into());
    }
    let active = battle_status(&mut client).await?;
    if !active.active || active.melee != 0 || active.shielding != 0 {
        return Err(format!("Battle Instinct did not activate with zero adjacent-monster bonuses: {active:?}").into());
    }
    row.pass("battle-instinct-snapshot", (saved, before, active));
    match client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_BATTLE_INSTINCT_STATUS_V1,
            wire::COMMAND_TYPE_BATTLE_INSTINCT_STATUS_QUERY,
            &[0],
        )
        .await
    {
        Ok(reply) if reply.status == CommandStatus::Rejected => {
            row.pass("malformed-refusal", reply.status)
        }
        Ok(reply) => row.fail("malformed-refusal", format!("reply={reply:?}")),
        Err(error) => { verdict::record_error(verdict::IndexedError::as_error(&error)); row.fail("malformed-refusal", error) },
    }
    logout(&mut client, world).await
}

async fn open_castle_entrance(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    use crate::map::door::AuthoredClosedDoor;
    use oteryn_protocol_oteryn::world_object::UseDisposition;

    let entrance = TilePos { x: 32314, y: 32180, floor: -7 };
    let outside = TilePos { y: 32181, ..entrance };
    let doors = world.facts().doors();
    let key = doors
        .keys()
        .find(|&key| {
            crate::map::view::placement_of(key).is_some_and(|(at, _)| at == entrance)
                && doors.authored_closed(key) == Some(AuthoredClosedDoor::KeyUnlocked)
        })
        .ok_or("King Tibianus route requires the authored unlocked castle door")?;
    let start = tile(client.world_spatial().actor_position)?;
    let path = route(world, start, |at| at == outside)
        .ok_or_else(|| format!("no player walk route from {start:?} to castle door {entrance:?}"))?;
    walk(client, world, &path).await?;

    use oteryn_protocol_oteryn::world_map::MapOrigin;
    let (_, ordinal) = crate::map::view::placement_of(key).ok_or("invalid castle door placement")?;
    let closed = world.base().tile(entrance.x, entrance.y, entrance.floor)
        .and_then(|tile| tile.appearance(ordinal))
        .ok_or("castle door lacks an authored appearance")?;
    let open = crate::map::door::open_of(u32::from(closed)).ok_or("castle door lacks an open form")?;
    let basis = spatial_basis::current(client)?;
    let map = client.session_mut().world_map_snapshot().ok_or("castle door requires a served map view")?;
    let item = map.tiles.iter()
        .find(|tile| tile.position == basis.position(entrance))
        .and_then(|tile| tile.items.iter().find(|item| {
            matches!(item.origin, MapOrigin::BaseOrdinal { ordinal: seen, .. } if seen == ordinal)
        }))
        .ok_or("castle door missing from served tile at its authored stack ordinal")?;
    let MapOrigin::BaseOrdinal { object_revision, .. } = item.origin else {
        return Err("castle door map origin changed".into());
    };
    if item.appearance_id == closed {
        let mut placement = map.header.bundle_digest.to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let outcome = client.use_object(&placement, object_revision).await?;
        if outcome.status != CommandStatus::Accepted || outcome.disposition != UseDisposition::Committed {
            return Err(format!(
                "castle door USE refused at {entrance:?} ordinal={ordinal} appearance={closed} revision={object_revision}: {outcome:?}"
            ).into());
        }
    } else if u32::from(item.appearance_id) != open {
        return Err(format!("unexpected castle door appearance at {entrance:?}: {item:?}").into());
    }
    evidence(&format!(
        "C11_CASTLE_DOOR position={entrance:?} ordinal={ordinal} observed_appearance={} revision={object_revision}",
        item.appearance_id
    ));
    // The immutable route model still has the authored closed pose. Check this
    // single step against the door cell before resuming normal floor routing.
    walk_with_landings(client, &[StepDirection::North], move |at, direction| {
        (at == outside && direction == StepDirection::North).then_some(entrance)
    })
    .await
}

async fn promotion_forecast(client: &mut DevClientSession) -> TestResult<oteryn_protocol_oteryn::cyclopedia_character::protection::Forecast> {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;
    let reply = client.session_mut().gated_command(
        wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
        wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
        &[wire::protection::SELECTOR],
    ).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("Cyclopedia promotion forecast refused: {reply:?}").into());
    }
    wire::protection::Forecast::decode(&reply.payload)
        .ok_or_else(|| "invalid Cyclopedia promotion forecast payload".into())
}

async fn battle_status(client: &mut DevClientSession) -> TestResult<oteryn_protocol_oteryn::battle_instinct_status::BattleInstinctStatus> {
    use oteryn_protocol_oteryn::battle_instinct_status as wire;
    let reply = client.session_mut().gated_command(wire::CAPABILITY_BATTLE_INSTINCT_STATUS_V1, wire::COMMAND_TYPE_BATTLE_INSTINCT_STATUS_QUERY, &[]).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("Battle Instinct query refused: {reply:?}").into());
    }
    wire::BattleInstinctStatus::decode(&reply.payload).ok_or_else(|| "invalid Battle Instinct payload".into())
}

async fn wheel_command(client: &mut DevClientSession, request: &oteryn_protocol_oteryn::wheel::WheelAllocationRequest) -> TestResult<oteryn_protocol_oteryn::wheel::WheelAllocationReply> {
    use oteryn_protocol_oteryn::wheel;
    let payload = wheel::encode_wheel_request(request).map_err(|error| format!("Wheel request: {error:?}"))?;
    let reply = client.session_mut().gated_command(wheel::CAPABILITY_WHEEL_ALLOCATION_V1, wheel::COMMAND_TYPE_WHEEL_ALLOCATION, &payload).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("Wheel command refused: {reply:?}").into());
    }
    wheel::decode_wheel_reply(&reply.payload).map_err(|error| format!("Wheel reply: {error:?}").into())
}

async fn walk_to_npc(
    client: &mut DevClientSession,
    world: &BundleWorld,
    placement: TilePos,
    name: &str,
) -> TestResult<oteryn_dev_client::EntityRef> {
    let start = tile(client.world_spatial().actor_position)?;
    // Both NPCs can wander two tiles. Stay within two tiles of their placement
    // before resolving the live actor, and require an actual movement step.
    let path = route(world, start, |at| {
        at != start && at.floor == placement.floor
            && at.x.abs_diff(placement.x).max(at.y.abs_diff(placement.y)) <= 2
    })
    .ok_or_else(|| format!("no player walk route from {start:?} to {name} at {placement:?}"))?;
    walk(client, world, &path).await?;
    let basis = spatial_basis::current(client)?;
    let origin = basis.position(placement);
    client.world_entities().and_then(|entities| {
        entities.iter().find(|npc| {
            npc.kind == oteryn_dev_client::EntityKind::Npc
                && npc.position.floor == origin.floor
                && npc.position.x.abs_diff(origin.x).max(npc.position.y.abs_diff(origin.y)) <= 2
                && npc.display_name.is_some_and(|display| {
                    display.as_str().is_ok_and(|observed| observed.eq_ignore_ascii_case(name))
                })
        })
    }).map(|npc| npc.entity).ok_or_else(|| format!("{name} not visible near {placement:?}").into())
}

async fn logout(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    use oteryn_protocol_oteryn::logout::LogoutResult;
    let safe = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| tile.protection_zone())
    }).ok_or("step=fresh-admission-monk observed=no-reachable-protection-zone need=safe-combat-logout")?;
    walk(client, world, &safe).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    loop {
        let reply = logout_retrying_busy(client).await?.0;
        match reply {
            Some(LogoutResult::Accepted) => return Ok(()),
            Some(LogoutResult::InFight { .. }) if tokio::time::Instant::now() < deadline => {
                client.service_liveness(Duration::from_millis(250)).await?;
                client.take_events();
            }
            _ => return Err(format!("C11 logout refused: {reply:?}").into()),
        }
    }
}

fn gap(action: &str, failures: &[String]) -> String {
    let prefix = format!("step={action} observed=");
    failures.iter().find(|failure| failure.starts_with(&prefix))
        .map_or_else(|| format!("C11_GAP step={action} unproven"), |failure| format!("C11_GAP {failure}"))
}

async fn admitted_build(client: &mut DevClientSession) -> TestResult<(u8, u32)> {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[wire::CharacterTab::GeneralStats as u8, 0, 0, 50],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("GeneralStats refused: {reply:?}").into());
    }
    let vocation = general_stats::validate(&reply.payload, general_stats::selected(client))?
        .ok_or("GeneralStats missing vocation")?;
    Ok((vocation, u32::from_le_bytes(reply.payload[9..13].try_into()?)))
}

fn is_vocation(vocation: u8, key: &str) -> bool {
    oteryn_protocol_oteryn::cyclopedia_character::vocation_id(key) == Some(vocation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gap_reports_the_observed_blocker_for_its_action() {
        let failures = ["step=battle-instinct-snapshot observed=Wheel refused".to_owned()];
        assert_eq!(
            gap("battle-instinct-snapshot", &failures),
            "C11_GAP step=battle-instinct-snapshot observed=Wheel refused"
        );
        assert_eq!(gap("malformed-refusal", &failures), "C11_GAP step=malformed-refusal unproven");
    }
}
