#[test]
fn browse_field_moves_both_directions_and_up_receives_field_delta() -> Result<(), BoxError> {
    std::thread::Builder::new().stack_size(64<<20).spawn(||block_on(async {
        let selected:&[u32]=&[4,6,12,14,18,230];
        let position=item_view::WorldTilePosition {x:100,y:200,floor:0};
        let field=container_tree::ContainerViews {views:vec![container_tree::ContainerView {view_id:3,container_handle:handle(50),parent_handle:None,capacity:30,entries:vec![entry(51,3031,5)],browse:Some(browse_field::BrowseFieldPage {position,index:0,total:1})}]};
        let field_bytes=container_tree::encode_container_views_for(&field,true).map_err(|e|format!("{e:?}"))?;
        let (client,mut server)=tokio::io::duplex(65536);
        let peer=tokio::spawn(async move {
            let map=oteryn_protocol_oteryn::world_map::encode_world_map_snapshot(&oteryn_protocol_oteryn::world_map::WorldMapViewSnapshot {header:oteryn_protocol_oteryn::world_map::MapViewHeader {content_generation:CONTENT_GENERATION,bundle_digest:[0x22;32],reset_epoch:0,origin:at(0,0),first_visible_floor:0},tiles:vec![]}).map_err(|e|format!("{e:?}"))?;
            join_peer_with(&mut server,selected,selected,&[],item_snapshot(vec![])?,&[(14,1,1,vec![]),(17,0,1,map)]).await?;
            for (sequence,destination) in [(41,ItemMoveDestination::Ground(position)),(42,ItemMoveDestination::MainBackpack)] {
                let wire=read_frame(&mut server).await?;
                let envelope=decode_wire_envelope(&wire)?;
                let command=envelope.client_command(1)?;
                let intent=item_view::decode_item_move_intent_for(command.payload,item_view::ItemMoveSelection {equip_drop:true,container_tree:true,world_interaction:false, manual_sort: false}).map_err(|e|format!("{e:?}"))?;
                assert_eq!(intent.destination,destination);
                write_frame(&mut server,&encode_command_result(1,sequence,command.command_id,CommandStatus::Accepted,&[8,1])?).await?;
            }
            let wire=read_frame(&mut server).await?;
            let envelope=decode_wire_envelope(&wire)?;
            let command=envelope.client_command(1)?;
            assert_eq!(command.command_type,21);
            assert_eq!(container_tree::decode_container_view_intent(command.payload),Ok(container_tree::ContainerViewIntent::Up {view_id:3}));
            write_frame(&mut server,&encode_state_delta(1,43,14,1,2,1,&field_bytes)?).await?;
            write_frame(&mut server,&encode_command_result(1,44,command.command_id,CommandStatus::Accepted,&[8,1])?).await?;
            Ok::<(),BoxError>(())
        });
        let mut admission=item_admission()?;admission.supported_capabilities=selected;
        let mut session=Session::admit(client,admission).await?;
        for destination in [ItemMoveDestination::Ground(position),ItemMoveDestination::MainBackpack] {
            assert_eq!(session.move_item_counted(item_movement::MoveRequest {source:handle(51),destination,count:None}).await?.outcome,ItemMoveOutcome::Moved);
        }
        let payload=container_tree::encode_container_view_intent(&container_tree::ContainerViewIntent::Up {view_id:3}).map_err(|e|format!("{e:?}"))?;
        session.gated_command(14,21,&payload).await?;
        assert!(session.take_events().iter().any(|event|matches!(event,SessionEvent::ContainerViews(update) if update.value==field)));
        peer.await??;
        Ok(())
    }))?.join().map_err(|_|"browse-field test panicked")??
}

#[test]
fn map_delta_bounds_handles_in_retained_view() -> Result<(), BoxError> {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| block_on(async {
        use oteryn_protocol_oteryn::world_map::*;
        let selected: &[u32] = &[4,6,12,14,18];
        let header = MapViewHeader { content_generation:CONTENT_GENERATION, bundle_digest:[0x22;32], reset_epoch:0, origin:at(0,0), first_visible_floor:0 };
        let item = MapItem { definition:MapDefinition::Item(std::num::NonZeroU32::MIN), count:1, sub_type:0, origin:MapOrigin::Handle(std::num::NonZeroU64::MIN), appearance_id:1 };
        let mut remaining = MAX_MAP_VIEW_HANDLES;
        let mut tiles = vec![];
        for y in -5..5 { for x in -7..4 {
            let count = remaining.min(MAX_TILE_ITEMS);
            if count > 0 { tiles.push(MapTile { position:at(x,y), items:vec![item;count], more:false, ground_speed:150 }); remaining -= count; }
        }}
        assert_eq!(remaining, 0);
        tiles.sort_by_key(|tile| tile_order(&tile.position));
        let map = encode_world_map_snapshot(&WorldMapViewSnapshot { header, tiles }).map_err(|e|format!("{e:?}"))?;
        let (client, mut server) = tokio::io::duplex(262144);
        let peer = tokio::spawn(async move { join_peer_with(&mut server,selected,selected,&[],item_snapshot(vec![])?,&[(14,1,1,vec![]),(17,0,1,map)]).await });
        let mut admission = item_admission()?; admission.supported_capabilities=selected;
        let mut session = Session::admit(client,admission).await?;
        peer.await??;
        let value = WorldMapViewDelta { header, tiles:vec![MapTile { position:at(4,0),items:vec![item],more:false,ground_speed:150 }],cleared:vec![] };
        let encoded = encode_world_map_delta(&header,&value).map_err(|e|format!("{e:?}"))?;
        let wire = encode_state_delta(1,41,17,0,1,1,&encoded)?;
        let envelope = decode_wire_envelope(&wire)?;
        assert!(map_transport::apply(&mut session,41,envelope.payload()).is_err());
        assert_eq!(session.world_map_snapshot().map(|s| s.tiles.iter().map(|t|t.items.len()).sum::<usize>()),Some(MAX_MAP_VIEW_HANDLES));
        Ok(())
    }))?.join().map_err(|_|"map budget test panicked")??
}
