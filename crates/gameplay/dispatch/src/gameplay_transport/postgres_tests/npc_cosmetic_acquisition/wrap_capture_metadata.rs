//! Capture-only facts; no authority or inferred definition reference is created.
use super::*;
use sha2::{Digest, Sha256};
pub(super) async fn metadata(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &crate::gameplay_transport::connection::AdmittedSession,
    public_artifact: &[u8],
) -> TestResult<serde_json::Value> {
    let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let generation: [u8; 32] = Sha256::digest(public_artifact).into();
    let active = owner.active_generation.ok_or("actual capture activation")?;
    assert_eq!(
        generation,
        active.identity().client_artifact_digest(),
        "independent qualified public bytes, never a copied wire generation"
    );
    let (viewer, runtime_generation) = {
        let runtime = owner.runtime.lock().await;
        let viewer = runtime
            .house_actor_state(
                admitted.runtime_actor.ok_or("native wrap viewer")?,
                admitted.game_session_id,
            )
            .map_err(debug)?;
        (viewer, runtime.content_pin().client_artifact_digest())
    };
    assert_eq!(
        runtime_generation, generation,
        "real named-House map/native session has the qualified public generation"
    );
    let root = super::super::raid_live::hare_production_activation::repository_root();
    let keys = std::fs::read(root.join("tools/content-schema/native-gameplay/item-keys.json"))?;
    let rules =
        std::fs::read(root.join("tools/content-schema/native-gameplay/furniture-rules.json"))?;
    let ids = [
        12699, 23722, 25103, 25104, 25903, 25904, 25905, 25906, 25907, 25908, 25909, 25910, 25911,
        25912, 25913, 25914, 28525, 28696, 31382, 32509, 32510, 32511, 32512, 32513, 32514, 32515,
        32516, 32517, 32518, 32519, 32520, 32521, 32522, 32523, 32524, 32525, 32526, 32527, 32528,
        32529, 32530, 32531, 32532, 32533, 32534, 32535, 32536, 32541, 32542, 32543, 32544, 32545,
        32546, 32547, 32548, 32549, 32550, 32551, 32552, 32553, 32554, 32555, 32556, 32557, 32558,
        32559, 32560, 32561, 32562, 32563, 32564, 32565, 32566,
    ];
    let native = active
        .native_gameplay()
        .ok_or("actual native capture content")?;
    let index = native.item_index().ok_or("actual native capture keys")?;
    let mut references = serde_json::Map::new();
    for id in ids {
        let key = format!("oteryn:item.tibia.i{id}");
        let reference = index
            .definition_ref("Item", &key, "definition-r1")
            .ok_or("qualified XML wrap source reference")?;
        assert!(
            native
                .item_policy(&key, "definition-r1")
                .is_some_and(|p| p.record().admission.materializable)
        );
        references.insert(id.to_string(), serde_json::json!(reference.get()));
    }
    Ok(serde_json::json!({
        "crystal_sha":"201189f871d23597b82caf88fd611454a2ddefb8",
        "xml_sha256":"8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509",
        "dat_sha256":"17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1",
        "furniture_rules_sha256":hex(&Sha256::digest(&rules)),"item_keys_sha256":hex(&Sha256::digest(&keys)),
        "client_content_generation":hex(&generation),"public_artifact_sha256":hex(&Sha256::digest(public_artifact)),
        "wrap_advisory_definition_refs":references,
        "viewer":{"house":viewer.house,"runtime_cell":[viewer.position.x,viewer.position.y,viewer.position.floor],"native_cell":[viewer.position.x,viewer.position.y,-i32::from(viewer.position.floor)],"scope_generation":viewer.generation.get(),"lease_generation":viewer.lease.generation()},
        "session":hex(admitted.game_session_id.as_bytes()),"connection_generation":admitted.continuity.connection_generation,
        "selected_capabilities":admitted.continuity.selected_capabilities.as_slice()
    }))
}
