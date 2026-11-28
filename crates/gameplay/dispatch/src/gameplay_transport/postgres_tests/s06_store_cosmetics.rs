//! Global Store cosmetic offers, using the production Store and appearance command routes.
use super::fixture::{Harness, debug, id};
use super::store_fixtures::{FakePlatform, Ledger, purchase, purchase_request, serve, uuid};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::AdmittedSession;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::native_admission_source::store::StoreClient;
use oteryn_protocol_oteryn::{appearance, store};
use std::sync::{Arc, Mutex};

const MOUNTS: &[(&str, u64, u8)] = &[
    ("arctic-unicorn", 870, 0),
    ("batcat", 870, 0),
    ("jungle-tiger", 750, 0),
];
const OUTFITS: &[(&str, u64, u8)] = &[
    ("ceremonial-garb", 750, 3),
    ("retro-citizen", 870, 0),
    ("retro-hunter", 870, 0),
    ("retro-knight", 870, 0),
    ("retro-mage", 870, 0),
];

#[test]
fn s06_global_store_mounts_are_selectable_and_charge_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, true)
}

#[test]
fn s06_global_store_outfits_and_addons_are_selectable_and_charge_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, false)
}

fn run(admin: String, mounts: bool) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("s06-store-cosmetics", move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(scenario(admin, mounts))
    })
}

async fn scenario(admin: String, mounts: bool) -> TestResult {
    let h = Harness::create_appearance(admin, "s06_store_mounts").await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (generation, pin) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(world, "content-1")?;
    let room = pin.qualified_room().clone();
    let (channel_pin, _, _, _) = pin.into_channel_parts();
    let deps = Dependencies::with_content(
        channel_pin,
        room,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    let mut admitted = deps.admitted;
    let ledger = Arc::new(Mutex::new(Ledger {
        balance: 10_000,
        world: uuid(admitted.world_id.as_bytes()),
        enforce_active_import: true,
        ..Ledger::default()
    }));
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = generation.active();
    owner.store = Some(Arc::new(tokio::sync::Mutex::new(StoreClient::new(
        Box::new(FakePlatform(ledger.clone())),
        "oteryn-game/store-consumer".into(),
    ))));
    {
        let mut runtime = deps.runtime.lock().await;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                deps.actor,
                admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
    }
    deps.initialize(&owner).await;
    let (session, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(admitted.game_session_id)
        .await?;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(session);
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        &offered,
        &[
            3,
            4,
            6,
            appearance::CAPABILITY_APPEARANCE_V1,
            store::CAPABILITY_STORE_V1,
        ],
    )
    .ok_or("Store and appearance capabilities")?;

    let (category, prefix, cases) = if mounts {
        ("cosmetics.mounts", "mount", MOUNTS)
    } else {
        ("cosmetics.outfits", "outfit", OUTFITS)
    };
    let quotes = catalogue(&owner, &mut admitted, category).await?;
    if mounts {
        assert!(
            quotes
                .iter()
                .all(|offer| !offer.offer_key.starts_with("store:mount:benevolent-"))
        );
    }

    let before = super::item_cosmetics_fixtures::positive::appearance(&owner, &admitted).await?;
    for (index, &(slug, price, addons)) in cases.iter().enumerate() {
        let offer = format!("store:{prefix}:{slug}");
        let key = if mounts {
            format!("oteryn:content.mount.{}", slug.replace('-', "_"))
        } else {
            format!("oteryn:outfit.{}", slug.replace('-', "_"))
        };
        let quoted = quotes
            .iter()
            .find(|quote| quote.offer_key == offer)
            .ok_or_else(|| format!("missing Store quote for {offer}"))?;
        assert_eq!(quoted.price, price);
        assert_eq!(quoted.currency, "TIBIA_COIN");
        assert!(
            before
                .catalogue
                .mounts
                .iter()
                .all(|option| option.key != key)
        );
        assert!(
            before
                .catalogue
                .outfits
                .iter()
                .all(|option| option.key != key)
        );

        let seed = 80 + 2 * u8::try_from(index)?;
        let request = purchase_request(seed, &offer, price)?;
        let receipt = purchase(&owner, &mut admitted, request.clone()).await?;
        assert_eq!(receipt.code, store::StorePurchaseCode::Accepted, "{offer}");
        assert_eq!(purchase(&owner, &mut admitted, request).await?, receipt);
        let duplicate = purchase_request(seed + 1, &offer, price)?;
        assert_eq!(
            purchase(&owner, &mut admitted, duplicate).await?.code,
            store::StorePurchaseCode::OfferUnavailable,
        );

        let after = super::item_cosmetics_fixtures::positive::appearance(&owner, &admitted).await?;
        let outfit = if mounts {
            assert!(
                after
                    .catalogue
                    .mounts
                    .iter()
                    .any(|option| option.key == key)
            );
            after.catalogue.outfits.first().ok_or("starter outfit")?
        } else {
            let outfit = after
                .catalogue
                .outfits
                .iter()
                .find(|option| option.key == key)
                .ok_or("purchased outfit")?;
            assert_eq!(outfit.permitted_addons, addons);
            outfit
        };
        let change = appearance::Change {
            context: after.catalogue.context,
            expected_root_revision: after.catalogue.root_revision,
            prior: after.catalogue.selected,
            selection: appearance::Selection {
                outfit_key: outfit.key.clone(),
                colours: [0; 4],
                addons,
                mount_key: mounts.then(|| key.clone()),
            },
            content_revision: owner
                .appearance_context()
                .ok_or("appearance content")?
                .1
                .content_revision()
                .into(),
        };
        let command = admitted.continuity.next_command_id;
        let result = super::quest_world_callbacks_fixtures::wire(
            &owner,
            &mut admitted,
            command,
            appearance::COMMAND_TYPE_APPEARANCE_CHANGE,
            change.encode().map_err(debug)?,
        )
        .await?;
        assert!(matches!(
            appearance::ChangeResult::decode(&result).map_err(debug)?,
            appearance::ChangeResult::Committed(_),
        ));
        let saved = super::item_cosmetics_fixtures::positive::appearance(&owner, &admitted).await?;
        let selection = saved
            .catalogue
            .selected
            .ok_or("persisted selection")?
            .selection;
        if mounts {
            assert_eq!(selection.mount_key.as_deref(), Some(key.as_str()));
        } else {
            assert_eq!(selection.outfit_key, key);
            assert_eq!(selection.addons, addons);
        }
    }
    assert_eq!(
        ledger.lock().map_err(|_| "Store ledger")?.debits,
        cases.iter().map(|(_, price, _)| *price).collect::<Vec<_>>(),
    );
    let counts: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_character_store_cosmetics),
                (SELECT count(*) FROM store_delivery_receipts)",
    )
    .fetch_one(&h.pool)
    .await?;
    let count = i64::try_from(cases.len())?;
    assert_eq!(counts, (count, count));
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

async fn catalogue(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    category: &str,
) -> TestResult<Vec<store::StoreOffer>> {
    let mut request = store::StoreCatalogueRequest {
        category: category.into(),
        page_cursor: String::new(),
    };
    let mut offers = Vec::new();
    for _ in 0..4 {
        let values = serve(
            owner,
            admitted,
            &[(
                store::COMMAND_TYPE_STORE_CATALOGUE_PAGE,
                store::encode_catalogue_request(&request).map_err(debug)?,
            )],
        )
        .await?;
        let page = store::decode_catalogue_page(&values[0].1).map_err(debug)?;
        offers.extend(page.offers);
        if page.next_page_cursor.is_empty() {
            return Ok(offers);
        }
        request.page_cursor = page.next_page_cursor;
    }
    Err("Store catalogue paging did not finish".into())
}
