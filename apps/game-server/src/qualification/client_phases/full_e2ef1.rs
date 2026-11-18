// E2EF1: native movement commands against node_boot's production-loaded world.
// Mount as the provisioning phase: run BEFORE its control-socket fixtures.
use super::*;
#[path = "provision.rs"]
mod original;

pub(super) async fn e2ef1_logout_for_return(
    client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult {
    e2ef1::logout(client).await
}
pub(in crate::qualification) fn item_index()
-> TestResult<crate::content::item_ref::ItemDefinitionIndex> {
    original::item_index()
}
pub(super) fn item_reference(
    index: &crate::content::item_ref::ItemDefinitionIndex,
    id: u32,
) -> TestResult<std::num::NonZeroU32> {
    original::item_reference(index, id)
}
/// The fixture provisioning alone, without the E2EF1 movement journey; for
/// groups (creature sweep) that provision a character more than once.
pub(super) async fn provision_fixture(
    client: oteryn_dev_client::DevClientSession,
    clients: &SeamClients<'_>,
    index: usize,
    expected_level: u32,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    original::provision_at_level(client, clients, index, expected_level).await
}
pub(super) async fn provision(
    mut client: oteryn_dev_client::DevClientSession,
    clients: &SeamClients<'_>,
    index: usize,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    if index != 0 {
        return Err("E2EF1 must precede the first profile provisioning".into());
    }
    e2ef1::logout(&mut client).await?;
    drop(client);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let movement = connect_world(clients, 0x90).await?;
    let mut movement = e2ef1::run(movement, clients, world).await?;
    e2ef1::logout(&mut movement).await?;
    drop(movement);
    let client = connect_character(clients, 0x91, index).await?;
    original::provision(client, clients, index).await
}
#[derive(Debug)]
pub(super) struct MovementReturnFailure(Box<dyn std::error::Error>);

impl std::fmt::Display for MovementReturnFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "movement row could not restore its origin: {}", self.0)
    }
}

impl std::error::Error for MovementReturnFailure {}

pub(super) mod e2ef1 {
    use super::*;
    use oteryn_dev_client::{DevClientSession, EntityDetail, EntityRef};
    use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
    use std::collections::BTreeSet;

    const ROWS: [(&str, &[&str]); 6] = [
        (
            "M01",
            &[
                "paced-step",
                "autowalk",
                "stop",
                "turn",
                "follow",
                "relogin",
            ],
        ),
        ("M02", &["far-use-retry", "far-move-retry", "relogin"]),
        (
            "M03",
            &["stairs", "teleport", "aggressive-exhaust", "relogin"],
        ),
        (
            "M04",
            &["collision", "push-player", "push-creature", "relogin"],
        ),
        (
            "M05",
            &["basic-door", "unlocked-key-door", "locked-refusal"],
        ),
        ("M07", &["carried-rope", "rope-ascent", "relogin"]),
    ];
    const DIRS: [StepDirection; 4] = [
        StepDirection::North,
        StepDirection::East,
        StepDirection::South,
        StepDirection::West,
    ];

    // Rows the journey never finished: the row in progress carries the abort
    // cause, the rows after it are blocked by it.
    #[derive(Default)]
    struct Verdicts {
        done: BTreeSet<&'static str>,
        current: Option<&'static str>,
        aborted: Option<verdict::Cause>,
        action_failure: Option<(String, verdict::Cause)>,
    }
    impl Drop for Verdicts {
        fn drop(&mut self) {
            let current = self.current.unwrap_or(ROWS[0].0);
            let (root, action) = self.action_failure.take().map_or_else(
                || (format!("{current}/lifecycle"), None),
                |(root, cause)| (root, Some(cause)),
            );
            // Without a recorded error the journey future was cancelled by its caller's deadline.
            let cause = action
                .or(self.aborted.take())
                .unwrap_or(verdict::Cause::Deadline { phase: "cancelled" });
            let blocked = verdict::Cause::BlockedBy { root: root.clone() };
            for (row, _) in ROWS {
                if !self.done.contains(row) {
                    let suffix = if row == current { &cause } else { &blocked }.suffix(&root);
                    evidence(&format!(
                        "e2e_all scenario={row}.full status=FAIL seconds=0 evidence=journey-aborted:apps/game-server/src/qualification/client_phases/full_e2ef1.rs{suffix}"
                    ));
                }
            }
        }
    }
    #[derive(Default)]
    struct Checks {
        row: &'static str,
        passed: BTreeSet<&'static str>,
        missing: BTreeMap<&'static str, &'static str>,
        active: Option<&'static str>,
    }
    impl Checks {
        fn pass(&mut self, case: &'static str, detail: impl std::fmt::Debug) {
            evidence(&format!(
                "E2EF1_CHECK row={} case={case} status=PASS observed={detail:?}",
                self.row
            ));
            self.passed.insert(case);
        }
        fn missing(&mut self, case: &'static str, source: &'static str, reason: &str) {
            evidence(&format!(
                "E2EF1_MISSING row={} case={case} source={source} reason={reason}",
                self.row
            ));
            self.missing.insert(case, source);
        }
        fn action_failure(
            &self,
            required: &[&str],
            error: &(dyn std::error::Error + 'static),
        ) -> (String, verdict::Cause) {
            let action = self.active.unwrap_or_else(|| {
                required
                    .iter()
                    .find(|action| **action != "relogin" && !self.passed.contains(**action))
                    .copied()
                    .unwrap_or(if self.row == "M04" {
                        "protection-zone-return"
                    } else {
                        "action"
                    })
            });
            (format!("{}/{action}", self.row), verdict::classify(error))
        }
        fn complete(&self, required: &[&str]) -> bool {
            required.iter().all(|case| self.passed.contains(case))
        }
    }

    #[derive(Debug)]
    struct MissingAuthoredTarget(String);
    impl std::fmt::Display for MissingAuthoredTarget {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&self.0)
        }
    }
    impl std::error::Error for MissingAuthoredTarget {}
    const WORLD_TARGET_SOURCE: &str = "tools/qualification/node_boot/run.sh:461";
    const SPELL_TARGET_SOURCE: &str = "content/abilities/definitions/player-spells.json:1";

    #[derive(Clone, Copy, Debug)]
    struct Object {
        at: TilePos,
        ordinal: u8,
        appearance: u16,
        action: Option<u16>,
    }
    fn position(at: TilePos, basis: spatial_basis::Basis) -> ActorPosition {
        basis.position(at)
    }
    fn distance(a: TilePos, b: TilePos) -> u16 {
        if a.floor == b.floor {
            a.x.abs_diff(b.x).max(a.y.abs_diff(b.y))
        } else {
            u16::MAX
        }
    }
    fn target(world: &BundleWorld, object: Object, revision: u64) -> TestResult<WorldObjectTarget> {
        let key = crate::map::view::placement_key(object.at, object.ordinal)
            .ok_or("object placement key")?;
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        Ok(WorldObjectTarget {
            placement,
            expected_revision: revision,
        })
    }
    fn objects(bytes: &[u8]) -> TestResult<Vec<Object>> {
        let mut objects = Vec::new();
        oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
            for tile in sector.tiles {
                for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate()
                {
                    if let Some(appearance) =
                        crate::map::palette_appearance(&manifest.palette[item.palette as usize].key)
                    {
                        objects.push(Object {
                            at: TilePos {
                                x: tile.x,
                                y: tile.y,
                                floor: sector.floor,
                            },
                            ordinal: u8::try_from(ordinal).map_err(|_| {
                                oteryn_world_bundle::Error::Limit("movement ordinal".into())
                            })?,
                            appearance,
                            action: item.attrs.action,
                        });
                    }
                }
            }
            Ok(())
        })?;
        objects.sort_by_key(|object| (object.at, object.ordinal));
        Ok(objects)
    }
    async fn pump(client: &mut DevClientSession) -> TestResult {
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
        Ok(())
    }
    async fn until(
        client: &mut DevClientSession,
        predicate: impl Fn(&DevClientSession) -> bool,
    ) -> TestResult {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        while !predicate(client) {
            if tokio::time::Instant::now() >= deadline {
                return Err("movement observation deadline".into());
            }
            pump(client).await?;
        }
        Ok(())
    }
    async fn until_with_peer(
        client: &mut DevClientSession,
        peer: &mut DevClientSession,
        predicate: impl Fn(&DevClientSession) -> bool,
    ) -> TestResult {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        while !predicate(client) {
            if tokio::time::Instant::now() >= deadline {
                return Err("two-client movement observation deadline".into());
            }
            pump(client).await?;
            pump(peer).await?;
        }
        Ok(())
    }
    async fn stable(client: &mut DevClientSession, millis: u64) -> TestResult {
        let at = client.world_spatial().actor_position;
        let deadline = tokio::time::Instant::now() + Duration::from_millis(millis);
        while tokio::time::Instant::now() < deadline {
            pump(client).await?;
            if client.world_spatial().actor_position != at {
                return Err("stopped/refused actor moved".into());
            }
        }
        Ok(())
    }
    fn retry_target_gate(status: CommandStatus, payload: &[u8], stationary: bool) -> bool {
        status == CommandStatus::Rejected && payload.is_empty() && stationary
    }

    async fn target_command(client: &mut DevClientSession, payload: &[u8]) -> TestResult {
        use oteryn_protocol_oteryn::target_move::*;
        decode_target_move_intent(payload).map_err(|e| format!("target intent: {e:?}"))?;
        let before = client.world_spatial().actor_position;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        loop {
            if client.world_spatial().actor_position != before {
                return Err("target retry origin displaced".into());
            }

            let result = tokio::time::timeout_at(
                deadline,
                client.session_mut().gated_command(
                    CAPABILITY_TARGET_MOVE_V1,
                    COMMAND_TYPE_TARGET_MOVE_INTENT,
                    payload,
                ),
            )
            .await
            .map_err(|_| "command retry deadline expired")??;
            if tokio::time::Instant::now() >= deadline {
                return Err("command retry deadline expired".into());
            }
            if result.status == CommandStatus::Accepted && result.payload == [8, 1] {
                return Ok(());
            }
            if !retry_target_gate(
                result.status,
                &result.payload,
                client.world_spatial().actor_position == before,
            ) || tokio::time::Instant::now() >= deadline
            {
                return Err(format!("target command: {result:?}").into());
            }
            // serve/command.rs refuses before dispatch when its input permit or
            // current transfer guard is unavailable; the empty result ran no Turn.
            evidence(&format!(
                "E2EF1_TARGET retry=unavailable-input command_id={}",
                result.command_id
            ));
            tokio::time::timeout_at(deadline, pump(client))
                .await
                .map_err(|_| "command retry deadline expired")??;
        }
    }
    async fn autowalk(client: &mut DevClientSession, at: TilePos) -> TestResult {
        let basis = spatial_basis::current(client)?;
        use oteryn_dev_client::client_runtime::target_move::{WalkDisposition, send};
        let result = send(client.session_mut(), position(at, basis)).await?;
        if result.status != CommandStatus::Accepted
            || result.disposition != Some(WalkDisposition::Accepted)
        {
            return Err(format!("autowalk refused: {result:?}").into());
        }
        Ok(())
    }

    fn own(client: &DevClientSession) -> TestResult<oteryn_dev_client::WorldSpatialEntity> {
        let entities = client.world_entities().ok_or("entity view missing")?;
        entities
            .iter()
            .find(|e| &e.entity.identity == entities.own_identity())
            .copied()
            .ok_or_else(|| "own entity missing".into())
    }
    fn candidate_goals(candidates: &[Object], range: u16) -> TestResult<BTreeMap<TilePos, Object>> {
        let radius = i16::try_from(range)?;
        let mut goals = BTreeMap::new();
        for object in candidates {
            for dx in -radius..=radius {
                for dy in -radius..=radius {
                    if dx.unsigned_abs().max(dy.unsigned_abs()) != range
                        || (range == 1 && dx != 0 && dy != 0)
                    {
                        continue;
                    }
                    let Some((x, y)) = object
                        .at
                        .x
                        .checked_add_signed(dx)
                        .zip(object.at.y.checked_add_signed(dy))
                    else {
                        continue;
                    };
                    goals
                        .entry(TilePos {
                            x,
                            y,
                            floor: object.at.floor,
                        })
                        .or_insert(*object);
                }
            }
        }
        Ok(goals)
    }

    async fn approach(
        client: &mut DevClientSession,
        world: &BundleWorld,
        objects: &[Object],
        wanted: impl Fn(Object) -> bool,
        range: u16,
    ) -> TestResult<Object> {
        let from = actor_tile(client)?;
        let mut candidates: Vec<_> = objects.iter().copied().filter(|o| wanted(*o)).collect();
        candidates.sort_by_key(|object| (distance(from, object.at), object.at, object.ordinal));
        let candidate_count = candidates.len();
        let within_old_budget = candidates
            .iter()
            .filter(|o| distance(from, o.at) <= 160)
            .count();
        evidence(&format!(
            "E2EF1_APPROACH from={from:?} candidates={candidate_count} within_160={within_old_budget} nearest={:?}",
            candidates.first()
        ));
        if candidates.is_empty() {
            return Err(MissingAuthoredTarget(format!(
                "no exact authored target: from={from:?} candidates=0 within_160=0 searched=0"
            ))
            .into());
        }
        let goals = candidate_goals(&candidates, range)?;
        let path = reach_route(client, world, from, |at| goals.contains_key(&at)).await?;
        match path {
            Ok(path) => {
                let start = actor_tile(client)?;
                let landing = path.iter().try_fold(start, |at, direction| destination(world, at, *direction)).ok_or("approach landing absent")?;
                let object = goals.get(&landing).copied().ok_or("approach candidate absent")?;
                evidence(&format!("E2EF1_APPROACH phase=walk_start next_command_id={} steps={}", client.next_command_id(), path.len()));
                let started = std::time::Instant::now();
                walk(client, world, &path).await.inspect_err(|error| evidence(&format!("E2EF1_APPROACH phase=walk_error next_command_id={} elapsed_ms={} error={error}", client.next_command_id(), started.elapsed().as_millis())))?;
                Ok(object)
            }
            Err(failure) => Err(format!("no reachable exact authored target: from={from:?} candidates={candidate_count} within_160={within_old_budget} {failure}").into()),
        }
    }

    async fn m01(
        client: &mut DevClientSession,
        peer: &mut DevClientSession,
        world: &BundleWorld,
        checks: &mut Checks,
    ) -> TestResult {
        let basis = spatial_basis::current(client)?;
        if spatial_basis::current(peer)? != basis {
            return Err("peer spatial basis differs from actor".into());
        }
        let start = actor_tile(client)?;
        let path = route(world, start, |at| distance(at, start) == 4)
            .ok_or("no four-tile movement lane")?;
        let first = *path.first().ok_or("empty movement lane")?;
        let moved = client.step(first).await?;
        if moved.disposition != StepDisposition::Moved
            || moved.world_spatial_delta.is_none()
            || Some(actor_tile(client)?) != destination(world, start, first)
        {
            return Err("paced first step did not reach its predicted landing".into());
        }
        let next = *path.get(1).ok_or("short movement lane")?;
        let second_from = actor_tile(client)?;
        let paced_at = std::time::Instant::now();
        let moved = client.step(next).await?;
        if moved.disposition != StepDisposition::Moved
            || moved.world_spatial_delta.is_none()
            || Some(actor_tile(client)?) != destination(world, second_from, next)
        {
            return Err("paced second step did not reach its predicted landing".into());
        }
        if paced_at.elapsed() < Duration::from_millis(40) {
            return Err("second immediate STEP bypassed the minimum server pacing beat".into());
        }
        checks.pass("paced-step", (paced_at.elapsed(), client.world_spatial()));
        let here = actor_tile(client)?;
        let goal_path =
            route(world, here, |at| distance(at, here) == 4).ok_or("autowalk lane absent")?;
        let mut goal = here;
        for direction in goal_path {
            goal = destination(world, goal, direction).ok_or("autowalk prediction")?;
        }
        autowalk(client, goal).await?;
        until_with_peer(client, peer, |c| {
            c.world_spatial().actor_position == position(goal, basis)
        })
        .await?;
        checks.pass("autowalk", goal);
        autowalk(client, start).await?;
        target_command(client, &[16, 1]).await?;
        stable(client, 1500).await?;
        if client.world_spatial().actor_position == position(start, basis) {
            return Err("STOP did not interrupt an unfinished route".into());
        }
        checks.pass("stop", client.world_spatial());
        let at = client.world_spatial().actor_position;
        for direction in DIRS {
            let payload = oteryn_protocol_oteryn::target_move::encode_target_move_intent(
                oteryn_protocol_oteryn::target_move::TargetMoveIntent::Turn(direction),
            )
            .map_err(|error| format!("turn intent: {error:?}"))?;
            target_command(client, &payload).await?;
            until_with_peer(client, peer, |c| own(c).is_ok_and(|e| matches!(e.detail, EntityDetail::Actor { direction: seen, .. } | EntityDetail::ActorWithIcons { direction: seen, .. } if seen == direction))).await?;
            if client.world_spatial().actor_position != at {
                return Err("turn changed position".into());
            }
        }
        checks.pass("turn", own(client)?);
        let peer_at = actor_tile(peer)?;
        let follow_origin = actor_tile(client)?;
        let path = route(world, peer_at, |at| distance(at, follow_origin) == 3)
            .ok_or("follow peer approach")?;
        walk(peer, world, &path).await?;
        let reference = own(peer)?.entity;
        until_with_peer(client, peer, |c| {
            c.world_entities()
                .is_some_and(|entities| entities.get(&reference).is_some())
        })
        .await?;
        let encoded = oteryn_protocol_oteryn::attack::encode_attack_target_intent(Some(&reference))
            .map_err(|e| format!("follow encoding: {e:?}"))?;
        let mut payload = vec![34, u8::try_from(encoded.len())?];
        payload.extend(encoded);
        target_command(client, &payload).await?;
        let peer_at = actor_tile(peer)?;
        until_with_peer(client, peer, |c| {
            served_tile(c.world_spatial().actor_position, basis)
                .is_ok_and(|at| distance(at, peer_at) == 1)
        })
        .await?;
        let follower_at = actor_tile(client)?;
        let path = route(world, peer_at, |at| distance(at, follower_at) == 4)
            .ok_or("moving follow target lane")?;
        walk(peer, world, &path).await?;
        let peer_at = actor_tile(peer)?;
        until_with_peer(client, peer, |c| {
            served_tile(c.world_spatial().actor_position, basis)
                .is_ok_and(|at| at != follower_at && distance(at, peer_at) == 1)
        })
        .await?;
        target_command(client, &[16, 1]).await?;
        stable(client, 1500).await?;
        checks.pass("follow", (reference, client.world_spatial()));
        Ok(())
    }

    async fn door_use(
        client: &mut DevClientSession,
        world: &BundleWorld,
        object: Object,
        revision: u64,
    ) -> TestResult<UseDisposition> {
        let target = target(world, object, revision)?;
        let command = client.next_command_id();
        let started = std::time::Instant::now();
        let session_key = client
            .join_snapshot()
            .game_session_id
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        evidence(&format!(
            "E2EF1_USE phase=send session={session_key} command_id={command} target={object:?}"
        ));
        let result = client
            .use_object(&target.placement, revision)
            .await
            .inspect_err(|error| {
                evidence(&format!(
                    "E2EF1_USE phase=error session={session_key} command_id={command} elapsed_ms={} server_sequence={} error={error}",
                    started.elapsed().as_millis(), client.last_server_sequence()
                ))
            })?;
        evidence(&format!(
            "E2EF1_USE phase=result session={session_key} command_id={command} elapsed_ms={} disposition={:?}",
            started.elapsed().as_millis(),
            result.disposition
        ));
        Ok(result.disposition)
    }
    async fn m02(
        client: &mut DevClientSession,
        world: &BundleWorld,
        objects: &[Object],
        checks: &mut Checks,
        revisions: &mut BTreeMap<(TilePos, u8), u64>,
        metadata: &oteryn_world_bundle::bundle::Visited,
    ) -> TestResult {
        let used = far_use(client, world, objects, checks, revisions).await;
        if let Err(error) = &used {
            if error.is::<MissingAuthoredTarget>() {
                checks.missing("far-use-retry", WORLD_TARGET_SOURCE, &error.to_string());
            }
            evidence(&format!(
                "E2EF1_GAP row=M02 case=far-use-retry error={error}"
            ));
        }
        let moved = far_move(client, world, checks, metadata).await;
        if let Err(error) = &moved {
            evidence(&format!(
                "E2EF1_GAP row=M02 case=far-move-retry error={error}"
            ));
        }
        used.and(moved)
    }
    async fn far_use(
        client: &mut DevClientSession,
        world: &BundleWorld,
        objects: &[Object],
        checks: &mut Checks,
        revisions: &mut BTreeMap<(TilePos, u8), u64>,
    ) -> TestResult {
        // Plain door orientations from map/door_registry.rs (not key, level or quest doors).
        let object = approach(
            client,
            world,
            objects,
            |o| retry_door(o.appearance, o.action) && world.facts().doors().at(o.at),
            3,
        )
        .await?;
        let before = actor_tile(client)?;
        let revision = *revisions.get(&(object.at, object.ordinal)).unwrap_or(&0);
        if door_use(client, world, object, revision).await? != UseDisposition::Committed
            || distance(actor_tile(client)?, object.at) > 1
            || actor_tile(client)? == before
        {
            return Err("far USE did not walk and commit retry".into());
        }
        // Restore the channel-scoped door, using its actual transition revision.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        loop {
            match door_use(client, world, object, revision + 1).await? {
                UseDisposition::Committed => break,
                UseDisposition::Exhausted if tokio::time::Instant::now() < deadline => {
                    pump(client).await?
                }
                other => return Err(format!("restore far-use door: {other:?}").into()),
            }
        }
        revisions.insert((object.at, object.ordinal), revision + 2);
        checks.pass("far-use-retry", (before, object, client.world_spatial()));
        Ok(())
    }
    fn dropped_handle_matches(
        origin: oteryn_protocol_oteryn::world_map::MapOrigin,
        prior: &BTreeSet<oteryn_dev_client::ItemHandle>,
    ) -> bool {
        matches!(origin, oteryn_protocol_oteryn::world_map::MapOrigin::Handle(handle) if !prior.contains(&handle))
    }
    fn item_quantity(
        entries: &[oteryn_dev_client::ItemEntry],
        item: &oteryn_dev_client::ItemEntry,
    ) -> u64 {
        entries
            .iter()
            .filter(|entry| {
                entry.item_definition_ref == item.item_definition_ref
                    && entry.sub_type == item.sub_type
            })
            .map(|entry| u64::from(entry.count.get()))
            .sum()
    }

    fn map_drop_matches(
        map: ActorPosition,
        actor: ActorPosition,
        basis: spatial_basis::Basis,
    ) -> TestResult<bool> {
        Ok(map.x == actor.x
            && map.y == actor.y
            && map.floor == i16::from(served_tile(actor, basis)?.floor))
    }
    fn ground_drop_position(
        at: ActorPosition,
        basis: spatial_basis::Basis,
    ) -> TestResult<oteryn_protocol_oteryn::item_view::WorldTilePosition> {
        Ok(oteryn_protocol_oteryn::item_view::WorldTilePosition {
            x: at.x,
            y: at.y,
            floor: i16::from(served_tile(at, basis)?.floor),
        })
    }

    async fn guarded_move(
        client: &mut DevClientSession,
        request: oteryn_dev_client::client_runtime::inventory::MoveRequest,
    ) -> TestResult<oteryn_dev_client::client_runtime::inventory::MoveResponse> {
        use oteryn_protocol_oteryn::item_view as wire;
        if request.count.is_some() {
            return Err("guarded MOVE requires whole-stack request".into());
        }
        let caps = client.selected_capabilities();
        let selection = wire::ItemMoveSelection {
            equip_drop: caps.contains(&12),
            container_tree: caps.contains(&14),
            world_interaction: caps.contains(&11),
        };
        let payload = wire::encode_item_move_intent_for(
            &wire::ItemMoveIntent {
                source: request.source,
                destination: request.destination,
            },
            selection,
        )
        .map_err(|e| format!("MOVE encode: {e:?}"))?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        loop {
            let result = tokio::time::timeout_at(
                deadline,
                client.session_mut().gated_command(4, 9, &payload),
            )
            .await
            .map_err(|_| "MOVE command deadline")??;
            if tokio::time::Instant::now() >= deadline {
                return Err("MOVE command deadline".into());
            }
            if retry_target_gate(result.status, &result.payload, true) {
                pump(client).await?;
                continue;
            }
            if result.status != CommandStatus::Accepted {
                return Err(format!("MOVE gate refused: {result:?}").into());
            }
            let (body, detail) = if client.selected_capabilities().contains(&66) {
                oteryn_protocol_oteryn::refusal_detail::split_refusal_detail(&result.payload)
                    .map_err(|e| format!("MOVE detail: {e:?}"))?
            } else {
                (result.payload.as_slice(), None)
            };
            let outcome = wire::decode_item_move_result_for(body, selection)
                .map_err(|e| format!("MOVE outcome: {e:?}"))?;
            if outcome == wire::ItemMoveOutcome::Moved && detail.is_some() {
                return Err("successful MOVE carried refusal detail".into());
            }
            return Ok(oteryn_dev_client::client_runtime::inventory::MoveResponse {
                outcome,
                detail,
            });
        }
    }

    async fn drop_lane(
        client: &mut DevClientSession,
        position: oteryn_protocol_oteryn::item_view::WorldTilePosition,
    ) -> TestResult<Vec<oteryn_dev_client::ItemEntry>> {
        use oteryn_protocol_oteryn::{browse_field as browse, container_tree as containers};
        let payload =
            browse::encode_browse_field_intent(&browse::BrowseFieldIntent::Open { position })
                .map_err(|e| format!("drop lane query: {e:?}"))?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        let result = loop {
            let basis = spatial_basis::current(client)?;
            if ground_drop_position(client.world_spatial().actor_position, basis)? != position {
                return Err("drop lane origin displaced".into());
            }
            let result = tokio::time::timeout_at(
                deadline,
                client.session_mut().gated_command(
                    browse::CAPABILITY_BROWSE_FIELD_V1,
                    browse::COMMAND_TYPE_BROWSE_FIELD_INTENT,
                    &payload,
                ),
            )
            .await
            .map_err(|_| "drop lane command deadline")??;
            if tokio::time::Instant::now() >= deadline {
                return Err("drop lane command deadline".into());
            }
            if retry_target_gate(result.status, &result.payload, true) {
                tokio::time::timeout_at(deadline, pump(client))
                    .await
                    .map_err(|_| "drop lane command deadline")??;
                continue;
            }
            break result;
        };
        if result.status != CommandStatus::Accepted
            || containers::decode_container_view_result(&result.payload)
                .map_err(|e| format!("drop lane result: {e:?}"))?
                != containers::ContainerViewOutcome::Opened
        {
            return Err(format!("drop lane refused: {result:?}").into());
        }

        loop {
            for event in client.take_events() {
                if let oteryn_dev_client::SessionEvent::ContainerViews(update) = event
                    && update.server_sequence > result.server_sequence
                {
                    for view in update.value.views {
                        if let Some(page) = view.browse
                            && page.position == position
                        {
                            if page.index != 0 || usize::from(page.total) != view.entries.len() {
                                return Err("drop lane needs a complete native page".into());
                            }
                            return Ok(view.entries);
                        }
                    }
                }
            }
            if tokio::time::Instant::now() >= deadline {
                return Err("drop lane observation deadline".into());
            }
            tokio::time::timeout_at(deadline, client.service_liveness(Duration::from_millis(25)))
                .await
                .map_err(|_| "drop lane observation deadline")??;
        }
    }

    fn acquire_movement_item(has_item: bool) -> bool {
        !has_item
    }

    async fn buy_rope(
        client: &mut DevClientSession,
        world: &BundleWorld,
        metadata: &oteryn_world_bundle::bundle::Visited,
    ) -> TestResult {
        const ROPE: u32 = 3003;
        full_gold::earn(
            client,
            world,
            &metadata.spawns,
            "movement-rope",
            full_gold::price(ROPE, 1)?,
        )
        .await?;
        full_gold::buy(client, world, &metadata.npcs, ROPE, 1).await?;
        Ok(())
    }

    async fn far_move(
        client: &mut DevClientSession,
        world: &BundleWorld,
        checks: &mut Checks,
        metadata: &oteryn_world_bundle::bundle::Visited,
    ) -> TestResult {
        use oteryn_dev_client::client_runtime::inventory::MoveRequest;
        use oteryn_dev_client::{ItemMoveDestination, ItemMoveOutcome};
        if acquire_movement_item(
            client
                .inventory()
                .is_some_and(|view| !view.entries.is_empty()),
        ) {
            buy_rope(client, world, metadata).await?;
        }
        let ground = client.world_spatial().actor_position;
        let basis = spatial_basis::current(client)?;
        let native_ground = ground_drop_position(ground, basis)?;
        let baseline = drop_lane(client, native_ground).await?;
        let item = client
            .inventory()
            .and_then(|i| {
                i.entries.iter().find(|entry| {
                    !baseline
                        .iter()
                        .any(|ground| ground.item_definition_ref == entry.item_definition_ref)
                })
            })
            .copied()
            .ok_or(
                "FAIL: no ordinarily acquired backpack item for far MOVE; no fixture mint allowed",
            )?;
        let quantity = |c: &DevClientSession| -> u64 {
            c.inventory()
                .map_or(0, |inventory| item_quantity(&inventory.entries, &item))
        };
        let before_quantity = quantity(client);
        if baseline
            .iter()
            .any(|entry| entry.item_definition_ref == item.item_definition_ref)
        {
            return Err(
                "far MOVE requires a drop lane without another instance of this definition".into(),
            );
        }
        let prior_handles: BTreeSet<_> = baseline.iter().map(|entry| entry.handle).collect();
        client.take_events();

        let dropped = guarded_move(
            client,
            MoveRequest {
                source: item.handle,
                count: None,
                destination: ItemMoveDestination::Ground(native_ground),
            },
        )
        .await?;
        if dropped.outcome != ItemMoveOutcome::Moved {
            return Err(format!("far MOVE setup drop: {dropped:?}").into());
        }
        let drop_sequence = client.last_server_sequence();
        // Map-capable clients receive Ground handles in WORLD_MAP, not as actors.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        let handle = loop {
            let mut found = BTreeSet::new();
            for event in client.take_events() {
                {
                    let tiles = match event {
                        oteryn_dev_client::SessionEvent::WorldMap(delta)
                            if delta.server_sequence > drop_sequence =>
                        {
                            delta.value.tiles
                        }
                        oteryn_dev_client::SessionEvent::WorldMapSnapshot(snapshot)
                            if snapshot.server_sequence > drop_sequence =>
                        {
                            snapshot.value.tiles
                        }
                        _ => continue,
                    };
                    for tile in tiles.iter().filter(|tile| {
                        map_drop_matches(tile.position, ground, basis).unwrap_or(false)
                    }) {
                        for entry in &tile.items {
                            if entry.definition
                                == oteryn_protocol_oteryn::world_map::MapDefinition::Item(
                                    item.item_definition_ref,
                                )
                                && let oteryn_protocol_oteryn::world_map::MapOrigin::Handle(handle) =
                                    entry.origin
                                && dropped_handle_matches(entry.origin, &prior_handles)
                                && entry.count == item.count.get()
                            {
                                found.insert(handle);
                            }
                        }
                    }
                }
            }
            if found.len() > 1 {
                return Err("ambiguous post-drop native handles".into());
            }
            if let Some(handle) = found.first() {
                break *handle;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err("dropped item did not publish a live native map handle".into());
            }
            client.service_liveness(Duration::from_millis(25)).await?;
        };
        let result: TestResult = async {
            let from = served_tile(ground, basis)?;
            let actual = actor_tile(client)?;
            let path = route(world, actual, |at| distance(at, from) == 3)
                .ok_or("far MOVE retreat route")?;
            walk(client, world, &path).await?;
            let before = client.world_spatial().actor_position;
            let moved = guarded_move(
                client,
                MoveRequest {
                    source: handle,
                    count: None,
                    destination: ItemMoveDestination::MainBackpack,
                },
            )
            .await?;
            if moved.outcome != ItemMoveOutcome::Moved
                || client.world_spatial().actor_position == before
                || distance(actor_tile(client)?, from) > 1
            {
                return Err(format!("far MOVE did not walk and retry: {moved:?}").into());
            }
            until(client, |c| quantity(c) == before_quantity).await?;
            checks.pass("far-move-retry", (ground, client.world_spatial()));
            Ok(())
        }
        .await;
        if let Err(error) = &result {
            evidence(&format!("E2EF1_MOVE_RECOVERY original_error={error}"));
            let restored = |c: &DevClientSession| quantity(c) == before_quantity;
            if !restored(client) {
                let drop = served_tile(ground, basis)?;
                let from = actor_tile(client)?;
                let path = reach_route(client, world, from, |at| distance(at, drop) <= 1)
                    .await?
                    .map_err(|failure| format!("dropped item recovery route: {failure}"))?;
                walk(client, world, &path).await?;
                let recovery = guarded_move(
                    client,
                    MoveRequest {
                        source: handle,
                        count: None,
                        destination: ItemMoveDestination::MainBackpack,
                    },
                )
                .await?;
                if recovery.outcome != ItemMoveOutcome::Moved {
                    return Err(format!("dropped item recovery refused: {recovery:?}").into());
                }
                until(client, restored).await?;
            }
        }
        result
    }

    fn floor_exhaust_spell<'a>(
        book: &'a crate::spell::SpellBook,
        level: u32,
        vocation: crate::spell::Vocation,
        mana: u32,
        max_mana: u32,
        eligibility: (
            Option<&[oteryn_protocol_oteryn::actor_spell_catalog::SpellCatalogEntry]>,
            bool,
            bool,
        ),
    ) -> Option<(std::num::NonZeroU32, &'a crate::spell::SpellDefinition)> {
        (1..=book.source_len())
            .filter_map(|i| {
                let index = std::num::NonZeroU32::new(u32::try_from(i).ok()?)?;
                Some((index, book.indexed(index)?))
            })
            .filter(|(index, spell)| {
                spell.aggressive
                    && eligibility.0.map_or_else(
                        || {
                            spell.level <= level
                                && spell.vocations.contains(&vocation)
                                && !spell.premium
                                && !spell.learning_required
                        },
                        |entries| {
                            entries
                                .iter()
                                .any(|entry| entry.cast_index == index.get() && entry.available)
                        },
                    )
                    && match spell.mana {
                        crate::spell::ManaCost::Fixed(cost) => cost <= mana,
                        crate::spell::ManaCost::PercentOfMaximum(percent) => {
                            u64::from(max_mana) * u64::from(percent) / 100 <= u64::from(mana)
                        }
                    }
                    && (!spell.needs_target || spell.target_or_direction)
                    && (!spell.needs_weapon || eligibility.1)
                    && (!spell.needs_shield || eligibility.2)
                    && matches!(
                        spell.carrier,
                        crate::spell::Carrier::Instant {
                            takes_parameter: false,
                            ..
                        }
                    )
                    && matches!(
                        spell.execution,
                        crate::spell::Execution::Effects(_)
                            | crate::spell::Execution::AbilityVariants(_)
                    )
            })
            .min_by_key(|(_, spell)| (spell.level, spell.cooldown_micros))
    }

    pub(in super::super) async fn fixture_spell_facts(
        client: &mut DevClientSession,
    ) -> TestResult<(u32, crate::spell::Vocation)> {
        use oteryn_protocol_oteryn::cyclopedia_character as wire;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        let result = loop {
            let result = tokio::time::timeout_at(
                deadline,
                client.session_mut().gated_command(
                    wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
                    388,
                    &[wire::CharacterTab::GeneralStats as u8, 0, 0, 1],
                ),
            )
            .await
            .map_err(|_| "command retry deadline expired")??;
            if tokio::time::Instant::now() >= deadline {
                return Err("command retry deadline expired".into());
            }
            if result.status == CommandStatus::Accepted {
                break result;
            }
            if !retry_target_gate(result.status, &result.payload, true)
                || tokio::time::Instant::now() >= deadline
            {
                return Err(format!("fixture GeneralStats refused: {result:?}").into());
            }
            tokio::time::timeout_at(deadline, pump(client))
                .await
                .map_err(|_| "command retry deadline expired")??;
        };
        let vocation_id =
            general_stats::validate(&result.payload, general_stats::selected(client))?
                .ok_or("fixture vocation absent from negotiated GeneralStats")?;
        let level = u32::from_le_bytes(result.payload[9..13].try_into()?);
        let key = [
            "none",
            "druid",
            "elder_druid",
            "sorcerer",
            "master_sorcerer",
            "knight",
            "elite_knight",
            "paladin",
            "royal_paladin",
            "monk",
            "exalted_monk",
        ]
        .into_iter()
        .find(|key| wire::vocation_id(key) == Some(vocation_id))
        .ok_or("fixture vocation id unknown")?;
        Ok((
            level,
            crate::spell::Vocation::from_key(key).ok_or("fixture vocation unknown")?,
        ))
    }

    fn stair_landing(from: i8, to: i8, teleport: bool) -> bool {
        from != to && !teleport
    }

    fn completed_row(actions: bool, durable: bool, returned: bool) -> bool {
        actions && durable && returned
    }

    async fn m03(
        client: &mut DevClientSession,
        world: &BundleWorld,
        objects: &[Object],
        checks: &mut Checks,
    ) -> TestResult {
        use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
        let book = native_spell_book()?;
        let (level, vocation) = fixture_spell_facts(client).await?;
        let catalog = client
            .session_mut()
            .spell_catalog()?
            .ok_or("floor exhaust native catalog absent")?;
        let input = native_input()?;
        let profiles: crate::content::native_gameplay::ItemProfilesDocument =
            serde_json::from_slice(
                &input
                    .item_profiles
                    .as_ref()
                    .ok_or("native item profiles absent")?
                    .bytes,
            )?;
        let index = item_index()?;
        let mut gear = (false, false);
        if let Some(inventory) = client.inventory() {
            for equipped in &inventory.equipment {
                use oteryn_protocol_oteryn::item_view::EquipmentSlot;
                if !matches!(
                    equipped.slot,
                    EquipmentSlot::LeftHand | EquipmentSlot::RightHand
                ) {
                    continue;
                }
                let Some((key, revision)) = index.definition_at(equipped.item.item_definition_ref)
                else {
                    continue;
                };
                let Some(record) = profiles.records.iter().find(|record| {
                    record.production_definition.production_key == key
                        && record.production_definition.revision_ref == revision
                }) else {
                    continue;
                };
                use crate::content::{ReferenceItemField as F, ReferenceWeaponType as W};
                if let F::Known(weapon) = &record.semantics.weapon {
                    gear.0 |= matches!(
                        weapon.weapon_type,
                        F::Known(W::Club | W::Sword | W::Axe | W::Fist)
                    );
                    gear.1 |= matches!(weapon.weapon_type, F::Known(W::Shield));
                }
            }
        }
        let pools = client.actor_vitals().ok_or("floor exhaust pools absent")?;
        let selected = floor_exhaust_spell(
            book,
            level,
            vocation,
            pools.mana,
            pools.max_mana,
            (Some(&catalog.entries), gear.0, gear.1),
        );
        let cooldown = selected.map_or(Duration::ZERO, |(_, spell)| {
            Duration::from_micros(
                spell
                    .groups
                    .iter()
                    .map(|group| group.cooldown_micros)
                    .chain([spell.cooldown_micros])
                    .max()
                    .unwrap_or(0),
            )
        });
        if selected.is_none() {
            checks.missing("aggressive-exhaust", SPELL_TARGET_SOURCE,
                &format!("no executable untargeted aggressive spell for observed fixture level={level} vocation={vocation:?}; no fixture promotion permitted"));
        }
        let mut ready = tokio::time::Instant::now();
        let mut exhausted = 0;
        for teleport in [false, true] {
            // Settle the spell's own cooldown before starting a distinct floor experiment.
            while tokio::time::Instant::now() < ready {
                pump(client).await?;
            }
            let result: TestResult = async {
                let from = actor_tile(client)?;
                let outside = |at: TilePos| world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| !tile.protection_zone());
                if teleport {
                    let object = approach(client, world, objects, |o| world.floors().teleport(o.at).is_some_and(|to| to.floor != o.at.floor && outside(to)), 1).await?;
                    let at = actor_tile(client)?;
                    let expected = world.floors().teleport(object.at).ok_or("teleport destination")?;
                    let direction = DIRS.into_iter().find(|d| destination(world, at, *d) == Some(expected)).ok_or("no automatic teleport entry edge")?;
                    walk(client, world, &[direction]).await?;
                    checks.pass("teleport", (object.at, expected));
                } else {
                    let path = route_with_edges(from, |at| at.floor != from.floor, |at, direction| {
                        let (to, teleport) = landing(world, at, direction)?;
                        (!teleport && (to.floor == at.floor || (stair_landing(at.floor, to.floor, teleport) && outside(to)))).then_some(to)
                    }).ok_or("no reachable outside-PZ stair transition")?;
                    walk(client, world, &path).await?;
                    if actor_tile(client)?.floor == from.floor { return Err("stairs did not change floor".into()); }
                    checks.pass("stairs", client.world_spatial());
                }
                let Some((index, _)) = selected else {
                    return Err("MISSING aggressive-exhaust: observed fixture has no eligible spell; source=content/abilities/definitions/player-spells.json:1".into());
                };
                let changed = std::time::Instant::now();
                let before = client.actor_vitals().copied().ok_or("floor exhaust vitals")?;
                let result = client.cast_spell(index, SpellTarget::None, false).await?;
                if result.disposition != SpellCastDisposition::CoolingDown || result.actor_vitals_delta.is_some() || client.actor_vitals() != Some(&before) { return Err(format!("floor action did not refuse without resource spend: {result:?}").into()); }
                let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
                loop {
                    client.service_liveness(Duration::from_millis(100)).await?;
                    client.take_events();
                    let result = client.cast_spell(index, SpellTarget::None, false).await?;
                    if result.disposition == SpellCastDisposition::Cast {
                        if changed.elapsed() < Duration::from_millis(1800) || client.actor_vitals().is_none_or(|v| v.mana >= before.mana) || result.actor_vitals_delta.is_none() { return Err("floor exhaust expired early or post-window cast spent no mana".into()); }
                        ready = tokio::time::Instant::now() + cooldown;
                        break;
                    }
                    if result.disposition != SpellCastDisposition::CoolingDown || client.actor_vitals().is_none_or(|v| v.mana != before.mana || v.soul != before.soul) || tokio::time::Instant::now() >= deadline { return Err(format!("aggressive action did not become executable after floor exhaust: {result:?}").into()); }
                }
                exhausted += 1;
                Ok(())
            }.await;
            if let Err(error) = result {
                if teleport && error.is::<MissingAuthoredTarget>() {
                    checks.missing("teleport", WORLD_TARGET_SOURCE, &error.to_string());
                }
                evidence(&format!(
                    "E2EF1_GAP row=M03 teleport={teleport} error={error}"
                ));
            }
        }
        if exhausted != 2 {
            return Err(
                "stairs and teleport must each prove two-second aggressive exhaust and recovery"
                    .into(),
            );
        }
        checks.pass(
            "aggressive-exhaust",
            "both_transitions_refuse_then_allow_same_cast",
        );
        Ok(())
    }

    use oteryn_protocol_oteryn::creature_push::CreaturePushDisposition;

    async fn push(
        client: &mut DevClientSession,
        reference: EntityRef,
        at: TilePos,
        expected: CreaturePushDisposition,
    ) -> TestResult {
        let basis = spatial_basis::current(client)?;
        use oteryn_protocol_oteryn::creature_push::*;
        let payload = encode_creature_push_intent(&CreaturePushIntent {
            target: reference,
            destination: position(at, basis),
        })
        .map_err(|e| format!("push encoding: {e:?}"))?;
        let result = client
            .session_mut()
            .gated_command(
                CAPABILITY_CREATURE_PUSH_V1,
                COMMAND_TYPE_CREATURE_PUSH_INTENT,
                &payload,
            )
            .await?;
        expect_push(result.status, &result.payload, expected)
    }

    fn expect_push(
        status: CommandStatus,
        payload: &[u8],
        expected: CreaturePushDisposition,
    ) -> TestResult {
        if status == CommandStatus::Rejected {
            return Err(verdict::HarnessError::boxed(
                verdict::Cause::ServerRefused {
                    code: status as u32,
                },
                format!("push command refused: {status:?}, payload={payload:?}"),
            ));
        }
        let [8, code @ 1..=7] = payload else {
            return Err(format!("malformed push reply: {status:?}, payload={payload:?}").into());
        };
        if status != CommandStatus::Accepted {
            return Err(format!("unexpected push command status: {status:?}").into());
        }
        if *code == expected as u8 {
            return Ok(());
        }
        let message = format!("push disposition {code}, expected {expected:?}");
        if *code == CreaturePushDisposition::Moved as u8 {
            return Err(message.into());
        }
        Err(verdict::HarnessError::boxed(
            verdict::Cause::ServerRefused {
                code: u32::from(*code),
            },
            message,
        ))
    }
    async fn m04(
        client: &mut DevClientSession,
        peer: &mut DevClientSession,
        world: &BundleWorld,
        bytes: &[u8],
        checks: &mut Checks,
    ) -> TestResult {
        let player = player_push(client, peer, world, checks).await;
        let player_action = checks.active;
        if let Err(error) = &player {
            evidence(&format!("E2EF1_GAP row=M04 case=push-player error={error}"));
        }
        checks.active = Some("push-creature");
        let creature = creature_push(client, world, bytes, checks).await;
        if let Err(error) = &creature {
            evidence(&format!(
                "E2EF1_GAP row=M04 case=push-creature error={error}"
            ));
        }
        // Leave the lethal encounter for the nearest authored protection zone.
        // The row's later origin return still enforces its fixture-town requirement.
        let left = return_to_protection_zone(client, world).await.map(|_| ());
        checks.active = if player.is_err() {
            player_action
        } else if creature.is_err() {
            Some("push-creature")
        } else {
            Some("protection-zone-return")
        };
        player.and(creature).and(left)
    }
    async fn player_push(
        client: &mut DevClientSession,
        peer: &mut DevClientSession,
        world: &BundleWorld,
        checks: &mut Checks,
    ) -> TestResult {
        checks.active = Some("push-player");
        let basis = spatial_basis::current(client)?;
        if spatial_basis::current(peer)? != basis {
            return Err("peer spatial basis differs from actor".into());
        }
        let here = actor_tile(client)?;
        let path = route(world, actor_tile(peer)?, |at| {
            distance(at, here) == 1
                && DIRS.into_iter().any(|d| {
                    destination(world, at, d)
                        .is_some_and(|next| next != here && distance(next, here) == 1)
                })
        })
        .ok_or("adjacent push-player lane")?;
        walk(peer, world, &path).await?;
        let original = actor_tile(peer)?;
        let reference = own(peer)?.entity;
        until_with_peer(client, peer, |c| {
            c.world_entities().is_some_and(|entities| {
                entities
                    .get(&reference)
                    .is_some_and(|e| e.position == position(original, basis))
            })
        })
        .await?;
        checks.active = Some("collision");
        push(
            client,
            reference,
            here,
            CreaturePushDisposition::NotEnoughRoom,
        )
        .await?;
        stable(peer, 300).await?;
        checks.pass("collision", (reference, here, original));
        checks.active = Some("push-player");
        let landing = DIRS
            .into_iter()
            .filter_map(|d| destination(world, original, d))
            .find(|at| *at != here && distance(*at, here) == 1)
            .ok_or("free push landing")?;
        push(client, reference, landing, CreaturePushDisposition::Moved).await?;
        until_with_peer(peer, client, |c| {
            c.world_spatial().actor_position == position(landing, basis)
        })
        .await?;
        until_with_peer(client, peer, |c| {
            c.world_entities().is_some_and(|entities| {
                entities
                    .get(&reference)
                    .is_some_and(|e| e.position == position(landing, basis))
            })
        })
        .await?;
        let saved = peer.world_spatial().actor_position;
        let (result, logout, attempts, busy_timed_out) = super::logout_retrying_busy(peer).await?;
        if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err(format!("pushed peer logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
        }
        // The caller reconnects this peer after the row and checks this exact saved cell.
        evidence(&format!("E2EF1_PUSH_SAVED position={saved:?}"));
        checks.pass("push-player", (reference, original, landing));
        Ok(())
    }
    async fn creature_push(
        client: &mut DevClientSession,
        world: &BundleWorld,
        bytes: &[u8],
        checks: &mut Checks,
    ) -> TestResult {
        let basis = spatial_basis::current(client)?;
        let metadata = oteryn_world_bundle::bundle::visit(bytes, |_, _| Ok(()))?;
        let creature = approach_spawn_target(
            client,
            world,
            &metadata.spawns,
            "oteryn:creature.rat",
            SpawnRespawn::Wait,
            None,
        )
        .await?
        .ok_or("no reachable adjacent rat to push")?;
        let here = actor_tile(client)?;
        let from = served_tile(creature.position, basis)?;
        let landing = DIRS
            .into_iter()
            .filter_map(|d| destination(world, from, d))
            .find(|at| *at != here && distance(*at, from) == 1)
            .ok_or("creature push lane")?;
        evidence(&format!(
            "E2EF1_PUSH phase=command command_type=98 from={from:?} to={landing:?} entity={:?}",
            creature.entity
        ));
        push(client, creature.entity, landing, CreaturePushDisposition::Moved).await.inspect_err(|error| {
            evidence(&format!("E2EF1_PUSH phase=command error={error} visibility_codec=domain:1,type:2 source=crates/session/src/domain_dispatch.rs:160"));
        })?;
        until(client, |c| {
            c.world_entities().is_some_and(|entities| {
                entities
                    .get(&creature.entity)
                    .is_some_and(|e| e.position == position(landing, basis))
            })
        })
        .await.inspect_err(|error| {
            evidence(&format!("E2EF1_PUSH phase=observe error={error} visibility_codec=domain:1,type:2 source=crates/session/src/domain_dispatch.rs:160"));
        })?;
        evidence("E2EF1_PUSH phase=observed");
        checks.pass("push-creature", (creature.entity, from, landing));
        Ok(())
    }

    async fn toggle(
        client: &mut DevClientSession,
        world: &BundleWorld,
        object: Object,
        revision: u64,
        expected: UseDisposition,
    ) -> TestResult {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        loop {
            let result = door_use(client, world, object, revision).await?;
            if result == expected {
                return Ok(());
            }
            if result != UseDisposition::Exhausted || tokio::time::Instant::now() >= deadline {
                return Err(format!(
                    "door {object:?} revision={revision}: {result:?}, wanted {expected:?}"
                )
                .into());
            }
            pump(client).await?;
        }
    }
    async fn door_cycle(
        client: &mut DevClientSession,
        world: &BundleWorld,
        object: Object,
        revision: u64,
    ) -> TestResult {
        let basis = spatial_basis::current(client)?;
        let at = actor_tile(client)?;
        let direction = DIRS
            .into_iter()
            .find(|direction| {
                let (dx, dy) = match direction {
                    StepDirection::North => (0, -1),
                    StepDirection::East => (1, 0),
                    StepDirection::South => (0, 1),
                    StepDirection::West => (-1, 0),
                    StepDirection::NorthEast => (1, -1),
                    StepDirection::SouthEast => (1, 1),
                    StepDirection::SouthWest => (-1, 1),
                    StepDirection::NorthWest => (-1, -1),
                };
                at.x.checked_add_signed(dx) == Some(object.at.x)
                    && at.y.checked_add_signed(dy) == Some(object.at.y)
            })
            .ok_or("door approach is diagonal; cardinal crossing required")?;
        let before = client.world_spatial().actor_position;
        if client.step_retrying(direction).await?.disposition != StepDisposition::Blocked
            || client.world_spatial().actor_position != before
        {
            return Err("closed door collision did not block".into());
        }
        toggle(client, world, object, revision, UseDisposition::Committed).await?;
        // The oracle keeps immutable closed-door collision. Here only the server
        // knows the new door state, so assert the actual native crossing directly.
        if client.step_retrying(direction).await?.disposition != StepDisposition::Moved
            || client.world_spatial().actor_position != position(object.at, basis)
        {
            return Err("open door did not admit crossing".into());
        }
        toggle(
            client,
            world,
            object,
            revision + 1,
            UseDisposition::Occupied,
        )
        .await?;
        let reverse = match direction {
            StepDirection::North => StepDirection::South,
            StepDirection::East => StepDirection::West,
            StepDirection::South => StepDirection::North,
            StepDirection::West => StepDirection::East,
            StepDirection::NorthEast => StepDirection::SouthWest,
            StepDirection::SouthEast => StepDirection::NorthWest,
            StepDirection::SouthWest => StepDirection::NorthEast,
            StepDirection::NorthWest => StepDirection::SouthEast,
        };
        if client.step_retrying(reverse).await?.disposition != StepDisposition::Moved
            || client.world_spatial().actor_position != before
        {
            return Err("door exit did not restore approach".into());
        }
        toggle(
            client,
            world,
            object,
            revision + 1,
            UseDisposition::Committed,
        )
        .await?;
        if client.step_retrying(direction).await?.disposition != StepDisposition::Blocked
            || client.world_spatial().actor_position != before
        {
            return Err("reclosed door did not block".into());
        }
        Ok(())
    }
    async fn m05(
        client: &mut DevClientSession,
        world: &BundleWorld,
        objects: &[Object],
        checks: &mut Checks,
        revisions: &mut BTreeMap<(TilePos, u8), u64>,
    ) -> TestResult {
        // Exact production registry families: plain 1638/1639 and key triple
        // locked 1628, unlocked 1629, open 1630. No name/keyword classification.
        let mut families_ok = true;
        for (appearance, case) in [(1638, "basic-door"), (1629, "unlocked-key-door")] {
            let result: TestResult = async {
                let object = approach(
                    client,
                    world,
                    objects,
                    |o| o.appearance == appearance && world.facts().doors().at(o.at),
                    1,
                )
                .await?;
                let revision = *revisions.get(&(object.at, object.ordinal)).unwrap_or(&0);
                door_cycle(client, world, object, revision).await?;
                revisions.insert((object.at, object.ordinal), revision + 2);
                checks.pass(case, object);
                Ok(())
            }
            .await;
            if let Err(error) = result {
                families_ok = false;
                evidence(&format!("E2EF1_GAP row=M05 case={case} error={error}"));
            }
        }
        let object = approach(
            client,
            world,
            objects,
            |o| o.appearance == 1628 && world.facts().doors().at(o.at),
            1,
        )
        .await?;
        let before = client.world_spatial().actor_position;
        toggle(client, world, object, 0, UseDisposition::Locked).await?;
        if client.world_spatial().actor_position != before {
            return Err("locked door use moved actor".into());
        }
        checks.pass("locked-refusal", object);
        if !families_ok {
            return Err("one or more door families incomplete".into());
        }
        Ok(())
    }
    async fn m07(
        client: &mut DevClientSession,
        world: &BundleWorld,
        objects: &[Object],
        checks: &mut Checks,
        metadata: &oteryn_world_bundle::bundle::Visited,
    ) -> TestResult {
        use oteryn_protocol_oteryn::world_object::*;
        checks.active = Some("rope-ascent");
        for capability in [CAPABILITY_ITEM_USE_V1, CAPABILITY_WORLD_INTERACTION_V1] {
            if !client.selected_capabilities().contains(&capability) {
                return Err(
                    oteryn_dev_client::SessionError::CapabilityNotSelected { capability }.into(),
                );
            }
        }
        checks.active = Some("carried-rope");
        let index = provisioning::item_index()?;
        let reference = provisioning::item_reference(&index, 3003)?;
        let carried_rope = |client: &DevClientSession| {
            client
                .inventory()
                .and_then(|i| {
                    i.entries
                        .iter()
                        .find(|entry| entry.item_definition_ref == reference)
                })
                .copied()
        };
        if acquire_movement_item(carried_rope(client).is_some()) {
            buy_rope(client, world, metadata).await?;
        }
        let rope = carried_rope(client).ok_or("ordinary rope purchase did not deliver the rope")?;
        checks.pass("carried-rope", rope);
        checks.active = Some("rope-ascent");
        // Production rope resolver's exact ground and top item source ids.
        let object = approach(
            client,
            world,
            objects,
            |o| {
                [
                    386, 421, 7762, 12202, 12936, 14238, 17238, 21965, 21966, 21967, 21968, 23363,
                    12935,
                ]
                .contains(&o.appearance)
            },
            1,
        )
        .await?;
        let intent = UseIntent {
            target: UseTarget::Item(rope.handle),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(target(world, object, 0)?)),
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent).map_err(|e| format!("rope intent: {e:?}"))?;
        let result = client.session_mut().world_context_use(&payload).await?;
        if result.status != CommandStatus::Accepted
            || decode_use_result(&result.payload).map_err(|e| format!("rope result: {e:?}"))?
                != UseDisposition::Committed
        {
            return Err(format!("rope ascent refused: {result:?}").into());
        }
        until(client, |c| {
            tile(c.world_spatial().actor_position).is_ok_and(|at| {
                at.floor == object.at.floor + 1
                    && at.x.abs_diff(object.at.x).max(at.y.abs_diff(object.at.y)) == 1
            })
        })
        .await?;
        if !client.inventory().is_some_and(|i| {
            i.entries
                .iter()
                .any(|e| e.item_definition_ref == reference && e.count == rope.count)
        }) {
            return Err("rope tool consumed or lost".into());
        }
        checks.pass("rope-ascent", (object, client.world_spatial()));
        Ok(())
    }

    fn inventory_signature(client: &DevClientSession) -> TestResult<Vec<(u32, u32, u32, u8)>> {
        let inventory = client
            .inventory()
            .ok_or("movement inventory snapshot missing")?;
        let mut items = Vec::new();
        let entry = |item: &oteryn_dev_client::ItemEntry, slot| {
            (
                item.item_definition_ref.get(),
                item.count.get(),
                item.sub_type,
                slot,
            )
        };
        if let Some(item) = &inventory.main_backpack {
            items.push(entry(item, 255));
        }
        items.extend(inventory.entries.iter().map(|item| entry(item, 0)));
        items.extend(
            inventory
                .equipment
                .iter()
                .map(|item| entry(&item.item, item.slot as u8)),
        );
        items.sort();
        Ok(items)
    }
    pub(super) async fn logout(client: &mut DevClientSession) -> TestResult {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
        loop {
            let logout = client.session_mut().logout().await?;
            if logout.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("movement logout lock did not settle: {logout:?}").into());
            }
            pump(client).await?;
        }
    }
    pub(super) async fn run(
        client: DevClientSession,
        clients: &SeamClients<'_>,
        world: &BundleWorld,
    ) -> TestResult<DevClientSession> {
        let mut verdicts = Verdicts::default();
        let result = rows(client, clients, world, &mut verdicts).await;
        if let Err(error) = &result {
            verdicts.aborted = Some(verdict::classify(error.as_ref()));
        }
        result
    }
    async fn rows(
        mut client: DevClientSession,
        clients: &SeamClients<'_>,
        world: &BundleWorld,
        verdicts: &mut Verdicts,
    ) -> TestResult<DevClientSession> {
        let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
        let objects = objects(&bytes)?;
        let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
        let mut failed = false;
        let mut revisions = BTreeMap::new();
        for (ordinal, (row, required)) in ROWS.iter().enumerate() {
            verdicts.current = Some(row);
            let origin = actor_tile(&mut client)?;
            let started = std::time::Instant::now();
            let mut checks = Checks {
                row,
                ..Checks::default()
            };
            let mut peer = if matches!(*row, "M01" | "M04") {
                Some(connect_character(clients, 0xa0 + u8::try_from(ordinal)?, 0).await?)
            } else {
                None
            };
            let result = match *row {
                "M01" => {
                    m01(
                        &mut client,
                        peer.as_mut().ok_or("follow peer missing")?,
                        world,
                        &mut checks,
                    )
                    .await
                }
                "M02" => {
                    m02(
                        &mut client,
                        world,
                        &objects,
                        &mut checks,
                        &mut revisions,
                        &metadata,
                    )
                    .await
                }
                "M03" => m03(&mut client, world, &objects, &mut checks).await,
                "M04" => {
                    m04(
                        &mut client,
                        peer.as_mut().ok_or("push peer missing")?,
                        world,
                        &bytes,
                        &mut checks,
                    )
                    .await
                }
                "M05" => m05(&mut client, world, &objects, &mut checks, &mut revisions).await,
                "M07" => m07(&mut client, world, &objects, &mut checks, &metadata).await,
                _ => unreachable!(),
            };
            if let Err(error) = &result {
                verdicts.action_failure = Some(checks.action_failure(required, error.as_ref()));
                evidence(&format!("E2EF1_GAP row={row} error={error}"));
            }
            // Every position-changing row saves its actual landing via ordinary
            // logout, gets a fresh Platform admission and compares the saved cell.
            let saved = client.world_spatial().actor_position;
            let inventory = inventory_signature(&client)?;
            let old_session = client.join_snapshot().game_session_id;
            let mut peer_durable = true;
            if let Some(mut peer) = peer.take() {
                let saved = peer.world_spatial().actor_position;
                let previous = peer.join_snapshot().game_session_id;
                if *row != "M04" || !checks.passed.contains("push-player") {
                    logout(&mut peer).await?;
                }
                drop(peer);
                let mut peer = connect_character(clients, 0xc0 + u8::try_from(ordinal)?, 0).await?;
                peer_durable &= peer.world_spatial().actor_position == saved
                    && peer.join_snapshot().game_session_id != previous;
                let back = route(world, actor_tile(&mut peer)?, |at| at == TOWN)
                    .ok_or("peer return route")?;
                walk(&mut peer, world, &back).await?;
                logout(&mut peer).await?;
            }
            logout(&mut client).await?;
            drop(client);
            client = connect_world(clients, 0xb0 + u8::try_from(ordinal)?).await?;
            let durable = client.world_spatial().actor_position == saved
                && client.join_snapshot().game_session_id != old_session
                && inventory_signature(&client)? == inventory
                && peer_durable;
            if durable && required.contains(&"relogin") {
                checks.pass("relogin", saved);
            }
            let restored: TestResult = async {
                let from = tile(client.world_spatial().actor_position)?;
                let back = match reach_route(&mut client, world, from, |at| at == origin).await? {
                    Ok(back) => back,
                    Err(_) => {
                        let planner = travel::Planner::load(world, &bytes)?;
                        travel::travel_to(
                            &mut client,
                            &planner,
                            row,
                            origin,
                            &|at| at == origin,
                            &mut travel::Passages::default(),
                        )
                        .await?;
                        if actor_tile(&mut client)? != origin {
                            return Err("row return did not reach its origin".into());
                        }
                        return Ok(());
                    }
                };
                walk(&mut client, world, &back).await?;
                if actor_tile(&mut client)? != origin {
                    return Err("row return did not reach its origin".into());
                }
                Ok(())
            }
            .await;
            let passed = completed_row(
                result.is_ok() && checks.complete(required),
                durable,
                restored.is_ok(),
            );
            failed |= !passed;
            let gaps: Vec<_> = required
                .iter()
                .filter(|case| !checks.passed.contains(**case))
                .copied()
                .collect();
            evidence(&format!(
                "E2EF1_ROW row={row} missing={gaps:?} result={result:?} durable={durable}"
            ));
            verdicts.done.insert(row);
            let action_failure = verdicts.action_failure.take();
            let suffix = if passed {
                String::new()
            } else if let Some((root, cause)) = action_failure {
                cause.suffix(&root)
            } else {
                let cause = match verdict::release_affects_verdict(&result, &restored) {
                    Err(error) => verdict::classify(error.as_ref()),
                    Ok(_) => verdict::Cause::Assertion,
                };
                cause.suffix(&format!(
                    "{row}/{}",
                    gaps.first().copied().unwrap_or(if restored.is_err() {
                        "return"
                    } else {
                        "durable"
                    })
                ))
            };
            evidence(&format!(
                "e2e_all scenario={row}.full status={} seconds={:.3} evidence=journey:apps/game-server/src/qualification/client_phases/full_e2ef1.rs{suffix}",
                if passed { "PASS" } else { "FAIL" },
                started.elapsed().as_secs_f64()
            ));
            if let Err(error) = restored {
                return Err(MovementReturnFailure(error).into());
            }
        }
        if failed {
            evidence("E2EF1_INCOMPLETE full_row_failures_reported_to_e2e_all");
        }
        Ok(client)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn m04_collision_verdict_keeps_the_classified_failure() -> TestResult {
            let mut checks = Checks {
                row: "M04",
                active: Some("push-player"),
                ..Checks::default()
            };
            let setup: Box<dyn std::error::Error> = "peer lane unavailable".into();
            let (root, cause) = checks.action_failure(ROWS[3].1, setup.as_ref());
            assert_eq!(cause.suffix(&root), " cause=assertion root=M04/push-player");

            checks.active = Some("collision");
            for (status, payload, code) in [
                (CommandStatus::Rejected, &[][..], 2),
                (CommandStatus::Accepted, &[8, 7][..], 7),
            ] {
                let failure = expect_push(status, payload, CreaturePushDisposition::NotEnoughRoom)
                    .err()
                    .ok_or("command or disposition must be refused")?;
                let (root, cause) = checks.action_failure(ROWS[3].1, failure.as_ref());
                assert_eq!(cause, verdict::Cause::ServerRefused { code });
                assert_eq!(
                    cause.suffix(&root),
                    " cause=server-refused root=M04/collision"
                );
            }
            assert!(
                expect_push(
                    CommandStatus::Accepted,
                    &[8, 5],
                    CreaturePushDisposition::NotEnoughRoom
                )
                .is_ok()
            );
            assert!(
                expect_push(
                    CommandStatus::Accepted,
                    &[8, 1],
                    CreaturePushDisposition::Moved
                )
                .is_ok()
            );
            Ok(())
        }

        #[test]
        fn empty_starter_requires_ordinary_movement_item_acquisition() {
            assert!(acquire_movement_item(false));
            assert!(!acquire_movement_item(true));
        }

        #[test]
        fn stairs_evidence_rejects_a_teleport_landing() {
            assert!(stair_landing(-7, -8, false));
            assert!(!stair_landing(-7, -8, true));
            assert!(!stair_landing(-7, -7, false));
        }

        #[test]
        fn movement_verdict_requires_successful_return() {
            assert!(completed_row(true, true, true));
            assert!(!completed_row(true, true, false));
            assert!(!completed_row(true, false, true));
            assert!(!completed_row(false, true, true));
        }

        #[test]
        fn qualification_floor_exhaust_spell_fits_pristine_sorcerer() -> TestResult {
            std::thread::Builder::new()
                .stack_size(64 << 20)
                .spawn(|| -> Result<(), String> {
                    let book = native_spell_book().map_err(|error| error.to_string())?;
                    let spell = book
                        .spoken("exevo infir flam hur")
                        .ok_or("floor exhaust spell missing")?
                        .spell;
                    assert!(spell.aggressive);
                    assert_eq!(spell.level, 1);
                    assert!(!spell.premium && !spell.learning_required && !spell.needs_target);
                    assert!(spell.vocations.contains(&crate::spell::Vocation::Sorcerer));
                    assert_eq!(spell.soul, 0);
                    Ok(())
                })?
                .join()
                .map_err(|_| "floor exhaust content test thread failed")??;
            Ok(())
        }

        #[test]
        fn qualification_map_handle_position_uses_native_floor() -> TestResult {
            let actor = ActorPosition {
                x: 32315,
                y: 32209,
                floor: 8,
            };
            let native = ActorPosition { floor: -8, ..actor };
            assert!(map_drop_matches(
                native,
                actor,
                spatial_basis::Basis::Legacy
            )?);
            Ok(())
        }

        #[test]
        fn qualification_ground_drop_uses_native_bundle_floor() -> TestResult {
            let at = ActorPosition {
                x: 32369,
                y: 32238,
                floor: 7,
            };
            let drop = ground_drop_position(at, spatial_basis::Basis::Legacy)?;
            assert_eq!(drop.floor, -7);
            Ok(())
        }

        #[test]
        fn qualification_target_gate_retries_only_empty_stationary_refusals() {
            assert!(retry_target_gate(CommandStatus::Rejected, &[], true));
            assert!(!retry_target_gate(CommandStatus::Rejected, &[8, 3], true));
            assert!(!retry_target_gate(CommandStatus::Rejected, &[], false));
            assert!(!retry_target_gate(CommandStatus::Accepted, &[], true));
        }

        #[test]
        fn qualification_authority_one_goal_set_covers_all_candidates() -> TestResult {
            let first = Object {
                at: TOWN,
                ordinal: 0,
                appearance: 2177,
                action: None,
            };
            let second = Object {
                at: TilePos {
                    x: TOWN.x + 10,
                    ..TOWN
                },
                ..first
            };
            let goals = candidate_goals(&[first, second], 1)?;
            assert_eq!(goals.len(), 8);
            assert!(!goals.contains_key(&TOWN));
            assert!(goals.values().any(|object| object.at == second.at));
            Ok(())
        }

        #[test]
        fn qualification_authority_drop_keeps_instance_identity() -> TestResult {
            use oteryn_protocol_oteryn::world_map::MapOrigin;
            let expected = std::num::NonZeroU64::MIN;
            let other = std::num::NonZeroU64::new(2).ok_or("distinct handle")?;
            assert!(!dropped_handle_matches(
                MapOrigin::Handle(other),
                &BTreeSet::from([other])
            ));
            assert!(dropped_handle_matches(
                MapOrigin::Handle(expected),
                &BTreeSet::from([other])
            ));
            Ok(())
        }
        #[test]
        fn qualification_authority_quantity_survives_merge() -> TestResult {
            let two = oteryn_dev_client::ItemEntry {
                appearance_id: 0,
                handle: std::num::NonZeroU64::MIN,
                item_definition_ref: std::num::NonZeroU32::MIN,
                count: std::num::NonZeroU32::new(2).ok_or("quantity")?,
                sub_type: 0,
            };
            let five = oteryn_dev_client::ItemEntry {
                count: std::num::NonZeroU32::new(5).ok_or("quantity")?,
                ..two
            };
            let merged = oteryn_dev_client::ItemEntry {
                count: std::num::NonZeroU32::new(7).ok_or("quantity")?,
                ..two
            };
            assert_eq!(item_quantity(&[two, five], &two), 7);
            assert_eq!(item_quantity(&[merged], &two), 7);
            assert_eq!(
                item_quantity(
                    &[oteryn_dev_client::ItemEntry {
                        sub_type: 1,
                        ..merged
                    }],
                    &two
                ),
                0
            );
            Ok(())
        }

        #[test]
        fn qualification_authority_affordable_facing_spell() -> TestResult {
            std::thread::Builder::new()
                .stack_size(64 << 20)
                .spawn(|| -> Result<(), String> {
                    let book = native_spell_book().map_err(|e| e.to_string())?;
                    let (_, spell) = floor_exhaust_spell(
                        book,
                        8,
                        crate::spell::Vocation::Sorcerer,
                        6,
                        100,
                        (None, false, false),
                    )
                    .ok_or("no affordable spell")?;
                    assert!(matches!(spell.mana, crate::spell::ManaCost::Fixed(cost) if cost <= 6));
                    Ok(())
                })?
                .join()
                .map_err(|_| "affordable spell test panicked")??;
            Ok(())
        }

        #[test]
        fn qualification_floor_exhaust_uses_observed_vocation() -> TestResult {
            std::thread::Builder::new()
                .stack_size(64 << 20)
                .spawn(|| -> Result<(), String> {
                    let book = native_spell_book().map_err(|error| error.to_string())?;
                    assert!(
                        floor_exhaust_spell(
                            book,
                            8,
                            crate::spell::Vocation::None,
                            u32::MAX,
                            u32::MAX,
                            (None, false, false)
                        )
                        .is_none()
                    );
                    let (_, spell) = floor_exhaust_spell(
                        book,
                        8,
                        crate::spell::Vocation::Sorcerer,
                        u32::MAX,
                        u32::MAX,
                        (None, false, false),
                    )
                    .ok_or("sorcerer novice spell absent")?;
                    assert_eq!(spell.level, 1);
                    assert!(spell.vocations.contains(&crate::spell::Vocation::Sorcerer));
                    Ok(())
                })?
                .join()
                .map_err(|_| "spell eligibility test thread failed")??;
            Ok(())
        }

        #[test]
        fn missing_authored_target_never_completes_a_row() {
            let mut checks = Checks::default();
            checks.missing(
                "teleport",
                WORLD_TARGET_SOURCE,
                "unreachable authored target",
            );
            assert!(!checks.complete(&["teleport"]));
            assert_eq!(checks.missing.get("teleport"), Some(&WORLD_TARGET_SOURCE));
        }

        #[test]
        fn full_rows_require_every_named_action() {
            for (_, required) in ROWS {
                let mut checks = Checks::default();
                for &case in required {
                    assert!(!checks.complete(required));
                    checks.passed.insert(case);
                }
                assert!(checks.complete(required));
                for &case in required {
                    checks.passed.remove(case);
                    assert!(!checks.complete(required));
                    checks.passed.insert(case);
                }
            }
        }
    }
}
