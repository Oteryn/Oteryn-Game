//! Missing L2 source bindings are checked only after fresh PostgreSQL admission.
//! Complete player journeys are additionally required before quest qualification.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

#[test]
fn questl2_sam_armor_has_exact_source_eligibility_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l2-batch5-sam")
}
#[test]
fn questl2_outfox_issues_both_source_storages_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l2-batch5-outfox")
}
#[test]
fn questl2_bone_pit_issues_the_source_container_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l2-batch5-bone")
}

pub(super) async fn exercise(
    _owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    _admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_SOURCE_BINDING {name} server_version={version}");
    let written: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_tracks")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(written, 0, "fresh admission must not seed mission progress");
    if name.ends_with("sam") {
        // Crystal startup/chest.lua:3176-3178 pins UID9226 at32455,31968,z14;
        // quest_system2.lua:97-103,378 requires Sam NPC2 before giving3397.
        // https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/scripts/actions/other/others/quest_system2.lua#L97
        // sha256:04990d261d85479bf9a75de040f6c158bc6eb557018a7ad359374400c2a49635.
        let mut found = None;
        for shard in crate::content::world_reward_claims::REWARD_CLAIM_SHARDS {
            let document: serde_json::Value = serde_json::from_str(shard)?;
            for record in document["records"].as_array().ok_or("claim records")? {
                let definition = &record["definition"];
                if definition["identity"]["key"]
                    == "oteryn:reward-claim.quest.u7_5.sams_old_backpack.armor"
                {
                    found = Some(definition.clone());
                }
            }
        }
        let definition = found.ok_or("Sam source armor claim is absent")?;
        let placement = &definition["placements"][0];
        assert_eq!(
            placement["quest_requirements"],
            serde_json::json!([{
                "track":"oteryn:quest-progress/quest/u7_5/sams_old_backpack/sams_old_backpack_npc",
                "minimum":2,"maximum":2
            }])
        );
        assert_eq!(
            placement["reward"]["items"][0]["item"]["key"],
            "oteryn:item.tibia.i3397"
        );
    } else {
        let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
        let key = if name.ends_with("outfox") {
            "oteryn:quest-reward/outfox-start"
        } else {
            "oteryn:quest-reward/bone-pit-issue"
        };
        let callback = callbacks
            .iter()
            .find(|c| c.key == key)
            .ok_or_else(|| format!("missing source mission issue binding {key}"))?;
        assert_eq!(callback.quest, "oteryn:quest.tibia_tales");
        if name.ends_with("outfox") {
            assert_eq!(callback.storage.len(), 2);
            assert!(callback.storage.iter().all(|s| s.set == 1));
        } else {
            let reward = callback.give.as_ref().ok_or("issued sample container")?;
            assert_eq!(reward.item, "oteryn:item.tibia.i4852");
            assert_eq!(reward.count, 1);
            assert_eq!(callback.minimum_level, Some(35));
        }
    }
    Ok(())
}
