use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

pub(super) const NPC: &str = "oteryn:npc.thanita";

#[test]
fn qtowerdef_password_is_issued_and_validated_through_npc_dispatch() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::qengine_rewards::scenario("qengine-rewards-m09-tower-password")
}

pub(super) async fn issue(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    thanita: EntityRef,
    command: u64,
) -> TestResult<String> {
    super::classic_quests::talk(owner, admitted, thanita, command, "hi").await?;
    let lines = super::classic_quests::talk(owner, admitted, thanita, command + 1, "mission").await?;
    let code = lines.iter().find_map(|line| {
        let (_, tail) = line.split_once("password you need to tell Bunny is ' ")?;
        let (code, _) = tail.split_once(" '")?;
        Some(code)
    }).ok_or("Thanita must issue a password")?;
    assert_eq!(code.len(), 4);
    assert!(code.bytes().all(|byte| byte.is_ascii_digit()));
    super::classic_quests::talk(owner, admitted, thanita, command + 2, "bye").await?;
    Ok(code.to_owned())
}
