//! Newhaven journey rows: Viola's Lost Wisdom mission, the NE mines chest and the two optional
//! bosses. Every row prints `SEAM_EVIDENCE NEWHAVEN_STEP <step> PASS|FAIL <detail>` and takes its
//! verdict only from what the wire and PostgreSQL showed during this run (NPC replies, inventory
//! and corpse deltas, the quest journal, `game_character_progression_state`, the bound
//! reward-claim placements and the bundle spawn table). Owner facts: the Newhaven chest holds
//! only the Lost Wisdom book and no Legion Helmet exists on Newhaven.
use super::*;
use oteryn_dev_client::{DevClientSession, client_runtime::npc_shop as runtime};
use oteryn_protocol_oteryn::npc_service as wire;
use std::collections::BTreeMap;

const SCARF: u32 = 3572;
const BOOK_OF_WISDOM: u32 = 51977;
/// Base value when the content carries no reward record (Crystal and Tibia Wiki revid 1205117;
/// 1600 was the double-XP event value).
const LOST_WISDOM_BASE_XP: u64 = 800;
const CORRUPTOR: &str = "oteryn:creature.the_corruptor";
const CHIEF: &str = "oteryn:creature.muglex_clan_chief";
const VIOLA: &str = "oteryn:npc.viola";
const GUSTAVO: &str = "oteryn:npc.gustavo_the_guard";
/// Vocation sets of crystalserver data-global/scripts/quests/newhaven/#ondroploot_the_corruptor.lua:
/// knight, paladin, druid, sorcerer and monk.
const VOCATION_SET: [u32; 10] = [
    3359, 3285, 3375, 10321, 48843, 3066, 8871, 3074, 48847, 48845,
];
const BOSS_RESPAWN_WAIT_S: u64 = 65;

type Observation = TestResult<(bool, String)>;
type Cells = Vec<(i8, u16, u16)>;

fn on_newhaven(x: u16, y: u16) -> bool {
    (32540..=32700).contains(&x) && (32380..=32560).contains(&y)
}

fn token(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(480)
        .collect()
}

fn row(step: &str, result: Observation) {
    let (ok, detail) = result.unwrap_or_else(|error| (false, format!("error={error}")));
    evidence(&format!(
        "SEAM_EVIDENCE NEWHAVEN_STEP {step} {} {}",
        if ok { "PASS" } else { "FAIL" },
        token(&detail)
    ));
}

fn vocation_hits(loot: &BTreeMap<u32, u32>) -> Vec<u32> {
    VOCATION_SET
        .iter()
        .copied()
        .filter(|id| loot.contains_key(id))
        .collect()
}

/// Item definition reference -> Tibia id for the ids this journey names.
fn known_refs() -> TestResult<BTreeMap<u32, u32>> {
    let index = provisioning::item_index()?;
    Ok(VOCATION_SET
        .iter()
        .copied()
        .chain([SCARF, BOOK_OF_WISDOM])
        .filter_map(|id| {
            provisioning::item_reference(&index, id)
                .ok()
                .map(|reference| (reference.get(), id))
        })
        .collect())
}

fn ref_count(client: &DevClientSession, reference: u32) -> u64 {
    client.inventory().map_or(0, |inventory| {
        inventory
            .entries
            .iter()
            .filter(|entry| entry.item_definition_ref.get() == reference)
            .map(|entry| u64::from(entry.count.get()))
            .sum()
    })
}

fn inventory_counts(client: &DevClientSession) -> BTreeMap<u32, u64> {
    let mut counts = BTreeMap::new();
    if let Some(inventory) = client.inventory() {
        for entry in inventory.entries.iter() {
            *counts.entry(entry.item_definition_ref.get()).or_insert(0) +=
                u64::from(entry.count.get());
        }
    }
    counts
}

/// Tibia ids (where known) and counts of a loot map keyed by item definition reference.
fn describe(loot: &BTreeMap<u32, u64>, names: &BTreeMap<u32, u32>) -> String {
    loot.iter()
        .map(|(reference, count)| match names.get(reference) {
            Some(id) => format!("i{id}x{count}"),
            None => format!("ref{reference}x{count}"),
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// Count time as progress of the running row, so the watchdog sees a real wait.
async fn settle(client: &mut DevClientSession, seconds: u64) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(seconds);
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
        verdict::Harness::global().progress("newhaven_journey");
    }
    Ok(())
}

async fn await_respawn(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    while !(newhaven_at_temple(client)
        && client
            .actor_vitals()
            .is_some_and(|vitals| vitals.health > 0 && vitals.health == vitals.max_health))
    {
        if tokio::time::Instant::now() >= deadline {
            return Err("Newhaven journey death did not respawn alive on the islet".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
        verdict::Harness::global().progress("newhaven_journey");
    }
    Ok(())
}

/// The Viola hand-in XP from the content reward record of `oteryn:quest.newhaven_quest`, else the
/// base value with its source.
fn expected_handin_xp() -> (u64, String) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let read = || -> TestResult<Option<u64>> {
        let document: serde_json::Value = serde_json::from_slice(&std::fs::read(
            root.join("content/quests/definitions/quests-00200-00299.json"),
        )?)?;
        Ok(document["records"].as_array().and_then(|records| {
            records
                .iter()
                .find(|record| {
                    record["definition"]["identity"]["key"] == "oteryn:quest.newhaven_quest"
                })
                .and_then(|record| {
                    record["definition"]["oteryn_recipe"]["payload"]["recipe"]["reward_intents"]
                        .as_array()?
                        .iter()
                        .find(|intent| intent["kind"] == "experience")?["count"]
                        .as_u64()
                })
        }))
    };
    match read() {
        Ok(Some(xp)) => (xp, "content reward record newhaven_quest".into()),
        _ => (
            LOST_WISDOM_BASE_XP,
            "no content reward record; base value per Tibia Wiki revid 1205117".into(),
        ),
    }
}

async fn experience(clients: &SeamClients<'_>) -> TestResult<u64> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let text: String = sqlx::query_scalar::<_, String>(
        "SELECT total_experience::text FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(clients.characters[1].as_slice())
    .fetch_one(&mut connection)
    .await?;
    Ok(text.split('.').next().unwrap_or("").parse()?)
}

fn npc_cells(bytes: &[u8], key: &str) -> TestResult<Cells> {
    let metadata = oteryn_world_bundle::bundle::visit(bytes, |_, _| Ok(()))?;
    let npc = metadata
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.key == key)
        .ok_or_else(|| format!("{key} has no placement in the bundle"))?;
    Ok(npc.placements.iter().map(|p| (p.floor, p.x, p.y)).collect())
}

/// Walk next to the NPC and say each text over the NPC conversation wire; the replies come back
/// as `text=>disposition lines`.
async fn talk_to(
    client: &mut DevClientSession,
    world: &BundleWorld,
    cells: &Cells,
    texts: &[&str],
) -> TestResult<Vec<String>> {
    let here = tile(client.world_spatial().actor_position)?;
    let path = route(world, here, |at| {
        cells
            .iter()
            .any(|&(f, x, y)| f == at.floor && at.x.abs_diff(x).max(at.y.abs_diff(y)) <= 3)
    })
    .ok_or("no walk route to the NPC")?;
    walk(client, world, &path).await?;
    let actor = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == oteryn_dev_client::EntityKind::Npc
                    && cells.iter().any(|&(f, x, y)| {
                        entity.position.x == i32::from(x)
                            && entity.position.y == i32::from(y)
                            && entity.position.floor == -i16::from(f)
                    })
            })
        })
        .map(|entry| entry.entity)
        .ok_or("NPC not visible after walking to its placement")?;
    let mut replies = Vec::new();
    for text in texts {
        let disposition = runtime::talk(
            client.session_mut(),
            &wire::NpcTalkIntent {
                npc_actor: actor,
                text: (*text).into(),
            },
        )
        .await?;
        let lines = if disposition == wire::NpcIntentDisposition::Ok {
            match tokio::time::timeout(
                Duration::from_secs(3),
                client
                    .session_mut()
                    .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1),
            )
            .await
            {
                Ok(Ok(delta)) => match wire::decode_npc_conversation(&delta.payload) {
                    Ok(conversation) => conversation.lines.join(" / "),
                    Err(error) => format!("undecodable conversation delta: {error:?}"),
                },
                _ => "no conversation delta".to_owned(),
            }
        } else {
            String::new()
        };
        replies.push(format!("{text}=>{disposition:?} [{lines}]"));
        verdict::Harness::global().progress("newhaven_journey");
    }
    Ok(replies)
}

async fn gustavo_goblin_quest(
    client: &mut DevClientSession,
    world: &BundleWorld,
    cells: &Cells,
) -> Observation {
    let replies = talk_to(client, world, cells, &["hi", "goblins", "mission", "yes"]).await?;
    let text = replies.join(" | ").to_lowercase();
    let offered = text.contains("kill") || text.contains(" 20 ");
    Ok((
        offered,
        format!(
            "goblin quest offered={offered} replies={}",
            replies.join(" | ")
        ),
    ))
}

async fn viola_mission(
    client: &mut DevClientSession,
    world: &BundleWorld,
    cells: &Cells,
) -> Observation {
    let replies = talk_to(client, world, cells, &["hi", "mission", "yes"]).await?;
    let text = replies.join(" | ").to_lowercase();
    let offered = text.contains("book") || text.contains("wisdom");
    Ok((
        offered,
        format!(
            "mission offers the book={offered} replies={}",
            replies.join(" | ")
        ),
    ))
}

struct Chest {
    at: TilePos,
    placement: Vec<u8>,
    claim: String,
    contents: String,
}

/// The bound reward-claim chests of the embedded content over this bundle, and the labelled
/// Legion claims with their cells (a claim outside the Newhaven box is no Newhaven chest).
fn bound_chests(world: &BundleWorld) -> TestResult<(Vec<Chest>, usize, String)> {
    let report = crate::content::world_reward_claims::bind_embedded_reward_claims(
        world.base().as_ref(),
        &|_| true,
    )?;
    let mut chests = Vec::new();
    let mut legion = Vec::new();
    for claim in &report.served {
        for placement in &claim.placements {
            let cell = &placement.cell;
            if claim.key.as_str().contains("legion") {
                legion.push(format!(
                    "{}@({},{},{})",
                    claim.key.as_str(),
                    cell.x,
                    cell.y,
                    cell.floor
                ));
            }
            if !on_newhaven(cell.x, cell.y) {
                continue;
            }
            let mut key = world.base().digest().to_vec();
            key.extend_from_slice(&placement.bundle_placement_key.to_be_bytes());
            chests.push(Chest {
                at: TilePos {
                    x: cell.x,
                    y: cell.y,
                    floor: cell.floor,
                },
                placement: key,
                claim: claim.key.as_str().to_owned(),
                contents: placement
                    .contents
                    .iter()
                    .map(|item| format!("{}x{}", item.item.key().as_str(), item.count))
                    .chain(std::iter::once(format!("{:?}", placement.placement)))
                    .collect::<Vec<_>>()
                    .join(","),
            });
        }
    }
    chests.sort_by_key(|chest| (chest.at, chest.claim.clone()));
    Ok((chests, report.served.len(), legion.join(";")))
}

async fn lost_wisdom_chest(
    client: &mut DevClientSession,
    world: &BundleWorld,
    book: Option<u32>,
) -> Observation {
    let (chests, served, legion) = bound_chests(world)?;
    let Some(chest) = chests.first() else {
        return Ok((
            false,
            format!(
                "no reward-claim chest is bound inside Newhaven (x 32540..32700, y 32380..32560) among {served} served claims; Legion claims: {legion}"
            ),
        ));
    };
    let book = book.ok_or("Book of Wisdom has no item definition reference")?;
    let here = tile(client.world_spatial().actor_position)?;
    let path = route(world, here, |at| {
        at.floor == chest.at.floor && at.x.abs_diff(chest.at.x).max(at.y.abs_diff(chest.at.y)) <= 1
    })
    .ok_or("no walk route to the Newhaven chest")?;
    walk(client, world, &path).await?;
    let before = inventory_counts(client);
    let first = client
        .session_mut()
        .use_object(&chest.placement, 0)
        .await?
        .disposition;
    settle(client, 2).await?;
    let after = inventory_counts(client);
    let gained: BTreeMap<u32, u64> = after
        .iter()
        .filter_map(|(reference, count)| {
            let delta = count.saturating_sub(before.get(reference).copied().unwrap_or(0));
            (delta > 0).then_some((*reference, delta))
        })
        .collect();
    let second = client
        .session_mut()
        .use_object(&chest.placement, 0)
        .await?
        .disposition;
    let only_book = gained.len() == 1 && gained.get(&book) == Some(&1);
    let committed = oteryn_protocol_oteryn::world_object::UseDisposition::Committed;
    let ok = first == committed && only_book && second != committed;
    Ok((
        ok,
        format!(
            "chest claim={} at={:?} first_use={first:?} gained_refs={gained:?} (book ref {book}) second_use={second:?} claim_contents={}",
            chest.claim, chest.at, chest.contents
        ),
    ))
}

async fn viola_handin(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    cells: &Cells,
    scarf: Option<u32>,
) -> Observation {
    let scarf = scarf.ok_or("Scarf has no item definition reference")?;
    let xp_before = experience(clients).await?;
    let scarf_before = ref_count(client, scarf);
    let journal_before = query_quest_journal(client, false).await?;
    let replies = talk_to(client, world, cells, &["hi", "mission", "yes"]).await?;
    settle(client, 2).await?;
    let xp_after = experience(clients).await?;
    let scarf_after = ref_count(client, scarf);
    let journal_after = query_quest_journal(client, false).await?;
    let newly_completed: Vec<u32> = journal_after
        .iter()
        .filter(|entry| {
            entry.completed
                && !journal_before
                    .iter()
                    .any(|old| old.quest_index == entry.quest_index && old.completed)
        })
        .map(|entry| entry.quest_index)
        .collect();
    let gained_xp = xp_after.saturating_sub(xp_before);
    let (expected_xp, xp_source) = expected_handin_xp();
    let ok =
        gained_xp == expected_xp && scarf_after == scarf_before + 1 && !newly_completed.is_empty();
    Ok((
        ok,
        format!(
            "xp {xp_before}->{xp_after} (+{gained_xp}, expected +{expected_xp} from {xp_source}) scarf {scarf_before}->{scarf_after} (expected +1) quests_newly_completed={newly_completed:?} journal_entries={}->{} replies={}",
            journal_before.len(),
            journal_after.len(),
            replies.join(" | ")
        ),
    ))
}

/// Walk to the creature's spawn, kill it by playing and return what its corpse held: the
/// inventory gain of the first moved item plus the entries still in the open corpse.
async fn kill(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    creature: &str,
) -> TestResult<BTreeMap<u32, u64>> {
    if newhaven_at_temple(client) {
        leave_islet(client, world).await?;
    }
    travel_to_spawn(client, world, spawns, creature).await?;
    let before = inventory_counts(client);
    client_hunt::hunt_newhaven(client, world, creature).await?;
    let mut loot = BTreeMap::new();
    for (reference, count) in inventory_counts(client) {
        let delta = count.saturating_sub(before.get(&reference).copied().unwrap_or(0));
        if delta > 0 {
            loot.insert(reference, delta);
        }
    }
    if let Some(view) = client.open_container() {
        for entry in view.entries.iter() {
            *loot.entry(entry.item_definition_ref.get()).or_insert(0) +=
                u64::from(entry.count.get());
        }
    }
    Ok(loot)
}

async fn recover(client: &mut DevClientSession) {
    if newhaven_hunt_died(client) {
        row(
            "recover-after-death",
            await_respawn(client)
                .await
                .map(|()| (true, "respawned alive on the islet".into())),
        );
    }
}

async fn boss(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    names: &BTreeMap<u32, u32>,
    step: &str,
    creature: &str,
    judge: impl Fn(&BTreeMap<u32, u32>) -> bool,
) -> bool {
    let killed = kill(client, world, spawns, creature).await;
    let killed_ok = killed.is_ok();
    row(
        step,
        killed.map(|loot| {
            let ids: BTreeMap<u32, u32> = loot
                .keys()
                .filter_map(|reference| names.get(reference).map(|id| (*id, 1)))
                .collect();
            (
                judge(&ids),
                format!(
                    "{creature} killed by playing; loot={} vocation_set_items={:?}",
                    describe(&loot, names),
                    vocation_hits(&ids)
                ),
            )
        }),
    );
    recover(client).await;
    killed_ok
}

/// Runs after the death relog of the Newhaven leg on the same character.
pub(super) async fn run(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) {
    verdict::Harness::global().progress("newhaven_journey");
    if newhaven_at_temple(client)
        && let Err(error) = leave_islet(client, world).await
    {
        row("leave-islet-for-journey", Err(error));
        return;
    }
    let cells = |key| -> TestResult<Cells> {
        npc_cells(&std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?, key)
    };
    let names = known_refs();
    let by_id = |id: u32| {
        names
            .as_ref()
            .ok()
            .and_then(|map| map.iter().find(|(_, v)| **v == id).map(|(r, _)| *r))
    };
    let (scarf, book) = (by_id(SCARF), by_id(BOOK_OF_WISDOM));
    match cells(GUSTAVO) {
        Ok(cells) => row(
            "gustavo-goblin-quest-offer",
            gustavo_goblin_quest(client, world, &cells).await,
        ),
        Err(error) => row("gustavo-goblin-quest-offer", Err(error)),
    }
    let viola = cells(VIOLA);
    match &viola {
        Ok(cells) => row(
            "viola-mission-offer",
            viola_mission(client, world, cells).await,
        ),
        Err(error) => row("viola-mission-offer", Err(error.to_string().into())),
    }
    row(
        "lost-wisdom-chest",
        lost_wisdom_chest(client, world, book).await,
    );
    match &viola {
        Ok(cells) => row(
            "viola-lost-wisdom-handin",
            viola_handin(client, clients, world, cells, scarf).await,
        ),
        Err(error) => row("viola-lost-wisdom-handin", Err(error.to_string().into())),
    }
    let names = names.unwrap_or_default();
    let first = boss(
        client,
        world,
        spawns,
        &names,
        "the-corruptor-first-kill-vocation-set",
        CORRUPTOR,
        |ids| !vocation_hits(ids).is_empty(),
    )
    .await;
    if first {
        // Respawn is 60 s; a second kill must not drop the vocation set again.
        let waited = settle(client, BOSS_RESPAWN_WAIT_S).await;
        match waited {
            Ok(()) => {
                boss(
                    client,
                    world,
                    spawns,
                    &names,
                    "the-corruptor-second-kill-no-vocation-set",
                    CORRUPTOR,
                    |ids| vocation_hits(ids).is_empty(),
                )
                .await;
            }
            Err(error) => row("the-corruptor-second-kill-no-vocation-set", Err(error)),
        }
    } else {
        row(
            "the-corruptor-second-kill-no-vocation-set",
            Ok((
                false,
                "not run: the first Corruptor kill did not complete".into(),
            )),
        );
    }
    if spawns.creatures.iter().any(|entry| entry.key == CHIEF) {
        boss(
            client,
            world,
            spawns,
            &names,
            "muglex-clan-chief-kill-and-loot",
            CHIEF,
            |_| true,
        )
        .await;
    } else {
        let muglex: Vec<&str> = spawns
            .creatures
            .iter()
            .map(|entry| entry.key.as_str())
            .filter(|key| key.contains("muglex"))
            .collect();
        row(
            "muglex-clan-chief-kill-and-loot",
            Ok((
                false,
                format!(
                    "{CHIEF} has no spawn in the bundle spawn table; muglex spawns present: {muglex:?}"
                ),
            )),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualification_nhj_token_is_one_bounded_line() {
        let long = "x ".repeat(400);
        let one = token(&format!("a\nb\t c  {long}"));
        assert!(!one.contains('\n') && !one.contains('\t') && !one.contains("  "));
        assert!(one.chars().count() <= 480);
        assert!(one.starts_with("a b c x"));
    }

    #[test]
    fn qualification_nhj_vocation_set_hits_are_the_scripted_items_only() {
        let loot: BTreeMap<u32, u32> = [(3264, 1), (3359, 1), (48847, 1)].into();
        assert_eq!(vocation_hits(&loot), vec![3359, 48847]);
        assert!(vocation_hits(&[(3264, 1)].into()).is_empty());
        assert!(on_newhaven(32571, 32507) && !on_newhaven(32143, 31910));
    }
}
