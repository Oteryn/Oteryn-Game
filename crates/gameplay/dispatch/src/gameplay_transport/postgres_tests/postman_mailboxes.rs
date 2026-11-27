use super::TestResult;
use super::postman_mailbox_fixtures::{ALTERNATE, AXE, LETTERBAG};
use super::quest_world_callbacks_fixtures::{callback, literal_uses::postman_mailbox};

#[test]
fn postman_folda_repairs_mailbox_with_either_selected_tool() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("postman-folda", || {
        for tool in [AXE, ALTERNATE] {
            postman_mailbox(callback("postman-folda-mailbox")?, 1, tool)?;
        }
        Ok(())
    })
}

#[test]
fn postman_santa_mailbox_transforms_only_the_selected_letterbag() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("postman-santa", || {
        postman_mailbox(callback("postman-letterbag-delivery")?, 2, LETTERBAG)
    })
}
