//! Real framed House gift USEs, with donor branch identities selected before USE.
use super::*;
use oteryn_gameplay_items::reward_bag::reward_bag_bounded_draw;
pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    mut admitted: AdmittedSession,
    reference: std::num::NonZeroU32,
    case: Case,
) -> TestResult {
    let gift_command = if case == Case::GiftDragonDeathHigh {
        2
    } else {
        1
    };
    let actor = admitted.runtime_actor.ok_or("House gift actor")?;
    let item_id = match case {
        Case::GiftPanda | Case::GiftPandaFar => 5080,
        Case::GiftSantaDoll => 6511,
        Case::GiftDragon
        | Case::GiftDragonDeath
        | Case::GiftDragonDeathCrash
        | Case::GiftDragonDeathHigh => 6566,
        _ => 2995,
    };
    let source = if matches!(
        case,
        Case::GiftPanda | Case::GiftPandaFar | Case::GiftSantaDoll
    ) {
        super::id(71)
    } else {
        let (domain, upper, wanted) = if matches!(
            case,
            Case::GiftDragon
                | Case::GiftDragonDeath
                | Case::GiftDragonDeathCrash
                | Case::GiftDragonDeathHigh
        ) {
            (211, 5, 4)
        } else {
            (210, 6, u64::from(case == Case::GiftBankKeep))
        };
        let source = (10000_u32..100000)
            .map(|nonce| {
                let mut s = super::id(71);
                s[12..].copy_from_slice(&nonce.to_be_bytes());
                s
            })
            .find(|source| {
                reward_bag_bounded_draw(
                    *admitted.game_session_id.as_bytes(),
                    gift_command,
                    *source,
                    domain,
                    upper,
                ) == Some(wanted)
            })
            .ok_or("bounded gift identity fixture")?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        for statement in [
            "UPDATE game_item_instances SET item_instance_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid",
            "UPDATE game_item_house_interior_locations SET item_instance_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid",
            "UPDATE game_item_house_reclaim_provenance SET item_instance_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid",
        ] {
            sqlx::query(statement)
                .bind(source.as_slice())
                .bind(super::id(71).as_slice())
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        source
    };
    let before:Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
    let custody:Value=sqlx::query_scalar("SELECT jsonb_build_object('location',to_jsonb(h),'provenance',to_jsonb(p)) FROM game_item_house_interior_locations h JOIN game_item_house_reclaim_provenance p USING(item_instance_id) WHERE h.item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
    if matches!(
        case,
        Case::GiftDragon
            | Case::GiftDragonDeath
            | Case::GiftDragonDeathCrash
            | Case::GiftDragonDeathHigh
    ) {
        let mut states = owner.states.spell_states.lock().await;
        let (_, _, state) = states
            .actors
            .iter_mut()
            .find(|(a, s, _)| *a == actor && *s == admitted.game_session_id)
            .ok_or("gift owned state")?;
        state.restore_saved_vitals(
            if matches!(
                case,
                Case::GiftDragonDeath | Case::GiftDragonDeathCrash | Case::GiftDragonDeathHigh
            ) {
                1
            } else {
                7
            },
            9,
        );
    }
    let hp = |session: crate::foundation::GameSessionId| async move {
        let states = owner.states.spell_states.lock().await;
        states
            .actors
            .iter()
            .find(|(a, s, _)| *a == actor && *s == session)
            .map(|(_, _, s)| s.vitals().health)
            .ok_or("House gift HP")
    };
    let before_hp = hp(admitted.game_session_id).await?;
    let body = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let capture = std::sync::Arc::new(std::sync::Mutex::new(
        super::wire::HouseWireCapture::default(),
    ));
    capture
        .lock()
        .map_err(|_| "gift death observation")?
        .wait_for_death = matches!(
        case,
        Case::GiftDragonDeath | Case::GiftDragonDeathCrash | Case::GiftDragonDeathHigh
    );
    if matches!(
        case,
        Case::GiftDragonDeath | Case::GiftDragonDeathCrash | Case::GiftDragonDeathHigh
    ) {
        let mut exit = vec![8, 2, 18, u8::try_from(HOUSE.len())?];
        exit.extend(HOUSE.as_bytes());
        capture
            .lock()
            .map_err(|_| "live death exit capture")?
            .exit_after_death = Some(exit);
    }
    let view = || HouseExchange {
        capture: Some(std::sync::Arc::clone(&capture)),
        rotate: false,
        current: Some(reference),
        after: None,
        nested: None,
        foreign_after_snapshot: None,
    };
    if case == Case::GiftDragonDeathCrash {
        super::death_crash::install_fault(h).await?;
        let exchanged = exchange(owner, admitted, gift_command, &body, view()).await;
        return super::death_crash::qualify(h, owner, admitted, &capture, exchanged.is_err()).await;
    }
    let (next, result, original, _) = if case == Case::GiftDragonDeathHigh {
        let lamp = owner
            .active_generation
            .and_then(|active| active.native_gameplay())
            .and_then(|native| native.item_index())
            .and_then(|index| {
                index.definition_ref("Item", "oteryn:item.tibia.i2914", "definition-r1")
            })
            .ok_or("actual pinned lamp2914")?;
        super::wire::exchange_with_lamp(
            owner,
            admitted,
            gift_command,
            &body,
            view(),
            super::wire::HouseLampPrelude {
                harness: h,
                item: id(72),
                definition: lamp,
            },
        )
        .await?
    } else {
        exchange(owner, admitted, gift_command, &body, view()).await?
    };
    admitted = next;
    if case == Case::GiftPandaFar {
        assert_eq!(
            result,
            UseDisposition::NotPossible,
            "current actor and genuine House custody cannot authorize a gift farther than one tile"
        );
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_consume_transform_records WHERE source_item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?,0);
        let after:Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(after, before);
        return Ok(());
    }
    assert_eq!(
        result,
        UseDisposition::Committed,
        "source Santa gift USE must commit in actual House custody"
    );
    if matches!(
        case,
        Case::GiftDragonDeath | Case::GiftDragonDeathCrash | Case::GiftDragonDeathHigh
    ) {
        super::gifts_live_death::qualify(h, owner, admitted, &capture).await?;
        if case == Case::GiftDragonDeathHigh {
            let actual = owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position();
            assert_eq!(
                [actual.x, actual.y, i32::from(actual.floor)],
                super::gift_death::TEMPLE,
                "current stored source town8 Temple"
            );
            let timer = capture
                .lock()
                .map_err(|_| "actual lamp lifetime capture")?
                .timer_before_death
                .clone()
                .ok_or("actual lamp clock before lethal USE")?;
            super::death_ground::qualify(h, owner, deps, admitted, id(70), &[id(72)], &timer)
                .await?;
        }
        return Ok(());
    }

    let after:Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(after["item_instance_id"], before["item_instance_id"]);
    assert_eq!(
        (after["quantity"].as_i64(), after["lifecycle"].as_i64()),
        (Some(1), Some(1))
    );
    assert_eq!(
        after["definition_production_key"],
        serde_json::json!(format!(
            "oteryn:item.tibia.i{}",
            if case == Case::GiftBankBreak {
                2996
            } else {
                item_id
            }
        ))
    );
    let retained:Value=sqlx::query_scalar("SELECT effects FROM game_item_consume_transform_records WHERE game_session_id=encode($1,'hex')::uuid AND command_id=1 AND source_kind=5").bind(admitted.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    if case == Case::GiftPanda {
        assert_eq!(retained["speech"], "Hug me!");
    }
    if case == Case::GiftSantaDoll {
        const VOICES: [&str; 6] = [
            "Ho ho ho",
            "Jingle bells, jingle bells...",
            "Have you been naughty?",
            "Have you been nice?",
            "Merry Christmas!",
            "Can you stop squeezing me now... I'm starting to feel a little sick.",
        ];
        let draw = reward_bag_bounded_draw(*admitted.game_session_id.as_bytes(), 1, source, 212, 6)
            .ok_or("House Santa Doll voice draw")?;
        assert_eq!(retained["speech"], VOICES[usize::try_from(draw)?]);
        assert_eq!(hp(admitted.game_session_id).await?, before_hp);
    }
    if matches!(
        case,
        Case::GiftDragon
            | Case::GiftDragonDeath
            | Case::GiftDragonDeathCrash
            | Case::GiftDragonDeathHigh
    ) {
        assert_eq!(retained["speech"], "You... will.... burn!!");
        assert_eq!(retained["damage"], 1);
        assert_eq!(hp(admitted.game_session_id).await?, before_hp - 1);
    }
    if matches!(case, Case::GiftBankBreak | Case::GiftBankKeep) {
        let coin = if case == Case::GiftBankBreak {
            3031
        } else {
            3035
        };
        let n:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id,world_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.definition_production_key=$2 AND i.quantity=1 AND i.lifecycle=1")
            .bind(admitted.item_fence.ok_or("gift fence")?.character_id.as_bytes().as_slice()).bind(format!("oteryn:item.tibia.i{coin}")).fetch_one(&h.pool).await?;
        assert_eq!(
            n, 1,
            "the clicked House bank pays the actor's backpack exactly once"
        );
        let progress:i64=sqlx::query_scalar("SELECT coalesce(max(progress),0) FROM game_character_achievement_progress WHERE character_id=encode($1,'hex')::uuid AND achievement_key='oteryn:achievement/allowance_collector'")
            .bind(admitted.item_fence.ok_or("gift fence")?.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(progress, i64::from(case == Case::GiftBankBreak));
    }
    let after_custody:Value=sqlx::query_scalar("SELECT jsonb_build_object('location',to_jsonb(h),'provenance',to_jsonb(p)) FROM game_item_house_interior_locations h JOIN game_item_house_reclaim_provenance p USING(item_instance_id) WHERE h.item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        after_custody, custody,
        "gift preserves placement/reclaim identity"
    );
    if case == Case::GiftDragon {
        super::gift_presentation::assert_and_capture(owner, admitted, &capture).await?;
    }
    capture.lock().map_err(|_| "gift cues")?.effects.clear();
    capture
        .lock()
        .map_err(|_| "gift speech")?
        .chat_lines
        .clear();
    owner
        .states
        .command_ingress
        .forget(admitted.game_session_id);
    admitted.continuity.next_command_id = 1;
    let (_, replay, _, _) = exchange(
        owner,
        admitted,
        1,
        &original,
        HouseExchange {
            current: None,
            ..view()
        },
    )
    .await?;
    assert_eq!(replay, UseDisposition::Committed);
    assert!(
        capture
            .lock()
            .map_err(|_| "gift replay cues")?
            .effects
            .is_empty()
    );
    assert!(
        capture
            .lock()
            .map_err(|_| "gift replay speech")?
            .chat_lines
            .is_empty()
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_consume_transform_records WHERE source_item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?,1);
    eprintln!("SRV7_SANTA_HOUSE_GIFT_EXECUTED source={item_id} case={case:?}");
    Ok(())
}

pub(super) fn far_world(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
) -> TestResult<crate::map::boot::BundleWorld> {
    use crate::map::boot::{self, tests as fixture};
    use oteryn_world_bundle::bundle;
    let (bytes, mut pins) =
        fixture::bundle_with_items("oteryn:item.tibia.i1629", "oteryn:item.tibia.i1000");
    let mut data = bundle::read(&bytes).map_err(debug)?;
    let sector = data.sectors.first_mut().ok_or("sector")?;
    let mut entrance = sector
        .tiles
        .iter()
        .find(|t| t.x == 3)
        .ok_or("grass tile")?
        .clone();
    for tile in &mut sector.tiles {
        if tile.x == 7 {
            tile.house = 3437;
            tile.items.truncate(1);
        }
        if tile.x == 6 {
            tile.house = 3437;
            tile.items.last_mut().ok_or("door")?.attrs.door = Some(42);
        }
        tile.x += 32303;
        tile.y += 32804;
    }
    entrance.x = 32309;
    entrance.y = 32805;
    let mut far = entrance.clone();
    far.x = 32313;
    far.y = 32804;
    far.house = 3437;
    sector.tiles.push(far);
    sector.tiles.push(entrance);
    sector.sx = 1009;
    sector.sy = 1025;
    data.manifest.world.min_x = 32288;
    data.manifest.world.min_y = 32800;
    data.manifest.world.max_x = 32320;
    data.manifest.world.max_y = 32832;
    let bytes = oteryn_world_bundle_compiler::bundle::write_with_npcs(
        &data.manifest,
        &data.sectors,
        &data.spawns,
        &data.npcs,
    )
    .map_err(debug)?;
    pins.digest = bundle::read(&bytes).map_err(debug)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    boot::boot(
        &bytes,
        &boot::BootPins {
            bundle: pins,
            map_revision: crate::map::overlay::map_revision(&base),
            start: crate::map::overlay::TilePos {
                x: 32309,
                y: 32805,
                floor: -7,
            },
        },
        world,
        channel,
        fixture::door_items,
    )
    .map_err(debug)
}
