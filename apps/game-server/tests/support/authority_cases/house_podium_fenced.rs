//! W18: the runtime podium path (`DurabilityRoot::set_house_podium` /
//! `load_house_podiums`, migration 0561) under the named-House session fence, its
//! content and ownership checks, and persistence across a restarted durability owner.
use super::house_podium::{current_house, house_fence, placed_podiums};
use crate::domain::appearance::{AppearanceContent, EarnedCosmetic, Sex, UnlockFact};
use crate::durability::DurabilityRoot;
use crate::durability::house_podium::{HousePodiumRefusal, HousePodiumSet};
use crate::foundation::GameSessionId;
use crate::item_transfer_postgres_cases as native;
use crate::item_transfer_postgres_cases::{
    CHARACTER, Harness, TestResult, configured_admin, id, runtime,
};
use oteryn_gameplay_housing::house_podium::PodiumContent;

const OUTFIT: &str = "oteryn:content.outfit.citizen";
const MOUNT: &str = "oteryn:content.mount.widow_queen";
const BOSS: &str = "oteryn:creature.ferumbras";
const RAT: &str = "oteryn:creature.rat";
const ZEBRA: &str = "oteryn:content.mount.zebra";
const ACCOUNT: u8 = 40;

/// Citizen is a starter outfit; the zebra mount is earned through the `zebra` grant and
/// the widow queen mount exists but is never earned.
struct Content;
impl AppearanceContent<String> for Content {
    fn palette_size(&self) -> u16 {
        133
    }
    fn earned_grant_keys(&self) -> Vec<String> {
        vec!["zebra".into()]
    }
    fn earned_unlock(
        &self,
        grant: &str,
        provenance: &str,
        target: EarnedCosmetic<'_>,
    ) -> UnlockFact {
        if grant == "zebra" && provenance == "content-1" && target == EarnedCosmetic::Mount(ZEBRA) {
            UnlockFact::Compatible
        } else {
            UnlockFact::Absent
        }
    }
    fn outfit_addon_count(&self, outfit: &String) -> Option<u8> {
        (outfit == OUTFIT).then_some(2)
    }
    fn outfit_is_starter(&self, outfit: &String, _: Sex) -> bool {
        outfit == OUTFIT
    }
    fn addon_is_starter(&self, _: &String, _: Sex, _: u8) -> bool {
        false
    }
    fn mount_exists(&self, mount: &String) -> bool {
        mount == MOUNT || mount == ZEBRA
    }
}
impl PodiumContent for Content {
    fn has_outfit(&self, key: &str) -> bool {
        key == OUTFIT
    }
    fn has_mount(&self, key: &str) -> bool {
        key == MOUNT || key == ZEBRA
    }
    fn has_creature(&self, key: &str, boss: bool) -> bool {
        key == if boss { BOSS } else { RAT }
    }
}

fn set(item: [u8; 16], expected: u64, kind: u8, key: &str) -> HousePodiumSet {
    HousePodiumSet {
        item_instance_id: item,
        expected_revision: expected,
        display_kind: kind,
        display_key: key.into(),
        direction: 1,
    }
}

#[test]
fn house_podium_runtime_path_is_fenced_checked_and_survives_restart() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async {
        let h = Harness::create(admin, "house_podium_fenced").await?;
        let result: TestResult = async {
            println!("POSTGRES_STARTED house_podium_fenced");
            let items = placed_podiums(&h).await?;
            let item = |seed: u8| items.get(&seed).copied().ok_or("podium");
            // A quest granted the account the zebra; this fixture's Character has no creation sex, so
            // it can wear no outfit (`character_sex::read`).
            sqlx::query("INSERT INTO game_account_appearance_unlock_receipts VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'zebra','content-1',encode($3,'hex')::uuid,'quest',$1)")
                .bind(id(70).as_slice()).bind(id(ACCOUNT).as_slice()).bind(id(CHARACTER).as_slice())
                .execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_account_appearance_unlocks VALUES(encode($1,'hex')::uuid,'zebra',encode($2,'hex')::uuid)")
                .bind(id(ACCOUNT).as_slice()).bind(id(70).as_slice())
                .execute(&h.pool).await?;
            let house = current_house()?;
            let fence = house_fence()?;
            let put = |request| h.root.set_house_podium(&h.node, fence, house.clone(), request, Content);
            assert_eq!(put(set(item(91)?, 0, 2, ZEBRA)).await.map_err(native::debug)?, Ok(1));
            assert_eq!(put(set(item(92)?, 0, 3, BOSS)).await.map_err(native::debug)?, Ok(1));
            assert_eq!(put(set(item(94)?, 0, 3, RAT)).await.map_err(native::debug)?, Ok(1));

            // Each refusal changes one fact and leaves the stored displays unchanged.
            for (request, refusal) in [
                (set(item(91)?, 1, 2, MOUNT), HousePodiumRefusal::NotOwned),
                (set(item(91)?, 1, 1, OUTFIT), HousePodiumRefusal::NotOwned),
                (set(item(91)?, 1, 1, "oteryn:content.outfit.invented"), HousePodiumRefusal::UnknownContent),
                (set(item(92)?, 1, 3, RAT), HousePodiumRefusal::UnknownContent),
                (set(item(94)?, 1, 3, BOSS), HousePodiumRefusal::UnknownContent),
                (set(item(92)?, 1, 1, OUTFIT), HousePodiumRefusal::WrongDisplay),
                (set(item(91)?, 1, 3, RAT), HousePodiumRefusal::WrongDisplay),
                (set(item(93)?, 0, 1, OUTFIT), HousePodiumRefusal::WrongItem),
                (set(id(99), 0, 1, OUTFIT), HousePodiumRefusal::WrongItem),
                (set(item(91)?, 0, 2, ZEBRA), HousePodiumRefusal::Changed),
                (HousePodiumSet { direction: 4, ..set(item(91)?, 1, 1, OUTFIT) }, HousePodiumRefusal::InvalidInput),
            ] {
                assert_eq!(put(request).await.map_err(native::debug)?, Err(refusal));
            }
            let stale = crate::durability::item_transfer::CurrentCharacterItemFence {
                game_session_id: GameSessionId::decode(&native::id(62)).map_err(native::debug)?,
                ..fence
            };
            assert_eq!(
                h.root.set_house_podium(&h.node, stale, house.clone(), set(item(91)?, 1, 1, OUTFIT), Content)
                    .await.map_err(native::debug)?,
                Err(HousePodiumRefusal::StaleSession)
            );
            assert_eq!(
                h.root.load_house_podiums(&h.node, stale, house.clone()).await.map_err(native::debug)?,
                Err(HousePodiumRefusal::StaleSession)
            );

            // The runtime role can call the fenced reader, which still refuses a stale session.
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
            let denied = sqlx::query("SELECT * FROM game_house_podium_load_fenced(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,2,1,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid,$5)")
                .bind(native::id(62).as_slice()).bind(id(CHARACTER).as_slice())
                .bind(id(1).as_slice()).bind(id(native::WORLD).as_slice()).bind(house.house_key())
                .fetch_all(&mut *tx).await.expect_err("stale session");
            assert_eq!(denied.as_database_error().map(|e| e.message()), Some("PODIUM_STALE_SESSION"));
            tx.rollback().await?;

            // A restarted durability owner on a new pool reads the committed displays.
            let restarted = DurabilityRoot::connect_test_runtime(&h.database.url)?;
            assert!(restarted.maintain_ready_once().await?);
            let mut shown = restarted
                .load_house_podiums(&h.node, fence, house.clone())
                .await
                .map_err(native::debug)?
                .map_err(native::debug)?
                .into_iter()
                .map(|d| (d.item_instance_id, d.podium_kind, d.display_kind, d.display_key, d.direction, d.revision))
                .collect::<Vec<_>>();
            shown.sort();
            let mut expected = vec![
                (item(91)?, 1, 2, ZEBRA.to_owned(), 1, 1),
                (item(92)?, 2, 3, BOSS.to_owned(), 1, 1),
                (item(94)?, 3, 3, RAT.to_owned(), 1, 1),
            ];
            expected.sort();
            assert_eq!(shown, expected);
            assert_eq!(
                restarted.set_house_podium(&h.node, fence, house.clone(), set(item(94)?, 1, 3, RAT), Content)
                    .await.map_err(native::debug)?,
                Ok(2)
            );
            Ok(())
        }
        .await;
        h.cleanup().await?;
        result
    })
}
