//! Opt-in content census and real client appearance journeys.
//! APP-3 catalogue: crates/gameplay/dispatch/src/gameplay_transport/appearance_change/catalogue.rs:10
//! Riding: crates/gameplay/dispatch/src/gameplay_transport/appearance_change.rs:97
//! Speed forecast: crates/gameplay/dispatch/src/gameplay_transport/actor_movement.rs:186,221
//! Seed receipts: apps/game-server/migrations/0093_character_appearance.sql:69
use super::*;
use full_row_s06::{apply_appearance, change, own_binding, snapshot, wait_for_selection};
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_gameplay_characters::speed::{StepSpeedTable, player_effective_speed};
use oteryn_protocol_oteryn::logout::LogoutResult;
use oteryn_protocol_oteryn::{actor_motion, appearance};
use serde::Deserialize;

const BATCH: &str = "OTERYN_E2E_COSMETIC_SWEEP_BATCH";
const DEFAULT: (usize, usize) = (0, 3);

pub(super) fn opted_in() -> bool {
    std::env::var("NODE_BOOT_E2E_ONLY").is_ok_and(|list| {
        list.split(',')
            .any(|group| group.trim() == "cosmetic_sweep")
    })
}

fn batch(value: Option<&str>) -> TestResult<(usize, usize)> {
    let Some(value) = value else {
        return Ok(DEFAULT);
    };
    let number = |part: &str| {
        (!part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| part.parse::<usize>().ok())
            .flatten()
    };
    value
        .split_once(':')
        .and_then(|(start, len)| Some((number(start)?, number(len)?)).filter(|&(_, len)| len > 0))
        .ok_or_else(|| format!("invalid {BATCH}: expected start:len with len > 0").into())
}

#[derive(Deserialize)]
struct Authoring {
    document: Content,
}

#[derive(Deserialize)]
struct Content {
    content_revision: String,
    outfits: Vec<Outfit>,
    mounts: Vec<Mount>,
    grant_mappings: Vec<Grant>,
    fallbacks: Fallbacks,
}

#[derive(Deserialize)]
struct Fallbacks {
    female: Fallback,
}

#[derive(Deserialize)]
struct Fallback {
    outfit_key: String,
}

#[derive(Deserialize)]
struct LookId {
    id: u32,
}

#[derive(Deserialize)]
struct Outfit {
    key: String,
    female: LookId,
    addon_count: u8,
    starter_female: bool,
    starter_addons_female: u8,
}

#[derive(Deserialize)]
struct Mount {
    key: String,
    look: LookId,
    speed_bonus: u16,
}

#[derive(Deserialize)]
struct Grant {
    grant_key: String,
    target: Target,
    compatible_provenance: Vec<String>,
    sex: Option<String>,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(tag = "kind")]
enum Target {
    Outfit { outfit_key: String },
    Addon { outfit_key: String, addon: u8 },
    Mount { mount_key: String },
}

#[derive(Deserialize)]
struct Taming {
    item_key: String,
    grant_key: String,
    mount_key: String,
}

impl Taming {
    fn census_key(&self) -> String {
        format!("{}.{}", self.grant_key, self.item_key)
    }
}

enum Entry<'a> {
    Outfit(&'a Outfit, u8),
    Mount(&'a Mount, Option<&'a Taming>),
    Unadmitted,
}

fn content(root: &Path) -> TestResult<Content> {
    Ok(
        serde_json::from_slice::<Authoring>(&std::fs::read(root.join("outfits/authoring.json"))?)?
            .document,
    )
}

fn census<'a>(
    content: &'a Content,
    taming: &'a [Taming],
    root: &Path,
) -> TestResult<BTreeMap<String, Entry<'a>>> {
    let mut entries = BTreeMap::new();
    let mut insert = |key: String, entry| -> TestResult {
        if entries.insert(key.clone(), entry).is_some() {
            return Err(format!("duplicate cosmetic census key {key}").into());
        }
        Ok(())
    };
    for outfit in &content.outfits {
        insert(outfit.key.clone(), Entry::Outfit(outfit, 0))?;
        for addon in 1..=outfit.addon_count {
            insert(
                format!("{}.addon{addon}", outfit.key),
                Entry::Outfit(outfit, addon),
            )?;
        }
    }
    for mount in &content.mounts {
        insert(mount.key.clone(), Entry::Mount(mount, None))?;
    }
    for tame in taming {
        let mount = content
            .mounts
            .iter()
            .find(|mount| mount.key == tame.mount_key);
        insert(
            tame.census_key(),
            mount.map_or(Entry::Unadmitted, |mount| Entry::Mount(mount, Some(tame))),
        )?;
    }
    // The mount authoring index also contains legacy declarations. Preserve any
    // declaration with no admitted appearance metadata as an explicit failing entry.
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("mounts/index.json"))?)?;
    for shard in index["shards"]
        .as_array()
        .ok_or("mount index missing shards")?
    {
        let path = shard
            .as_str()
            .and_then(|value| value.strip_prefix("content/cosmetics/"))
            .ok_or("invalid mount shard path")?;
        let data: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join(path))?)?;
        for record in data["records"]
            .as_array()
            .ok_or("mount shard missing records")?
        {
            let key = record["declaration"]["identity"]["key"]
                .as_str()
                .ok_or("invalid mount key")?;
            entries.entry(key.to_owned()).or_insert(Entry::Unadmitted);
        }
    }
    Ok(entries)
}

fn target(outfit: &Outfit, addon: u8) -> Target {
    if addon == 0 {
        Target::Outfit {
            outfit_key: outfit.key.clone(),
        }
    } else {
        Target::Addon {
            outfit_key: outfit.key.clone(),
            addon,
        }
    }
}

fn addon_mask(addon: u8) -> u8 {
    if addon == 0 { 0 } else { 1 << (addon - 1) }
}

fn grant<'a>(
    content: &'a Content,
    target: &Target,
    only: Option<&str>,
) -> Option<(&'a str, &'a str)> {
    content.grant_mappings.iter().find_map(|grant| {
        (grant.target == *target
            && grant.sex.as_deref().is_none_or(|sex| sex == "female")
            && only.is_none_or(|key| key == grant.grant_key))
        .then(|| {
            Some((
                grant.grant_key.as_str(),
                grant.compatible_provenance.first()?.as_str(),
            ))
        })
        .flatten()
    })
}

// The same receipt/fact pair consumed by character_appearance/earned_unlocks.rs:31.
// Write before admission on the dedicated sweep fixture, in one statement.
async fn seed(clients: &SeamClients<'_>, key: &str, provenance: &str) -> TestResult {
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    sqlx::query(
        "WITH candidate AS (
           SELECT game_character_uuid_v7() AS id, account_id, character_id
           FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid
         ), receipt AS (
           INSERT INTO game_account_appearance_unlock_receipts
             (grant_occurrence_id,account_id,unlock_key,content_revision,granting_character_id,source_kind,source_event_id)
           SELECT id,account_id,$2,$3,character_id,'quest',uuid_send(id) FROM candidate
           WHERE NOT EXISTS (
             SELECT 1 FROM game_account_appearance_unlocks u
             WHERE u.account_id=candidate.account_id AND u.unlock_key=$2
           )
           RETURNING grant_occurrence_id,account_id,unlock_key
         )
         INSERT INTO game_account_appearance_unlocks(account_id,unlock_key,grant_occurrence_id)
         SELECT account_id,unlock_key,grant_occurrence_id FROM receipt",
    ).bind(clients.characters[0].as_slice()).bind(key).bind(provenance)
        .execute(&mut db).await?;
    Ok(())
}

struct Rows<'a> {
    key: &'a str,
    steps: &'a [&'a str],
    next: usize,
    failed: usize,
    last: std::time::Instant,
}

impl Rows<'_> {
    fn emit(&mut self, step: &str, status: &str, detail: &str, cause: &str) {
        let seconds = self.last.elapsed().as_secs_f64();
        self.last = std::time::Instant::now();
        let (mode, detail) = detail.strip_prefix("mode=").map_or(("", detail), |text| {
            let (mode, detail) = text.split_once(' ').unwrap_or((text, ""));
            (mode, detail)
        });
        let detail = detail.replace(|c: char| c.is_whitespace() || matches!(c, '=' | '|'), "_");
        let detail = if mode.is_empty() {
            detail
        } else {
            format!("mode={mode}_{detail}")
        };
        // The cloud result reader strips the .full transport suffix.
        let key = self.key.replace(':', ".");
        evidence(&format!(
            "e2e_all scenario=COS.{key}.{step}.full status={status} seconds={seconds:.3} evidence={detail} {cause}"
        ));
    }

    fn pass(&mut self, detail: &str) {
        self.emit(self.steps[self.next], "PASS", detail, "");
        self.next += 1;
    }

    fn fail(&mut self, error: &dyn std::error::Error) {
        let step = self.steps[self.next];
        let detail = error.to_string();
        let cause = if detail.contains("no admitted appearance definition") {
            "unadmitted-definition"
        } else if detail.contains("no authored grant mapping") {
            "missing-grant-mapping"
        } else {
            step
        };
        self.emit(step, "FAIL", &detail, &format!("cause={cause}"));
        self.next += 1;
        self.failed += 1;
    }

    fn block_rest(&mut self, error: &dyn std::error::Error) {
        let failed = self.steps[self.next];
        self.fail(error);
        while self.next < self.steps.len() {
            self.emit(
                self.steps[self.next],
                "FAIL",
                &format!("blocked-by-{failed}"),
                &format!("cause=blocked-by-{failed}"),
            );
            self.next += 1;
            self.failed += 1;
        }
    }
}

pub(super) async fn run(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult {
    if !opted_in() {
        return Ok(());
    }
    let (start, len) = match std::env::var(BATCH) {
        Ok(value) => batch(Some(&value))?,
        Err(std::env::VarError::NotPresent) => batch(None)?,
        Err(error) => return Err(error.into()),
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/cosmetics");
    let content = content(&root)?;
    let taming: Vec<Taming> =
        serde_json::from_slice(&std::fs::read(root.join("taming-native.json"))?)?;
    let entries = census(&content, &taming, &root)?;
    let total = entries.len();
    evidence(&format!("cosmetic_sweep census total={total}"));
    if start >= total {
        return Err(format!("{BATCH} start {start} is beyond {total} entries").into());
    }
    let selected = len.min(total - start);
    let mut pass = 0;
    let mut fail = 0;
    for (key, entry) in entries.iter().skip(start).take(selected) {
        verdict::Harness::global().progress("cosmetic-sweep-entry");
        let steps: &[&str] = match entry {
            Entry::Outfit(..) => &["unlock", "equip", "observer", "relog"],
            _ => &[
                "unlock", "equip", "observer", "mount", "speed", "unmount", "relog",
            ],
        };
        let mut rows = Rows {
            key,
            steps,
            next: 0,
            failed: 0,
            last: std::time::Instant::now(),
        };
        if let Err(error) = journey(&mut rows, clients, world, &content, entry).await {
            rows.block_rest(error.as_ref());
        }
        pass += rows.next - rows.failed;
        fail += rows.failed;
    }
    evidence(&format!(
        "cosmetic_sweep summary total={total} start={start} selected={selected} rows_pass={pass} rows_fail={fail}"
    ));
    if fail != 0 {
        return Err(format!("cosmetic sweep: {fail} FAIL rows").into());
    }
    Ok(())
}

async fn prepare(
    clients: &SeamClients<'_>,
    content: &Content,
    outfit: &Outfit,
    addon: u8,
    mount: Option<(&Mount, Option<&Taming>)>,
) -> TestResult<String> {
    wait_character_released(clients, 0).await?;
    let mut sources = Vec::new();
    let base = target(outfit, 0);
    if !outfit.starter_female {
        sources.push(grant(content, &base, None).ok_or("outfit has no authored grant mapping")?);
    }
    if addon != 0 && outfit.starter_addons_female & addon_mask(addon) == 0 {
        let target = target(outfit, addon);
        sources.push(grant(content, &target, None).ok_or("addon has no authored grant mapping")?);
    }
    if let Some((mount, tame)) = mount {
        let target = Target::Mount {
            mount_key: mount.key.clone(),
        };
        sources.push(
            grant(content, &target, tame.map(|tame| tame.grant_key.as_str()))
                .ok_or("mount has no authored grant mapping for this source")?,
        );
    }
    let mut keys = Vec::new();
    for (key, provenance) in sources {
        seed(clients, key, provenance).await?;
        keys.push(key);
    }
    Ok(if keys.is_empty() {
        "mode=starter".to_owned()
    } else {
        format!("mode=seeded grants:{}", keys.join(","))
    })
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let (result, _, _, _) = logout_retrying_busy(client).await?;
    if result != Some(LogoutResult::Accepted) {
        return Err(format!("logout was refused: {result:?}").into());
    }
    Ok(())
}

async fn journey(
    rows: &mut Rows<'_>,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    content: &Content,
    entry: &Entry<'_>,
) -> TestResult {
    let (outfit, addon, mount) = match entry {
        Entry::Outfit(outfit, addon) => (*outfit, *addon, None),
        Entry::Mount(mount, tame) => {
            let fallback = content
                .outfits
                .iter()
                .find(|outfit| outfit.key == content.fallbacks.female.outfit_key)
                .ok_or("content fallback outfit missing")?;
            (fallback, 0, Some((*mount, *tame)))
        }
        Entry::Unadmitted => {
            return Err("no admitted appearance definition for this content key".into());
        }
    };
    let mode = prepare(clients, content, outfit, addon, mount).await?;
    let mut client = connect_character(clients, 0xc9, 0).await?;
    let mut observer = None;
    let result = exercise(
        rows,
        &mut client,
        &mut observer,
        clients,
        world,
        content,
        outfit,
        addon,
        mount,
        &mode,
    )
    .await;
    let observer_release = if let Some(mut observer) = observer {
        logout(&mut observer).await
    } else {
        Ok(())
    };
    let release = logout(&mut client).await;
    result?;
    observer_release?;
    release?;
    rows.pass("fresh admission retained selected look, colours and unlock");
    Ok(())
}

fn has_unlock(
    catalogue: &appearance::Catalogue,
    outfit: &Outfit,
    addons: u8,
    mount: Option<&str>,
) -> bool {
    catalogue.outfits.iter().any(|option| {
        option.key == outfit.key
            && option.look_type == outfit.female.id
            && option.permitted_addons & addons == addons
    }) && mount.is_none_or(|mount| catalogue.mounts.iter().any(|key| key == mount))
}

#[allow(
    clippy::too_many_arguments,
    reason = "one journey binds distinct fixture, content, and wire state"
)]
async fn exercise(
    rows: &mut Rows<'_>,
    client: &mut DevClientSession,
    observer: &mut Option<DevClientSession>,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    content: &Content,
    outfit: &Outfit,
    addon: u8,
    mount: Option<(&Mount, Option<&Taming>)>,
    mode: &str,
) -> TestResult {
    let mut state = snapshot(client)?;
    let mount_key = mount.map(|(mount, _)| mount.key.as_str());
    let addons = addon_mask(addon);
    if !has_unlock(&state.catalogue, outfit, addons, mount_key) {
        return Err(format!("{mode} expected outfit look {}, addon mask {addons} and mount {mount_key:?} in wire catalogue", outfit.female.id).into());
    }
    rows.pass(mode);
    *observer = Some(connect_character(clients, 0xca, 1).await?);
    let observer = observer.as_mut().ok_or("observer fixture missing")?;
    let mut observer_state = snapshot(observer)?;
    let previous_colours = state
        .catalogue
        .selected
        .as_ref()
        .map(|s| s.selection.colours);
    let colours = if previous_colours == Some([11, 22, 33, 44]) {
        [45, 34, 23, 12]
    } else {
        [11, 22, 33, 44]
    };
    let selection = appearance::Selection {
        outfit_key: outfit.key.clone(),
        colours,
        addons,
        mount_key: mount_key.map(str::to_owned),
    };
    let expected = appearance::Look::Outfit {
        look_type: outfit.female.id,
        colours,
        addons,
    };
    let request = appearance::Change {
        context: state.catalogue.context,
        expected_root_revision: state.catalogue.root_revision,
        prior: state.catalogue.selected.clone(),
        selection: selection.clone(),
        content_revision: content.content_revision.clone(),
    };
    let (status, result, _) = change(client, &request).await?;
    let (CommandStatus::Accepted, appearance::ChangeResult::Committed(revision)) = (status, result)
    else {
        return Err(format!("appearance change refused: {status:?} {result:?}").into());
    };
    wait_for_selection(client, &mut state, revision, &selection).await?;
    let binding = own_binding(client, &state)?;
    if binding.look != expected {
        return Err(format!(
            "self look mismatch: expected {expected:?}, got {:?}",
            binding.look
        )
        .into());
    }
    let entity = binding.entity;
    let selection_revision = binding.selection_revision;
    rows.pass("own wire binding matches authored look, colour and addon");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in observer.take_events() {
            if let SessionEvent::Appearance(delta) = event {
                apply_appearance(&mut observer_state, delta.value);
            }
        }
        if observer
            .world_entities()
            .is_some_and(|entities| entities.get(&entity).is_some())
            && observer_state.bindings.iter().any(|binding| {
                binding.entity == entity
                    && binding.look == expected
                    && binding.selection_revision == selection_revision
            })
        {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("observer did not receive target entity with the selected look".into());
        }
        observer.service_liveness(Duration::from_millis(25)).await?;
    }
    rows.pass("second client received changed look for same entity");
    if let Some((mount, _)) = mount {
        let (_, level, _) = creature_sweep::read_stats(client).await?;
        ride(client, true).await?;
        let (motion, position) = wait_motion(client, entity, mount.look.id).await?;
        wait_motion(observer, entity, mount.look.id).await?;
        rows.pass("both clients received the authored mounted look");
        match check_speed(world, position, &motion, level, mount.speed_bonus) {
            Ok(()) => rows.pass("wire prewalk forecasts match content speed bonus"),
            Err(error) => rows.fail(error.as_ref()),
        }
        ride(client, false).await?;
        let (motion, position) = wait_motion(client, entity, 0).await?;
        wait_motion(observer, entity, 0).await?;
        check_speed(world, position, &motion, level, 0)?;
        rows.pass("both clients cleared mount look and base forecast returned");
    }
    let previous = client.join_snapshot().game_session_id;
    logout(client).await?;
    *client = connect_character(clients, 0xcb, 0).await?;
    let persisted = snapshot(client)?;
    let binding = own_binding(client, &persisted)?;
    if client.join_snapshot().game_session_id == previous
        || persisted
            .catalogue
            .selected
            .as_ref()
            .map(|selected| &selected.selection)
            != Some(&selection)
        || binding.look != expected
        || binding.selection_revision != selection_revision
        || !has_unlock(&persisted.catalogue, outfit, addons, mount_key)
    {
        return Err("fresh admission did not preserve selection, look, colours and unlock".into());
    }
    Ok(())
}

async fn ride(client: &mut DevClientSession, riding: bool) -> TestResult {
    let reply = client
        .session_mut()
        .gated_command(
            actor_motion::CAPABILITY_ACTOR_MOTION_V1,
            actor_motion::COMMAND_TYPE_MOUNT,
            &actor_motion::encode_mount(riding),
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "mount command riding={riding} was refused: {:?}",
            reply.status
        )
        .into());
    }
    Ok(())
}

async fn wait_motion(
    client: &mut DevClientSession,
    entity: appearance::EntityRef,
    look: u32,
) -> TestResult<(actor_motion::ActorMotion, TilePos)> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(entry) = client
            .world_entities()
            .and_then(|entities| entities.get(&entity))
            && let Some(motion) = entry.motion
            && motion.mount_look_type == look
        {
            return Ok((motion, tile(entry.position)?));
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("wire motion did not show mount look {look}").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
}

fn forecast(level: u32, bonus: u16, ground: u16) -> Option<u32> {
    let speed = player_effective_speed(level, 0, i64::from(bonus));
    let duration = StepSpeedTable::embedded()?.step_duration(speed, ground)?;
    u32::try_from(duration.as_millis()).ok()
}

fn check_speed(
    world: &BundleWorld,
    position: TilePos,
    motion: &actor_motion::ActorMotion,
    level: u32,
    bonus: u16,
) -> TestResult {
    let mut checked = false;
    for (&ms, (dx, dy)) in motion.prewalk_ms[..4]
        .iter()
        .zip([(0, -1), (1, 0), (0, 1), (-1, 0)])
    {
        if ms == 0 {
            continue;
        }
        let x = position
            .x
            .checked_add_signed(dx)
            .ok_or("forecast x outside map")?;
        let y = position
            .y
            .checked_add_signed(dy)
            .ok_or("forecast y outside map")?;
        let ground = world.ground_speed(TilePos {
            x,
            y,
            floor: position.floor,
        });
        let expected = forecast(level, bonus, ground);
        if expected != Some(ms) {
            return Err(format!("forecast mismatch observed {ms}ms, expected {expected:?}ms at ground {ground}, level {level}, bonus {bonus}").into());
        }
        checked = true;
    }
    if !checked {
        return Err("no nonzero cardinal speed forecast on the wire".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_requires_nonempty_decimal_slice() -> TestResult {
        assert_eq!(batch(None)?, (0, 3));
        assert_eq!(batch(Some("0:20"))?, (0, 20));
        for invalid in [
            "",
            "0",
            ":1",
            "1:",
            "1:0",
            "+1:2",
            " 1:2",
            "2:1:1",
            "999999999999999999999999999999:1",
        ] {
            assert!(batch(Some(invalid)).is_err(), "{invalid}");
        }
        Ok(())
    }

    #[test]
    fn authored_census_includes_each_addon_mount_and_taming_key() -> TestResult {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/cosmetics");
        let content = content(&root)?;
        let native: Vec<Taming> =
            serde_json::from_slice(&std::fs::read(root.join("taming-native.json"))?)?;
        let source: Vec<Taming> =
            serde_json::from_slice(&std::fs::read(root.join("taming.json"))?)?;
        let entries = census(&content, &native, &root)?;
        for tame in source {
            assert!(
                native
                    .iter()
                    .any(|native| native.grant_key == tame.grant_key
                        && native.mount_key == tame.mount_key)
            );
            assert!(entries.contains_key(&tame.census_key()));
        }
        for outfit in &content.outfits {
            assert!(entries.contains_key(&outfit.key));
            for addon in 1..=outfit.addon_count {
                assert_eq!(addon_mask(addon).count_ones(), 1);
                assert!(entries.contains_key(&format!("{}.addon{addon}", outfit.key)));
            }
        }
        for mount in &content.mounts {
            assert!(entries.contains_key(&mount.key));
        }
        Ok(())
    }

    #[test]
    fn unlock_oracle_requires_authored_look_addon_and_mount() {
        let outfit = Outfit {
            key: "outfit".into(),
            female: LookId { id: 431 },
            addon_count: 2,
            starter_female: false,
            starter_addons_female: 0,
        };
        let mut catalogue = appearance::Catalogue {
            context: appearance::Context {
                content_generation: [0; 32],
                cosmetics_digest: [0; 32],
            },
            root_revision: 1,
            selected: None,
            palette: Vec::new(),
            outfits: vec![appearance::OutfitOption {
                key: outfit.key.clone(),
                look_type: 431,
                permitted_addons: 2,
            }],
            mounts: vec!["mount".into()],
        };
        assert!(has_unlock(
            &catalogue,
            &outfit,
            addon_mask(2),
            Some("mount")
        ));
        assert!(!has_unlock(
            &catalogue,
            &outfit,
            addon_mask(1),
            Some("mount")
        ));
        assert!(!has_unlock(
            &catalogue,
            &outfit,
            addon_mask(2),
            Some("other")
        ));
        catalogue.outfits[0].look_type = 430;
        assert!(!has_unlock(
            &catalogue,
            &outfit,
            addon_mask(2),
            Some("mount")
        ));
    }

    #[test]
    fn forecast_applies_content_bonus_before_table_lookup() {
        assert_eq!(forecast(100, 10, 150), forecast(110, 0, 150));
        assert!(
            forecast(100, 10, 150)
                .zip(forecast(100, 0, 150))
                .is_some_and(|(mounted, walking)| mounted < walking)
        );
        assert_eq!(forecast(100, 10, 0), None);
    }
}
