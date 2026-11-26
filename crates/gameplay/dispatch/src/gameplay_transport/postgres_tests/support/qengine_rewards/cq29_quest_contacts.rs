use super::*;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

pub(in super::super) const VASE: Position = Position {
    x: 32337,
    y: 31812,
    floor: -6,
};
pub(in super::super) const THIEVES: &str = "qengine-rewards-cq29-thieves";
pub(super) const SENTRY: &str = "qengine-rewards-cq29-sentry";

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    case: &str,
) -> TestResult {
    let npc_key = match case {
        THIEVES => "oteryn:npc.dorian",
        SENTRY => "oteryn:npc.servant_sentry",
        _ => return Err("unknown quest contact case".into()),
    };
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([4, 6, 11, 15, 18]);
    selected.sort_unstable();
    selected.dedup();
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("quest contact capabilities")?;
    let (npc, oswald) = if case == THIEVES {
        let [dorian, oswald] = super::super::classic_quests_fixtures::install(
            owner,
            admitted,
            [npc_key, "oteryn:npc.oswald"],
        )
        .await?;
        (dorian, Some(oswald))
    } else {
        let [sentry] =
            super::super::classic_quests_fixtures::install(owner, admitted, [npc_key]).await?;
        (sentry, None)
    };
    let mut command = 10;
    macro_rules! say {
        ($word:literal) => {{
            command += 1;
            talk(owner, admitted, npc, command, $word).await?
        }};
    }
    assert_eq!(say!("hi"), NpcIntentDisposition::Ok);
    if case == SENTRY {
        assert_eq!(quantity(h, 12077).await?, 0);
        assert_eq!(say!("help"), NpcIntentDisposition::Ok);
        assert_eq!(say!("yes"), NpcIntentDisposition::Ok);
        assert_eq!(quantity(h, 12077).await?, 1);
        assert_eq!(say!("slime"), NpcIntentDisposition::Ok);
        assert_eq!(say!("yes"), NpcIntentDisposition::Ok);
        assert_eq!(quantity(h, 12077).await?, 1);
        return Ok(());
    }

    assert_eq!(say!("join"), NpcIntentDisposition::Ok);
    assert_eq!(say!("yes"), NpcIntentDisposition::Ok);
    assert_eq!(say!("mission"), NpcIntentDisposition::Ok);
    assert_eq!(say!("mission"), NpcIntentDisposition::Ok);
    assert_eq!(say!("yes"), NpcIntentDisposition::Rejected);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: VASE.x - 1,
                ..VASE
            },
            item: "oteryn:item.tibia.i3044".into(),
            count: 11,
        },
        19001,
        true,
    )
    .await?;
    assert_eq!(quantity(h, 3044).await?, 11);
    assert_eq!(say!("mission"), NpcIntentDisposition::Ok);
    assert_eq!(say!("yes"), NpcIntentDisposition::Ok);
    assert_eq!(quantity(h, 3044).await?, 1);
    assert_eq!(
        super::super::questpc7_ferumbras_fixtures::value(
            owner,
            admitted,
            "oteryn:quest-progress/quest/u8_2/the_thieves_guild_quest/questline"
        )
        .await?,
        2
    );
    assert_eq!(say!("mission"), NpcIntentDisposition::Ok);
    assert_eq!(
        super::super::questpc7_ferumbras_fixtures::value(
            owner,
            admitted,
            "oteryn:quest-progress/quest/u8_2/the_thieves_guild_quest/mission02"
        )
        .await?,
        1
    );
    let witness = Position {
        x: VASE.x - 1,
        ..VASE
    };
    questl2_passages::provision(owner, admitted, 7889, 1, 19002, witness).await?;
    pick_vase(owner, admitted).await?;
    assert_eq!(quantity(h, 7889).await?, 0);
    assert_eq!(quantity(h, 227).await?, 1);
    assert_eq!(say!("mission"), NpcIntentDisposition::Ok);
    assert_eq!(say!("yes"), NpcIntentDisposition::Ok);
    assert_eq!(quantity(h, 227).await?, 0);
    assert_eq!(say!("mission"), NpcIntentDisposition::Ok);
    assert_eq!(say!("bye"), NpcIntentDisposition::Ok);
    let oswald = oswald.ok_or("Oswald")?;
    for word in ["hi", "invitation", "yes"] {
        command += 1;
        assert_eq!(
            talk(owner, admitted, oswald, command, word).await?,
            NpcIntentDisposition::Ok
        );
    }
    command += 1;
    assert_eq!(
        talk(owner, admitted, oswald, command, "yes").await?,
        NpcIntentDisposition::Rejected
    );
    assert_eq!(quantity(h, 7933).await?, 0);
    questl2_passages::provision(owner, admitted, 3035, 10, 19003, witness).await?;
    for word in ["invitation", "yes", "yes", "bye"] {
        command += 1;
        assert_eq!(
            talk(owner, admitted, oswald, command, word).await?,
            NpcIntentDisposition::Ok
        );
    }
    assert_eq!(quantity(h, 7933).await?, 1);
    assert_eq!(quantity(h, 3035).await?, 0);
    assert_eq!(say!("hi"), NpcIntentDisposition::Ok);
    assert_eq!(say!("mission"), NpcIntentDisposition::Ok);
    assert_eq!(say!("yes"), NpcIntentDisposition::Ok);
    assert_eq!(quantity(h, 7933).await?, 0);
    assert_eq!(
        super::super::questpc7_ferumbras_fixtures::value(
            owner,
            admitted,
            "oteryn:quest-progress/quest/u8_2/the_thieves_guild_quest/questline"
        )
        .await?,
        4
    );
    Ok(())
}

async fn pick_vase(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    use oteryn_gameplay_world::map::view::MapFacts;
    use oteryn_protocol_oteryn::world_object::{
        MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
        encode_item_use_intent,
    };

    let map = owner.bundle_map().ok_or("Sarina's room")?;
    let key = oteryn_world_bundle::bundle::placement_key(VASE.floor, VASE.x, VASE.y, 1)
        .ok_or("Sarina chest placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        })),
        use_with_position: None,
    })
    .map_err(debug)?;
    let lock_pick = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", "oteryn:item.tibia.i7889", "definition-r1"))
        .ok_or("lock pick policy")?;
    let (next, outcome, _) = super::super::item_craft_fixtures::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        Some(lock_pick),
    )
    .await?;
    *admitted = next;
    assert_eq!(outcome, UseDisposition::Committed);
    Ok(())
}

async fn quantity(h: &Harness, item: u32) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
        .bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}
