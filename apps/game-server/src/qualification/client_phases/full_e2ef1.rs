// E2EF1: native movement commands against node_boot's production-loaded world.
// Mount as the provisioning phase: run BEFORE its control-socket fixtures.
use super::*;
#[path = "provision.rs"]
mod original;

pub(super) fn item_index() -> TestResult<crate::content::item_ref::ItemDefinitionIndex> {
    original::item_index()
}
pub(super) fn item_reference(
    index: &crate::content::item_ref::ItemDefinitionIndex,
    id: u32,
) -> TestResult<std::num::NonZeroU32> {
    original::item_reference(index, id)
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
    let mut movement = e2ef1::run(movement, clients, &world).await?;
    e2ef1::logout(&mut movement).await?;
    drop(movement);
    let client = connect_character(clients, 0x91, index).await?;
    original::provision(client, clients, index).await
}
mod e2ef1 {
    use super::*;
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
    const DIRS: [StepDirection; 4] = [
        StepDirection::North,
        StepDirection::East,
        StepDirection::South,
        StepDirection::West,
    ];

    #[derive(Default)]
    struct Verdicts(BTreeSet<&'static str>);
    impl Drop for Verdicts {
        fn drop(&mut self) {
            for (row, _) in ROWS {
                if !self.0.contains(row) {
                    evidence(&format!(
                        "e2e_all scenario={row}.full status=FAIL seconds=0 evidence=journey-aborted:apps/game-server/src/qualification/client_phases/full_e2ef1.rs"
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
        appearance: u16,
    }
    fn position(at: TilePos) -> ActorPosition {
        ActorPosition {
            x: i32::from(at.x),
            y: i32::from(at.y),
            floor: -i16::from(at.floor),
        }
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
        use oteryn_dev_client::client_runtime::target_move::{WalkDisposition, send};
        let result = send(client.session_mut(), position(at)).await?;
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
    async fn approach(
        client: &mut DevClientSession,
        world: &BundleWorld,
        objects: &[Object],
        wanted: impl Fn(Object) -> bool,
        range: u16,
    ) -> TestResult<Object> {
        let from = tile(client.world_spatial().actor_position)?;
        for &object in objects
            .iter()
            .filter(|object| wanted(**object) && distance(from, object.at) <= 160)
        {
            if let Some(path) = route(world, from, |at| {
                distance(at, object.at) == range
                    && (range != 1 || at.x == object.at.x || at.y == object.at.y)
            }) {
                walk(client, world, &path).await?;
                return Ok(object);
            }
        }
        Err("no reachable exact authored target within town budget".into())
    }

    async fn m01(
        client: &mut DevClientSession,
        peer: &mut DevClientSession,
        world: &BundleWorld,
        checks: &mut Checks,
    ) -> TestResult {
        let start = tile(client.world_spatial().actor_position)?;
        let path = route(world, start, |at| distance(at, start) == 4)
            .ok_or("no four-tile movement lane")?;
        let first = *path.first().ok_or("empty movement lane")?;
        let moved = client.step(first).await?;
        if moved.disposition != StepDisposition::Moved
            || moved.world_spatial_delta.is_none()
            || Some(tile(client.world_spatial().actor_position)?)
                != destination(world, start, first)
        {
            return Err("paced first step did not reach its predicted landing".into());
        }
        let next = *path.get(1).ok_or("short movement lane")?;
        let second_from = tile(client.world_spatial().actor_position)?;
        let paced_at = std::time::Instant::now();
        let moved = client.step(next).await?;
        if moved.disposition != StepDisposition::Moved
            || moved.world_spatial_delta.is_none()
            || Some(tile(client.world_spatial().actor_position)?)
                != destination(world, second_from, next)
        {
            return Err("paced second step did not reach its predicted landing".into());
        }
        if paced_at.elapsed() < Duration::from_millis(40) {
            return Err("second immediate STEP bypassed the minimum server pacing beat".into());
        }
        checks.pass("paced-step", (paced_at.elapsed(), client.world_spatial()));
        let here = tile(client.world_spatial().actor_position)?;
        let goal_path =
            route(world, here, |at| distance(at, here) == 4).ok_or("autowalk lane absent")?;
        let mut goal = here;
        for direction in goal_path {
            goal = destination(world, goal, direction).ok_or("autowalk prediction")?;
        }
        autowalk(client, goal).await?;
        until_with_peer(client, peer, |c| {
            c.world_spatial().actor_position == position(goal)
        })
        .await?;
        checks.pass("autowalk", goal);
        autowalk(client, start).await?;
        target_command(client, &[16, 1]).await?;
        stable(client, 1500).await?;
        if client.world_spatial().actor_position == position(start) {
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
        let path = route(world, peer_at, |at| {
            distance(
                at,
                tile(client.world_spatial().actor_position).unwrap_or(start),
            ) == 3
        })
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
        let peer_at = tile(peer.world_spatial().actor_position)?;
        until_with_peer(client, peer, |c| {
            tile(c.world_spatial().actor_position).is_ok_and(|at| distance(at, peer_at) == 1)
        })
        .await?;
        let follower_at = tile(client.world_spatial().actor_position)?;
        let path = route(world, peer_at, |at| distance(at, follower_at) == 4)
            .ok_or("moving follow target lane")?;
        walk(peer, world, &path).await?;
        let peer_at = tile(peer.world_spatial().actor_position)?;
        until_with_peer(client, peer, |c| {
            tile(c.world_spatial().actor_position)
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
        // 1638/1639 is a plain door family (not a key, level or quest door).
        let object = approach(
            client,
            world,
            objects,
            |o| o.appearance == 1638 && world.facts().doors().at(o.at),
            3,
        )
        .await?;
        let before = tile(client.world_spatial().actor_position)?;
        let revision = *revisions.get(&(object.at, object.ordinal)).unwrap_or(&0);
        if door_use(client, world, object, revision).await? != UseDisposition::Committed
            || distance(tile(client.world_spatial().actor_position)?, object.at) > 1
            || tile(client.world_spatial().actor_position)? == before
        {
            return Err("far USE did not walk and commit retry".into());
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
        use oteryn_dev_client::{ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome};
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
            .move_item(&ItemMoveIntent {
                source: item.handle,
                destination: ItemMoveDestination::Ground(WorldTilePosition {
                    x: ground.x,
                    y: ground.y,
                    floor: ground.floor,
                }),
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
            .move_item(&ItemMoveIntent {
                source: handle,
                destination: ItemMoveDestination::MainBackpack,
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

    async fn m03(
        client: &mut DevClientSession,
        world: &BundleWorld,
        objects: &[Object],
        checks: &mut Checks,
    ) -> TestResult {
        use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
        let book = native_spell_book()?;
        let spell = book
            .spoken("exevo flam hur")
            .ok_or("aggressive spell content absent: flame wave")?
            .spell;
        if !spell.aggressive {
            return Err("flame wave is not aggressive".into());
        }
        let index = (1..=book.source_len())
            .filter_map(|i| u32::try_from(i).ok().and_then(std::num::NonZeroU32::new))
            .find(|i| book.indexed(*i).is_some_and(|s| s.key == spell.key))
            .ok_or("flame wave source index")?;
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
                    let object = approach(client, world, objects, |o| world.floors().teleport(o.at).is_some_and(|to| to.floor != o.at.floor && outside(to)), 1).await?;
                    let at = tile(client.world_spatial().actor_position)?;
                    let expected = world.floors().teleport(object.at).ok_or("teleport destination")?;
                    let direction = DIRS.into_iter().find(|d| destination(world, at, *d) == Some(expected)).ok_or("no automatic teleport entry edge")?;
                    walk(client, world, &[direction]).await?;
                    checks.pass("teleport", (object.at, expected));
                } else {
                    let path = route(world, from, |at| at.floor != from.floor && outside(at)).ok_or("no reachable outside-PZ stair transition")?;
                    walk(client, world, &path).await?;
                    if tile(client.world_spatial().actor_position)?.floor == from.floor { return Err("stairs did not change floor".into()); }
                    checks.pass("stairs", client.world_spatial());
                }
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
        use oteryn_protocol_oteryn::creature_push::*;
        let payload = encode_creature_push_intent(&CreaturePushIntent {
            target: reference,
            destination: position(at),
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
        player.and(creature)
    }
    async fn player_push(
        client: &mut DevClientSession,
        peer: &mut DevClientSession,
        world: &BundleWorld,
        checks: &mut Checks,
    ) -> TestResult {
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
                    .is_some_and(|e| e.position == position(original))
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
            c.world_spatial().actor_position == position(landing)
        })
        .await?;
        until_with_peer(client, peer, |c| {
            c.world_entities().is_some_and(|entities| {
                entities
                    .get(&reference)
                    .is_some_and(|e| e.position == position(landing))
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
                    .is_some_and(|e| e.position == position(landing))
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
        toggle(client, world, object, revision, UseDisposition::Committed).await?;
        // The oracle keeps immutable closed-door collision. Here only the server
        // knows the new door state, so assert the actual native crossing directly.
        if client.step_retrying(direction).await?.disposition != StepDisposition::Moved
            || client.world_spatial().actor_position != position(object.at)
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
    ) -> TestResult {
        use oteryn_protocol_oteryn::world_object::*;
        let index = provisioning::item_index()?;
        let reference = provisioning::item_reference(&index, 3003)?;
        let rope = client.inventory().and_then(|i| i.entries.iter().find(|entry| entry.item_definition_ref == reference)).copied().ok_or("FAIL: ordinarily acquired rope content:oteryn:item.tibia.i3003 absent; starter_grant.rs grants only a backpack; fixture provisioning forbidden for full evidence")?;
        checks.pass("carried-rope", rope);
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
        let result = client
            .session_mut()
            .gated_command(CAPABILITY_ITEM_USE_V1, COMMAND_TYPE_USE_INTENT, &payload)
            .await?;
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
        mut client: DevClientSession,
        clients: &SeamClients<'_>,
        world: &BundleWorld,
    ) -> TestResult<DevClientSession> {
        let mut verdicts = Verdicts::default();
        let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
        let objects = objects(&bytes)?;
        let origin = tile(client.world_spatial().actor_position)?;
        let mut failed = false;
        let mut revisions = BTreeMap::new();
        for (ordinal, (row, required)) in ROWS.iter().enumerate() {
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
                "M05" => m05(&mut client, world, &objects, &mut checks, &mut revisions).await,
                "M07" => m07(&mut client, world, &objects, &mut checks).await,
                _ => unreachable!(),
            };
            if let Err(error) = &result {
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
                let back =
                    route(world, tile(saved)?, |at| at == TOWN).ok_or("peer return route")?;
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
            let passed = result.is_ok() && durable && checks.complete(required);
            failed |= !passed;
            let gaps: Vec<_> = required
                .iter()
                .filter(|case| !checks.passed.contains(**case))
                .copied()
                .collect();
            evidence(&format!(
                "E2EF1_ROW row={row} missing={gaps:?} result={result:?} durable={durable}"
            ));
            verdicts.0.insert(row);
            evidence(&format!(
                "e2e_all scenario={row}.full status={} seconds={:.3} evidence=journey:apps/game-server/src/qualification/client_phases/full_e2ef1.rs",
                if passed { "PASS" } else { "FAIL" },
                started.elapsed().as_secs_f64()
            ));
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

    #[cfg(test)]
    mod tests {
        use super::*;
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
