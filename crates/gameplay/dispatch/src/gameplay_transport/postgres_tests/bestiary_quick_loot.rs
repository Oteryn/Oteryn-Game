use super::cyclopedia_fixtures::support::exchange;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::charm_progression as bestiary;
use oteryn_protocol_oteryn::charm_progression::catalogue::{Request, quick_loot};
use oteryn_protocol_oteryn::quick_loot as loot;

#[test]
fn bestiary_drop_reference_adds_and_removes_quick_loot_filter_through_dispatch() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Bestiary Quick Loot thread failed")?
        .map_err(Into::into)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "bestiary_quick_loot", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                world,
                "content-1",
            )?;
        let frame = pin.frame_binding().digest();
        let room = pin.qualified_room().clone();
        let deps = Dependencies::with_content(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                crate::foundation::MovementLocalPosition {
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = controller.active();
        deps.initialize(&owner).await;
        let active = controller.active().ok_or("active generation")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(active.identity().server_artifact_digest().as_slice())
            .bind(active.identity().client_artifact_digest().as_slice()).bind(frame.as_slice())
            .execute(&h.pool).await?;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        let fence = admitted.item_fence.ok_or("item fence")?;
        owner
            .runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        let base = [
            OfferedCapability {
                id: 4,
                requires: &[],
            },
            OfferedCapability {
                id: 6,
                requires: &[],
            },
            OfferedCapability {
                id: 98,
                requires: &[4],
            },
        ];
        use crate::gameplay_transport::charm_progression::offered_for_sources;
        let without_index = offered_for_sources(&base, (true, false));
        assert!(
            !without_index
                .iter()
                .any(|offer| offer.id == quick_loot::CAPABILITY)
        );
        let offers = offered_for_sources(&base, owner.charm_progression_offers_available());
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[4, 6, 67, 98]).ok_or("legacy selection")?;

        let gold = "oteryn:item.tibia.i3031";
        let races =
            crate::gameplay_transport::charm_progression::canonical_races().ok_or("races")?;
        let (race, item) = races
            .iter()
            .enumerate()
            .find_map(|(index, race)| {
                let reward = native.reward_table().row(race.key()).ok()?;
                let drop = reward.loot_table.entries.first()?;
                (drop.item.production_key == gold).then(|| (index + 1, drop.item.clone()))
            })
            .ok_or("creature with first drop gold coin")?;
        let query = quick_loot::request(Request {
            after: 0,
            race: u16::try_from(race)?,
            category: 0,
        })
        .map_err(debug)?;
        assert_eq!(
            exchange(
                &owner,
                &mut admitted,
                1,
                bestiary::COMMAND_TYPE_CHARM_PROGRESSION,
                &query
            )
            .await?
            .0,
            CommandStatus::Rejected
        );
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[4, 6, 67, 98, 609]).ok_or("drop selection")?;
        let (status, bytes) = exchange(
            &owner,
            &mut admitted,
            2,
            bestiary::COMMAND_TYPE_CHARM_PROGRESSION,
            &query,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        let page = quick_loot::Page::decode(&bytes).map_err(debug)?;
        assert!(page.catalogue.cards[0].detail.contains("gold coin"));
        let reference = page
            .references
            .first()
            .copied()
            .flatten()
            .ok_or("drop was not editable")?;
        assert_eq!(
            native
                .item_index()
                .and_then(|index| index.definition_at(reference)),
            Some((item.production_key.as_str(), item.revision_ref.as_str())),
        );
        let legacy = bestiary::catalogue::encode_request(
            bestiary::catalogue::quick_loot::decode_request(&query).map_err(debug)?,
        )
        .map_err(debug)?;
        let (status, bytes) = exchange(
            &owner,
            &mut admitted,
            3,
            bestiary::COMMAND_TYPE_CHARM_PROGRESSION,
            &legacy,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            bestiary::catalogue::decode_page(&bytes).map_err(debug)?,
            page.catalogue
        );
        for (id, action, present) in [
            (4, loot::QuickLootAction::FilterAdd, true),
            (5, loot::QuickLootAction::FilterRemove, false),
        ] {
            let bytes = loot::encode_quick_loot_request(&loot::QuickLootRequest {
                action,
                corpse: None,
                selected_item: None,
                review_filters: false,
                category: None,
                recipient: None,
                recipient_id: None,
                fallback_to_main: false,
                filter_mode: Some(loot::QuickLootFilterMode::Skip),
                filter_definition_ref: Some(reference.get()),
            })
            .map_err(debug)?;
            assert_eq!(
                exchange(
                    &owner,
                    &mut admitted,
                    id,
                    loot::COMMAND_TYPE_QUICK_LOOT,
                    &bytes
                )
                .await?
                .0,
                CommandStatus::Accepted
            );
            let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_quick_loot_filters WHERE character_id=encode($1,'hex')::uuid AND NOT accepted AND item_production_key=$2)")
                .bind(admitted.item_fence.ok_or("item fence")?.character_id.as_bytes().as_slice())
                .bind(gold).fetch_one(&h.pool).await?;
            assert_eq!(exists, present);
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
