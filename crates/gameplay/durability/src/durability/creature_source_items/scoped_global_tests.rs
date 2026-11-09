//! Serialization controls only: no raw fixture here authorizes physical Items.
use super::*;
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{ChannelId, ChannelRuntimeV1, NodeId, WorldId};

type TestResult = std::result::Result<(), Box<dyn std::error::Error + Send + Sync>>;
fn id(tag: u8) -> [u8; 16] {
    let mut bytes = [tag; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}
fn intent(instance: bool) -> TestResultValue {
    let world = WorldId::decode(&id(1)).map_err(|e| format!("{e:?}"))?;
    let channel = ChannelId::decode(&id(2)).map_err(|e| format!("{e:?}"))?;
    let scope = if instance {
        RuntimeScopeRefV1::instance(world, id(3)).map_err(|e| format!("{e:?}"))?
    } else {
        RuntimeScopeRefV1::Channel {
            world_id: world,
            channel_id: channel,
        }
    };
    let node = NodeId::decode(&id(4)).map_err(|e| format!("{e:?}"))?;
    let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
        scope,
        node,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        1,
        ChannelContentPin::test(world),
    )
    .map_err(|e| format!("{e:?}"))?;
    let stamp = runtime.issue_owner_work().map_err(|e| format!("{e:?}"))?;
    let cause = ScopedGlobalGroundCause {
        action: GlobalFireAction::Create,
        identity: ScopedGlobalIdentity {
            scope_identity: if instance { id(3) } else { id(2) },
            scope,
            ownership_generation: 1,
            node: id(4),
            node_registration_revision: 1,
            assignment_revision: 1,
            stamp,
        },
        source_items: vec![],
        definition: TypedDefinitionRef {
            family: "Encounter".into(),
            production_key: "oteryn:encounter.soul_war_taint_zones".into(),
            revision_ref: "definition-r1".into(),
        },
        occurrence: "registered-global:CreateGoshnarSpiteFire:1:0:create".into(),
        content: [1; 32],
        client: [2; 32],
        activation: 1,
        map: [3; 32],
        frame: [4; 32],
        body: [5; 32],
        cast: [6; 32],
    };
    Ok(ScopedGroundCauseRef::Global(&cause).intent_fields(id(1), 1, &[]))
}
type TestResultValue =
    std::result::Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>;
#[test]
fn srx3_registered_global_intent_preserves_real_channel_scope_without_actor() -> TestResult {
    let value = intent(false)?;
    assert_eq!(value["runtime_scope_kind"], 1);
    assert_eq!(
        value["channel"],
        id(2).iter().map(|b| format!("{b:02x}")).collect::<String>()
    );
    assert!(value["instance"].is_null());
    assert!(value["creature"].is_null());
    assert!(value["player"].is_null());
    Ok(())
}
#[test]
fn srx3_registered_global_intent_preserves_real_instance_scope_without_actor() -> TestResult {
    let value = intent(true)?;
    assert_eq!(value["runtime_scope_kind"], 2);
    assert!(value["channel"].is_null());
    assert_eq!(
        value["instance"],
        id(3).iter().map(|b| format!("{b:02x}")).collect::<String>()
    );
    assert!(value["creature"].is_null());
    assert!(value["player"].is_null());
    Ok(())
}
