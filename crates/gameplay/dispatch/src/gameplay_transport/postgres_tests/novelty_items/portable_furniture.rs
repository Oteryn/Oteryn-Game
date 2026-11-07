//! Genuine Channel ordinary USE; House-only placement is never selected here.
use super::*;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
const MUSIC: &[u32] = &[
    2948, 2949, 2950, 2951, 2952, 2953, 2954, 2955, 2956, 2957, 2958, 2965, 2966, 3219, 12602,
];
pub(super) fn music_contents() -> Vec<(&'static str, i64)> {
    vec![
        ("oteryn:item.tibia.i2948", 1),
        ("oteryn:item.tibia.i2949", 1),
        ("oteryn:item.tibia.i2950", 1),
        ("oteryn:item.tibia.i2951", 1),
        ("oteryn:item.tibia.i2952", 1),
        ("oteryn:item.tibia.i2953", 1),
        ("oteryn:item.tibia.i2954", 1),
        ("oteryn:item.tibia.i2955", 1),
        ("oteryn:item.tibia.i2956", 1),
        ("oteryn:item.tibia.i2957", 1),
        ("oteryn:item.tibia.i2958", 1),
        ("oteryn:item.tibia.i2965", 1),
        ("oteryn:item.tibia.i2966", 1),
        ("oteryn:item.tibia.i3219", 1),
        ("oteryn:item.tibia.i12602", 1),
    ]
}
pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    source: [u8; 16],
    case: Case,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|a| a.native_gameplay())
        .ok_or("native")?;
    assert_ground_plan_catalogue(native)?;
    let index = native.item_index().ok_or("Item index")?;
    let body = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let before:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
    let ids = if case == Case::PortableMusic {
        MUSIC
    } else {
        &[2914][..]
    };
    let mut first_payload = None;
    for (ordinal, id) in ids.iter().enumerate() {
        let key = format!("oteryn:item.tibia.i{id}");
        let reference = index
            .definition_ref("Item", &key, "definition-r1")
            .ok_or("portable definition")?;
        let command = u64::try_from(ordinal)?.checked_add(1).ok_or("command")?;
        let (next, result, actual) = super::super::rust_lottery::exchange(
            owner,
            admitted,
            command,
            &body,
            Some(reference),
            None,
        )
        .await?;
        assert_eq!(
            result,
            UseDisposition::Committed,
            "ordinary Channel source{id} USE must invoke the qualified portable rule"
        );
        admitted = next;
        if first_payload.is_none() {
            first_payload = Some(actual.clone());
        }
        if matches!(case, Case::PortableLamp | Case::GroundPortableLamp) {
            let after:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
            if case == Case::GroundPortableLamp {
                assert!(sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid)").bind(source.as_slice()).fetch_one(&h.pool).await?,"ordinary Ground USE preserves real Ground custody");
            }
            assert_eq!(
                after["definition_production_key"],
                "oteryn:item.tibia.i2915"
            );
            assert_eq!(
                after["minted_transaction_id"],
                before["minted_transaction_id"]
            );
            assert!(sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid AND deadline_at>floor(extract(epoch FROM statement_timestamp())*1000)::bigint AND remaining_ms=180000)").bind(source.as_slice()).fetch_one(&h.pool).await?);
            owner
                .states
                .command_ingress
                .forget(admitted.game_session_id);
            admitted.continuity.next_command_id = command;
            let (_, replay, _) =
                super::super::rust_lottery::exchange(owner, admitted, command, &actual, None, None)
                    .await?;
            assert_eq!(replay, UseDisposition::Committed);
            return Ok(());
        }
        eprintln!("SRV7_PORTABLE_MUSIC_PG_EXECUTED item={id} command={command}");
    }
    let emitted = {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let (_, events) = states
            .sounds
            .observe(
                &runtime,
                admitted.runtime_actor.ok_or("portable cue actor")?,
                admitted.game_session_id,
                0,
                true,
            )
            .ok_or("actual current native presentation publisher")?;
        events
            .into_iter()
            .filter_map(|event| match event.kind {
                oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect {
                    effect_id,
                    ..
                } => Some(effect_id),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let expected:Vec<String>=sqlx::query_scalar("SELECT effects->'cues'->0->>'effect_id' FROM game_item_consume_transform_records WHERE effects->>'source_item_id'=ANY($1) ORDER BY command_id").bind(MUSIC.iter().map(u32::to_string).collect::<Vec<_>>()).fetch_all(&h.pool).await?;
    assert_eq!(
        emitted.iter().map(u32::to_string).collect::<Vec<_>>(),
        expected,
        "all15 exact committed source cues reach the real native publisher"
    );
    owner
        .states
        .command_ingress
        .forget(admitted.game_session_id);
    admitted.continuity.next_command_id = 1;
    let (_, replayed, _) = super::super::rust_lottery::exchange(
        owner,
        admitted,
        1,
        &first_payload.ok_or("original portable cue payload")?,
        None,
        None,
    )
    .await?;
    assert_eq!(replayed, UseDisposition::Committed);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_consume_transform_records WHERE effects->>'source_item_id'=ANY($1)").bind(MUSIC.iter().map(u32::to_string).collect::<Vec<_>>()).fetch_one(&h.pool).await?,15);
    Ok(())
}

fn assert_ground_plan_catalogue(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult {
    let ground = oteryn_gameplay_items::novelty_items::GroundEffect {
        x: 32309,
        y: 32805,
        floor: 7,
    };
    const PORTABLE: &[u32] = &[
        2911, 2912, 2914, 2915, 2917, 2918, 2920, 2921, 2922, 2923, 2924, 2925, 2977, 2978, 3046,
        3047, 5812, 5813, 6488, 6489, 9063, 9064, 9065, 9066, 20280, 20281, 22153, 22154, 28915,
        28917, 30362, 30363, 30364, 31674, 32760, 32761, 33331, 33333, 34264, 34266, 34268, 34269,
        34326, 35909, 36019, 36478, 36480, 36978, 36979, 36996, 36998, 37052, 37053, 37054, 37056,
        37061, 37063, 37111, 37113, 37543, 37544, 37545, 37580, 37581, 37582, 37743, 37745, 37749,
        37750, 38677, 38704, 39697, 39698, 39699, 39700, 39701, 39702, 39757, 39759, 39761,
    ];
    for id in PORTABLE.iter().chain(MUSIC.iter()) {
        let definition = crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: format!("oteryn:item.tibia.i{id}"),
            revision_ref: "definition-r1".into(),
        };
        let plan = oteryn_gameplay_items::novelty_items::plan_ground(
            native,
            &definition,
            [7; 16],
            1,
            [8; 16],
            ground.clone(),
            false,
        )
        .ok_or_else(|| format!("qualified Ground plan absent for{id}"))?;
        assert_eq!(
            plan.effects["ground"],
            serde_json::to_value(&ground)?,
            "source{id} must use actual supplied Ground position"
        );
    }
    Ok(())
}
