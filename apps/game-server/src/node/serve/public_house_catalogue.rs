//! The checked map determines active physical houses and their public metadata.
use super::*;
use crate::durability::public_projections::house_catalogue::{
    PublicHouseCatalogue, PublicHouseMetadata,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const SHARDS: [&str; 2] = [
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../content/houses/houses-00000-00499.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../content/houses/houses-00500-00994.json"
    )),
];
#[derive(Deserialize)]
struct Shard {
    houses: Vec<House>,
}
#[derive(Deserialize)]
struct Identity {
    key: String,
}
#[derive(Deserialize)]
struct Ref {
    key: String,
}
#[derive(Deserialize)]
struct Provenance {
    engine_house: EngineHouse,
}
#[derive(Deserialize)]
struct EngineHouse {
    engine: String,
    house_id: u32,
    revision: String,
}
#[derive(Deserialize)]
struct House {
    identity: Identity,
    name: String,
    kind: String,
    town: Ref,
    size_sqm: u32,
    beds: u16,
    rent_gold: u64,
    provenance: Provenance,
    tiles: Vec<[u16; 3]>,
    #[serde(default)]
    retired: bool,
}
fn metadata(map: &crate::map::boot::BundleWorld) -> Result<Vec<PublicHouseMetadata>, ()> {
    // House catalogue owner contract §3: names/town/size/beds/rent are committed
    // client facts; engine house ids are a source join only, never public identity.
    let mut expected = map.facts().house_ids().collect::<BTreeSet<_>>();
    let mut keys = BTreeSet::new();
    let mut source_ids = BTreeSet::new();
    let mut records = Vec::new();
    let mut digest = Sha256::new();
    digest.update(b"oteryn:public-house-catalogue:v2\0");
    digest.update(map.map_revision().as_bytes());
    for shard in SHARDS {
        digest.update(shard.as_bytes());
    }
    let digest: String = digest
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    for raw in SHARDS {
        let shard: Shard = serde_json::from_str(raw).map_err(|_| ())?;
        for h in shard.houses {
            let source = &h.provenance.engine_house;
            if source.engine != "crystalserver"
                || source.revision != "00ce02a57ca5a12e48f32a3476e37471167e4c3f"
                || source.house_id == 0
                || !source_ids.insert(source.house_id)
                || !keys.insert(h.identity.key.clone())
            {
                return Err(());
            }
            if !expected.remove(&source.house_id) {
                continue;
            }
            // A donor numeric id alone cannot select metadata for another physical address.
            if !h.tiles.iter().any(|[x, y, z]| {
                i8::try_from(*z).ok().filter(|z| *z <= 15).is_some_and(|z| {
                    map.facts().house_id(crate::map::overlay::TilePos {
                        x: *x,
                        y: *y,
                        floor: -z,
                    }) == Some(source.house_id)
                })
            }) {
                return Err(());
            }
            if h.retired {
                continue;
            }
            if !crate::native_admission_source::public_projections::projection_name_valid(&h.name)
                || h.size_sqm == 0
                || h.rent_gold == 0
                || h.rent_gold > i64::MAX as u64
            {
                return Err(());
            }
            records.push(PublicHouseMetadata {
                house_key: h.identity.key,
                name: h.name,
                kind: h.kind,
                town: h.town.key,
                size_sqm: h.size_sqm,
                beds: h.beds,
                rent_gold: h.rent_gold,
                content_digest: digest.clone(),
            });
        }
    }
    if !expected.is_empty() {
        return Err(());
    }
    records.sort_unstable_by(|a, b| a.house_key.cmp(&b.house_key));
    Ok(records)
}

/// Source failure holds the house watermark; publication never gates gameplay.
pub(super) async fn capture(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    material: &Material,
    assignment: &RuntimeScopeAssignment,
    reset_epoch: u64,
    bundle: Option<&crate::map::boot::BundleWorld>,
) {
    if bundle.is_none() && !public_enabled(material) {
        return;
    }
    let mut ticks = tokio::time::interval(Duration::from_secs(5));
    ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        ticks.tick().await;
        if capture_once(
            root,
            authority,
            node,
            material,
            assignment,
            reset_epoch,
            bundle.map_or("", |world| world.map_revision()),
            || {
                bundle
                    .map(metadata)
                    .transpose()
                    .map(Option::unwrap_or_default)
            },
        )
        .await
        {
            return;
        }
    }
}

fn public_enabled(material: &Material) -> bool {
    material
        .config
        .platform
        .public_projections
        .as_ref()
        .is_some_and(|p| p.enabled)
}

async fn capture_once(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    material: &Material,
    assignment: &RuntimeScopeAssignment,
    reset_epoch: u64,
    map_revision: &str,
    metadata: impl FnOnce() -> Result<Vec<PublicHouseMetadata>, ()>,
) -> bool {
    let update = |houses| PublicHouseCatalogue {
        world: material.world,
        channel: material.channel,
        generation: assignment.ownership_generation,
        reset_epoch,
        map_revision: map_revision.to_owned(),
        houses,
    };
    let publish = public_enabled(material);
    if publish
        && (root.await_ready().await.is_err()
            || root
                .sync_public_house_catalogue(authority, node, update(None))
                .await
                .is_err())
    {
        event("event=public_projections family=house source=catalogue result=unavailable");
        return false;
    }
    let records = match metadata() {
        Ok(rows) => rows,
        Err(()) => {
            event("event=public_projections family=house source=catalogue result=unavailable");
            return false;
        }
    };
    if !records.is_empty()
        && (root.await_ready().await.is_err()
            || root
                .admit_world_house_properties(authority, node, update(Some(records.clone())))
                .await
                .is_err())
    {
        event("event=public_projections family=house source=catalogue result=unavailable");
        return false;
    }
    if !publish {
        return true;
    }
    if root.await_ready().await.is_err()
        || root
            .sync_public_house_catalogue(authority, node, update(Some(records)))
            .await
            .is_err()
    {
        event("event=public_projections family=house source=catalogue result=unavailable");
        return false;
    }
    true
}
#[cfg(test)]
mod tests {
    use super::*;

    fn catalogue_material() -> crate::bestiary_postgres_harness::TestResult<Material> {
        use crate::bestiary_postgres_harness::{CHANNEL, WORLD, debug, id};
        let document = format!(
            r#"{}
[platform.public_projections]
enabled = true
client_certificate_file = "/missing/public.crt"
client_key_file = "/missing/public.key"
source_authority = "public"
epoch_fence_file = "/missing/public.fence"
"#,
            super::super::super::config::tests::NODE
        );
        let config = NodeConfig::parse(document.as_bytes())?;
        let pair = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
        let chain = vec![pair.cert.der().clone()];
        let key = PrivateKeyDer::Pkcs8(rustls::pki_types::PrivatePkcs8KeyDer::from(
            pair.signing_key.serialize_der(),
        ));
        let producer = ProducerDescriptor::new(
            config.platform.source_authority.clone(),
            ("127.0.0.1".into(), 8443),
            "localhost".into(),
            "localhost".into(),
            chain.clone(),
            chain.clone(),
            key.clone_key(),
        )
        .map_err(debug)?;
        Ok(Material {
            config,
            gameplay_chain: chain,
            gameplay_key: key,
            evidence: FreshEvidenceSource::new(producer.clone()),
            intents: producer.clone(),
            store: producer,
            runtime_status: None,
            account_characters: std::sync::Mutex::new(None),
            character_commands: std::sync::Mutex::new(None),
            bazaar: std::sync::Mutex::new(None),
            public_projections: std::sync::Mutex::new(None),
            world_event_outcomes: std::sync::Mutex::new(None),
            descriptor: DescriptorRegistration {
                revision: 1,
                facts: Vec::new(),
                installed_at: 1,
            },
            launch: LaunchAuthorizationFile {
                version: 1,
                secret: "01".repeat(32),
                launch_binding: "fixture".into(),
                supersedes: None,
            },
            s2: None,
            world: WorldId::decode(&id(WORLD)).map_err(debug)?,
            channel: ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
        })
    }

    async fn seed_epoch(
        h: &crate::bestiary_postgres_harness::Harness,
        world: WorldId,
    ) -> crate::bestiary_postgres_harness::TestResult {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(123),'1970-01')")
            .bind(world.as_bytes().as_slice()).bind([1u8;32].as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
            .bind(world.as_bytes().as_slice())
            .execute(&h.pool)
            .await?;
        Ok(())
    }

    #[test]
    fn activated_catalogue_admits_private_properties_with_world_digest()
    -> crate::bestiary_postgres_harness::TestResult {
        use crate::bestiary_postgres_harness::{Harness, TestResult, debug, runtime, scope};

        let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
            return Ok(());
        };
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || -> Result<(), String> {
                runtime().map_err(debug)?.block_on(async {
                    let h = Harness::create(admin, "catalogue_house_admission", false).await?;
                    let result: TestResult = async {
                        let mut material = catalogue_material()?;
                        material.config.platform.public_projections = None;
                        seed_epoch(&h, material.world).await?;
                        let seal = h.recovery.seal_current().map_err(debug)?;
                        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                        let assignment = h.root.read_runtime_scope_assignment(scope()?).await.map_err(debug)?.ok_or("assignment")?;
                        let house = PublicHouseMetadata {
                            house_key: "oteryn:content.house.admiral_s_avenue_1".into(),
                            name: "Admiral's Avenue 1".into(),
                            kind: "private_house".into(),
                            town: "oteryn:content.area.city.thais".into(),
                            size_sqm: 35,
                            beds: 1,
                            rent_gold: 200,
                            content_digest: "02".repeat(32),
                        };
                        let guildhall = PublicHouseMetadata {
                            house_key: "oteryn:content.house.guildhall_fixture".into(),
                            kind: "guildhall".into(),
                            ..house.clone()
                        };
                        let records = || Ok(vec![house.clone(), guildhall.clone()]);
                        for _ in 0..2 {
                            assert!(capture_once(&h.root, &authority, &h.node, &material, &assignment, 0, "map-1", records).await);
                        }
                        let properties: Vec<(String, i16, i64, Vec<u8>)> = sqlx::query_as(
                            "SELECT house_key,state,rent_gold,content_digest FROM game_house_properties WHERE world_id=encode($1,'hex')::uuid"
                        ).bind(material.world.as_bytes().as_slice()).fetch_all(&h.pool).await?;
                        assert_eq!(properties, [(house.house_key.clone(), 1, 200, vec![1;32])], "activated personal property uses the World digest");
                        assert!(!capture_once(&h.root, &authority, &h.node, &material, &assignment, 1, "map-1", records).await);
                        for (table, count) in [("game_house_ownership", 1), ("game_house_auctions", 0), ("game_public_house_catalogue", 0)] {
                            assert_eq!(h.count(table).await?, count, "{table}");
                        }
                        Ok(())
                    }.await;
                    h.cleanup().await?;
                    result
                }).map_err(debug)
            })?
            .join().map_err(|_| "house admission thread failed")?.map_err(Into::into)
    }

    fn catalogue_recovery_case(fail_capture: bool) -> crate::bestiary_postgres_harness::TestResult {
        use crate::bestiary_postgres_harness::{
            Harness, TestResult, configured_admin, debug, runtime, scope,
        };
        use sqlx::Executor;
        let Some(admin) = configured_admin() else {
            return Ok(());
        };
        std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
            runtime().map_err(debug)?.block_on(async {
                let h = Harness::create(admin, if fail_capture { "catalogue_capture" } else { "catalogue_pending" }, false).await?;
                let result: TestResult = async {
                    let material = catalogue_material()?;
                    seed_epoch(&h, material.world).await?;
                    if fail_capture {
                        h.pool.execute("CREATE SEQUENCE catalogue_failure; CREATE FUNCTION catalogue_fail_once() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.ready AND nextval('catalogue_failure')=1 THEN RAISE EXCEPTION 'transient capture failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER catalogue_fail_once BEFORE UPDATE ON game_public_house_catalogue_state FOR EACH ROW EXECUTE FUNCTION catalogue_fail_once() ").await?;
                    }
                    if !fail_capture {
                        h.pool.execute("CREATE SEQUENCE catalogue_failure; CREATE FUNCTION catalogue_fail_initial_once() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF nextval('catalogue_failure')=1 THEN RAISE EXCEPTION 'transient initial publication failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER catalogue_fail_initial_once BEFORE INSERT ON game_public_house_catalogue_state FOR EACH ROW EXECUTE FUNCTION catalogue_fail_initial_once()").await?;
                    }
                    let seal = h.recovery.seal_current().map_err(debug)?;
                    let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                    let assignment = h.root.read_runtime_scope_assignment(scope()?).await.map_err(debug)?.ok_or("assignment")?;
                    let held = if fail_capture { None } else { Some(h.root.try_issue_semantic_pass()?) };
                    let mut worker = Box::pin(capture(&h.root, &authority, &h.node, &material, &assignment, 0, None));
                    // Start under contention. The trigger counters below independently
                    // prove the worker reached a failing SQL attempt and retried it.
                    let completed = poll_fn(|cx| Poll::Ready(worker.as_mut().poll(cx).is_ready())).await;
                    drop(held);
                    if !completed {
                        tokio::time::timeout(Duration::from_secs(20), worker).await?;
                    }
                    let ready: Option<bool> = sqlx::query_scalar("SELECT ready FROM game_public_house_catalogue_state WHERE world_id=encode($1,'hex')::uuid")
                        .bind(material.world.as_bytes().as_slice()).fetch_optional(&h.pool).await?;
                    assert_eq!(ready, Some(true), "both catalogue stages must recover without restart");
                    if fail_capture {
                        let attempts: i64 = sqlx::query_scalar("SELECT last_value FROM catalogue_failure").fetch_one(&h.pool).await?;
                        assert_eq!(attempts, 2);
                    } else {
                        let attempts: i64 = sqlx::query_scalar("SELECT last_value FROM catalogue_failure").fetch_one(&h.pool).await?;
                        assert_eq!(attempts, 3, "failed pending, retried pending, then complete publication");
                    }
                    Ok(())
                }.await;
                h.cleanup().await?;
                result
            }).map_err(debug)
        })?.join().map_err(|_| "catalogue thread failed")?.map_err(Into::into)
    }

    #[test]
    fn catalogue_retries_initial_publication_after_root_contention()
    -> crate::bestiary_postgres_harness::TestResult {
        catalogue_recovery_case(false)
    }

    #[test]
    fn catalogue_retries_capture_after_pending_publication()
    -> crate::bestiary_postgres_harness::TestResult {
        catalogue_recovery_case(true)
    }

    #[test]
    fn public_house_catalogue_all_995_records_parse_with_source_identity_and_tiles()
    -> Result<(), serde_json::Error> {
        let mut keys = BTreeSet::new();
        let mut ids = BTreeSet::new();
        let mut count = 0;
        for raw in SHARDS {
            // This is the production parser/type, not a permissive JSON-only reader.
            let shard: Shard = serde_json::from_str(raw)?;
            for house in shard.houses {
                count += 1;
                assert_eq!(house.provenance.engine_house.engine, "crystalserver");
                assert_eq!(
                    house.provenance.engine_house.revision,
                    "00ce02a57ca5a12e48f32a3476e37471167e4c3f"
                );
                assert!(house.provenance.engine_house.house_id > 0);
                assert!(ids.insert(house.provenance.engine_house.house_id));
                assert!(keys.insert(house.identity.key.clone()));
                assert!(house.identity.key.starts_with("oteryn:content.house."));
                assert!(house.identity.key.len() <= 128);
                assert!(house.town.key.starts_with("oteryn:content.area.city."));
                assert!(matches!(
                    house.kind.as_str(),
                    "private_house" | "shop" | "guildhall"
                ));
                assert!(
                    crate::native_admission_source::public_projections::projection_name_valid(
                        &house.name
                    )
                );
                assert!(
                    house.size_sqm > 0 && house.rent_gold > 0 && house.rent_gold <= i64::MAX as u64
                );
                assert!(!house.tiles.is_empty());
                assert!(house.tiles.iter().all(|[_, _, z]| *z <= 15));
                let tiles = house.tiles.iter().copied().collect::<BTreeSet<_>>();
                assert_eq!(tiles.len(), house.tiles.len());
            }
        }
        assert_eq!(count, 995);
        assert_eq!(keys.len(), 995);
        assert_eq!(ids.len(), 995);
        Ok(())
    }
}
