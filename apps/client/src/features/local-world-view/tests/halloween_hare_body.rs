type HarePlayError = Box<dyn std::error::Error + Send + Sync>;
fn hare_hex(value: &str) -> Result<Vec<u8>, HarePlayError> {
    if !value.len().is_multiple_of(2) {
        return Err("odd captured Hare hex".into());
    }
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| Ok(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?))
        .collect()
}
fn hare_bytes(
    value: &serde_json::Value,
    key: &str,
    digest: &str,
) -> Result<Vec<u8>, HarePlayError> {
    use sha2::Digest;
    let bytes = hare_hex(value[key].as_str().ok_or("actual Hare frame bytes")?)?;
    let actual = sha2::Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(
        Some(actual.as_str()),
        value[digest].as_str(),
        "actual producer frame digest"
    );
    Ok(bytes)
}
#[test]
fn srv7_actual_source_hare_body_reaches_native_scene() -> Result<(), HarePlayError> {
    std::thread::Builder::new().stack_size(64<<20).spawn(|| {
        tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
            use tokio::io::{AsyncReadExt,AsyncWriteExt};
            use oteryn_session::{Admission,Session,CharacterId,Look,EntityKind};
            let capture:serde_json::Value=serde_json::from_str(include_str!("halloween-hare-body-wire.json"))?;
            assert_eq!(capture["schema"],"OTERYN_SEASONAL_HARE_NATIVE_PARTIAL_PLAY/v1");
            let source=&capture["source"];
            let initial=hare_bytes(&capture,"initial_frames_hex","initial_sha256")?;
            let character=CharacterId::decode(&hare_hex(source["character_id_hex"].as_str().ok_or("actual observer CharacterId")?)?)?;
            let actor:[u8;16]=hare_hex(source["actor_hex"].as_str().ok_or("actual observer actor")?)?.try_into().map_err(|_|"actual observer identity width")?;
            let(client,mut server)=tokio::io::duplex(1<<20);
            let(done,wait)=tokio::sync::oneshot::channel::<()>();
            let peer=tokio::spawn(async move {
                let n=usize::try_from(server.read_u32().await?)?;
                if n>1<<20{return Err("bounded native bootstrap".into());}
                let mut bootstrap=vec![0;n];server.read_exact(&mut bootstrap).await?;
                server.write_all(&initial).await?;
                let _=wait.await;
                Ok::<(),HarePlayError>(())
            });
            let mut session=Session::admit(client,Admission{appearance_items:None,schema_revision:1,character_id:character,
                admission_material:b"srv7-read-only-source-frame-play",client_build_id:"srv7-native-play",
                supported_capabilities:oteryn_session::CLIENT_SUPPORTED_CAPABILITIES,deadline:std::time::Duration::from_secs(3)}).await?;
            assert!(session.selected_capabilities().contains(&25), "real production appearance25 must be negotiated before body presentation proof");
            assert!(!session.selected_capabilities().contains(&104), "ordinary seasonal bodies do not require Forge");
            let bytes=session.gated_snapshot(17).ok_or("actual source Hare map")?.payload.clone();
            let map=oteryn_session::world_map::decode_world_map_snapshot(&bytes).map_err(|e|format!("actual map: {e:?}"))?;
            assert_eq!(source["observer_native"]["x"].as_i64(),Some(i64::from(map.header.origin.x)));
            assert_eq!(source["observer_native"]["y"].as_i64(),Some(i64::from(map.header.origin.y)));
            assert_eq!(source["observer_native"]["floor"].as_i64(),Some(i64::from(map.header.origin.floor)));
            let(link,_commands,outbox)=play_channel();
            outbox.properties_initialize(&session);
            outbox.appearance_initialize(&session);
            outbox.map_initialize(Some(bytes));
            let mut view=PlayView::from_join(session.join_snapshot())?;
            view.set_own_identity(actor);
            view.set_live_map(&map)?;
            let mut held=Vec::new();
            crate::play::session_stepper::publish(&mut session,&outbox,&mut held);
            view.tick(&link).map_err(|e|format!("actual source native frame: {e:?}"))?;
            let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            view.local.set_world(std::sync::Arc::new(crate::map::World::resolve_live_map(&root,&map)?));
            view.rebuild()?;
            let hare=view.appearances.iter().find(|entry|entry.entity.kind==EntityKind::Creature&&entry.entity.position.x==32624&&entry.entity.position.y==32018).ok_or("actual source Hare entity reaches native presentation")?;
            assert!(matches!(hare.look,Some(Look::Outfit{look_type:74,..})),"actual source Hare must have its sourceLook74 instead of the legacy placeholder: {:?}",hare.look);
            eprintln!("SRV7_HARE_BODY decoded_source_look=74 entity_floor={} native_map_floor={}",hare.entity.position.floor,map.header.origin.floor);
            assert_eq!(hare.entity.position.floor,map.header.origin.floor,"actual source Hare must reach the captured native map plane");
            assert_eq!(view.map_center(),(map.header.origin.x,map.header.origin.y,map.header.origin.floor),"native actor updates preserve the actual map plane");
            let raw=session.visible_appearances().into_iter().find(|entry|entry.entity.kind==EntityKind::Creature&&entry.entity.position.x==32624&&entry.entity.position.y==32018).ok_or("actual raw Hare remains in Session")?;
            assert_eq!(raw.entity.position.floor,7,"view projection never rewrites the server observation");
            view.rebuild()?;
            assert_eq!(view.appearances.iter().find(|entry|entry.entity.entity==raw.entity.entity).ok_or("same projected Hare after rebuild")?.entity.position.floor,map.header.origin.floor,"repeated native rebuild never double-converts negative floors");
            if std::env::var_os(crate::map::ASSET_DIR_ENV).is_some() { hare_official_pixels(&view)?; }
            let _=done.send(());peer.await??;
            Ok::<(),HarePlayError>(())
        })
    })?.join().map_err(|_|"source Hare native body thread")?
}

include!("halloween_hare_pixels.rs");
