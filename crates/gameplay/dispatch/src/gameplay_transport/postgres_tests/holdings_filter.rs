//! Filtering must precede paging through the admitted server and the retained client model.
use super::fixture::id;
use super::fixture::{Harness, run};
use super::{Dependencies, TestResult, debug, prepare_current_session, reset};
use crate::gameplay_transport::{
    capabilities::{OfferedCapability, SelectedCapabilities},
    connection::FirstEntryOutcome,
};
use oteryn_client_ui::cyclopedia_character::{
    CyclopediaCharacterPanel, Intent, apply_result, request,
};
use oteryn_protocol_oteryn::cyclopedia_character::items::Location;
use oteryn_session::{CommandStatus, GatedCommandResult};
#[test]
fn holdings_filter_reaches_off_page_display_name_through_dispatch_and_client_model() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256 << 20).spawn(|| {
        run(async |admin| {
            reset();
            let h = Harness::create_with_admission_seed(admin, "holdings_filter", true, false).await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let deps = Dependencies::new()?;
            deps.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
            let (controller, _) = super::death_ml_loss_composed::production_generation_with_pin(deps.admitted.world_id, "content-1")?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = controller.active();
            deps.initialize(&owner).await;
            let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id).await?.0;
            let mut admitted = deps.admitted;
            admitted.first_entry = FirstEntryOutcome::Positioned;
            admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
            let offers = [OfferedCapability { id: 199, requires: &[] }, OfferedCapability { id: 501, requires: &[199] }];
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(&offers, &[199,501]).ok_or("select holdings")?;
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
            for i in 0..26u8 {
                let key = if i == 25 { "oteryn:item.tibia.i3031".to_owned() } else { format!("oteryn:item.a{i:02}") };
                sqlx::query("INSERT INTO game_character_stash_balances(character_id,world_id,definition_key,definition_revision,quantity,last_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,'definition-r1',7,encode($4,'hex')::uuid)")
                    .bind(id(41).as_slice()).bind(admitted.world_id.as_bytes().as_slice()).bind(key).bind(id(100+i).as_slice()).execute(&mut *tx).await?;
            }
            tx.commit().await?;
            let mut panel = CyclopediaCharacterPanel { selected: true, holdings_selected: true, ..Default::default() };
            panel.items.search = "GoLd CoIn".into();
            panel.items.location = Some(Location::Stash);
            let command = request(&mut panel, Intent::Items).ok_or("holdings request")?;
            let (status, payload) = super::cyclopedia::support::exchange(&owner, &mut admitted, 1, command.kind, &command.payload).await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert!(apply_result(&mut panel, &GatedCommandResult { command_id: 1, status, server_sequence: 1, payload }));
            let page = panel.items.page.as_ref().ok_or("holdings page")?;
            assert_eq!((page.offset,page.total,page.rows.len()),(0,1,1));
            assert_eq!(panel.items.display_name(&page.rows[0]), "gold coin");
            assert_eq!(page.rows[0].quantities[2], 7);
            assert_eq!(panel.items.rows().len(), 1);
            assert!(request(&mut panel, Intent::ItemsNext).is_none());
            let command=oteryn_client_ui::cyclopedia_character::holdings_filter(
                &mut panel,"oteryn:item.tibia.i3031",Some(Location::Stash),
            ).ok_or("long key filter")?;
            assert!(command.payload.len()>20);
            let (status,payload)=super::cyclopedia::support::exchange(&owner,&mut admitted,2,command.kind,&command.payload).await?;
            assert_eq!(status,CommandStatus::Accepted);
            assert!(apply_result(&mut panel,&GatedCommandResult {command_id:2,status,server_sequence:2,payload}));
            assert_eq!(panel.items.page.as_ref().ok_or("key page")?.total,1);

            admitted.continuity.selected_capabilities=SelectedCapabilities::select(&offers,&[199]).ok_or("legacy selection")?;
            assert_eq!(super::cyclopedia::support::exchange(&owner,&mut admitted,3,command.kind,&command.payload).await?.0,CommandStatus::Rejected);
            drop(owner); drop(authority); drop(seal);
            h.cleanup().await
        }).map_err(|error| error.to_string())
    })?.join().map_err(|_| "holdings filter thread failed")??;
    Ok(())
}
