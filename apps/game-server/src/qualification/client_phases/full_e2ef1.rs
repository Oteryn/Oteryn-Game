// E2EF1: native movement commands against node_boot's production-loaded world.
// Mount as the provisioning phase: run BEFORE its control-socket fixtures.
use super::*;
#[path = "provision.rs"]
mod original;

pub(super) async fn with_peer_liveness<T>(
    client: &mut oteryn_dev_client::DevClientSession,
    work: impl std::future::Future<Output = TestResult<T>>,
) -> TestResult<T> {
    super::mp::observations::while_servicing_liveness(work, async || {
        client
            .session_mut()
            .service_liveness_discarding_events(Duration::from_millis(25))
            .await?;
        Ok(())
    })
    .await?
}

pub(super) fn item_index() -> TestResult<crate::content::item_ref::ItemDefinitionIndex> {
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
mod e2ef1 {
    use super::*;
    use crate::map::door::AuthoredClosedDoor;
    use oteryn_dev_client::{DevClientSession, EntityDetail, EntityKind, EntityRef};
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
            &["push-player", "push-creature", "collision", "relogin"],
        ),
        (
            "M05",
            &["basic-door", "unlocked-key-door", "locked-refusal"],
        ),
        ("M07", &["carried-rope", "rope-ascent", "relogin"]),
    ];
    const FLOOR_EXHAUST_SPELL: &str = "exevo infir flam hur";
    const MOVEMENT_LOGOUT_WINDOW: Duration = Duration::from_secs(70);
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
    }
    impl Drop for Verdicts {
        fn drop(&mut self) {
            let current = self.current.unwrap_or(ROWS[0].0);
            let root = format!("{current}/lifecycle");
            // Without a recorded error the journey future was cancelled by its caller's deadline.
            let cause = self
                .aborted
                .take()
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
    }
    impl Checks {
        fn pass(&mut self, case: &'static str, detail: impl std::fmt::Debug) {
            evidence(&format!(
                "E2EF1_CHECK row={} case={case} status=PASS observed={detail:?}",
                self.row
            ));
            self.passed.insert(case);
        }
        fn complete(&self, required: &[&str]) -> bool {
            required.iter().all(|case| self.passed.contains(case))
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct Object {
        at: TilePos,
        ordinal: u8,
        kind: ObjectKind,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum ObjectKind {
        Door(AuthoredClosedDoor),
        Rope,
        Teleport(TilePos),
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
    fn objects(world: &BundleWorld, bytes: &[u8]) -> TestResult<Vec<Object>> {
        let doors = world.facts().doors();
        let mut objects: Vec<_> = doors
            .keys()
            .filter_map(|key| {
                let (at, ordinal) = crate::map::view::placement_of(key)?;
                Some(Object {
                    at,
                    ordinal,
                    kind: ObjectKind::Door(doors.authored_closed(key)?),
                })
            })
            .collect();
        oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
            for tile in sector.tiles {
                let at = TilePos {
                    x: tile.x,
                    y: tile.y,
                    floor: sector.floor,
                };
                let ground = world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .and_then(|tile| tile.ground())
                    .map(|(ordinal, _)| ordinal);
                for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate()
                {
                    let ordinal = u8::try_from(ordinal).map_err(|_| {
                        oteryn_world_bundle::Error::Limit("movement ordinal".into())
                    })?;
                    let kind = if let Some(to) = world
                        .floors()
                        .teleport(at)
                        .filter(|_| item.attrs.teleport.is_some())
                    {
                        ObjectKind::Teleport(to)
                    } else if ground.is_some()
                        && crate::map::palette_appearance(
                            &manifest.palette[item.palette as usize].key,
                        )
                        .is_some_and(|id| {
                            crate::map::floor::is_rope_spot_item(
                                u32::from(id),
                                ground == Some(ordinal),
                            )
                        })
                    {
                        ObjectKind::Rope
                    } else {
                        continue;
                    };
                    objects.push(Object { at, ordinal, kind });
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
    async fn target_command(client: &mut DevClientSession, payload: &[u8]) -> TestResult {
        use oteryn_protocol_oteryn::target_move::*;
        // Decode our native wire command too; no ad-hoc server-side operation.
        decode_target_move_intent(payload).map_err(|e| format!("target intent: {e:?}"))?;
        let result = client
            .session_mut()
            .gated_command(
                CAPABILITY_TARGET_MOVE_V1,
                COMMAND_TYPE_TARGET_MOVE_INTENT,
                payload,
            )
            .await?;
        if result.status != CommandStatus::Accepted || result.payload != [8, 1] {
            return Err(format!("target command: {result:?}").into());
        }
        Ok(())
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
    fn approach_stands(
        objects: &[Object],
        wanted: impl Fn(Object) -> bool,
        range: u16,
        usable_from: impl Fn(TilePos, Object) -> bool,
    ) -> BTreeMap<TilePos, Object> {
        let mut stands = BTreeMap::new();
        let radius = i32::from(range);
        for &object in objects.iter().filter(|&&object| wanted(object)) {
            for dx in -radius..=radius {
                for dy in -radius..=radius {
                    if dx.abs().max(dy.abs()) != radius || (range == 1 && dx != 0 && dy != 0) {
                        continue;
                    }
                    let (Some(x), Some(y)) = (
                        object.at.x.checked_add_signed(dx as i16),
                        object.at.y.checked_add_signed(dy as i16),
                    ) else {
                        continue;
                    };
                    let stand = TilePos {
                        x,
                        y,
                        floor: object.at.floor,
                    };
                    if usable_from(stand, object) {
                        stands.entry(stand).or_insert(object);
                    }
                }
            }
        }
        stands
    }

    async fn approach(
        client: &mut DevClientSession,
        world: &BundleWorld,
        objects: &[Object],
        wanted: impl Fn(Object) -> bool,
        range: u16,
        usable_from: impl Fn(TilePos, Object) -> bool,
    ) -> TestResult<Object> {
        let from = tile(client.world_spatial().actor_position)?;
        let stands = approach_stands(objects, wanted, range, usable_from);
        let path = route(world, from, |at| stands.contains_key(&at)).ok_or_else(|| {
            format!(
                "no reachable authored target within town budget from={from:?} candidate_stands={}",
                stands.len()
            )
        })?;
        walk(client, world, &path).await?;
        let at = tile(client.world_spatial().actor_position)?;
        stands
            .get(&at)
            .copied()
            .ok_or_else(|| format!("route missed authored approach at {at:?}").into())
    }

    fn teleport_entry(world: &BundleWorld, from: TilePos, object: Object) -> Option<StepDirection> {
        let ObjectKind::Teleport(to) = object.kind else {
            return None;
        };
        DIRS.into_iter().find(|direction| {
            let (dx, dy) = direction.delta();
            i32::from(from.x) + dx == i32::from(object.at.x)
                && i32::from(from.y) + dy == i32::from(object.at.y)
                && from.floor == object.at.floor
                && destination(world, from, *direction) == Some(to)
        })
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
        let start = TOWN;
        if !world
            .base()
            .tile(start.x, start.y, start.floor)
            .is_some_and(|tile| tile.protection_zone())
        {
            return Err("town entry is not a protection-zone tile".into());
        }
        let peer_from = tile(peer.world_spatial().actor_position)?;
        let peer_path = route(world, peer_from, |at| at == start).ok_or("peer town entry route")?;
        with_peer_liveness(client, walk(peer, world, &peer_path)).await?;
        let from = tile(client.world_spatial().actor_position)?;
        let entry_path = route(world, from, |at| at == start).ok_or("actor town entry route")?;
        with_peer_liveness(peer, walk(client, world, &entry_path)).await?;
        let (first, next) = DIRS
            .into_iter()
            .find_map(|first| {
                let adjacent = destination(world, start, first)?;
                DIRS.into_iter()
                    .find(|next| destination(world, adjacent, *next) == Some(start))
                    .map(|next| (first, next))
            })
            .ok_or("no reversible temple movement edge")?;
        let moved = client.step(first).await?;
        if moved.disposition != StepDisposition::Moved
            || moved.world_spatial_delta.is_none()
            || Some(tile(client.world_spatial().actor_position)?)
                != destination(world, start, first)
        {
            return Err("paced first step did not reach its predicted landing".into());
        }
        let second_from = tile(client.world_spatial().actor_position)?;
        let before_motion = own(client)?.motion.ok_or("actor motion forecast missing")?;
        let forecast = before_motion.prewalk_ms[usize::from(next as u8 - 1)];
        if forecast == 0 {
            return Err("occupied temple tile has zero prewalk forecast".into());
        }
        let paced_at = std::time::Instant::now();
        let moved = client.step(next).await?;
        let after_motion = own(client)?.motion.ok_or("actor step motion missing")?;
        let observed = after_motion.step_duration_ms;
        if moved.disposition != StepDisposition::Moved
            || moved.world_spatial_delta.is_none()
            || destination(world, second_from, next) != Some(start)
            || client.world_spatial().actor_position != peer.world_spatial().actor_position
            || client.world_spatial().actor_position != position(start, basis)
            || after_motion.translation_revision <= before_motion.translation_revision
            || moved.step_wait_ms != forecast
            || observed != forecast
        {
            return Err(format!(
                "paced step onto peer: forecast={forecast} observed={observed} step={moved:?}"
            )
            .into());
        }
        if paced_at.elapsed() < Duration::from_millis(40) {
            return Err("second immediate STEP bypassed the minimum server pacing beat".into());
        }
        checks.pass(
            "paced-step",
            (
                forecast,
                observed,
                paced_at.elapsed(),
                client.world_spatial(),
            ),
        );
        let here = tile(client.world_spatial().actor_position)?;
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
        with_peer_liveness(peer, stable(client, 1500)).await?;
        if client.world_spatial().actor_position == position(start, basis) {
            return Err("STOP did not interrupt an unfinished route".into());
        }
        checks.pass("stop", client.world_spatial());
        let at = client.world_spatial().actor_position;
        for direction in DIRS {
            target_command(client, &[26, 2, 8, direction as u8]).await?;
            until_with_peer(client, peer, |c| own(c).is_ok_and(|e| matches!(e.detail, EntityDetail::Actor { direction: seen, .. } if seen == direction))).await?;
            if client.world_spatial().actor_position != at {
                return Err("turn changed position".into());
            }
        }
        checks.pass("turn", own(client)?);
        let peer_at = tile(peer.world_spatial().actor_position)?;
        let actor_at = tile(client.world_spatial().actor_position)?;
        let occupied = full_e2ef3::occupied_tiles(peer);
        let path = route_avoiding(
            world,
            peer_at,
            |at| distance(at, actor_at) == 3,
            |at| !occupied.contains(&at),
        )
        .ok_or("follow peer approach")?;
        with_peer_liveness(client, walk(peer, world, &path)).await?;
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
        let peer_at = tile(peer.world_spatial().actor_position)?;
        until_with_peer(client, peer, |c| {
            tile(c.world_spatial().actor_position).is_ok_and(|at| distance(at, peer_at) == 1)
        })
        .await?;
        let follower_at = tile(client.world_spatial().actor_position)?;
        let occupied = full_e2ef3::occupied_tiles(peer);
        let path = route_avoiding(
            world,
            peer_at,
            |at| distance(at, follower_at) == 4,
            |at| !occupied.contains(&at),
        )
        .ok_or("moving follow target lane")?;
        with_peer_liveness(client, walk(peer, world, &path)).await?;
        let peer_at = tile(peer.world_spatial().actor_position)?;
        until_with_peer(client, peer, |c| {
            tile(c.world_spatial().actor_position)
                .is_ok_and(|at| at != follower_at && distance(at, peer_at) == 1)
        })
        .await?;
        target_command(client, &[16, 1]).await?;
        with_peer_liveness(peer, stable(client, 1500)).await?;
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
        Ok(client
            .use_object(&target.placement, revision)
            .await?
            .disposition)
    }
    async fn m02(
        client: &mut DevClientSession,
        world: &BundleWorld,
        objects: &[Object],
        checks: &mut Checks,
        revisions: &mut BTreeMap<(TilePos, u8), u64>,
    ) -> TestResult {
        let used = far_use(client, world, objects, checks, revisions).await;
        if let Err(error) = &used {
            evidence(&format!(
                "E2EF1_GAP row=M02 case=far-use-retry error={error}"
            ));
        }
        let moved = far_move(client, world, checks).await;
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
        let object = approach(
            client,
            world,
            objects,
            |o| o.kind == ObjectKind::Door(AuthoredClosedDoor::Plain),
            3,
            |_, _| true,
        )
        .await?;
        let before = tile(client.world_spatial().actor_position)?;
        let revision = *revisions.get(&(object.at, object.ordinal)).unwrap_or(&0);
        let disposition = door_use(client, world, object, revision).await?;
        let after = tile(client.world_spatial().actor_position)?;
        if disposition != UseDisposition::Committed
            || distance(after, object.at) > 1
            || after == before
        {
            return Err(format!(
                "far USE did not walk and commit retry: {disposition:?} from={before:?} to={after:?} target={object:?} revision={revision}"
            ).into());
        }
        checks.pass("far-use-retry", (before, object, client.world_spatial()));
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
        Ok(())
    }
    async fn far_move(
        client: &mut DevClientSession,
        world: &BundleWorld,
        checks: &mut Checks,
    ) -> TestResult {
        use oteryn_dev_client::client_runtime::inventory::MoveRequest;
        use oteryn_dev_client::{ItemMoveDestination, ItemMoveOutcome};
        use oteryn_protocol_oteryn::item_view::WorldTilePosition;
        let item = client
            .inventory()
            .and_then(|i| i.entries.first())
            .copied()
            .ok_or(
                "FAIL: no ordinarily acquired backpack item for far MOVE; no fixture mint allowed",
            )?;
        let ground = client.world_spatial().actor_position;
        let dropped = client
            .session_mut()
            .move_item_counted(MoveRequest {
                source: item.handle,
                destination: ItemMoveDestination::Ground(WorldTilePosition {
                    x: ground.x,
                    y: ground.y,
                    floor: ground.floor,
                }),
                count: None,
            })
            .await?;
        if dropped.outcome != ItemMoveOutcome::Moved {
            return Err(format!("far MOVE setup drop: {dropped:?}").into());
        }
        // Map-capable clients receive Ground handles in WORLD_MAP, not as actors.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        let handle = loop {
            let mut found = None;
            for event in client.take_events() {
                if let oteryn_dev_client::SessionEvent::WorldMap(delta) = event {
                    for tile in delta
                        .value
                        .tiles
                        .iter()
                        .filter(|tile| tile.position == ground)
                    {
                        for entry in &tile.items {
                            if entry.definition
                                == oteryn_protocol_oteryn::world_map::MapDefinition::Item(
                                    item.item_definition_ref,
                                )
                                && let oteryn_protocol_oteryn::world_map::MapOrigin::Handle(handle) =
                                    entry.origin
                            {
                                found = Some(handle);
                            }
                        }
                    }
                }
            }
            if let Some(handle) = found {
                break handle;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err("dropped item did not publish a live native map handle".into());
            }
            client.service_liveness(Duration::from_millis(25)).await?;
        };
        let from = tile(ground)?;
        let path =
            route(world, from, |at| distance(at, from) == 3).ok_or("far MOVE retreat route")?;
        walk(client, world, &path).await?;
        let before = client.world_spatial().actor_position;
        let moved = client
            .session_mut()
            .move_item_counted(MoveRequest {
                source: handle,
                destination: ItemMoveDestination::MainBackpack,
                count: None,
            })
            .await?;
        if moved.outcome != ItemMoveOutcome::Moved
            || client.world_spatial().actor_position == before
            || distance(tile(client.world_spatial().actor_position)?, from) > 1
        {
            return Err(format!("far MOVE did not walk and retry: {moved:?}").into());
        }
        until(client, |c| {
            c.inventory().is_some_and(|i| {
                i.entries.iter().any(|e| {
                    e.item_definition_ref == item.item_definition_ref && e.count == item.count
                })
            })
        })
        .await?;
        checks.pass("far-move-retry", (ground, client.world_spatial()));
        Ok(())
    }

    fn stair_landing(
        actual: Option<TilePos>,
        without_teleport: impl FnOnce() -> Option<TilePos>,
    ) -> Option<TilePos> {
        actual.filter(|at| without_teleport() == Some(*at))
    }

    async fn floor_exhaust_cast(
        client: &mut DevClientSession,
        spell: std::num::NonZeroU32,
    ) -> TestResult<(
        oteryn_protocol_oteryn::actor_spell::ActorVitals,
        oteryn_dev_client::CastOutcome,
    )> {
        use oteryn_protocol_oteryn::actor_spell::SpellTarget;

        client.take_events();
        let before = client
            .actor_vitals()
            .copied()
            .ok_or("floor exhaust vitals")?;
        let result = client.cast_spell(spell, SpellTarget::None, false).await?;
        // Vitals already pushed before the command result belong to the baseline.
        // The server emits any vitals effect of the cast after its result.
        let before_result = client
            .take_events()
            .iter()
            .rev()
            .filter_map(oteryn_dev_client::SessionEvent::actor_vitals)
            .find(|delta| delta.server_sequence < result.result_server_sequence)
            .map_or(before, |delta| delta.value);
        Ok((before_result, result))
    }

    async fn m03(
        client: &mut DevClientSession,
        world: &BundleWorld,
        objects: &[Object],
        checks: &mut Checks,
    ) -> TestResult {
        use oteryn_protocol_oteryn::actor_spell::SpellCastDisposition;
        let book = native_spell_book()?;
        let spell = book
            .spoken(FLOOR_EXHAUST_SPELL)
            .ok_or("aggressive starter spell content absent: Scorch")?
            .spell;
        if !spell.aggressive {
            return Err("Scorch is not aggressive".into());
        }
        let index = (1..=book.source_len())
            .filter_map(|i| u32::try_from(i).ok().and_then(std::num::NonZeroU32::new))
            .find(|i| book.indexed(*i).is_some_and(|s| s.key == spell.key))
            .ok_or("Scorch source index")?;
        let cooldown = Duration::from_micros(
            spell
                .groups
                .iter()
                .map(|g| g.cooldown_micros)
                .chain([spell.cooldown_micros])
                .max()
                .unwrap_or(0),
        );
        let mut ready = tokio::time::Instant::now();
        let mut exhausted = 0;
        for teleport in [false, true] {
            // Settle the spell's own cooldown before starting a distinct floor experiment.
            while tokio::time::Instant::now() < ready {
                pump(client).await?;
            }
            let result: TestResult = async {
                let from = tile(client.world_spatial().actor_position)?;
                let outside = |at: TilePos| world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| !tile.protection_zone());
                if teleport {
                    let object = approach(
                        client, world, objects,
                        |o| matches!(o.kind, ObjectKind::Teleport(to) if to.floor != o.at.floor && outside(to)),
                        1, |stand, o| teleport_entry(world, stand, o).is_some(),
                    ).await?;
                    let at = tile(client.world_spatial().actor_position)?;
                    let ObjectKind::Teleport(expected) = object.kind else { return Err("missing authored teleport".into()) };
                    let direction = teleport_entry(world, at, object).ok_or("no automatic teleport entry edge")?;
                    if at.floor != from.floor {
                        // The approach can descend the ordinary staircase. Let that floor
                        // exhaust settle before proving a new one from entering the portal.
                        let approach_exhaust = tokio::time::Instant::now()
                            + Duration::from_micros(crate::spell::STAIRHOP_MICROS);
                        while tokio::time::Instant::now() < approach_exhaust {
                            pump(client).await?;
                        }
                    }
                    walk_with_landings(client, &[direction], |origin, step| {
                        (origin == at && step == direction).then_some(expected)
                    }).await?;
                    checks.pass("teleport", (object.at, expected));
                } else {
                    let landing = |at, direction| {
                        stair_landing(destination(world, at, direction), || {
                            destination_with_teleports(world, at, direction, false)
                        })
                    };
                    let goal = |at: TilePos| at.floor != from.floor && outside(at);
                    let path = route_with_landings(from, goal, |_| true, landing)
                        .ok_or("no reachable outside-PZ stair transition")?;
                    walk_with_landings(client, &path, landing).await?;
                    if !goal(tile(client.world_spatial().actor_position)?) {
                        return Err("stairs did not reach another floor outside protection".into());
                    }
                    checks.pass("stairs", client.world_spatial());
                }
                let changed = std::time::Instant::now();
                let (before, result) = floor_exhaust_cast(client, index).await?;
                if result.disposition != SpellCastDisposition::CoolingDown
                    || result.actor_vitals_delta.is_some()
                    || client.actor_vitals() != Some(&before)
                {
                    return Err(format!("floor action did not refuse without resource spend: {result:?}").into());
                }
                let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
                loop {
                    client.service_liveness(Duration::from_millis(100)).await?;
                    let (before, result) = floor_exhaust_cast(client, index).await?;
                    if result.disposition == SpellCastDisposition::Cast {
                        if changed.elapsed() < Duration::from_millis(1800)
                            || result.actor_vitals_delta.as_ref().is_none_or(|delta| delta.value.mana >= before.mana)
                        {
                            return Err("floor exhaust expired early or post-window cast spent no mana".into());
                        }
                        ready = tokio::time::Instant::now() + cooldown;
                        break;
                    }
                    if result.disposition != SpellCastDisposition::CoolingDown
                        || result.actor_vitals_delta.is_some()
                        || client.actor_vitals() != Some(&before)
                    {
                        return Err(format!("floor exhaust retry was not a resource-free refusal: {result:?}").into());
                    }
                    if tokio::time::Instant::now() >= deadline {
                        return Err(format!("aggressive action did not become executable after floor exhaust: {result:?}").into());
                    }
                }
                exhausted += 1;
                Ok(())
            }.await;
            if let Err(error) = result {
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

    async fn push(
        client: &mut DevClientSession,
        reference: EntityRef,
        at: TilePos,
    ) -> TestResult<u8> {
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
        if result.status != CommandStatus::Accepted
            || result.payload.len() != 2
            || result.payload[0] != 8
        {
            return Err(format!("push result: {result:?}").into());
        }
        Ok(result.payload[1])
    }
    async fn m04(
        client: &mut DevClientSession,
        peer: &mut DevClientSession,
        world: &BundleWorld,
        bytes: &[u8],
        checks: &mut Checks,
    ) -> TestResult {
        let player = player_push(client, peer, world, checks).await;
        if let Err(error) = &player {
            evidence(&format!("E2EF1_GAP row=M04 case=push-player error={error}"));
        }
        let creature = creature_push(client, world, bytes, checks).await;
        if let Err(error) = &creature {
            evidence(&format!(
                "E2EF1_GAP row=M04 case=push-creature error={error}"
            ));
        }
        // Leave the rat spawn before the row's logout waits out the in-fight lock:
        // left there, the live rats kill this core Character, which then respawns
        // at the home temple beyond the core's town route budget.
        let left: TestResult = async {
            let back = route(world, tile(client.world_spatial().actor_position)?, |at| {
                at == TOWN
            })
            .ok_or("push-creature return route")?;
            walk(client, world, &back).await
        }
        .await;
        player.and(creature).and(left)
    }
    async fn player_push(
        client: &mut DevClientSession,
        peer: &mut DevClientSession,
        world: &BundleWorld,
        checks: &mut Checks,
    ) -> TestResult {
        let basis = spatial_basis::current(client)?;
        if spatial_basis::current(peer)? != basis {
            return Err("peer spatial basis differs from actor".into());
        }
        let here = tile(client.world_spatial().actor_position)?;
        let path = route(world, tile(peer.world_spatial().actor_position)?, |at| {
            distance(at, here) == 1
                && DIRS.into_iter().any(|d| {
                    destination(world, at, d)
                        .is_some_and(|next| next != here && distance(next, here) == 1)
                })
        })
        .ok_or("adjacent push-player lane")?;
        walk(peer, world, &path).await?;
        let original = tile(peer.world_spatial().actor_position)?;
        let reference = own(peer)?.entity;
        until_with_peer(client, peer, |c| {
            c.world_entities().is_some_and(|entities| {
                entities
                    .get(&reference)
                    .is_some_and(|e| e.position == position(original, basis))
            })
        })
        .await?;
        if push(client, reference, here).await? != 5 {
            return Err("occupied push destination did not refuse NotEnoughRoom".into());
        }
        stable(peer, 300).await?;
        checks.pass("collision", (reference, here, original));
        let landing = DIRS
            .into_iter()
            .filter_map(|d| destination(world, original, d))
            .find(|at| *at != here && distance(*at, here) == 1)
            .ok_or("free push landing")?;
        if push(client, reference, landing).await? != 1 {
            return Err("push-player did not move".into());
        }
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
        travel_to_spawn(client, world, &metadata.spawns, "oteryn:creature.rat").await?;
        let here = tile(client.world_spatial().actor_position)?;
        let creature = client
            .world_entities()
            .and_then(|entities| {
                entities.iter().find(|e| {
                    e.kind == EntityKind::Creature
                        && tile(e.position).is_ok_and(|at| distance(at, here) == 1)
                })
            })
            .copied()
            .ok_or("no adjacent live push creature")?;
        let from = tile(creature.position)?;
        let landing = DIRS
            .into_iter()
            .filter_map(|d| destination(world, from, d))
            .find(|at| *at != here && distance(*at, from) == 1)
            .ok_or("creature push lane")?;
        if push(client, creature.entity, landing).await? != 1 {
            return Err("live creature push did not move".into());
        }
        until(client, |c| {
            c.world_entities().is_some_and(|entities| {
                entities
                    .get(&creature.entity)
                    .is_some_and(|e| e.position == position(landing, basis))
            })
        })
        .await?;
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
                let from = client.world_spatial().actor_position;
                let detail = client.session_mut().last_refusal_detail();
                return Err(format!(
                    "door {object:?} revision={revision} from {from:?}: {result:?}, wanted {expected:?}; detail={detail:?}"
                )
                .into());
            }
            pump(client).await?;
        }
    }
    fn door_spaces_clear(
        mut occupants: impl Iterator<Item = (EntityKind, ActorPosition, bool)>,
        stand: ActorPosition,
        door: ActorPosition,
    ) -> bool {
        !occupants.any(|(kind, position, is_self)| {
            matches!(
                kind,
                EntityKind::Player | EntityKind::Creature | EntityKind::Npc
            ) && (position == door || (position == stand && !is_self))
        })
    }

    async fn wait_for_clear_door(
        client: &mut DevClientSession,
        stand: ActorPosition,
        door: ActorPosition,
    ) -> TestResult {
        until(client, |client| {
            client.world_spatial().actor_position == stand
                && client.world_entities().is_some_and(|entities| {
                    let occupants = entities.iter().map(|entity| {
                        (
                            entity.kind,
                            entity.position,
                            &entity.entity.identity == entities.own_identity(),
                        )
                    });
                    door_spaces_clear(occupants, stand, door)
                })
        })
        .await
        .map_err(|error| format!("door {door:?} approach {stand:?} did not clear: {error}").into())
    }

    async fn door_cycle(
        client: &mut DevClientSession,
        world: &BundleWorld,
        object: Object,
        revision: u64,
    ) -> TestResult {
        let basis = spatial_basis::current(client)?;
        let at = tile(client.world_spatial().actor_position)?;
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
        let door = position(object.at, basis);
        wait_for_clear_door(client, before, door).await?;
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
        wait_for_clear_door(client, before, door).await?;
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
        let mut families_ok = true;
        for (kind, case) in [
            (AuthoredClosedDoor::Plain, "basic-door"),
            (AuthoredClosedDoor::KeyLocked, "locked-refusal"),
            (AuthoredClosedDoor::KeyUnlocked, "unlocked-key-door"),
        ] {
            let result: TestResult = async {
                // Crystal 201189f8 CustomDoorTable includes the town gates 2177/2179.
                // The runtime can also report Plain for a key door without both closed forms.
                let object = approach(
                    client,
                    world,
                    objects,
                    |o| {
                        o.kind == ObjectKind::Door(kind)
                            && (kind != AuthoredClosedDoor::Plain
                                || world
                                    .base()
                                    .tile(o.at.x, o.at.y, o.at.floor)
                                    .and_then(|tile| tile.appearance(o.ordinal))
                                    .is_some_and(|id| matches!(id, 2177 | 2179))
                                    && crate::map::view::placement_key(o.at, o.ordinal)
                                        .is_some_and(|key| {
                                            world.facts().opening_door_enterable(world.base(), key)
                                        }))
                    },
                    1,
                    // End the door journey in protection so the cleanup budget
                    // is available for logout and release.
                    |stand, _| {
                        kind != AuthoredClosedDoor::KeyUnlocked || is_logout_rest_tile(world, stand)
                    },
                )
                .await?;
                if kind == AuthoredClosedDoor::KeyLocked {
                    let before = client.world_spatial().actor_position;
                    toggle(client, world, object, 0, UseDisposition::Locked).await?;
                    if client.world_spatial().actor_position != before {
                        return Err("locked door use moved actor".into());
                    }
                } else {
                    let revision = *revisions.get(&(object.at, object.ordinal)).unwrap_or(&0);
                    door_cycle(client, world, object, revision).await?;
                    revisions.insert((object.at, object.ordinal), revision + 2);
                }
                checks.pass(case, object);
                Ok(())
            }
            .await;
            if let Err(error) = result {
                families_ok = false;
                evidence(&format!("E2EF1_GAP row=M05 case={case} error={error}"));
            }
        }
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
    ) -> TestResult {
        use oteryn_protocol_oteryn::world_object::*;
        let index = provisioning::item_index()?;
        let reference = provisioning::item_reference(&index, 3003)?;
        let rope = client.inventory().and_then(|i| i.entries.iter().find(|entry| entry.item_definition_ref == reference)).copied().ok_or("FAIL: ordinarily acquired rope content:oteryn:item.tibia.i3003 absent; starter_grant.rs grants only a backpack; fixture provisioning forbidden for full evidence")?;
        checks.pass("carried-rope", rope);
        // The eastern Global rope spot has a walking route to Thais from every
        // upstairs landing. The western spot's south landing exits via a choke point.
        let rope_spot = TilePos {
            x: 32306,
            y: 32345,
            floor: -8,
        };
        let object = approach(
            client,
            world,
            objects,
            |o| o.kind == ObjectKind::Rope && o.at == rope_spot,
            1,
            |_, o| {
                DIRS.into_iter().any(|direction| {
                    let (dx, dy) = direction.delta();
                    let (Some(x), Some(y)) = (
                        o.at.x.checked_add_signed(dx as i16),
                        o.at.y.checked_add_signed(dy as i16),
                    ) else {
                        return false;
                    };
                    let floor = o.at.floor + 1;
                    world
                        .base()
                        .tile(x, y, floor)
                        .is_some_and(|tile| tile.walkable())
                        && world.enterable(TilePos { x, y, floor })
                })
            },
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

    type InventorySignature = Vec<(u32, u32, u32, u8)>;

    fn inventory_signature(client: &DevClientSession) -> TestResult<InventorySignature> {
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
        logout_by(client, tokio::time::Instant::now() + MOVEMENT_LOGOUT_WINDOW).await
    }
    async fn logout_by(
        client: &mut DevClientSession,
        deadline: tokio::time::Instant,
    ) -> TestResult {
        loop {
            let logout = client.session_mut().logout().await?;
            if logout.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("movement logout lock did not settle: {logout:?}").into());
            }
            client.service_liveness(Duration::from_millis(250)).await?;
            client.take_events();
        }
    }

    enum LandingLogoutWait {
        First,
        Busy,
        Combat(Duration),
    }

    async fn logout_after_combat(
        mut exchange: impl AsyncFnMut(
            LandingLogoutWait,
        )
            -> TestResult<oteryn_protocol_oteryn::logout::LogoutResult>,
    ) -> TestResult {
        use oteryn_protocol_oteryn::logout::LogoutResult;
        let mut release_deadline = None;
        let mut wait = LandingLogoutWait::First;
        loop {
            wait = match exchange(wait).await? {
                LogoutResult::Accepted => return Ok(()),
                // Global refreshes the lock after each hit. The time in combat
                // and the time retrying terminal release have separate owners.
                LogoutResult::InFight { retry_after_ms } => {
                    release_deadline = None;
                    LandingLogoutWait::Combat(Duration::from_millis(u64::from(retry_after_ms)))
                }
                LogoutResult::Busy => {
                    let now = tokio::time::Instant::now();
                    let deadline = release_deadline.get_or_insert(now + MOVEMENT_LOGOUT_WINDOW);
                    if now >= *deadline {
                        return Err("M07 logout release remained busy".into());
                    }
                    LandingLogoutWait::Busy
                }
            };
        }
    }

    async fn shelter_from_rope_combat(
        client: &mut DevClientSession,
        world: &BundleWorld,
        bundle: &[u8],
        landing: ActorPosition,
        fight_wait: Duration,
    ) -> TestResult {
        let ready = tokio::time::Instant::now() + fight_wait;
        let landing = tile(landing)?;
        let spawns = oteryn_world_bundle::bundle::visit(bundle, |_, _| Ok(()))?.spawns;
        let spiders: Vec<_> = spawns
            .sources
            .iter()
            .filter(|source| source.floor == landing.floor)
            .flat_map(|source| &source.points)
            .filter(|point| {
                spawns.creatures[point.creature as usize]
                    .key
                    .ends_with("spider")
            })
            .collect();
        // The shortest route crosses two authored Spider spawn cells. Use the
        // same-floor path east of the rope mouth with room to pass outside melee.
        let step = |at, direction| {
            destination(world, at, direction).filter(|next| {
                next.floor == landing.floor
                    && spiders
                        .iter()
                        .all(|spider| spider.x.abs_diff(next.x).max(spider.y.abs_diff(next.y)) > 2)
            })
        };
        let shelter =
            route_with_landings(landing, |at| is_logout_rest_tile(world, at), |_| true, step)
                .ok_or("M07 landing has no route to protection")?;
        evidence(&format!(
            "E2EF1_LOGOUT_SHELTER row=M07 from={landing:?} steps={}",
            shelter.len()
        ));
        walk_with_landings(client, &shelter, step).await?;
        let protected = tile(client.world_spatial().actor_position)?;
        if !is_logout_rest_tile(world, protected) {
            return Err("M07 shelter route missed protection".into());
        }

        // A hit while retreating may extend the last advertised expiry. The live
        // combat state keeps updating in protection, where enemies cannot prolong it.
        let deadline = tokio::time::Instant::now() + MOVEMENT_LOGOUT_WINDOW;
        loop {
            let combat = client
                .session_mut()
                .combat_state()
                .ok_or("M07 combat state absent")?;
            let now = tokio::time::Instant::now();
            if now >= ready && !combat.in_fight {
                break;
            }
            if now >= deadline {
                return Err(format!(
                    "M07 combat lock did not clear in protection at {protected:?}"
                )
                .into());
            }
            client
                .session_mut()
                .service_liveness_discarding_events(Duration::from_millis(250))
                .await?;
        }

        let back = route_with_landings(protected, |at| at == landing, |_| true, step)
            .ok_or("M07 shelter has no return route to the exact landing")?;
        walk_with_landings(client, &back, step).await
    }

    async fn logout_on_rope_landing(
        client: &mut DevClientSession,
        world: &BundleWorld,
        bundle: &[u8],
        landing: ActorPosition,
    ) -> TestResult {
        use oteryn_protocol_oteryn::logout::LogoutResult;

        let mut used_shelter = false;
        logout_after_combat(async |wait| {
            match wait {
                LandingLogoutWait::First => {}
                LandingLogoutWait::Busy => {
                    client.session_mut().service_liveness_discarding_events(Duration::from_millis(250)).await?;
                }
                LandingLogoutWait::Combat(wait) if !used_shelter => {
                    shelter_from_rope_combat(client, world, bundle, landing, wait).await?;
                    used_shelter = true;
                }
                LandingLogoutWait::Combat(wait) => {
                    client.session_mut().service_liveness_discarding_events(wait).await?;
                }
            }
            let position = client.world_spatial().actor_position;
            let vitals = client.actor_vitals();
            if position != landing || vitals.is_some_and(|vitals| vitals.health == 0) {
                return Err(format!("M07 actor lost the rope landing while awaiting logout: landing={landing:?} actual={position:?} vitals={vitals:?}").into());
            }
            let logout = client.session_mut().logout().await?;
            if let Some(LogoutResult::InFight { retry_after_ms }) = logout.result {
                let nearby: Vec<_> = client
                    .world_entities()
                    .into_iter()
                    .flat_map(|entities| entities.iter())
                    .filter(|entity| {
                        entity.kind == EntityKind::Creature && entity.position.floor == landing.floor
                    })
                    .map(|entity| (entity.display_name.as_ref().and_then(|name| name.as_str().ok()), entity.position))
                    .collect();
                evidence(&format!(
                    "E2EF1_LOGOUT_FIGHT row=M07 position={landing:?} retry_after_ms={retry_after_ms} vitals={:?} creatures={nearby:?}",
                    client.actor_vitals(),
                ));
            }
            logout.result.ok_or_else(|| format!("M07 landing logout refused: {logout:?}").into())
        })
        .await
    }
    /// M05 cleanup after its door checks (#3373): the doors leave the actor outside the
    /// town protection zone, where a live creature kept refreshing the 60 s PvE lock past
    /// the logout deadline. Log every visible creature as the lock-holder evidence, then
    /// walk into the nearest protection-zone tile, where the server admits the logout.
    /// The walk counts against the same `deadline` as the logout that follows it.
    async fn leave_fight(
        client: &mut DevClientSession,
        world: &BundleWorld,
        deadline: tokio::time::Instant,
    ) -> TestResult {
        let at = client.world_spatial().actor_position;
        let here = tile(at)?;
        let creatures: Vec<_> = client
            .world_entities()
            .map(|entities| {
                entities
                    .iter()
                    .filter(|e| e.kind == EntityKind::Creature)
                    .copied()
                    .collect()
            })
            .unwrap_or_default();
        for creature in creatures {
            evidence(&format!(
                "E2EF1_FIGHT_HOLDER actor={at:?} entity={:?} position={:?} vitals={:?}",
                creature.entity,
                creature.position,
                client.actor_vitals()
            ));
        }
        let resting_tile = |cell| is_logout_rest_tile(world, cell);
        if !resting_tile(here) {
            let path = route(world, here, resting_tile)
                .ok_or("no reachable protection zone that admits logout")?;
            evidence(&format!(
                "E2EF1_LEAVE_FIGHT from={at:?} steps={}",
                path.len()
            ));
            walk(client, world, &path).await?;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("M05 fight cleanup exceeded the logout deadline".into());
        }
        Ok(())
    }

    struct RestingLogout {
        position: ActorPosition,
        inventory: InventorySignature,
        session: crate::foundation::GameSessionId,
    }

    async fn logout_at_rest(
        client: &mut DevClientSession,
        world: &BundleWorld,
    ) -> TestResult<RestingLogout> {
        let deadline = tokio::time::Instant::now() + MOVEMENT_LOGOUT_WINDOW;
        tokio::time::timeout_at(deadline, leave_fight(client, world, deadline))
            .await
            .map_err(|_| "movement logout cleanup exceeded deadline reaching protection")??;
        let saved = RestingLogout {
            position: client.world_spatial().actor_position,
            inventory: inventory_signature(client)?,
            session: client.join_snapshot().game_session_id,
        };
        tokio::time::timeout_at(deadline, logout_by(client, deadline))
            .await
            .map_err(|_| "movement logout cleanup exceeded deadline awaiting logout")??;
        Ok(saved)
    }

    async fn m05_independent(
        clients: &SeamClients<'_>,
        world: &BundleWorld,
        objects: &[Object],
        checks: &mut Checks,
        revisions: &mut BTreeMap<(TilePos, u8), u64>,
    ) -> TestResult {
        let mut door_client = connect_character(clients, 0xa4, 0).await?;
        let origin = tile(door_client.world_spatial().actor_position)?;
        let doors = m05(&mut door_client, world, objects, checks, revisions).await;
        if let Err(error) = &doors {
            evidence(&format!("E2EF1_GAP row=M05 phase=doors error={error}"));
        }
        let saved = logout_at_rest(&mut door_client, world).await?;
        drop(door_client);
        let mut fresh = connect_character(clients, 0xb4, 0).await?;
        let durable = fresh.world_spatial().actor_position == saved.position
            && fresh.join_snapshot().game_session_id != saved.session
            && inventory_signature(&fresh)? == saved.inventory;
        let back =
            route(world, tile(saved.position)?, |at| at == origin).ok_or("M05 return route")?;
        walk(&mut fresh, world, &back).await?;
        logout(&mut fresh).await?;
        if !durable {
            return Err("M05 fresh admission did not preserve position and inventory".into());
        }
        doors
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
        let objects = objects(world, &bytes)?;
        let origin = tile(client.world_spatial().actor_position)?;
        let mut failed = false;
        let mut revisions = BTreeMap::new();
        for (ordinal, (row, required)) in ROWS.iter().enumerate() {
            let started = std::time::Instant::now();
            verdicts.current = Some(row);
            let mut checks = Checks {
                row,
                ..Checks::default()
            };
            if *row == "M05" {
                // Retire the primary session before the door journey so a parked
                // actor cannot take damage while waiting for M07.
                let saved = logout_at_rest(&mut client, world).await?;
                drop(client);
                let result =
                    m05_independent(clients, world, &objects, &mut checks, &mut revisions).await;
                if let Err(error) = &result {
                    evidence(&format!("E2EF1_GAP row=M05 error={error}"));
                }
                failed |= !record_row(
                    verdicts,
                    row,
                    required,
                    started,
                    &checks,
                    &result,
                    result.is_ok(),
                );
                client = connect_world(clients, 0x94).await?;
                if client.world_spatial().actor_position != saved.position
                    || client.join_snapshot().game_session_id == saved.session
                    || inventory_signature(&client)? != saved.inventory
                {
                    return Err(
                        "movement primary fresh admission changed saved position or inventory"
                            .into(),
                    );
                }
                let back = route(world, tile(saved.position)?, |at| at == origin)
                    .ok_or("movement primary has no return route")?;
                walk(&mut client, world, &back).await?;
                continue;
            }
            let mut peer = if matches!(*row, "M01" | "M04") {
                Some(
                    with_peer_liveness(
                        &mut client,
                        connect_character(clients, 0xa0 + u8::try_from(ordinal)?, 0),
                    )
                    .await?,
                )
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
                "M02" => m02(&mut client, world, &objects, &mut checks, &mut revisions).await,
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
                "M07" => m07(&mut client, world, &objects, &mut checks).await,
                _ => unreachable!(),
            };
            if let Err(error) = &result {
                evidence(&format!("E2EF1_GAP row={row} error={error}"));
            }
            // Compare the actual saved landing after a fresh admission.
            let saved = client.world_spatial().actor_position;
            let inventory = inventory_signature(&client)?;
            let old_session = client.join_snapshot().game_session_id;
            let mut peer_durable = true;
            if let Some(mut peer) = peer.take() {
                peer_durable = with_peer_liveness(&mut client, async {
                    let saved = peer.world_spatial().actor_position;
                    let previous = peer.join_snapshot().game_session_id;
                    if *row != "M04" || !checks.passed.contains("push-player") {
                        logout(&mut peer).await?;
                    }
                    drop(peer);
                    let mut peer =
                        connect_character(clients, 0xc0 + u8::try_from(ordinal)?, 0).await?;
                    let durable = peer.world_spatial().actor_position == saved
                        && peer.join_snapshot().game_session_id != previous;
                    let back =
                        route(world, tile(saved)?, |at| at == TOWN).ok_or("peer return route")?;
                    walk(&mut peer, world, &back).await?;
                    logout(&mut peer).await?;
                    Ok(durable)
                })
                .await?;
            }
            if *row == "M07" && checks.passed.contains("rope-ascent") {
                logout_on_rope_landing(&mut client, world, &bytes, saved).await?;
            } else {
                logout(&mut client).await?;
            }
            drop(client);
            client = connect_world(clients, 0xb0 + u8::try_from(ordinal)?).await?;
            let durable = client.world_spatial().actor_position == saved
                && client.join_snapshot().game_session_id != old_session
                && inventory_signature(&client)? == inventory
                && peer_durable;
            if durable && required.contains(&"relogin") {
                checks.pass("relogin", saved);
            }
            failed |= !record_row(verdicts, row, required, started, &checks, &result, durable);
            let back = route(world, tile(client.world_spatial().actor_position)?, |at| {
                at == origin
            })
            .ok_or("movement full row has no return route")?;
            walk(&mut client, world, &back).await?;
        }
        if failed {
            evidence("E2EF1_INCOMPLETE full_row_failures_reported_to_e2e_all");
        }
        Ok(client)
    }

    fn record_row(
        verdicts: &mut Verdicts,
        row: &'static str,
        required: &[&str],
        started: std::time::Instant,
        checks: &Checks,
        result: &TestResult,
        durable: bool,
    ) -> bool {
        let passed = result.is_ok() && durable && checks.complete(required);
        let gaps: Vec<_> = required
            .iter()
            .filter(|case| !checks.passed.contains(**case))
            .copied()
            .collect();
        evidence(&format!(
            "E2EF1_ROW row={row} missing={gaps:?} result={result:?} durable={durable}"
        ));
        verdicts.done.insert(row);
        let suffix = if passed {
            String::new()
        } else {
            let cause = match result {
                Err(error) => verdict::classify(error.as_ref()),
                Ok(()) => verdict::Cause::Assertion,
            };
            cause.suffix(&format!(
                "{row}/{}",
                gaps.first().copied().unwrap_or("durable")
            ))
        };
        evidence(&format!(
            "e2e_all scenario={row}.full status={} seconds={:.3} evidence=journey:apps/game-server/src/qualification/client_phases/full_e2ef1.rs{suffix}",
            if passed { "PASS" } else { "FAIL" },
            started.elapsed().as_secs_f64()
        ));
        passed
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[tokio::test(start_paused = true)]
        async fn qualification_m07_logout_follows_refreshed_combat_expiry() -> TestResult {
            use oteryn_protocol_oteryn::logout::LogoutResult;

            let start = tokio::time::Instant::now();
            let mut attempts = Vec::new();
            logout_after_combat(async |wait| {
                let wait = match wait {
                    LandingLogoutWait::First => Duration::ZERO,
                    LandingLogoutWait::Combat(wait) => wait,
                    LandingLogoutWait::Busy => Duration::from_millis(250),
                };
                tokio::time::advance(wait).await;
                let elapsed = start.elapsed().as_millis();
                attempts.push(elapsed);
                Ok(match elapsed {
                    0 => LogoutResult::Busy,
                    250 => LogoutResult::InFight {
                        retry_after_ms: 60_000,
                    },
                    // A hit 55 seconds into the lock renews it through the second wait.
                    60_250 => LogoutResult::InFight {
                        retry_after_ms: 55_000,
                    },
                    115_250 => LogoutResult::Busy,
                    115_500 => LogoutResult::Accepted,
                    _ => return Err(format!("unexpected logout attempt at {elapsed} ms").into()),
                })
            })
            .await?;
            assert_eq!(attempts, [0, 250, 60_250, 115_250, 115_500]);
            Ok(())
        }

        #[test]
        fn qualification_m05_door_requires_a_clear_approach_and_threshold() {
            let stand = ActorPosition {
                x: 32401,
                y: 32235,
                floor: 7,
            };
            let door = ActorPosition { x: 32402, ..stand };
            let own = (EntityKind::Player, stand, true);
            assert!(door_spaces_clear([own].into_iter(), stand, door));
            for kind in [EntityKind::Creature, EntityKind::Npc, EntityKind::Player] {
                for position in [stand, door] {
                    assert!(!door_spaces_clear(
                        [own, (kind, position, false)].into_iter(),
                        stand,
                        door
                    ));
                }
            }
            assert!(!door_spaces_clear(
                [(EntityKind::Player, door, true)].into_iter(),
                stand,
                door
            ));
        }

        #[test]
        fn qualification_m05_failed_cleanup_records_only_its_own_row() {
            let mut verdicts = Verdicts::default();
            let checks = Checks {
                row: "M05",
                ..Checks::default()
            };
            assert!(!record_row(
                &mut verdicts,
                "M05",
                ROWS[4].1,
                std::time::Instant::now(),
                &checks,
                &Err("M05 fight cleanup exceeded the logout deadline".into()),
                false,
            ));
            assert!(verdicts.done.contains("M05"));
            assert!(!verdicts.done.contains("M07"));
            assert!(verdicts.aborted.is_none());
        }

        #[test]
        fn qualification_review_m03_stairs_exclude_teleport_landing() {
            let upstairs = TilePos {
                floor: TOWN.floor + 1,
                ..TOWN
            };
            assert_eq!(stair_landing(Some(upstairs), || Some(TOWN)), None);
            assert_eq!(
                stair_landing(Some(upstairs), || Some(upstairs)),
                Some(upstairs)
            );
        }

        #[test]
        fn qualification_e2ef1_floor_exhaust_uses_a_served_starter_spell() -> TestResult {
            let spell = native_spell_book()?
                .spoken(FLOOR_EXHAUST_SPELL)
                .ok_or("missing floor exhaust spell")?
                .spell;
            assert!(spell.aggressive);
            assert_eq!(spell.level, 1);
            assert!(spell.vocations.contains(&crate::spell::Vocation::Sorcerer));
            assert!(!spell.learning_required && !spell.premium && !spell.needs_target);
            Ok(())
        }

        #[test]
        fn qualification_e2ef1_approach_can_select_authored_targets_on_another_floor() -> TestResult
        {
            let stand = TilePos {
                floor: TOWN.floor - 1,
                ..TOWN
            };
            let door = Object {
                at: TilePos {
                    x: stand.x + 1,
                    ..stand
                },
                ordinal: 2,
                kind: ObjectKind::Door(AuthoredClosedDoor::KeyLocked),
            };
            assert_eq!(distance(TOWN, door.at), u16::MAX);
            let stands = approach_stands(&[door], |o| o.kind == door.kind, 1, |_, _| true);
            let path = find_route(
                TOWN,
                |at| stands.contains_key(&at),
                |at, direction| (at == TOWN && direction == StepDirection::North).then_some(stand),
            )
            .map_err(|visited| format!("visited {visited} tiles without finding authored stand"))?;
            assert_eq!(path, [StepDirection::North]);
            assert_eq!(stands[&stand].ordinal, door.ordinal);
            Ok(())
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
