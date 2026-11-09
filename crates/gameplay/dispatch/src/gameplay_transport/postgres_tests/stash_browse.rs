//! Global stash selection through the production fenced reader, beyond the first page.
use super::death_ml_loss_composed::production_generation_with_pin;
use super::fixture::{Harness, debug, id, run};
use super::{TestResult, prepare_current_session};
use crate::domain::CharacterId;
use crate::durability::character_stash::StashCatalogueEntry;
use crate::foundation::WorldId;
use oteryn_protocol_oteryn::stash::StashFilter;

#[test]
fn stash_global_filter_precedes_pagination_and_stock_order_is_stable() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "stash browse panicked")??;
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "stash_global_browse", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let character = CharacterId::from_bytes(id(41)).map_err(debug)?;
        let (controller, _) = production_generation_with_pin(world, "content-1")?;
        let native = controller
            .active()
            .and_then(|active| active.native_gameplay())
            .ok_or("native pin")?;
        // Historical balances can reference absent revisions. Administrative fixture
        // seeding avoids manufacturing conversion receipts; the production reader is unchanged.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        for index in 0..40 {
            sqlx::query("INSERT INTO game_character_stash_balances(character_id,world_id,definition_key,definition_revision,quantity,last_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,'r1',$4,game_character_uuid_v7())")
                .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(format!("item.{index:03}"))
                .bind(i64::from(index+1)).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        // Resolve real item category and trader membership from the admitted book,
        // rather than accepting definition/category claims from the client.
        use crate::content::{
            ProjectV2ServiceOffer, ProjectV2ServiceOfferDirection, ReferenceItemField::Known,
            ReferenceItemType,
        };
        use crate::gameplay_transport::npc_talk::{NpcTalkBook, NpcTalkEntry};
        let rune = native.item_profiles().find(|record| matches!(&record.semantics.classification, Known(classification) if classification.item_type == Known(ReferenceItemType::Rune))).ok_or("rune profile")?;
        let offer = ProjectV2ServiceOffer {
            item: rune.authoring.item.clone(),
            direction: ProjectV2ServiceOfferDirection::BuyFromPlayer,
            unit_price: 1,
            currency: None,
            count: None,
            sub_type: None,
            parity_pending: false,
        };
        let book = NpcTalkBook::default().with(
            "npc.test",
            NpcTalkEntry::new("Sam", None).with_offers(vec![offer]),
        );
        let criteria = StashFilter {
            category: "RUNE".into(),
            trader: "sam".into(),
            ..Default::default()
        };
        let resolved =
            crate::gameplay_transport::stash::browse::catalogue(native, Some(&book), &criteria);
        assert!(
            resolved.iter().any(
                |entry| entry.key == rune.production_definition.production_key && entry.matched
            )
        );
        assert_eq!(resolved.iter().filter(|entry| entry.matched).count(), 1);
        let absent = StashFilter {
            trader: "Absent trader".into(),
            ..criteria
        };
        assert!(
            !crate::gameplay_transport::stash::browse::catalogue(native, Some(&book), &absent)
                .iter()
                .any(|entry| entry.matched)
        );
        let metadata: Vec<_> = (0..40)
            .map(|index| StashCatalogueEntry {
                key: format!("item.{index:03}"),
                revision: "r1".into(),
                name: format!("Item {index:03}"),
                matched: index >= 35,
            })
            .collect();
        let filter = StashFilter {
            search: "last five".into(),
            stock_order: true,
            ..Default::default()
        };
        let (rows, more) = h
            .root
            .read_stash_filtered_page(&authority, character, 0, native, &filter, &metadata)
            .await
            .map_err(debug)?;
        assert!(!more);
        assert_eq!(
            rows.iter().map(|row| row.quantity).collect::<Vec<_>>(),
            vec![40, 39, 38, 37, 36]
        );
        assert!(rows.iter().all(|row| row.migration_required));
        let (legacy, _) = h
            .root
            .read_stash_page(&authority, character, 0, native)
            .await
            .map_err(debug)?;
        assert!(!legacy.iter().any(|row| row.handle == rows[0].handle));
        let (first, more) = h
            .root
            .read_stash_filtered_page(
                &authority,
                character,
                0,
                native,
                &StashFilter::default(),
                &metadata,
            )
            .await
            .map_err(debug)?;
        assert!(more);
        assert_eq!(first.len(), 32);
        let (second, more) = h
            .root
            .read_stash_filtered_page(
                &authority,
                character,
                1,
                native,
                &StashFilter::default(),
                &metadata,
            )
            .await
            .map_err(debug)?;
        assert!(!more);
        assert_eq!(second.len(), 8);
        assert_eq!(second[7].handle, rows[0].handle);
        let historical = StashFilter {
            search: "ITEM.039".into(),
            ..Default::default()
        };
        let (historical, more) = h
            .root
            .read_stash_filtered_page(&authority, character, 0, native, &historical, &[])
            .await
            .map_err(debug)?;
        assert!(!more);
        assert_eq!(historical.len(), 1);
        assert_eq!(historical[0].handle, rows[0].handle);
        Ok(())
    })
}
