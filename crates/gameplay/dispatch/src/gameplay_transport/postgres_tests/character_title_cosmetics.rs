//! P12a: admitted cosmetic definitions + existing account grant receipts -> title command.
//! The authored content below is a synthetic test artifact, never production content population.
use super::cyclopedia::support::exchange;
use super::fixture::{Harness, id, run};
use super::{Dependencies, TestResult, debug, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::cosmetics_fixture::{
    activated_with_optional_cosmetics, cosmetics_test_packet,
};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::character_info::CharacterTitle;
use serde_json::{Value, json};
use std::collections::BTreeMap;

const OUTFITS: [(&str, [u32; 2], CharacterTitle); 3] = [
    ("crown", [1457, 1456], CharacterTitle::AdmirerOfTheCrown),
    ("golden", [1211, 1210], CharacterTitle::BigSpender),
    (
        "bounac",
        [1437, 1436],
        CharacterTitle::RoyalBounaceanAdvisor,
    ),
];

#[test]
fn character_title_cosmetics_real_dispatch_requires_compatible_earned_facts() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "title cosmetics scenario panicked")??;
    eprintln!(
        "character_title_cosmetics_real_dispatch_requires_compatible_earned_facts POSTGRES_CASE_COMPLETED"
    );
    Ok(())
}

fn packet() -> TestResult<Vec<u8>> {
    let mut packet = cosmetics_test_packet();
    let mut look_sources = BTreeMap::new();
    for shard in packet["source_shards"].as_array().ok_or("source shards")? {
        let decoded: Value = serde_json::from_str(shard["json"].as_str().ok_or("source shard")?)?;
        for row in decoded["records"].as_array().ok_or("source records")? {
            look_sources.insert(
                row["source_id"].as_u64().ok_or("look id")?,
                row["source_record_sha256"].clone(),
            );
        }
    }
    let author = &mut packet["authoring"];
    author["document"]["content_revision"] = "content-1".into();
    let mut outfits = author["document"]["outfits"]
        .as_array()
        .ok_or("outfits")?
        .clone();
    let outfit_template = outfits[0].clone();
    let mount_template = author["document"]["mounts"][0].clone();
    let mut mappings = Vec::new();
    for (name, looks, _) in OUTFITS {
        let key = format!("oteryn:outfit.title_{name}");
        let mut outfit = outfit_template.clone();
        outfit["key"] = key.clone().into();
        outfit["starter_female"] = false.into();
        outfit["starter_male"] = false.into();
        outfit["starter_addons_female"] = 0.into();
        outfit["starter_addons_male"] = 0.into();
        for (sex, look) in ["female", "male"].into_iter().zip(looks) {
            outfit[sex] = json!({"id":look,"source_record_sha256":look_sources.get(&u64::from(look)).ok_or("donor look source")?});
        }
        outfits.push(outfit);
        mappings.push(json!({"grant_key":format!("title.{name}.base"),"target":{"kind":"Outfit","outfit_key":key},"compatible_provenance":["title-fixture-r1"]}));
        for addon in 1..=2 {
            mappings.push(json!({"grant_key":format!("title.{name}.addon{addon}"),"target":{"kind":"Addon","outfit_key":key,"addon":addon},"compatible_provenance":["title-fixture-r1"]}));
        }
    }
    // Six further identities give nine earned outfits (eighteen sex looks). An earned
    // addon on the available starter later supplies the tenth canonical identity.
    for index in 1..=6 {
        let key = format!("oteryn:outfit.title_extra_{index:02}");
        let mut outfit = outfit_template.clone();
        outfit["key"] = key.clone().into();
        outfit["starter_female"] = false.into();
        outfit["starter_male"] = false.into();
        outfit["starter_addons_female"] = 0.into();
        outfit["starter_addons_male"] = 0.into();
        outfits.push(outfit);
        mappings.push(json!({"grant_key":format!("title.extra.{index:02}"),"target":{"kind":"Outfit","outfit_key":key},"compatible_provenance":["title-fixture-r1"]}));
    }
    mappings.push(json!({"grant_key":"title.starter.addon2","target":{"kind":"Addon","outfit_key":"oteryn:outfit.proposed","addon":2},"compatible_provenance":["title-fixture-r1"]}));
    let mut mounts = Vec::new();
    for index in 1..=10 {
        let key = format!("oteryn:mount.title_{index:02}");
        let mut mount = mount_template.clone();
        mount["key"] = key.clone().into();
        mounts.push(mount);
        mappings.push(json!({"grant_key":format!("title.mount.{index:02}"),"target":{"kind":"Mount","mount_key":key},"compatible_provenance":["title-fixture-r1"]}));
    }
    for key in ["title.mount.stale", "title.mount.store"] {
        mappings.push(json!({"grant_key":key,"target":{"kind":"Mount","mount_key":"oteryn:mount.title_10"},"compatible_provenance":["title-fixture-r1"]}));
    }
    author["document"]["outfits"] = outfits.into();
    author["document"]["mounts"] = mounts.into();
    author["document"]["grant_mappings"] = mappings.into();
    let originals = author["promotions"].as_array().ok_or("promotions")?.clone();
    let mut promotions: Vec<_> = originals
        .iter()
        .filter(|row| {
            !row["path"].as_str().is_some_and(|path| {
                path.starts_with("/outfits/")
                    || path.starts_with("/mounts/")
                    || path == "/grant_mappings"
            })
        })
        .cloned()
        .collect();
    for family in ["outfits", "mounts"] {
        let prefix = format!("/{family}/0/");
        for (index, row) in author["document"][family]
            .as_array()
            .ok_or("cosmetics")?
            .iter()
            .enumerate()
        {
            for original in originals.iter().filter(|p| {
                p["path"]
                    .as_str()
                    .is_some_and(|path| path.starts_with(&prefix))
            }) {
                let field = original["path"]
                    .as_str()
                    .ok_or("promotion path")?
                    .rsplit('/')
                    .next()
                    .ok_or("promotion field")?;
                let mut promotion = original.clone();
                promotion["path"] = format!("/{family}/{index}/{field}").into();
                promotion["value"] = row[field].clone();
                promotions.push(promotion);
            }
        }
    }
    let mut mapping_promotion = originals.first().ok_or("promotion template")?.clone();
    mapping_promotion["path"] = "/grant_mappings".into();
    mapping_promotion["value"] = author["document"]["grant_mappings"].clone();
    promotions.push(mapping_promotion);
    author["promotions"] = promotions.into();
    Ok(serde_json::to_vec(&packet)?)
}

async fn seed(h: &Harness, keys: &[String], provenance: &str, source_kind: &str) -> TestResult {
    // Existing immutable appearance receipt/fact schema; no title state is seeded.
    let mut tx = h.pool.begin().await?;
    for key in keys {
        sqlx::query("WITH receipt AS (INSERT INTO game_account_appearance_unlock_receipts(grant_occurrence_id,account_id,unlock_key,content_revision,granting_character_id,source_kind,source_event_id) SELECT game_character_uuid_v7(),account_id,$2,$3,character_id,$4,$5 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid RETURNING grant_occurrence_id,account_id,unlock_key) INSERT INTO game_account_appearance_unlocks(account_id,unlock_key,grant_occurrence_id) SELECT account_id,unlock_key,grant_occurrence_id FROM receipt")
            .bind(id(41).as_slice()).bind(key).bind(provenance).bind(source_kind).bind(key.as_bytes())
            .execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn command(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let next = admitted.continuity.next_command_id;
    exchange(owner, admitted, next, 113, payload).await
}

fn scenario() -> TestResult {
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use crate::gameplay_transport::connection::FirstEntryOutcome;
    run(async |admin| {
        reset();
        let h = Harness::create_appearance(admin, "character_title_cosmetics").await?;
        prepare_current_session(&h).await?;
        let mut initial: Vec<_> = (1..=9)
            .map(|index| format!("title.mount.{index:02}"))
            .collect();
        for (name, _, _) in OUTFITS {
            initial.push(format!("title.{name}.base"));
            initial.push(format!("title.{name}.addon1"));
        }
        initial.extend((1..=6).map(|index| format!("title.extra.{index:02}")));
        seed(&h, &initial, "title-fixture-r1", "quest").await?;
        seed(
            &h,
            &["title.mount.stale".into()],
            "retired-provenance",
            "quest",
        )
        .await?;
        seed(
            &h,
            &["title.mount.store".into()],
            "title-fixture-r1",
            "store",
        )
        .await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_optional_cosmetics(world, None, Some(&packet()?));
        let room = pin.qualified_room().clone();
        let (channel_pin, _, _, _) = pin.into_channel_parts();
        let deps = Dependencies::with_content(
            channel_pin,
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = controller.active();
        deps.initialize(&owner).await;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let current = crate::gameplay_transport::character_gameplay_fence::current(
            &owner,
            deps.admitted.game_session_id,
            false,
        )
        .await
        .ok_or("current fence")?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(current.item_fence());
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            &crate::gameplay_transport::character_info::offered(&[], true),
            &[97],
        )
        .ok_or("capability")?;
        for title in OUTFITS.map(|(_, _, title)| title).into_iter().chain([
            CharacterTitle::BeaststriderGrade1,
            CharacterTitle::TibiasTopmodelGrade1,
        ]) {
            assert_eq!(
                command(&owner, &mut admitted, &[2, title.id()]).await?.0,
                CommandStatus::Rejected,
                "missing addons, incompatible provenance and Store facts cannot qualify {title:?}"
            );
        }
        let before: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_titles WHERE title BETWEEN 46 AND 58",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(before, 0);
        let mut remaining: Vec<_> = OUTFITS
            .iter()
            .map(|(name, _, _)| format!("title.{name}.addon2"))
            .collect();
        remaining.push("title.mount.10".to_owned());
        remaining.push("title.starter.addon2".to_owned());
        seed(&h, &remaining, "title-fixture-r1", "quest").await?;
        assert_eq!(
            command(&owner, &mut admitted, &[0, 0, 0, 1]).await?.0,
            CommandStatus::Accepted
        );
        let stored: Vec<i16> = sqlx::query_scalar(
            "SELECT title FROM game_character_titles WHERE title BETWEEN 46 AND 58 ORDER BY title",
        )
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(
            stored,
            [46, 47, 52, 53, 54],
            "ten mounts, nine base grants plus one earned starter addon identity, and all three exact full outfit pairs"
        );
        assert_eq!(
            command(
                &owner,
                &mut admitted,
                &[2, CharacterTitle::BeaststriderGrade2.id()]
            )
            .await?
            .0,
            CommandStatus::Rejected
        );
        for (_, _, title) in OUTFITS {
            assert_eq!(
                command(&owner, &mut admitted, &[2, title.id()]).await?,
                (CommandStatus::Accepted, vec![2, title.id()])
            );
        }
        assert_eq!(
            command(
                &owner,
                &mut admitted,
                &[3, CharacterTitle::AdmirerOfTheCrown.id()]
            )
            .await?
            .0,
            CommandStatus::Accepted
        );
        // Cosmetic titles are permanent: a later view without an available definition does
        // not revoke historical achievement or the selected title.
        owner.active_generation = None;
        let (status, payload) = command(&owner, &mut admitted, &[0, 0, 0, 1]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            payload.last().copied(),
            Some(CharacterTitle::AdmirerOfTheCrown.id())
        );
        let after: Vec<i16> = sqlx::query_scalar(
            "SELECT title FROM game_character_titles WHERE title BETWEEN 46 AND 58 ORDER BY title",
        )
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(
            after, stored,
            "replay and content absence do not duplicate or revoke permanent grants"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
