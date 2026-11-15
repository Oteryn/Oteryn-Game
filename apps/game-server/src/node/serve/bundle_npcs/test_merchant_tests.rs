use super::*;

fn runtime() -> Result<ChannelRuntimeV1, BootError> {
    let world =
        crate::foundation::WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1])
            .map_err(|_| BootError::ContentActivation("test merchant fixture world"))?;
    let channel = crate::foundation::ChannelId::decode(&[
        1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2,
    ])
    .map_err(|_| BootError::ContentActivation("test merchant fixture channel"))?;
    let node =
        crate::foundation::NodeId::decode(&[1, 0, 0, 0, 0, 3, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 3])
            .map_err(|_| BootError::ContentActivation("test merchant fixture node"))?;
    ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        node,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        2,
        oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin::test(world),
    )
    .map_err(|_| BootError::ContentActivation("test merchant fixture runtime"))
}

#[test]
fn test_merchant_flag_controls_boot_actor_and_services() -> Result<(), BootError> {
    for enabled in [false, true] {
        let boot = test_merchant_boot(
            NpcBoot::Skip {
                reason: "no_catalogue",
                npcs: 0,
                placements: 0,
            },
            enabled,
        )?;
        let key = crate::content::npc_catalogue::test_merchant::NPC;
        assert_eq!(boot.admits(key), enabled);
        let mut runtime = runtime()?;
        assert_eq!(place(&mut runtime, Some(&boot))?, usize::from(enabled));
        let visible = runtime.visible_entities();
        assert_eq!(visible.npcs.len(), usize::from(enabled));
        if enabled {
            let npc = &visible.npcs[0];
            let pack = crate::content::npc_catalogue::test_merchant::pack()
                .map_err(|_| BootError::ContentActivation("test merchant placement test pack"))?;
            assert_eq!(runtime.npc_key(npc.actor), Some(key));
            assert_eq!(
                (
                    i64::from(npc.position.x),
                    i64::from(npc.position.y),
                    i64::from(npc.position.floor)
                ),
                // Runtime cells use the positive Tibia floor; bundle placements use -7.
                (
                    i64::from(pack.placement.x),
                    i64::from(pack.placement.y),
                    i64::from(pack.placement.floor.unsigned_abs())
                )
            );
        }
        let book = talk_book(Some(&boot));
        assert_eq!(
            book.as_ref().and_then(|book| book.get(key)).is_some(),
            enabled
        );
        if let Some(book) = book {
            let entry = book
                .get(key)
                .ok_or(BootError::ContentActivation("test merchant test entry"))?;
            assert!(!entry.offers().is_empty());
            let pack = crate::content::npc_catalogue::test_merchant::pack()
                .map_err(|_| BootError::ContentActivation("test merchant test pack"))?;
            for category in &pack.categories {
                let count = entry
                    .offers()
                    .iter()
                    .filter(|offer| {
                        crate::content::npc_catalogue::test_merchant::contains(
                            Some(&category.keyword),
                            offer,
                        )
                    })
                    .count();
                assert!((1..=1024).contains(&count));
                assert!(oteryn_gameplay_npc::npc_talk::asks_to_trade_for(
                    key,
                    &category.keyword
                ));
            }
            assert!(!oteryn_gameplay_npc::npc_talk::asks_to_trade_for(
                key, "trade"
            ));
            assert_ne!(
                book.trade_revision(key, Some(&pack.categories[0].keyword)),
                book.trade_revision(key, Some(&pack.categories[1].keyword))
            );
        }
    }
    Ok(())
}
