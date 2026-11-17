use super::*;
use oteryn_gameplay_items::imbuement::{imbuement_recipe, scroll_for_imbuement};

async fn supply(h: &Harness, entries: &[(&str, u32)]) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (key, quantity) in entries {
        sqlx::query("WITH i AS (INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,'Item',$2,'definition-r1',$3,1,game_character_uuid_v7()) RETURNING item_instance_id) INSERT INTO game_item_container_entries(character_id,world_id,parent_item_instance_id,item_instance_id,placement_ordinal,placed_transaction_id) SELECT encode($4,'hex')::uuid,encode($1,'hex')::uuid,encode($5,'hex')::uuid,i.item_instance_id,(SELECT COALESCE(max(placement_ordinal),0)+1 FROM game_item_container_entries WHERE parent_item_instance_id=encode($5,'hex')::uuid),game_character_uuid_v7() FROM i")
            .bind(id(42).as_slice()).bind(key).bind(i64::from(*quantity)).bind(id(41).as_slice()).bind(id(70).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn unlock(h: &Harness, family: u8) -> TestResult {
    let (quest, key) = if family == 26 {
        (
            "oteryn:quest.the_dream_courts_quest",
            "oteryn:quest-progress/quest/u12_00/the_dream_courts/main/questline".to_owned(),
        )
    } else {
        let boss = match family {
            21 => "dragonking",
            22 => "thorn_knight",
            23 => "horror",
            24 => "lloyd",
            25 => "lady_tenebris",
            _ => return Err("family".into()),
        };
        (
            "oteryn:quest.forgotten_knowledge_quest",
            format!("oteryn:quest-progress/quest/u11_02/forgotten_knowledge/{boss}_killed"),
        )
    };
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,1,2)")
        .bind(id(41).as_slice()).bind(key).bind(quest).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    index: &crate::content::item_ref::ItemDefinitionIndex,
) -> TestResult {
    let blank = index
        .definition_ref("Item", "oteryn:item.tibia.i51442", "definition-r1")
        .ok_or("blank")?;
    let unproven = sqlx::query("UPDATE game_item_instances SET definition_production_key='oteryn:item.tibia.i51739',last_transaction_id=game_character_uuid_v7() WHERE definition_production_key='oteryn:item.tibia.i51442'").execute(&h.pool).await;
    assert_eq!(
        unproven
            .as_ref()
            .err()
            .and_then(sqlx::Error::as_database_error)
            .and_then(|error| error.code())
            .as_deref(),
        Some("23514")
    );
    let mut command = 1;
    let (next, status, reply) = exchange(
        owner,
        blank,
        admitted,
        command,
        Some(ImbuementAction::Query),
        true,
    )
    .await?;
    admitted = next;
    command += 1;
    assert_eq!(status, CommandStatus::Accepted);
    let offers = reply.ok_or("reply")?.window.ok_or("window")?.offers;
    assert_eq!(offers.len(), 48);
    for family in 21..=26 {
        for tier in [2, 3] {
            let recipe = imbuement_recipe(family, tier).ok_or("recipe")?;
            let scroll = scroll_for_imbuement(family, tier).ok_or("scroll")?;
            let offer = u8::try_from(
                offers
                    .iter()
                    .position(|o| o.family == family && o.tier == tier)
                    .ok_or("offer")?,
            )?;
            let mut entries = recipe.materials.to_vec();
            entries.push((CRYSTAL, u32::try_from(recipe.apply_gold / 10_000)?));
            // The initial blank remains until the first inscription; every later one is new.
            if family != 21 || tier != 2 {
                entries.push(("oteryn:item.tibia.i51442", 1));
            }
            supply(h, &entries).await?;
            let before: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_item_imbuement_records")
                    .fetch_one(&h.pool)
                    .await?;
            if tier == 3 {
                step(
                    owner,
                    blank,
                    &mut admitted,
                    command,
                    ImbuementAction::Apply {
                        slot: 0,
                        imbuement: offer,
                    },
                    ImbuementResult::RequirementNotMet,
                )
                .await?;
                command += 1;
                assert_eq!(
                    sqlx::query_scalar::<_, i64>(
                        "SELECT count(*) FROM game_item_imbuement_records"
                    )
                    .fetch_one(&h.pool)
                    .await?,
                    before
                );
                unlock(h, family).await?;
            }
            step(
                owner,
                blank,
                &mut admitted,
                command,
                ImbuementAction::Apply {
                    slot: 0,
                    imbuement: offer,
                },
                ImbuementResult::Ok,
            )
            .await?;
            command += 1;
            let stored: (i16, i16, i64, String) = sqlx::query_as("SELECT category,tier,fee_gold_units,output_production_key FROM game_item_imbuement_records ORDER BY command_id DESC LIMIT 1").fetch_one(&h.pool).await?;
            let scroll_key = format!("oteryn:item.tibia.i{}", scroll.item_id);
            assert_eq!(
                stored,
                (
                    i16::from(family - 1),
                    i16::from(tier),
                    i64::try_from(recipe.apply_gold)?,
                    scroll_key.clone()
                )
            );
            let source = index
                .definition_ref("Item", &scroll_key, "definition-r1")
                .ok_or("scroll definition")?;
            let target_key = if family == 26 {
                "oteryn:item.tibia.i3079"
            } else {
                SWORD
            };
            let target = index
                .definition_ref("Item", target_key, "definition-r1")
                .ok_or("target")?;
            let (next, status, result) =
                exchange_item_use(owner, target, source, admitted, command).await?;
            admitted = next;
            command += 1;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!(
                result,
                oteryn_protocol_oteryn::world_object::UseDisposition::Committed
            );
            let row: (i16, i16, i64) = sqlx::query_as("SELECT b.category,b.tier,b.remaining_ms FROM game_item_imbuements b JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key=$1").bind(target_key).fetch_one(&h.pool).await?;
            assert_eq!(row, (i16::from(family - 1), i16::from(tier), 72_000_000));
            // An Etcher clears the installed row through the same real USE-with router.
            supply(h, &[(ETCHER, 1)]).await?;
            let etcher = index
                .definition_ref("Item", ETCHER, "definition-r1")
                .ok_or("etcher")?;
            let (next, _, result) =
                exchange_item_use(owner, target, etcher, admitted, command).await?;
            admitted = next;
            command += 1;
            assert_eq!(
                result,
                oteryn_protocol_oteryn::world_object::UseDisposition::Committed
            );
        }
    }
    Ok(())
}
