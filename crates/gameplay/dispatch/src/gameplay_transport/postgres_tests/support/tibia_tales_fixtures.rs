use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_gameplay_world::map::view::MapFacts;
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

use crate::gameplay_transport::connection::FreshAdmissionAuthority;

use super::classic_quests_fixtures::{install, talk};
use super::fixture::{Harness, debug};
use super::m39_bone_journey_fixtures::{carried, stage, use_at};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

const ROOT: &str = "oteryn:quest-progress/quest/u8_1/";

pub(super) async fn exercise(
    name: &str,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    target: Position,
) -> TestResult {
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 4, 6, 11, 15, 18]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("Tibia Tales capabilities")?;

    if name.ends_with("-hallowed") {
        return Box::pin(hallowed(owner, admitted, harness)).await;
    }
    if name.ends_with("-appease") {
        use NpcIntentDisposition::{Ok as Accepted, Rejected};

        let track = format!("{ROOT}tibia_tales/to_appease_the_mighty_quest");
        assert_eq!(stage(harness, &track).await?, None);
        let [kazzan, melchior, umar, ubaid] = install(
            owner,
            admitted,
            [
                "oteryn:npc.kazzan",
                "oteryn:npc.melchior",
                "oteryn:npc.umar",
                "oteryn:npc.ubaid",
            ],
        )
        .await?;
        for (index, (npc, phrase, expected, disposition)) in [
            (kazzan, "hi", None, Accepted),
            (kazzan, "mission", None, Accepted),
            (kazzan, "yes", None, Accepted),
            (kazzan, "yes", Some(1), Accepted),
            (kazzan, "report", Some(1), Rejected),
            (kazzan, "bye", Some(1), Accepted),
            (melchior, "hi", Some(1), Accepted),
            (melchior, "word of greeting", Some(1), Accepted),
            (melchior, "bye", Some(1), Accepted),
            (umar, "djanni'hah", Some(1), Accepted),
            (umar, "mission", Some(1), Accepted),
            (umar, "kazzan", Some(2), Accepted),
            (umar, "bye", Some(2), Accepted),
            (kazzan, "hi", Some(2), Accepted),
            (kazzan, "report", Some(2), Rejected),
            (kazzan, "bye", Some(2), Accepted),
            (ubaid, "djanni'hah", Some(2), Accepted),
            (ubaid, "mission", Some(2), Accepted),
            (ubaid, "kazzan", Some(3), Accepted),
            (ubaid, "bye", Some(3), Accepted),
            (kazzan, "hi", Some(3), Accepted),
            (kazzan, "mission", Some(4), Accepted),
            (kazzan, "report", Some(4), Rejected),
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(
                super::qengine_rewards_fixtures::talk(
                    owner,
                    admitted,
                    npc,
                    1000 + u64::try_from(index)?,
                    phrase,
                )
                .await?,
                disposition,
                "{phrase}"
            );
            assert_eq!(stage(harness, &track).await?, expected, "{phrase}");
        }
        assert_eq!(carried(harness, 3035).await?, 20);
        return Ok(());
    }

    let (npc, suffix, tool, transformed, rewards, terminal) = if name.ends_with("-booze") {
        (
            "oteryn:npc.boozer",
            "tibia_tales/ultimate_booze_quest",
            Some(138),
            Some(136),
            &[(5710, 1), (3035, 10)][..],
            3,
        )
    } else if name.ends_with("-exterminator") {
        (
            "oteryn:npc.padreia",
            "tibia_tales/the_exterminator",
            Some(135),
            Some(2874),
            &[(3033, 1), (3032, 1), (3030, 1), (3029, 1)][..],
            3,
        )
    } else {
        (
            "oteryn:npc.budrik",
            "to_outfox_afox_quest",
            None,
            Some(139),
            &[(875, 1)][..],
            2,
        )
    };
    let track = format!("{ROOT}{suffix}");
    let [npc] = install(owner, admitted, [npc]).await?;
    assert_eq!(stage(harness, &track).await?, None);
    for (id, phrase) in [(1001, "hi"), (1002, "mission"), (1003, "yes")] {
        talk(owner, admitted, npc, id, phrase).await?;
    }
    assert_eq!(stage(harness, &track).await?, Some(1));
    let initial_xp: i64 =
        sqlx::query_scalar("SELECT total_experience::bigint FROM game_character_progression_state")
            .fetch_one(&harness.pool)
            .await?;
    if let Some(tool) = tool {
        assert_eq!(carried(harness, tool).await?, 1);
        assert_eq!(
            use_tool(owner, admitted, target, tool).await?,
            UseDisposition::Committed
        );
        assert_eq!(stage(harness, &track).await?, Some(2));
        assert_eq!(carried(harness, tool).await?, 0);
    } else {
        assert_eq!(
            use_at(owner, admitted, target).await?.0,
            UseDisposition::Committed
        );
        assert_eq!(stage(harness, &track).await?, Some(1));
    }
    if let Some(item) = transformed {
        assert_eq!(carried(harness, item).await?, 1);
    }
    talk(owner, admitted, npc, 1004, "hi").await?;
    talk(owner, admitted, npc, 1005, "mission").await?;
    assert_eq!(stage(harness, &track).await?, Some(terminal));
    for &(item, quantity) in rewards {
        assert_eq!(
            carried(harness, item).await?,
            quantity,
            "reward item {item}"
        );
    }
    let remaining = if name.ends_with("-exterminator") {
        1
    } else {
        0
    };
    assert_eq!(
        carried(harness, transformed.ok_or("quest item")?).await?,
        remaining
    );
    let experience: i64 =
        sqlx::query_scalar("SELECT total_experience::bigint FROM game_character_progression_state")
            .fetch_one(&harness.pool)
            .await?;
    assert_eq!(
        experience - initial_xp,
        if name.ends_with("-booze") { 100 } else { 0 }
    );
    Ok(())
}

async fn use_tool(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    target: Position,
    tool: u32,
) -> TestResult<UseDisposition> {
    use_tool_at(owner, admitted, target, 1, tool).await
}

pub(super) async fn use_tool_at(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    target: Position,
    ordinal: u8,
    tool: u32,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let map = owner.bundle_map().ok_or("map")?;
    let placement_key =
        oteryn_world_bundle::bundle::placement_key(target.floor, target.x, target.y, ordinal)
            .ok_or("target placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&placement_key.to_be_bytes());
    let item = format!("oteryn:item.tibia.i{tool}");
    let index = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|native| native.item_index())
        .ok_or("item index")?;
    let definition = index
        .definition_ref("Item", &item, index.revision(&item).ok_or("tool revision")?)
        .ok_or("tool definition")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(definition),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(placement_key),
        })),
        use_with_position: None,
    })
    .map_err(debug)?;
    let (next, disposition, _) = super::item_craft_fixtures::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        Some(definition),
    )
    .await?;
    *admitted = next;
    Ok(disposition)
}

const GRAVES: &[(Position, Position, &[u32])] = &[
    (
        pos(33188, 31688, -6),
        pos(33189, 31689, -6),
        &[103, 4453, 1979],
    ),
    (pos(33190, 31691, -6), pos(33190, 31690, -6), &[103, 1979]),
    (pos(33192, 31688, -6), pos(33191, 31688, -6), &[103, 1979]),
    (pos(33196, 31691, -6), pos(33195, 31691, -6), &[103, 1979]),
    (pos(33197, 31687, -6), pos(33196, 31687, -6), &[103, 1979]),
    (pos(33202, 31689, -6), pos(33203, 31689, -6), &[103, 1979]),
    (pos(33203, 31687, -6), pos(33203, 31686, -6), &[103, 1979]),
    (pos(33204, 31688, -6), pos(33205, 31688, -6), &[103, 1979]),
    (pos(33203, 31682, -6), pos(33204, 31682, -6), &[103, 1979]),
    (
        pos(33205, 31680, -6),
        pos(33206, 31680, -6),
        &[103, 6482, 1979],
    ),
    (pos(33210, 31670, -7), pos(33211, 31670, -7), &[992, 1979]),
    (
        pos(33215, 31667, -7),
        pos(33214, 31666, -7),
        &[103, 997, 1979],
    ),
    (
        pos(33219, 31671, -7),
        pos(33220, 31672, -7),
        &[103, 1000, 1979],
    ),
    (pos(33223, 31665, -7), pos(33223, 31666, -7), &[992, 1979]),
    (pos(33217, 31661, -7), pos(33216, 31661, -7), &[4525, 1979]),
];

const fn pos(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}

pub(super) const HALLOWED_START: Position = pos(33189, 31689, -6);
const HALLOWED_NPCS: [Position; 2] = [pos(33189, 31690, -6), pos(33191, 31690, -6)];

pub(super) fn hallowed_world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups_fixtures::{ExpectedTile, NativeCrop, native_scene};
    let mut crops = Vec::new();
    for (at, items) in GRAVES
        .iter()
        .flat_map(|&(target, approach, ids)| [(target, ids), (approach, &[][..])])
        .chain(HALLOWED_NPCS.into_iter().map(|at| (at, &[][..])))
    {
        let bytes = if at.floor == -6 {
            include_bytes!("../../../../../../../content/world/placements/region-z06-x129-y123.b3")
                .as_slice()
        } else {
            include_bytes!("../../../../../../../content/world/placements/region-z07-x129-y123.b3")
                .as_slice()
        };
        crops.push(NativeCrop {
            bytes,
            region: (at.floor.unsigned_abs(), 129, 123),
            bounds: (at.x, at.y, at.x + 1, at.y + 1),
            start: (HALLOWED_START.x, HALLOWED_START.y),
            expected: if items.is_empty() {
                Vec::new()
            } else {
                vec![ExpectedTile {
                    position: (at.x, at.y),
                    ids: items.to_vec(),
                    counted_entry: None,
                }]
            },
        });
    }
    native_scene(native, crops, &[])
}

async fn hallowed(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
) -> TestResult {
    let questline = format!("{ROOT}rest_in_hallowed_ground/questline");
    let water = format!("{ROOT}rest_in_hallowed_ground/holy_water");
    let [amanda, costello] = super::classic_quests_fixtures::install_positions(
        owner,
        admitted,
        ["oteryn:npc.amanda", "oteryn:npc.costello"],
        HALLOWED_NPCS,
    )
    .await?;
    for (index, (npc, phrase)) in [
        (amanda, "hi"),
        (amanda, "mission"),
        (amanda, "yes"),
        (amanda, "bye"),
        (costello, "hi"),
        (costello, "holy water"),
        (costello, "amanda"),
        (costello, "bye"),
        (amanda, "hi"),
        (amanda, "mission"),
        (amanda, "bye"),
    ]
    .into_iter()
    .enumerate()
    {
        talk(owner, admitted, npc, 2000 + u64::try_from(index)?, phrase).await?;
    }
    assert_eq!(stage(harness, &questline).await?, Some(3));
    assert_eq!(stage(harness, &water).await?, Some(0));
    assert_eq!(carried(harness, 133).await?, 1);

    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    for (i, &(target, approach, ids)) in GRAVES.iter().enumerate() {
        super::l6_native_signs_fixtures::relocate(owner, actor, session, approach)
            .await
            .map_err(|error| format!("grave {target:?} approach {approach:?}: {error}"))?;
        let ordinal = u8::try_from(ids.len() - 1)?;
        assert_eq!(
            use_tool_at(owner, admitted, target, ordinal, 133).await?,
            UseDisposition::Committed,
            "grave {target:?}"
        );
        if i == 0 {
            assert_ne!(
                use_tool_at(owner, admitted, target, ordinal, 133).await?,
                UseDisposition::Committed
            );
        }
        let last = i + 1 == GRAVES.len();
        assert_eq!(
            stage(harness, &water).await?,
            Some(if last { -1 } else { i64::try_from(i)? + 1 })
        );
        assert_eq!(
            stage(harness, &questline).await?,
            Some(if last { 4 } else { 3 })
        );
    }
    assert_eq!(carried(harness, 133).await?, 0);
    assert_eq!(carried(harness, 2874).await?, 1);
    super::l6_native_signs_fixtures::relocate(owner, actor, session, HALLOWED_START).await?;
    talk(owner, admitted, amanda, 2011, "hi").await?;
    talk(owner, admitted, amanda, 2012, "mission").await?;
    assert_eq!(stage(harness, &questline).await?, Some(5));
    assert_eq!(carried(harness, 268).await?, 5);
    assert_eq!(carried(harness, 266).await?, 5);
    Ok(())
}
