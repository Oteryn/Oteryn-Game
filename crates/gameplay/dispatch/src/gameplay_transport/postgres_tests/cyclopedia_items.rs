//! The added selector through the real admitted command dispatcher and fenced owner.
use super::fixture::{Harness, run};
use super::{Dependencies, TestResult, debug, prepare_current_session, reset};
use crate::gameplay_transport::{
    capabilities::{OfferedCapability, SelectedCapabilities},
    connection::FirstEntryOutcome,
};
use oteryn_protocol_oteryn::{
    CommandStatus,
    cyclopedia_character::{COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB, items},
};
#[test]
fn cyclopedia_items_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "item summary dispatch thread failed")??;
    Ok(())
}
fn scenario() -> TestResult {
    use super::cyclopedia::support::exchange;
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "cyclopedia_items", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        let command = COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB;
        assert_eq!(
            exchange(&owner, &mut admitted, 1, command, &items::request(0))
                .await?
                .0,
            CommandStatus::Rejected
        );
        let offers = [OfferedCapability {
            id: 199,
            requires: &[],
        }];
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[199]).ok_or("select cyclopedia")?;
        assert_eq!(
            exchange(&owner, &mut admitted, 2, command, &[items::SELECTOR])
                .await?
                .0,
            CommandStatus::Rejected
        );
        let (status, bytes) =
            exchange(&owner, &mut admitted, 3, command, &items::request(0)).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let page = items::Page::decode(&bytes).map_err(debug)?;
        assert_eq!(page.offset, 0);
        assert_eq!(
            page.rows.len(),
            usize::from(page.total).min(items::PAGE_SIZE)
        );
        assert_eq!(
            exchange(&owner, &mut admitted, 4, command, &items::request(u16::MAX))
                .await?
                .0,
            CommandStatus::Rejected
        );
        admitted.item_fence = None;
        let error = exchange(&owner, &mut admitted, 5, command, &items::request(0))
            .await
            .err()
            .ok_or("a connection without an item fence remained usable")?;
        assert!(
            error.to_string().contains("AdmittedThenDisconnected"),
            "{error}"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
