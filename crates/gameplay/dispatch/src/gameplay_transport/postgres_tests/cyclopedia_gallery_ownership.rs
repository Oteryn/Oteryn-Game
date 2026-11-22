use super::cyclopedia_fixtures::support::exchange;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_familiar::{
    DurableFamiliarState, FamiliarLifecycleV2, FamiliarLifecycleWriter, FamiliarStateOccurrence,
    FamiliarStateRequest,
};
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use gallery::owned::{Page, Query, Source};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::cyclopedia_character::{
    COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB, gallery,
};

#[test]
fn cyclopedia_gallery_filters_acquisition_sources_and_lists_owned_familiars_through_dispatch()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Cyclopedia gallery thread failed")?
        .map_err(Into::into)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "cycler_gallery", true, false).await?;
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
        let room = pin.qualified_room().clone();
        let deps = Dependencies::with_content(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = controller.active();
        deps.initialize(&owner).await;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence =
            super::death_ml_loss_composed_fixtures::current_fence(&owner, admitted.game_session_id)
                .await?;
        admitted.item_fence = Some(fence.item_fence());

        let active = controller.active().ok_or("active generation")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        let mut familiar_looks = native
            .policies()
            .policies()
            .iter()
            .filter(|policy| policy.is_familiar)
            .filter_map(|policy| policy.outfit_appearance());
        let familiar_look = familiar_looks.next().ok_or("first familiar look")?;
        let unowned = familiar_looks
            .find(|look| *look != familiar_look)
            .ok_or("second familiar look")?;
        let progression = h
            .root
            .read_character_progression(&authority, fence.character_id)
            .await
            .map_err(debug)?
            .ok_or("progression")?;
        h.root
            .commit_character_familiar_state(
                &authority,
                &h.node,
                fence,
                FamiliarStateRequest {
                    occurrence: FamiliarStateOccurrence::from_bytes(id(183)).map_err(debug)?,
                    before: DurableFamiliarState::default(),
                    after: DurableFamiliarState {
                        lifecycle: Some(FamiliarLifecycleV2 {
                            version: 2,
                            familiar_remaining_ms: 0,
                            open: false,
                            writer: FamiliarLifecycleWriter::from_fence(&fence).map_err(debug)?,
                            clean_end_game_session_id: None,
                        }),
                        granted_looks: vec![familiar_look],
                        ..DurableFamiliarState::default()
                    },
                    content_revision: progression.context.content,
                    policy_revision: progression.policy_revision,
                    policy_digest: native.source_digest(),
                },
            )
            .await
            .map_err(debug)?;
        grant_quest_outfit(&h, fence.character_id).await?;

        let offers = [
            OfferedCapability {
                id: 199,
                requires: &[],
            },
            OfferedCapability {
                id: 611,
                requires: &[199],
            },
        ];
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[199]).ok_or("legacy selection")?;
        let command = COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB;
        let query = Query {
            offset: 0,
            outfit_source: None,
        }
        .encode();
        assert_eq!(
            exchange(&owner, &mut admitted, 1, command, &query).await?.0,
            CommandStatus::Rejected
        );
        let (status, bytes) =
            exchange(&owner, &mut admitted, 2, command, &gallery::request(0)).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert!(
            gallery::Page::decode(&bytes)
                .map_err(debug)?
                .entries
                .iter()
                .any(|entry| {
                    entry.kind == gallery::Kind::Familiar && entry.look_type == unowned
                })
        );

        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[199, 611]).ok_or("gallery selection")?;
        let (status, bytes) = exchange(&owner, &mut admitted, 3, command, &query).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let page = Page::decode(&bytes).map_err(debug)?;
        assert!(
            page.gallery
                .entries
                .iter()
                .zip(&page.sources)
                .any(|(entry, sources)| {
                    entry.kind == gallery::Kind::Familiar
                        && entry.look_type == familiar_look
                        && *sources == Source::FamiliarGrant as u8
                })
        );
        assert!(
            !page.gallery.entries.iter().any(|entry| {
                entry.kind == gallery::Kind::Familiar && entry.look_type == unowned
            })
        );

        let quest = Query {
            offset: 0,
            outfit_source: Some(Source::Quest),
        }
        .encode();
        let (status, bytes) = exchange(&owner, &mut admitted, 4, command, &quest).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let page = Page::decode(&bytes).map_err(debug)?;
        assert_eq!(page.gallery.total, 1);
        assert_eq!(page.gallery.entries[0].kind, gallery::Kind::Outfit);
        assert_eq!(page.gallery.entries[0].key, "oteryn:outfit.afflicted");
        assert_eq!(page.sources, [Source::Quest as u8]);

        let starter = Query {
            offset: 0,
            outfit_source: Some(Source::Starter),
        }
        .encode();
        let (status, bytes) = exchange(&owner, &mut admitted, 5, command, &starter).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let page = Page::decode(&bytes).map_err(debug)?;
        assert!(!page.gallery.entries.is_empty());
        assert!(
            page.gallery
                .entries
                .iter()
                .all(|entry| entry.kind == gallery::Kind::Outfit)
        );
        assert!(
            page.sources
                .iter()
                .all(|sources| sources & Source::Starter as u8 != 0)
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn grant_quest_outfit(h: &Harness, character: crate::domain::CharacterId) -> TestResult {
    let key = "oteryn:outfit.afflicted/quest-grant/0";
    let account: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?;
    let event = sqlx::types::Uuid::from_bytes(id(184));
    let mut tx = h.pool.begin().await?;
    sqlx::query("INSERT INTO game_account_appearance_unlock_receipts(grant_occurrence_id,account_id,unlock_key,content_revision,granting_character_id,source_kind,source_event_id) VALUES($1,$2,$3,$4,encode($5,'hex')::uuid,'quest',$6)")
        .bind(event).bind(account).bind(key).bind(oteryn_gameplay_quests::quest::reward_callbacks::PROVENANCE)
        .bind(character.as_bytes().as_slice()).bind(event.as_bytes().as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_account_appearance_unlocks(account_id,unlock_key,grant_occurrence_id) VALUES($1,$2,$3)")
        .bind(account).bind(key).bind(event).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
