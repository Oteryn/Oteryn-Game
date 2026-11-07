#[test]
fn srv7_actual_cake_golem_item6393_decodes_and_reaches_native_scene() -> Result<(), HarePlayError> {
    std::thread::Builder::new().stack_size(64<<20).spawn(|| {
        tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
            use tokio::io::{AsyncReadExt,AsyncWriteExt};
            use oteryn_session::{Admission,Session,CharacterId,Look,EntityKind};
            let capture:serde_json::Value=serde_json::from_str(include_str!("cake-golem-wire.json"))?;
            assert_eq!(capture["schema"],"OTERYN_SEASONAL_GOLEM_NATIVE_CAPTURE/v1");
            let source=&capture["source"];
            let initial=hare_bytes(&capture,"initial_frames_hex","initial_frames_sha256")?;
            let character=CharacterId::decode(&hare_hex(source["character_id_hex"].as_str().ok_or("actual Golem viewer CharacterId")?)?)?;
            let identity:[u8;16]=hare_hex(source["actor_identity_hex"].as_str().ok_or("actual Golem identity")?)?.try_into().map_err(|_|"actual Golem identity width")?;
            let expected=u32::try_from(source["item_6393_definition_ref"].as_u64().ok_or("actual indexed Golem reference")?)?;
            let(client,mut server)=tokio::io::duplex(1<<20);
            let(done,wait)=tokio::sync::oneshot::channel::<()>();
            let peer=tokio::spawn(async move {
                let size=usize::try_from(server.read_u32().await?)?;
                if size>1<<20{return Err("bounded native bootstrap".into());}
                let mut bootstrap=vec![0;size];server.read_exact(&mut bootstrap).await?;
                server.write_all(&initial).await?;let _=wait.await;
                Ok::<(),HarePlayError>(())
            });
            // Actual baseline must decode the server's6393 through real admission;
            // after RED use the same sealed projection as the production native entry.
            let mut session=Session::admit(client,Admission{appearance_items:oteryn_client_assets::seasonal_item_projection::seasonal_item_projection().map(|projection| oteryn_session::AppearanceItemDictionary {
                    content_generation: projection.content_generation,
                    definition_refs: &projection.definition_refs,
                }),schema_revision:1,character_id:character,
                admission_material:b"srv7-read-only-source-golem-play",client_build_id:"srv7-native-play",
                supported_capabilities:oteryn_session::CLIENT_SUPPORTED_CAPABILITIES,deadline:std::time::Duration::from_secs(3)}).await
                .map_err(|error|format!("actual source Item6393 must decode through production Session: {error:?}"))?;
            assert!(session.selected_capabilities().contains(&25));assert!(!session.selected_capabilities().contains(&104));
            let bytes=session.gated_snapshot(17).ok_or("actual Golem map")?.payload.clone();
            let map=oteryn_session::world_map::decode_world_map_snapshot(&bytes).map_err(|error|format!("actual Golem map:{error:?}"))?;
            let viewer=&source["observer_runtime_cell"];
            assert_eq!(viewer[0].as_i64(),Some(i64::from(map.header.origin.x)));
            assert_eq!(viewer[1].as_i64(),Some(i64::from(map.header.origin.y)));
            assert_eq!(viewer[2].as_i64().map(|floor|-floor),Some(i64::from(map.header.origin.floor)));
            let actor=*session.world_entities().ok_or("actual own source entity")?.own_identity();
            let(link,_commands,outbox)=play_channel();outbox.properties_initialize(&session);outbox.appearance_initialize(&session);outbox.map_initialize(Some(bytes));
            let mut view=PlayView::from_join(session.join_snapshot())?;view.set_own_identity(actor);view.set_live_map(&map)?;
            crate::play::session_stepper::publish(&mut session,&outbox,&mut Vec::new());view.tick(&link).map_err(|error|format!("actual Golem native frame:{error:?}"))?;
            let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");view.local.set_world(std::sync::Arc::new(crate::map::World::resolve_live_map(&root,&map)?));view.rebuild()?;
            let body=view.appearances.iter().find(|entry|entry.entity.entity.identity==identity&&entry.entity.kind==EntityKind::Creature).ok_or("actual Golem reaches native view")?;
            assert_eq!(body.entity.entity.generation,source["actor_generation"].as_u64().ok_or("actual Golem generation")?);
            assert_eq!(body.look,Some(Look::Item{definition_ref:expected}));
            assert_eq!(body.entity.position.floor,map.header.origin.floor);
            assert_eq!(Some(i64::from(body.entity.position.x)),source["source_runtime_cell"][0].as_i64());
            assert_eq!(Some(i64::from(body.entity.position.y)),source["source_runtime_cell"][1].as_i64());
            if std::env::var_os(crate::map::ASSET_DIR_ENV).is_some() {seasonal_item_official_pixels(&view,identity,6393)?;}
            // Official pixels are checked separately after this real decoder RED,
            // using the source body6393 mapped from the sealed typed reference.
            let _=done.send(());peer.await??;Ok::<(),HarePlayError>(())
        })
    })?.join().map_err(|_|"source Golem native thread")?
}
