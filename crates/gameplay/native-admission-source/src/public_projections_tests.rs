use super::*;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

const AUTHORITY: &str = "oteryn:game:primary";
const WORLD: &str = "01934f10-7c02-7001-805b-3b1122334401";
const WORLD_2: &str = "01934f10-7c02-7001-805b-3b1122334402";
const CHARACTER: &str = "01934f10-7c04-7001-805b-3b1122334401";
const GUILD: &str = "01934f10-7c06-7001-805b-3b1122334401";
const MAX_TX: Duration = Duration::from_secs(3);
const CUT_MS: i64 = 1_790_000_000_400;

fn row(index: usize, rank: u64, name: &str) -> HighscoreRow {
    HighscoreRow {
        rank,
        character_id: format!("01934f10-7c04-7001-805b-3b11223{index:05x}"),
        name: name.into(),
        vocation: Some(4),
        level: 412,
        value: 11_400_000_000,
    }
}

fn highscores(rows: Vec<HighscoreRow>) -> HighscoresSnapshot {
    HighscoresSnapshot {
        world_id: WORLD.into(),
        category: 1,
        filter: 0,
        projection_epoch: 1,
        projection_revision: 17,
        source_observed_at: 1_790_000_000,
        computed_at: 1_790_000_000,
        rows,
    }
}

fn profile(entry: Option<ProfileEntry>) -> ProfileSnapshot {
    ProfileSnapshot {
        character_id: CHARACTER.into(),
        projection_epoch: 1,
        projection_revision: 9,
        source_observed_at: 1_790_000_000,
        profile: entry,
    }
}

fn entry(guild: Option<ProfileGuild>) -> ProfileEntry {
    ProfileEntry {
        world_id: WORLD.into(),
        name: "Aldric".into(),
        vocation: 4,
        level: 412,
        guild_visible: true,
        guild,
    }
}

fn online_entry(name: &str) -> OnlineEntry {
    OnlineEntry {
        name: name.into(),
        vocation: 4,
        level: 412,
    }
}

fn online(characters: Vec<OnlineEntry>) -> OnlineSnapshot {
    OnlineSnapshot {
        world_id: WORLD.into(),
        projection_epoch: 1,
        projection_revision: 8812,
        source_observed_at: 1_790_000_000,
        characters,
    }
}

#[test]
fn bodies_match_the_platform_fixtures() {
    let body = encode_highscores(AUTHORITY, &highscores(vec![row(1, 1, "Aldric")])).unwrap();
    // The fixture's character id is the canonical one.
    let body = body.replace("01934f10-7c04-7001-805b-3b1122300001", CHARACTER);
    assert_eq!(
        body,
        concat!(
            r#"{"contract_version":1,"operation":"PublishWorldHighscoresV1","#,
            r#""source_authority":"oteryn:game:primary","#,
            r#""world_id":"01934f10-7c02-7001-805b-3b1122334401","category":1,"filter":0,"#,
            r#""projection_epoch":"1","projection_revision":"17","#,
            r#""source_observed_at":"1790000000","computed_at":"1790000000","#,
            r#""rows":[{"rank":"1","character_id":"01934f10-7c04-7001-805b-3b1122334401","#,
            r#""name":"Aldric","vocation":4,"level":"412","value":"11400000000"}]}"#
        )
    );
    let guild = ProfileGuild {
        guild_id: GUILD.into(),
        rank_level: 3,
    };
    assert_eq!(
        encode_profile(AUTHORITY, &profile(Some(entry(Some(guild))))).unwrap(),
        concat!(
            r#"{"contract_version":1,"operation":"PublishCharacterProfileV1","#,
            r#""source_authority":"oteryn:game:primary","#,
            r#""character_id":"01934f10-7c04-7001-805b-3b1122334401","#,
            r#""projection_epoch":"1","projection_revision":"9","#,
            r#""source_observed_at":"1790000000","profile":[{"#,
            r#""world_id":"01934f10-7c02-7001-805b-3b1122334401","name":"Aldric","#,
            r#""vocation":4,"level":"412","guild_visible":true,"guild":[{"#,
            r#""guild_id":"01934f10-7c06-7001-805b-3b1122334401","rank_level":3}]}]}"#
        )
    );
    assert_eq!(
        encode_online(AUTHORITY, &online(vec![online_entry("Aldric")])).unwrap(),
        concat!(
            r#"{"contract_version":1,"operation":"PublishWorldOnlineV1","#,
            r#""source_authority":"oteryn:game:primary","#,
            r#""world_id":"01934f10-7c02-7001-805b-3b1122334401","#,
            r#""projection_epoch":"1","projection_revision":"8812","#,
            r#""source_observed_at":"1790000000","#,
            r#""characters":[{"name":"Aldric","vocation":4,"level":"412"}]}"#
        )
    );
    assert_eq!(
        encode_watermark(
            AUTHORITY,
            Family::WorldOnline,
            1,
            1_790_000_000,
            1_790_000_003
        )
        .unwrap(),
        concat!(
            r#"{"contract_version":1,"operation":"PublishPublicProjectionWatermarkV1","#,
            r#""source_authority":"oteryn:game:primary","family":"world_online","#,
            r#""projection_epoch":"1","complete_through":"1790000000","#,
            r#""observed_at":"1790000003"}"#
        )
    );
}

/// The shared `character_profile` wire fixtures (contract §4.2), which the Platform
/// consumer mirrors: each file is the exact request body, with no trailing newline.
#[test]
fn profile_matches_the_shared_wire_fixtures() {
    const VISIBLE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../docs/contracts/fixtures/public-projections/character_profile_v1.json"
    ));
    const HIDDEN: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../docs/contracts/fixtures/public-projections/character_profile_guild_hidden_v1.json"
    ));
    let guild = || {
        Some(ProfileGuild {
            guild_id: GUILD.into(),
            rank_level: 3,
        })
    };
    assert_eq!(
        encode_profile(AUTHORITY, &profile(Some(entry(guild())))).unwrap(),
        VISIBLE
    );
    // A visibility change advances the revision (§5); the guild is still sent.
    let mut hidden = profile(Some(ProfileEntry {
        guild_visible: false,
        ..entry(guild())
    }));
    hidden.projection_revision = 10;
    hidden.source_observed_at = 1_790_000_060;
    assert_eq!(encode_profile(AUTHORITY, &hidden).unwrap(), HIDDEN);
}

#[test]
fn empty_lists_are_snapshots() {
    assert!(
        encode_highscores(AUTHORITY, &highscores(vec![]))
            .unwrap()
            .ends_with(r#""rows":[]}"#)
    );
    assert!(
        encode_profile(AUTHORITY, &profile(None))
            .unwrap()
            .ends_with(r#""profile":[]}"#)
    );
    assert!(
        encode_profile(AUTHORITY, &profile(Some(entry(None))))
            .unwrap()
            .ends_with(r#""guild_visible":true,"guild":[]}]}"#)
    );
    // A hidden guild is still sent, marked non-public (§4.2).
    let mut hidden = entry(Some(ProfileGuild {
        guild_id: GUILD.into(),
        rank_level: 3,
    }));
    hidden.guild_visible = false;
    assert!(
        encode_profile(AUTHORITY, &profile(Some(hidden)))
            .unwrap()
            .contains(r#""guild_visible":false,"guild":[{"guild_id":"#)
    );
    assert!(
        encode_online(AUTHORITY, &online(vec![]))
            .unwrap()
            .ends_with(r#""characters":[]}"#)
    );
    for family in [Family::Highscores, Family::CharacterProfile] {
        let body = encode_watermark(AUTHORITY, family, 1, 0, 0).unwrap();
        assert!(body.contains(&format!(r#""family":"{}""#, family.wire())));
    }
}

#[test]
fn vocation_ids_follow_the_contract_table() {
    // Game's `ReferenceBaseVocation` (content::reference_playable): Druid 1, Knight 2, Monk 3, Paladin 4,
    // Sorcerer 5, None 6; a promoted vocation counts as its base.
    let table = [
        ("druid", 1),
        ("elder_druid", 1),
        ("knight", 2),
        ("elite_knight", 2),
        ("monk", 3),
        ("exalted_monk", 3),
        ("paladin", 4),
        ("royal_paladin", 4),
        ("sorcerer", 5),
        ("master_sorcerer", 5),
        ("none", 6),
    ];
    for (key, id) in table {
        assert_eq!(vocation_id(key), Some(id), "{key}");
    }
    for unknown in ["", "Knight", "warrior", "4"] {
        assert_eq!(vocation_id(unknown), None);
    }
}

#[test]
fn worst_case_bodies_fit_their_bounds() {
    let name = "N".repeat(64);
    let rows: Vec<_> = (0..MAX_HIGHSCORE_ROWS)
        .map(|i| {
            let mut r = row(i, u64::try_from(i + 1).unwrap(), &name);
            r.vocation = Some(6);
            r.level = u64::from(u32::MAX);
            r.value = u64::MAX;
            r
        })
        .collect();
    let mut full = highscores(rows.clone());
    full.category = 11;
    full.projection_epoch = u64::MAX;
    full.projection_revision = u64::MAX;
    let body = encode_highscores(AUTHORITY, &full).unwrap();
    assert!(body.len() <= HIGHSCORES_BYTES, "{}", body.len());
    let mut over = full.clone();
    over.rows.push(row(MAX_HIGHSCORE_ROWS, 1001, "Extra"));
    assert!(encode_highscores(AUTHORITY, &over).is_err());

    let mut long = entry(Some(ProfileGuild {
        guild_id: GUILD.into(),
        rank_level: 20,
    }));
    long.name.clone_from(&name);
    long.level = u64::MAX;
    long.vocation = 6;
    let mut big = profile(Some(long));
    big.projection_revision = u64::MAX;
    let authority = "a".repeat(128);
    let body = encode_profile(&authority, &big).unwrap();
    assert!(body.len() <= PROFILE_BYTES, "{}", body.len());

    // 61 bytes of prefix and three letters counting up: distinct and sorted.
    let sorted: Vec<_> = (0..MAX_ONLINE)
        .map(|i| {
            let letter = |n: usize| char::from(b'a' + u8::try_from(n % 26).unwrap());
            OnlineEntry {
                name: format!(
                    "{}{}{}{}",
                    &name[..61],
                    letter(i / 676),
                    letter(i / 26),
                    letter(i)
                ),
                vocation: 6,
                level: u64::MAX,
            }
        })
        .collect();
    let mut many = online(sorted);
    many.projection_revision = u64::MAX;
    let body = encode_online(&authority, &many).unwrap();
    assert!(body.len() <= ONLINE_BYTES, "{}", body.len());
    many.characters.push(online_entry("~"));
    assert!(encode_online(&authority, &many).is_err());

    let body =
        encode_watermark(&authority, Family::CharacterProfile, u64::MAX, 0, i64::MAX).unwrap();
    assert!(body.len() <= WATERMARK_BYTES, "{}", body.len());
}

#[test]
fn vocation_filters_preserve_world_competition_ranks() {
    let snapshot = |filter, ranks: &[u64]| HighscoresSnapshot {
        filter,
        ..highscores(
            ranks
                .iter()
                .enumerate()
                .map(|(index, rank)| row(index, *rank, "Aldric"))
                .collect(),
        )
    };
    for ranks in [&[2, 5, 5, 9][..], &[1, 1, 3][..]] {
        let body = encode_highscores(AUTHORITY, &snapshot(4, ranks)).unwrap();
        let decoded: serde_json::Value = serde_json::from_str(&body).unwrap();
        let encoded_ranks: Vec<_> = decoded["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["rank"].as_str().unwrap())
            .collect();
        assert_eq!(
            encoded_ranks,
            ranks.iter().map(u64::to_string).collect::<Vec<_>>()
        );
    }
    for ranks in [&[3, 2][..], &[0][..], &[2, 2, 3][..], &[4, 4, 4, 5][..]] {
        assert!(encode_highscores(AUTHORITY, &snapshot(4, ranks)).is_err());
    }
    for ranks in [&[2][..], &[1, 3][..]] {
        assert!(encode_highscores(AUTHORITY, &snapshot(0, ranks)).is_err());
    }
}

#[test]
fn highscores_outside_the_contract_are_refused() {
    let refused =
        |rows: Vec<HighscoreRow>| encode_highscores(AUTHORITY, &highscores(rows)).is_err();
    // Competition ranks.
    let tied = vec![
        row(1, 1, "Aa"),
        row(2, 2, "Bb"),
        row(3, 2, "Cc"),
        row(4, 4, "Dd"),
    ];
    assert!(!refused(tied));
    assert!(refused(vec![row(1, 2, "Aa")]));
    assert!(refused(vec![row(1, 1, "Aa"), row(2, 3, "Bb")]));
    assert!(refused(vec![
        row(1, 1, "Aa"),
        row(2, 1, "Bb"),
        row(3, 2, "Cc")
    ]));
    // One row per Character.
    assert!(refused(vec![row(1, 1, "Aa"), row(1, 2, "Aa")]));
    // An unclassifiable vocation is refused, never published as 0.
    let mut unknown = row(1, 1, "Aa");
    unknown.vocation = None;
    assert!(refused(vec![unknown]));
    for id in [0, 7] {
        let mut outside = row(1, 1, "Aa");
        outside.vocation = Some(id);
        assert!(refused(vec![outside]));
    }
    // A base-vocation list holds only that base vocation.
    let mut paladins = highscores(vec![row(1, 1, "Aa")]);
    paladins.filter = 4;
    assert!(encode_highscores(AUTHORITY, &paladins).is_ok());
    paladins.filter = 2;
    assert!(encode_highscores(AUTHORITY, &paladins).is_err());
    let mut zero = row(1, 1, "Aa");
    zero.level = 0;
    assert!(refused(vec![zero]));
    assert!(refused(vec![row(1, 1, "Al\"dric")]));
    for (category, filter) in [(0, 0), (12, 0), (1, 6)] {
        let mut snapshot = highscores(vec![]);
        snapshot.category = category;
        snapshot.filter = filter;
        assert!(encode_highscores(AUTHORITY, &snapshot).is_err());
    }
    let mut never = highscores(vec![]);
    never.computed_at = 0;
    assert!(encode_highscores(AUTHORITY, &never).is_err());
    let mut v4 = highscores(vec![]);
    v4.world_id = "01934f10-7c02-4001-805b-3b1122334401".into();
    assert!(encode_highscores(AUTHORITY, &v4).is_err());
    assert!(encode_highscores("bad authority", &highscores(vec![])).is_err());
}

#[test]
fn profiles_online_lists_and_watermarks_outside_the_contract_are_refused() {
    for rank_level in [0, 21] {
        let guild = ProfileGuild {
            guild_id: GUILD.into(),
            rank_level,
        };
        assert!(encode_profile(AUTHORITY, &profile(Some(entry(Some(guild))))).is_err());
    }
    for id in [0, 7] {
        let mut outside = entry(None);
        outside.vocation = id;
        assert!(encode_profile(AUTHORITY, &profile(Some(outside))).is_err());
        let mut online_outside = online_entry("Aldric");
        online_outside.vocation = id;
        assert!(encode_online(AUTHORITY, &online(vec![online_outside])).is_err());
    }
    let mut zero = profile(None);
    zero.projection_epoch = 0;
    assert!(encode_profile(AUTHORITY, &zero).is_err());

    let unsorted = vec![online_entry("Bera"), online_entry("Aldric")];
    assert!(encode_online(AUTHORITY, &online(unsorted.clone())).is_err());
    assert!(online_digest(&unsorted).is_none());
    let duplicate = vec![online_entry("Aldric"), online_entry("Aldric")];
    assert!(encode_online(AUTHORITY, &online(duplicate)).is_err());
    // Byte order: uppercase sorts before lowercase.
    let bytes = vec![online_entry("Zed"), online_entry("adam")];
    assert!(encode_online(AUTHORITY, &online(bytes)).is_ok());

    assert!(encode_watermark(AUTHORITY, Family::WorldOnline, 0, 0, 0).is_err());
    assert!(encode_watermark(AUTHORITY, Family::WorldOnline, 1, 5, 4).is_err());
    assert!(encode_watermark(AUTHORITY, Family::WorldOnline, 1, -1, 4).is_err());
}

#[derive(Clone, Default)]
struct Clock(Arc<Mutex<Duration>>);

impl ReportClock for Clock {
    fn elapsed(&self) -> Duration {
        *self.0.lock().unwrap()
    }
    fn unix_now(&self) -> i64 {
        0
    }
    /// Virtual time: the clock jumps when the sleep is first polled.
    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send {
        let clock = self.0.clone();
        async move { *clock.lock().unwrap() += duration }
    }
}

#[derive(Default)]
struct Store {
    epoch: u64,
    highscores_source: bool,
    profiles: Vec<ProfileSnapshot>,
    batches: Vec<HighscoresBatch>,
    cleared: Vec<String>,
    held_seen: Vec<Vec<String>>,
    worlds: Vec<(String, Option<Vec<OnlineEntry>>)>,
    online: Vec<OnlineState>,
    handoffs: Vec<NameHandoff>,
    persisted: usize,
    cut_ms: i64,
    /// `online_cut` calls that find the connection busy before one reads.
    cut_busy: usize,
    /// Gameplay waits for the durability holder.
    demand: bool,
    /// Store passes run (every call but `gameplay_waiting`).
    passes: usize,
    clear_passes: usize,
}

type Shared = Arc<Mutex<Store>>;

impl PublicStore for Shared {
    fn gameplay_waiting(&self) -> bool {
        self.lock().unwrap().demand
    }
    async fn outbox_facts(&self) -> Result<OutboxFacts, ()> {
        let mut store = self.lock().unwrap();
        store.passes += 1;
        Ok(OutboxFacts {
            projection_epoch: store.epoch,
            profile_oldest_ms: store.profiles.first().map(|_| 1_790_000_001_000),
            highscores: store.highscores_source.then_some(None),
            now_ms: 1_790_000_010_000,
        })
    }
    async fn next_profiles(
        &self,
        held: &[String],
        limit: usize,
    ) -> Result<Vec<ProfileSnapshot>, ()> {
        let mut store = self.lock().unwrap();
        store.passes += 1;
        store.held_seen.push(held.to_vec());
        Ok(store
            .profiles
            .iter()
            .filter(|p| !held.contains(&p.character_id))
            .take(limit)
            .cloned()
            .collect())
    }
    async fn clear_profiles(&self, acknowledged: &[(String, u64, u64)]) -> Result<(), ()> {
        let mut store = self.lock().unwrap();
        store.passes += 1;
        store.clear_passes += 1;
        for (id, epoch, revision) in acknowledged {
            store.profiles.retain(|p| {
                !(p.character_id == *id
                    && p.projection_epoch <= *epoch
                    && p.projection_revision <= *revision)
            });
            store.cleared.push(id.clone());
        }
        Ok(())
    }
    async fn next_highscores(&self, held: &[String]) -> Result<Option<HighscoresBatch>, ()> {
        let mut store = self.lock().unwrap();
        store.passes += 1;
        Ok(store
            .batches
            .iter()
            .find(|b| !held.contains(&b.subject()))
            .cloned())
    }
    async fn clear_highscores(
        &self,
        world: &str,
        category: u8,
        epoch: u64,
        revision: u64,
    ) -> Result<(), ()> {
        let mut store = self.lock().unwrap();
        store.passes += 1;
        store.batches.retain(|b| {
            !(b.world_id == world
                && b.category == category
                && b.projection_epoch <= epoch
                && b.projection_revision <= revision)
        });
        store.cleared.push(format!("{world}:{category}"));
        Ok(())
    }
    async fn online_cut(&self) -> Result<OnlineCut, ()> {
        let mut store = self.lock().unwrap();
        store.passes += 1;
        if store.cut_busy > 0 {
            store.cut_busy -= 1;
            return Err(());
        }
        let worlds = store
            .worlds
            .iter()
            .map(|(world_id, characters)| OnlineWorld {
                world_id: world_id.clone(),
                characters: characters.clone(),
                state: store
                    .online
                    .iter()
                    .find(|s| s.world_id == *world_id)
                    .cloned(),
            })
            .collect();
        Ok(OnlineCut {
            projection_epoch: store.epoch,
            cut_ms: store.cut_ms,
            worlds,
            handoffs: store.handoffs.clone(),
        })
    }
    async fn persist_online(&self, states: &[OnlineState]) -> Result<(), ()> {
        let mut store = self.lock().unwrap();
        store.passes += 1;
        for state in states {
            store.online.retain(|s| s.world_id != state.world_id);
            store.online.push(state.clone());
            store.persisted += 1;
        }
        Ok(())
    }
    async fn ack_online(&self, states: &[OnlineState], handoffs: &[NameHandoff]) -> Result<(), ()> {
        let mut store = self.lock().unwrap();
        store.passes += 1;
        store.handoffs.retain(|h| !handoffs.contains(h));
        for state in states {
            for stored in &mut store.online {
                if stored.world_id == state.world_id
                    && stored.projection_epoch == state.projection_epoch
                    && stored.projection_revision == state.projection_revision
                {
                    stored.acked = true;
                }
            }
        }
        Ok(())
    }
}

/// A reply per send in order; `None` never answers. An empty queue is `503`.
#[derive(Default)]
struct Sink {
    replies: Mutex<VecDeque<Option<Result<Delivery, NotDelivered>>>>,
    sent: Mutex<Vec<(Operation, String)>>,
    /// Each exchange advances this clock by this much and then answers. The
    /// jump happens inside `send`, never through the clock's sleep, so a
    /// latency never trips the exchange cut (`within`): the reply always
    /// arrives, however long the latency.
    latency: Mutex<Option<(Clock, Duration)>>,
}

impl Sink {
    fn with(replies: Vec<Option<Result<Delivery, NotDelivered>>>) -> Self {
        Self {
            replies: Mutex::new(replies.into()),
            sent: Mutex::default(),
            latency: Mutex::default(),
        }
    }
    fn sent(&self) -> Vec<(Operation, String)> {
        self.sent.lock().unwrap().clone()
    }
    fn operations(&self) -> Vec<Operation> {
        self.sent().into_iter().map(|(op, _)| op).collect()
    }
}

impl PublicSink for Sink {
    async fn send(&self, operation: Operation, body: &str) -> Result<Delivery, NotDelivered> {
        self.sent.lock().unwrap().push((operation, body.into()));
        if let Some((clock, latency)) = self.latency.lock().unwrap().clone() {
            *clock.0.lock().unwrap() += latency;
        }
        let reply = self
            .replies
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Some(Err(NotDelivered::Unavailable)));
        match reply {
            Some(reply) => reply,
            None => std::future::pending().await,
        }
    }
}

struct Fence(Option<u64>);

impl EpochFence for Fence {
    fn read(&mut self) -> Result<u64, FenceUnusable> {
        self.0.ok_or(FenceUnusable)
    }
    fn persist(&mut self, epoch: u64) -> Result<(), FenceUnusable> {
        self.0 = Some(epoch);
        Ok(())
    }
}

type TestPublisher = Publisher<Clock, Shared, Sink, Fence>;

fn publisher(
    store: &Shared,
    replies: Vec<Option<Result<Delivery, NotDelivered>>>,
) -> TestPublisher {
    {
        let mut s = store.lock().unwrap();
        if s.epoch == 0 {
            s.epoch = 1;
        }
    }
    Publisher::new(
        Clock::default(),
        store.clone(),
        Sink::with(replies),
        Fence(Some(1)),
        AUTHORITY.into(),
        MAX_TX,
    )
}

fn block_on<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(future)
}

const OK: Option<Result<Delivery, NotDelivered>> = Some(Ok(Delivery::Accepted));

/// An outbox loop whose watermarks are not yet due.
fn quiet() -> OutboxLoop {
    OutboxLoop {
        watermark_due: Duration::from_secs(3_600),
        ..OutboxLoop::default()
    }
}

#[test]
fn watermarks_go_out_only_for_families_with_a_source() {
    let store = Shared::default();
    let p = publisher(&store, vec![OK, OK, OK]);
    let mut state = OutboxLoop::default();
    block_on(p.outbox_step(&mut state));
    let sent = p.sink.sent();
    assert_eq!(sent.len(), 1);
    assert!(sent[0].1.contains(r#""family":"character_profile""#));
    // Nothing queued: now less the maximum transaction duration.
    assert!(
        sent[0]
            .1
            .contains(r#""complete_through":"1790000007","observed_at":"1790000010""#)
    );
    store.lock().unwrap().highscores_source = true;
    *p.clock.0.lock().unwrap() = WATERMARK_PERIOD;
    block_on(p.outbox_step(&mut state));
    let families: Vec<_> = p.sink.sent()[1..]
        .iter()
        .map(|(_, body)| body.contains(r#""family":"highscores""#))
        .collect();
    assert_eq!(families.len(), 2);
    assert!(families.contains(&true));
}

#[test]
fn a_delivered_profile_is_cleared() {
    let store = Shared::default();
    store
        .lock()
        .unwrap()
        .profiles
        .push(profile(Some(entry(None))));
    let p = publisher(&store, vec![OK]);
    let mut state = quiet();
    assert!(block_on(p.outbox_step(&mut state)));
    assert_eq!(p.sink.operations(), [Operation::PublishCharacterProfileV1]);
    assert_eq!(store.lock().unwrap().cleared, [CHARACTER]);
    assert!(store.lock().unwrap().profiles.is_empty());
}

#[test]
fn a_definite_refusal_holds_the_subject_and_raises_the_alarm() {
    let store = Shared::default();
    store.lock().unwrap().profiles.push(profile(None));
    let p = publisher(&store, vec![Some(Err(NotDelivered::Conflict))]);
    let mut state = quiet();
    assert!(!block_on(p.outbox_step(&mut state)));
    assert_eq!(p.alarms.load(atomic::Ordering::Relaxed), 1);
    // Still queued, now skipped for the hold.
    assert_eq!(store.lock().unwrap().profiles.len(), 1);
    assert!(!block_on(p.outbox_step(&mut state)));
    assert_eq!(p.sink.operations().len(), 1);
    assert_eq!(
        store.lock().unwrap().held_seen.last().unwrap(),
        &[CHARACTER.to_owned()]
    );
    // After the hold it is retried.
    *p.clock.0.lock().unwrap() += DEFINITE_HOLD;
    state.watermark_started = p.clock.elapsed();
    state.watermark_due = p.clock.elapsed() + Duration::from_secs(3_600);
    block_on(p.outbox_step(&mut state));
    assert_eq!(p.sink.operations().len(), 2);
}

#[test]
fn a_full_hold_set_waits_for_a_hold_to_end_and_never_reselects_a_held_subject() {
    let store = Shared::default();
    store.lock().unwrap().profiles = (0..MAX_HELD + 40).map(numbered).collect();
    let refused = Some(Err(NotDelivered::Malformed));
    let p = publisher(&store, vec![refused; MAX_HELD + PROFILE_BATCH]);
    let mut state = quiet();
    for _ in 0..MAX_HELD / PROFILE_BATCH + 3 {
        block_on(p.outbox_step(&mut state));
    }
    // Refused batches fill the hold set; every held subject is skipped by
    // every read, so none is sent twice.
    let sent = p.sink.sent();
    assert_eq!(sent.len(), MAX_HELD);
    assert_eq!(
        sent.iter()
            .map(|(_, body)| body)
            .collect::<HashSet<_>>()
            .len(),
        MAX_HELD
    );
    assert_eq!(p.alarms.load(atomic::Ordering::Relaxed), MAX_HELD as u64);
    assert_eq!(
        store.lock().unwrap().held_seen.last().unwrap().len(),
        MAX_HELD - PROFILE_BATCH
    );
    // Full: the family waits without reading the store.
    let passes = store.lock().unwrap().passes;
    assert!(!block_on(p.outbox_step(&mut state)));
    assert_eq!(store.lock().unwrap().passes, passes);
    // The first batch's holds end: those subjects are retried, once.
    *p.clock.0.lock().unwrap() = DEFINITE_HOLD;
    state.watermark_started = p.clock.elapsed();
    state.watermark_due = p.clock.elapsed() + Duration::from_secs(3_600);
    block_on(p.outbox_step(&mut state));
    let sent = p.sink.sent();
    assert_eq!(sent.len(), MAX_HELD + PROFILE_BATCH);
    assert_eq!(sent[MAX_HELD..], sent[..PROFILE_BATCH]);
    assert_eq!(
        store.lock().unwrap().held_seen.last().unwrap().len(),
        MAX_HELD - PROFILE_BATCH
    );
}

#[test]
fn a_failed_outbox_watermark_backs_off_or_is_held() {
    let store = Shared::default();
    let unavailable = Some(Err(NotDelivered::Unavailable));
    let p = publisher(
        &store,
        vec![
            unavailable,
            unavailable,
            unavailable,
            OK,
            Some(Err(NotDelivered::Conflict)),
            OK,
        ],
    );
    let mut state = OutboxLoop::default();
    let at = |p: &TestPublisher, state: &mut OutboxLoop, seconds: u64| {
        *p.clock.0.lock().unwrap() = Duration::from_secs(seconds);
        block_on(p.outbox_step(state));
        p.sink.operations().len()
    };
    // Backoff 1 s, then 2 s: the 3 s period still decides.
    assert_eq!(at(&p, &mut state, 0), 1);
    assert_eq!(state.watermarks[0].backoff, BACKOFF_MIN);
    assert_eq!(at(&p, &mut state, 3), 2);
    assert_eq!(at(&p, &mut state, 6), 3);
    // Backoff 4 s from 6 s: the watermark due at 9 s waits until 10 s.
    assert_eq!(state.watermarks[0].at, Duration::from_secs(10));
    assert_eq!(at(&p, &mut state, 9), 3);
    assert_eq!(at(&p, &mut state, 12), 4);
    assert_eq!(state.watermarks[0], Gate::default());
    // A definite refusal: the alarm, and no watermark for `DEFINITE_HOLD`.
    assert_eq!(at(&p, &mut state, 15), 5);
    assert_eq!(p.alarms.load(atomic::Ordering::Relaxed), 1);
    assert_eq!(at(&p, &mut state, 18), 5);
    assert_eq!(at(&p, &mut state, 15 + 297), 5);
    assert_eq!(at(&p, &mut state, 15 + 300), 6);
    assert_eq!(state.watermarks[0], Gate::default());
    assert_eq!(p.alarms.load(atomic::Ordering::Relaxed), 1);
}

#[test]
fn a_transient_failure_backs_the_family_off() {
    let store = Shared::default();
    store.lock().unwrap().profiles.push(profile(None));
    let p = publisher(&store, vec![Some(Err(NotDelivered::RateLimited))]);
    let mut state = quiet();
    assert!(!block_on(p.outbox_step(&mut state)));
    assert_eq!(state.retry_at[0], BACKOFF_MIN);
    assert_eq!(p.alarms.load(atomic::Ordering::Relaxed), 0);
    // Backing off: nothing is sent.
    block_on(p.outbox_step(&mut state));
    assert_eq!(p.sink.operations().len(), 1);
    *p.clock.0.lock().unwrap() = BACKOFF_MIN;
    block_on(p.outbox_step(&mut state));
    assert_eq!(p.sink.operations().len(), 2);
    assert_eq!(state.backoff[0], BACKOFF_MIN * 2);
}

#[test]
fn an_unclassifiable_entry_publishes_an_empty_profile() {
    // A name outside the wire grammar, a level of 0 or an unknown vocation:
    // published as `profile: []` and cleared, never held (§4.2).
    for bad in [
        ProfileEntry {
            name: "Al\u{2028}dric".into(),
            ..entry(None)
        },
        ProfileEntry {
            level: 0,
            ..entry(None)
        },
        ProfileEntry {
            vocation: 0,
            ..entry(None)
        },
    ] {
        let store = Shared::default();
        store.lock().unwrap().profiles.push(profile(Some(bad)));
        let p = publisher(&store, vec![OK]);
        let mut state = quiet();
        assert!(block_on(p.outbox_step(&mut state)));
        let sent = p.sink.sent();
        assert_eq!(sent.len(), 1);
        assert_eq!(
            sent[0].1,
            encode_profile(AUTHORITY, &profile(None)).unwrap()
        );
        assert!(sent[0].1.ends_with(r#""profile":[]}"#));
        assert_eq!(p.alarms.load(atomic::Ordering::Relaxed), 0);
        assert_eq!(store.lock().unwrap().cleared, [CHARACTER]);
    }
}

#[test]
fn an_invalid_envelope_is_held_not_sent() {
    let store = Shared::default();
    let mut bad = profile(Some(entry(None)));
    bad.character_id = "not-a-uuid".into();
    store.lock().unwrap().profiles.push(bad);
    let p = publisher(&store, vec![]);
    let mut state = quiet();
    block_on(p.outbox_step(&mut state));
    assert!(p.sink.operations().is_empty());
    assert_eq!(p.alarms.load(atomic::Ordering::Relaxed), 1);
    assert!(store.lock().unwrap().cleared.is_empty());
}

fn numbered(index: usize) -> ProfileSnapshot {
    ProfileSnapshot {
        character_id: format!("01934f10-7c04-7001-805b-3b11223{index:05x}"),
        ..profile(None)
    }
}

#[test]
fn profiles_are_read_and_cleared_in_batches() {
    let store = Shared::default();
    store.lock().unwrap().profiles = (0..PROFILE_BATCH + 3).map(numbered).collect();
    let p = publisher(&store, vec![OK; PROFILE_BATCH + 3]);
    let mut state = quiet();
    assert!(block_on(p.outbox_step(&mut state)));
    {
        let s = store.lock().unwrap();
        // One read pass and one clear pass for the whole batch.
        assert_eq!(s.passes, 2);
        assert_eq!(s.clear_passes, 1);
        assert_eq!(s.cleared.len(), PROFILE_BATCH);
        assert_eq!(s.profiles.len(), 3);
    }
    assert_eq!(p.sink.operations().len(), PROFILE_BATCH);
    assert!(block_on(p.outbox_step(&mut state)));
    assert!(store.lock().unwrap().profiles.is_empty());
    assert_eq!(store.lock().unwrap().clear_passes, 2);
}

#[test]
fn a_transient_failure_stops_the_batch_and_clears_what_was_acknowledged() {
    let store = Shared::default();
    store.lock().unwrap().profiles = (0..4).map(numbered).collect();
    let p = publisher(
        &store,
        vec![
            OK,
            Some(Err(NotDelivered::Conflict)),
            OK,
            Some(Err(NotDelivered::Unavailable)),
        ],
    );
    let mut state = quiet();
    assert!(!block_on(p.outbox_step(&mut state)));
    let s = store.lock().unwrap();
    // The refused one is held, the batch went on; the transient failure
    // stopped it; both acknowledged ones cleared in one pass.
    assert_eq!(
        s.cleared,
        [numbered(0).character_id, numbered(2).character_id]
    );
    assert_eq!(s.clear_passes, 1);
    assert_eq!(p.alarms.load(atomic::Ordering::Relaxed), 1);
    assert!(state.retry_at[0] > p.clock.elapsed());
}

#[test]
fn the_publisher_yields_to_waiting_gameplay_and_paces_its_passes() {
    let store = Shared::default();
    store.lock().unwrap().profiles.push(profile(None));
    store.lock().unwrap().demand = true;
    let p = publisher(&store, vec![OK, OK]);
    let mut state = quiet();
    // Gameplay waits: no store pass runs and nothing is sent.
    assert!(!block_on(p.outbox_step(&mut state)));
    assert!(!block_on(p.scan(&mut ScanLoop::default())));
    assert_eq!(store.lock().unwrap().passes, 0);
    assert!(p.sink.operations().is_empty());
    // Once it is served, the publisher goes on, each pass at least
    // `STORE_PASS_GAP` after the previous one ended.
    store.lock().unwrap().demand = false;
    let before = p.clock.elapsed();
    assert!(block_on(p.outbox_step(&mut state)));
    assert_eq!(store.lock().unwrap().passes, 2);
    assert!(p.clock.elapsed() >= before + STORE_PASS_GAP);
}

fn batch() -> HighscoresBatch {
    let list = vec![row(1, 1, "Aldric")];
    HighscoresBatch {
        world_id: WORLD.into(),
        category: 1,
        projection_epoch: 1,
        projection_revision: 17,
        source_observed_at: 1_790_000_000,
        computed_at: 1_790_000_000,
        lists: [list.clone(), vec![], vec![], vec![], list, vec![]],
    }
}

#[test]
fn world_ranked_vocation_lists_are_published_together() {
    let store = Shared::default();
    let mut knight = row(1, 1, "Aldric");
    knight.vocation = Some(2);
    let mut sorcerer = row(2, 2, "Bera");
    sorcerer.vocation = Some(5);
    sorcerer.value -= 1;
    let mut snapshot = batch();
    snapshot.lists = [
        vec![knight.clone(), sorcerer.clone()],
        vec![],
        vec![knight],
        vec![],
        vec![],
        vec![sorcerer],
    ];
    store.lock().unwrap().batches.push(snapshot);
    let p = publisher(&store, vec![OK; FILTERS]);
    let mut state = quiet();
    state.highscores_source = true;
    state.turn = 1;
    assert!(block_on(p.outbox_step(&mut state)));
    let published: Vec<serde_json::Value> = p
        .sink
        .sent()
        .iter()
        .map(|(_, body)| serde_json::from_str(body).unwrap())
        .collect();
    assert_eq!(published.len(), FILTERS);
    assert_eq!(published[2]["rows"][0]["rank"], "1");
    assert_eq!(published[5]["rows"][0]["rank"], "2");
    assert_eq!(store.lock().unwrap().cleared, [format!("{WORLD}:1")]);
}

#[test]
fn highscores_resume_with_the_filters_not_yet_acknowledged() {
    let store = Shared::default();
    store.lock().unwrap().batches.push(batch());
    let p = publisher(
        &store,
        vec![OK, OK, OK, Some(Err(NotDelivered::Unavailable)), OK, OK, OK],
    );
    let mut state = quiet();
    state.highscores_source = true;
    state.turn = 1;
    assert!(!block_on(p.outbox_step(&mut state)));
    assert_eq!(state.progress.as_ref().unwrap().3, 0b111);
    assert!(store.lock().unwrap().cleared.is_empty());
    *p.clock.0.lock().unwrap() = Duration::from_secs(1);
    state.turn = 1;
    assert!(block_on(p.outbox_step(&mut state)));
    let filters: Vec<_> = p
        .sink
        .sent()
        .iter()
        .map(|(_, body)| {
            let at = body.find(r#""filter":"#).unwrap() + 9;
            body[at..=at].to_owned()
        })
        .collect();
    assert_eq!(filters, ["0", "1", "2", "3", "3", "4", "5"]);
    assert_eq!(store.lock().unwrap().cleared, [format!("{WORLD}:1")]);
    assert!(state.progress.is_none());
}

#[test]
fn a_store_below_the_fence_publishes_nothing() {
    let store = Shared::default();
    store.lock().unwrap().profiles.push(profile(None));
    let mut p = publisher(&store, vec![OK, OK]);
    p.fence = Mutex::new(Fence(Some(2)));
    let mut state = OutboxLoop::default();
    block_on(p.outbox_step(&mut state));
    assert!(p.sink.operations().is_empty());
    store
        .lock()
        .unwrap()
        .worlds
        .push((WORLD.into(), Some(vec![])));
    assert!(!block_on(p.scan(&mut ScanLoop::default())));
    assert!(p.sink.operations().is_empty());
    // A higher epoch moves the fence first.
    store.lock().unwrap().epoch = 3;
    block_on(p.outbox_step(&mut OutboxLoop::default()));
    assert_eq!(p.fence.lock().unwrap().0, Some(3));
    assert!(!p.sink.operations().is_empty());
}

fn scanned(store: &Shared, worlds: Vec<(&str, Option<Vec<OnlineEntry>>)>) {
    let mut s = store.lock().unwrap();
    s.cut_ms = CUT_MS;
    s.worlds = worlds.into_iter().map(|(w, c)| (w.to_owned(), c)).collect();
}

#[test]
fn a_scan_publishes_only_changed_worlds_at_new_revisions() {
    let store = Shared::default();
    scanned(
        &store,
        vec![
            (WORLD, Some(vec![online_entry("Aldric")])),
            (WORLD_2, Some(vec![])),
        ],
    );
    let p = publisher(&store, vec![OK; 8]);
    let mut state = ScanLoop::default();
    assert!(block_on(p.scan(&mut state)));
    assert_eq!(p.sink.operations().len(), 2);
    assert!(
        p.sink.sent()[0]
            .1
            .contains(r#""projection_revision":"1","source_observed_at":"1790000000""#)
    );
    assert!(store.lock().unwrap().online.iter().all(|s| s.acked));
    assert_eq!(
        *p.last_cut.lock().unwrap(),
        Some(GoodCut {
            projection_epoch: 1,
            cut_ms: CUT_MS,
            read_at: Duration::ZERO
        })
    );
    // Unchanged: nothing sent.
    assert!(block_on(p.scan(&mut state)));
    assert_eq!(p.sink.operations().len(), 2);
    // One World changes: only it, at the next revision.
    store.lock().unwrap().worlds[1].1 = Some(vec![online_entry("Bera")]);
    assert!(block_on(p.scan(&mut state)));
    let sent = p.sink.sent();
    assert_eq!(sent.len(), 3);
    assert!(sent[2].1.contains(WORLD_2));
    assert!(sent[2].1.contains(r#""projection_revision":"2""#));
}

/// §7.3: a name handoff is settled (deleted) only once Platform holds a list
/// of the old holder's World taken at a cut that sees it: the World is
/// unchanged and acknowledged, or acknowledged now, or has no snapshot at all;
/// or once it is older than the hold. A World whose send failed keeps its
/// handoff, so its new holder stays held.
#[test]
fn a_scan_settles_only_name_handoffs_platform_has_seen() {
    const WORLD_3: &str = "01934f10-7c02-7001-805b-3b1122334403";
    let store = Shared::default();
    scanned(
        &store,
        vec![
            (WORLD, Some(vec![online_entry("Aldric")])),
            (WORLD_2, Some(vec![online_entry("Cato")])),
        ],
    );
    let p = publisher(
        &store,
        vec![OK, OK, Some(Err(NotDelivered::Unavailable)), OK],
    );
    let mut state = ScanLoop::default();
    assert!(block_on(p.scan(&mut state)));

    let cut_ms = CUT_MS + 10_000;
    let hold = i64::try_from(NAME_HANDOFF_HOLD.as_millis()).unwrap();
    let handoff = |key: &str, world: &str, vacated_at_ms: i64| NameHandoff {
        name_key: key.into(),
        character_id: CHARACTER.into(),
        world_id: world.into(),
        vacated_at_ms,
    };
    let unchanged = handoff("aldo", WORLD, cut_ms - 1_000);
    let failed = handoff("cato", WORLD_2, cut_ms - 1_000);
    let absent = handoff("dara", WORLD_3, cut_ms - 1_000);
    let expired = handoff("eli", WORLD_2, cut_ms - hold);
    {
        let mut s = store.lock().unwrap();
        s.cut_ms = cut_ms;
        // Cato was deleted: WORLD_2's list changes and its send fails.
        s.worlds[1].1 = Some(vec![]);
        s.handoffs = vec![
            unchanged.clone(),
            failed.clone(),
            absent.clone(),
            expired.clone(),
        ];
    }
    *p.clock.0.lock().unwrap() += ONLINE_INTERVAL;
    assert!(!block_on(p.scan(&mut state)));
    assert_eq!(
        p.sink.operations().len(),
        3,
        "only the changed World is sent"
    );
    assert_eq!(store.lock().unwrap().handoffs, vec![failed]);

    // Its retry is acknowledged: settled.
    *p.clock.0.lock().unwrap() += ONLINE_INTERVAL;
    assert!(block_on(p.scan(&mut state)));
    assert_eq!(p.sink.operations().len(), 4);
    assert!(store.lock().unwrap().handoffs.is_empty());
}

#[test]
fn an_unacknowledged_revision_is_resent_unchanged() {
    let store = Shared::default();
    scanned(&store, vec![(WORLD, Some(vec![online_entry("Aldric")]))]);
    let p = publisher(&store, vec![Some(Err(NotDelivered::Unavailable)), OK]);
    let mut state = ScanLoop::default();
    assert!(!block_on(p.scan(&mut state)));
    assert!(p.last_cut.lock().unwrap().is_none());
    store.lock().unwrap().cut_ms = CUT_MS + 10_000;
    *p.clock.0.lock().unwrap() += ONLINE_INTERVAL;
    assert!(block_on(p.scan(&mut state)));
    let sent = p.sink.sent();
    assert_eq!(sent[0].1, sent[1].1);
    assert_eq!(store.lock().unwrap().persisted, 1);
}

#[test]
fn a_world_over_its_bound_fails_the_scan_without_truncation() {
    let store = Shared::default();
    scanned(&store, vec![(WORLD, None), (WORLD_2, Some(vec![]))]);
    let p = publisher(&store, vec![OK]);
    assert!(!block_on(p.scan(&mut ScanLoop::default())));
    assert_eq!(p.sink.operations().len(), 1);
    assert!(p.sink.sent()[0].1.contains(WORLD_2));
    assert!(p.last_cut.lock().unwrap().is_none());
}

#[test]
fn a_world_over_its_bound_is_held_like_a_definite_refusal() {
    let store = Shared::default();
    scanned(&store, vec![(WORLD, None), (WORLD_2, Some(vec![]))]);
    let p = publisher(&store, vec![OK; 4]);
    let mut state = ScanLoop::default();
    assert!(!block_on(p.scan(&mut state)));
    assert_eq!(p.alarms.load(atomic::Ordering::Relaxed), 1);
    assert_eq!(state.next_retry(), None);
    // Held for `DEFINITE_HOLD`: later scans neither process it nor alarm again.
    for _ in 0..3 {
        *p.clock.0.lock().unwrap() += ONLINE_INTERVAL;
        assert!(!block_on(p.scan(&mut state)));
    }
    assert_eq!(p.alarms.load(atomic::Ordering::Relaxed), 1);
    assert_eq!(p.sink.operations().len(), 1);
    assert!(p.last_cut.lock().unwrap().is_none());
    // Still over its bound when the hold ends: refused and held again.
    *p.clock.0.lock().unwrap() = DEFINITE_HOLD;
    assert!(!block_on(p.scan(&mut state)));
    assert_eq!(p.alarms.load(atomic::Ordering::Relaxed), 2);
    // Within its bound after that hold: sent, and the scan completes.
    store.lock().unwrap().worlds[0].1 = Some(vec![online_entry("Aldric")]);
    *p.clock.0.lock().unwrap() += DEFINITE_HOLD - ONLINE_INTERVAL;
    assert!(!block_on(p.scan(&mut state)));
    assert_eq!(p.sink.operations().len(), 1);
    *p.clock.0.lock().unwrap() += ONLINE_INTERVAL;
    assert!(block_on(p.scan(&mut state)));
    assert_eq!(p.alarms.load(atomic::Ordering::Relaxed), 2);
    let sent = p.sink.sent();
    assert_eq!(sent.len(), 2);
    assert!(sent[1].1.contains(WORLD) && sent[1].1.contains("Aldric"));
}

#[test]
fn a_scan_past_its_deadline_fails() {
    let store = Shared::default();
    scanned(&store, vec![(WORLD, Some(vec![])), (WORLD_2, Some(vec![]))]);
    let p = publisher(&store, vec![OK, None]);
    assert!(!block_on(p.scan(&mut ScanLoop::default())));
    // The unanswered exchange is cut at its own bound.
    assert!(p.clock.elapsed() >= EXCHANGE_BOUND);
    assert!(p.last_cut.lock().unwrap().is_none());
    // The acknowledged World is recorded; the other is resent next scan.
    let online = store.lock().unwrap().online.clone();
    assert_eq!(online.iter().filter(|s| s.acked).count(), 1);

    // Both accepted, each within the exchange bound, but the last after the
    // deadline (this fake's exchanges run one after the other): the scan fails
    // and its cut is not a good one, though both acknowledgements are kept.
    // The sink's latency never trips the cut (it is not a sleep), so only the
    // scan deadline, checked after the sends, fails this scan.
    let store = Shared::default();
    scanned(&store, vec![(WORLD, Some(vec![])), (WORLD_2, Some(vec![]))]);
    let p = publisher(&store, vec![OK, OK]);
    *p.sink.latency.lock().unwrap() = Some((p.clock.clone(), Duration::from_millis(2_600)));
    assert!(!block_on(p.scan(&mut ScanLoop::default())));
    assert!(p.clock.elapsed() > ONLINE_SCAN_DEADLINE);
    assert!(p.last_cut.lock().unwrap().is_none());
    let online = store.lock().unwrap().online.clone();
    assert_eq!(online.iter().filter(|s| s.acked).count(), 2);
}

#[test]
fn the_online_scan_deadline_runs_from_the_cut() {
    let store = Shared::default();
    scanned(&store, vec![(WORLD, Some(vec![]))]);
    // The connection is busy for the first 25 cut reads (100 ms apart); a
    // 1 s maximum transaction leaves the cut until 4 s after the start.
    store.lock().unwrap().cut_busy = 25;
    let mut p = publisher(&store, vec![OK]);
    p.max_transaction = Duration::from_secs(1);
    *p.sink.latency.lock().unwrap() = Some((p.clock.clone(), Duration::from_millis(2_900)));
    assert!(block_on(p.scan(&mut ScanLoop::default())));
    let cut = p.last_cut.lock().unwrap().unwrap();
    assert_eq!(cut.read_at, Duration::from_millis(2_500));
    // Acknowledged more than 5 s after the scan started, within 5 s of the cut.
    assert!(p.clock.elapsed() > ONLINE_SCAN_DEADLINE);
    assert!(p.clock.elapsed() <= cut.read_at + ONLINE_SCAN_DEADLINE);
}

#[test]
fn an_exchange_is_cut_at_the_exchange_bound() {
    // world_online: the scan leaves 5 s, but an exchange that has not answered
    // after 3 s is a timeout; the World is not acknowledged and backs off.
    let store = Shared::default();
    scanned(&store, vec![(WORLD, Some(vec![]))]);
    let p = publisher(&store, vec![None]);
    let mut state = ScanLoop::default();
    assert!(!block_on(p.scan(&mut state)));
    let cut_at = p.clock.elapsed();
    assert!(
        cut_at >= EXCHANGE_BOUND && cut_at < ONLINE_SCAN_DEADLINE,
        "{cut_at:?}"
    );
    assert_eq!(state.next_retry(), Some(cut_at + BACKOFF_MIN));
    assert!(store.lock().unwrap().online.iter().all(|s| !s.acked));
    assert!(p.last_cut.lock().unwrap().is_none());

    // A profile's phase leaves far more, but its exchange is cut at 3 s too:
    // it stays queued and the family backs off.
    let store = Shared::default();
    store.lock().unwrap().profiles.push(profile(None));
    let p = publisher(&store, vec![None]);
    let mut state = quiet();
    assert!(!block_on(p.outbox_step(&mut state)));
    assert_eq!(p.clock.elapsed(), EXCHANGE_BOUND);
    assert_eq!(store.lock().unwrap().profiles.len(), 1);
    assert!(store.lock().unwrap().cleared.is_empty());
    assert!(!block_on(p.outbox_step(&mut state)));
    assert_eq!(p.sink.operations().len(), 1, "backing off");
}

#[test]
fn a_failed_online_watermark_backs_off_or_is_held() {
    let store = Shared::default();
    scanned(&store, vec![(WORLD, Some(vec![]))]);
    let p = publisher(
        &store,
        vec![
            OK,
            Some(Err(NotDelivered::Unavailable)),
            Some(Err(NotDelivered::Unauthenticated)),
            OK,
        ],
    );
    assert!(block_on(p.scan(&mut ScanLoop::default())));
    let start = p.clock.elapsed();
    block_on(p.online_watermark());
    assert_eq!(p.sink.operations().len(), 2);
    // Backing off for 1 s.
    block_on(p.online_watermark());
    assert_eq!(p.sink.operations().len(), 2);
    *p.clock.0.lock().unwrap() = start + BACKOFF_MIN;
    block_on(p.online_watermark());
    assert_eq!(p.sink.operations().len(), 3);
    assert_eq!(p.alarms.load(atomic::Ordering::Relaxed), 1);
    // Held for `DEFINITE_HOLD` after the refusal.
    *p.clock.0.lock().unwrap() = start + DEFINITE_HOLD;
    block_on(p.online_watermark());
    assert_eq!(p.sink.operations().len(), 3);
    *p.clock.0.lock().unwrap() = start + BACKOFF_MIN + DEFINITE_HOLD;
    block_on(p.online_watermark());
    assert_eq!(p.sink.operations().len(), 4);
    assert_eq!(*p.online_watermark_gate.lock().unwrap(), Gate::default());
}

#[test]
fn the_online_watermark_follows_the_last_good_scan() {
    let store = Shared::default();
    scanned(&store, vec![(WORLD, Some(vec![]))]);
    let p = publisher(&store, vec![OK, OK]);
    block_on(p.online_watermark());
    assert!(p.sink.operations().is_empty());
    assert!(block_on(p.scan(&mut ScanLoop::default())));
    *p.clock.0.lock().unwrap() = Duration::from_millis(2_700);
    block_on(p.online_watermark());
    let sent = p.sink.sent();
    assert_eq!(sent[1].0, Operation::PublishPublicProjectionWatermarkV1);
    // Cut 1790000000.400 + 2.7 s = 1790000003.100.
    assert!(sent[1].1.contains(
        r#""family":"world_online","projection_epoch":"1","complete_through":"1790000000","observed_at":"1790000003""#
    ));
}

#[test]
fn a_definite_refusal_holds_the_world() {
    let store = Shared::default();
    scanned(&store, vec![(WORLD, Some(vec![]))]);
    let p = publisher(&store, vec![Some(Err(NotDelivered::Malformed)), OK]);
    let mut state = ScanLoop::default();
    assert!(!block_on(p.scan(&mut state)));
    assert_eq!(p.alarms.load(atomic::Ordering::Relaxed), 1);
    assert!(!block_on(p.scan(&mut state)));
    assert_eq!(p.sink.operations().len(), 1);
    *p.clock.0.lock().unwrap() += DEFINITE_HOLD;
    assert!(block_on(p.scan(&mut state)));
    assert_eq!(p.sink.operations().len(), 2);
}

#[test]
fn an_unchanged_online_watermark_is_not_resent() {
    let store = Shared::default();
    scanned(&store, vec![(WORLD, Some(vec![]))]);
    let p = publisher(&store, vec![OK; 4]);
    let mut state = ScanLoop::default();
    assert!(block_on(p.scan(&mut state)));
    block_on(p.online_watermark());
    assert_eq!(p.sink.operations().len(), 2);
    // Same cut: nothing is sent.
    block_on(p.online_watermark());
    assert_eq!(p.sink.operations().len(), 2);
    // A later good scan moves `complete_through`: sent again.
    store.lock().unwrap().cut_ms = CUT_MS + ONLINE_INTERVAL_MS;
    assert!(block_on(p.scan(&mut state)));
    block_on(p.online_watermark());
    let sent = p.sink.sent();
    assert_eq!(sent.len(), 3);
    assert!(sent[2].1.contains(r#""complete_through":"1790000010""#));
}

const ONLINE_INTERVAL_MS: i64 = 10_000;

#[test]
fn line_and_paragraph_separators_are_outside_the_name_grammar() {
    for name in [
        "Al\u{2028}dric",
        "Al\u{2029}dric",
        "Al\u{85}dric",
        "Ald\u{e9}ric",
    ] {
        assert!(!name_valid(name), "{name:?}");
        assert!(
            encode_profile(
                AUTHORITY,
                &profile(Some(ProfileEntry {
                    name: name.into(),
                    ..entry(None)
                }))
            )
            .is_err()
        );
        assert!(encode_online(AUTHORITY, &online(vec![online_entry(name)])).is_err());
        assert!(encode_highscores(AUTHORITY, &highscores(vec![row(1, 1, name)])).is_err());
    }
}

#[test]
fn highscores_filters_and_categories_match_the_platform_consumer() {
    // Filter 0 is all; 1..5 are the base vocations by their wire vocation id
    // (Platform `highscoreVocationFilter`: Druid 1, Knight 2, Monk 3,
    // Paladin 4, Sorcerer 5).
    for (index, (name, id)) in HIGHSCORE_FILTERS.iter().enumerate() {
        assert_eq!(usize::from(*id), index);
        if *id == 0 {
            assert_eq!(*name, "all");
        } else {
            assert_eq!(vocation_id(name), Some(*id), "{name}");
        }
    }
    // Platform `CommunityDataPolicy` category ids 1..9, then 10 and 11.
    let platform = [
        ("experience", 1),
        ("magic", 2),
        ("fist", 3),
        ("club", 4),
        ("sword", 5),
        ("axe", 6),
        ("distance", 7),
        ("shielding", 8),
        ("fishing", 9),
    ];
    for (name, id) in platform {
        assert!(HIGHSCORE_CATEGORIES.contains(&(name, id)), "{name}");
    }
    for (index, (_, id)) in HIGHSCORE_CATEGORIES.iter().enumerate() {
        assert_eq!(usize::from(*id), index + 1);
        let mut snapshot = highscores(vec![]);
        snapshot.category = *id;
        assert!(encode_highscores(AUTHORITY, &snapshot).is_ok());
    }
    for filter in 1..u8::try_from(FILTERS).unwrap() {
        let mut snapshot = highscores(vec![row(1, 1, "Aa")]);
        snapshot.filter = filter;
        snapshot.rows[0].vocation = Some(filter);
        assert!(encode_highscores(AUTHORITY, &snapshot).is_ok());
    }
}

/// The retry state of `world`: its backoff and when it is due.
fn retry_of(state: &ScanLoop, world: &str) -> Option<(Duration, Duration)> {
    state.retry(world).map(|r| (r.backoff, r.at))
}

#[test]
fn a_transient_online_failure_backs_the_world_off_from_one_to_sixty_seconds() {
    let store = Shared::default();
    scanned(
        &store,
        vec![
            (WORLD, Some(vec![online_entry("Aldric")])),
            (WORLD_2, Some(vec![])),
        ],
    );
    let p = publisher(&store, vec![Some(Err(NotDelivered::RateLimited)), OK]);
    let mut state = ScanLoop::default();
    assert!(!block_on(p.scan(&mut state)));
    assert_eq!(p.sink.operations().len(), 2);
    // The first retry is due about 1 s after the failure, not at the scan interval.
    let (backoff, at) = retry_of(&state, WORLD).unwrap();
    assert_eq!(backoff, BACKOFF_MIN);
    assert!(at > p.clock.elapsed() && at <= p.clock.elapsed() + BACKOFF_MIN);
    assert_eq!(state.next_retry(), Some(at));
    assert!(retry_of(&state, WORLD_2).is_none());
    // Not yet due: neither a retry nor a scan sends it, and the scan fails.
    block_on(p.retry_scan(&mut state));
    assert_eq!(p.sink.operations().len(), 2);
    assert!(!block_on(p.scan(&mut state)));
    assert_eq!(p.sink.operations().len(), 2);
    assert_eq!(retry_of(&state, WORLD), Some((backoff, at)));
    // Due: the retry sends only that World; each failure doubles the backoff up to 60 s.
    store.lock().unwrap().worlds[1].1 = Some(vec![online_entry("Bera")]);
    let mut expected = BACKOFF_MIN;
    for _ in 0..8 {
        let (_, at) = retry_of(&state, WORLD).unwrap();
        *p.clock.0.lock().unwrap() = at;
        let sent = p.sink.operations().len();
        block_on(p.retry_scan(&mut state));
        let after = p.sink.sent();
        assert_eq!(after.len(), sent + 1);
        assert!(after[sent].1.contains(WORLD) && !after[sent].1.contains(WORLD_2));
        expected = (expected * 2).min(BACKOFF_MAX);
        let (backoff, next) = retry_of(&state, WORLD).unwrap();
        assert_eq!(backoff, expected);
        assert!(next > at && next <= p.clock.elapsed() + backoff);
    }
    assert_eq!(expected, BACKOFF_MAX);
    // A delivery resets the backoff; a retry never completes a scan.
    let (_, at) = retry_of(&state, WORLD).unwrap();
    *p.clock.0.lock().unwrap() = at;
    p.sink.replies.lock().unwrap().push_back(OK);
    block_on(p.retry_scan(&mut state));
    assert!(retry_of(&state, WORLD).is_none());
    assert_eq!(state.next_retry(), None);
    assert!(p.last_cut.lock().unwrap().is_none());
    let acked: Vec<bool> = store
        .lock()
        .unwrap()
        .online
        .iter()
        .map(|s| s.acked)
        .collect();
    assert_eq!(acked, vec![true, true]);
    // The next scan sends the other World's change and completes.
    p.sink.replies.lock().unwrap().push_back(OK);
    assert!(block_on(p.scan(&mut state)));
    assert!(p.sink.sent().last().unwrap().1.contains(WORLD_2));
    assert!(p.last_cut.lock().unwrap().is_some());
}

#[test]
fn a_backing_off_world_retried_by_a_scan_resets_on_delivery() {
    let store = Shared::default();
    scanned(&store, vec![(WORLD, Some(vec![]))]);
    let p = publisher(&store, vec![Some(Err(NotDelivered::Unavailable)), None, OK]);
    let mut state = ScanLoop::default();
    assert!(!block_on(p.scan(&mut state)));
    assert_eq!(retry_of(&state, WORLD).unwrap().0, BACKOFF_MIN);
    // A timeout is transient too.
    *p.clock.0.lock().unwrap() = retry_of(&state, WORLD).unwrap().1;
    block_on(p.retry_scan(&mut state));
    assert_eq!(retry_of(&state, WORLD).unwrap().0, BACKOFF_MIN * 2);
    // A due World is sent by a full scan as well, and its delivery resets the backoff.
    *p.clock.0.lock().unwrap() = retry_of(&state, WORLD).unwrap().1;
    assert!(block_on(p.scan(&mut state)));
    assert!(retry_of(&state, WORLD).is_none());
    assert_eq!(p.sink.operations().len(), 3);
}

#[test]
fn a_definite_refusal_or_a_vanished_world_ends_its_backoff() {
    let store = Shared::default();
    scanned(&store, vec![(WORLD, Some(vec![])), (WORLD_2, Some(vec![]))]);
    let p = publisher(
        &store,
        vec![
            Some(Err(NotDelivered::Unavailable)),
            Some(Err(NotDelivered::Unavailable)),
            Some(Err(NotDelivered::Conflict)),
        ],
    );
    let mut state = ScanLoop::default();
    assert!(!block_on(p.scan(&mut state)));
    assert!(retry_of(&state, WORLD).is_some() && retry_of(&state, WORLD_2).is_some());
    // WORLD_2 leaves the cut; WORLD is refused: held, not retried.
    store.lock().unwrap().worlds.truncate(1);
    *p.clock.0.lock().unwrap() = state.next_retry().unwrap();
    block_on(p.retry_scan(&mut state));
    assert_eq!(state.next_retry(), None);
    assert_eq!(p.alarms.load(atomic::Ordering::Relaxed), 1);
    assert_eq!(p.sink.operations().len(), 3);
}

#[test]
fn a_retry_that_cannot_read_the_cut_backs_off_again() {
    let store = Shared::default();
    scanned(&store, vec![(WORLD, Some(vec![]))]);
    let p = publisher(&store, vec![Some(Err(NotDelivered::Unavailable))]);
    let mut state = ScanLoop::default();
    assert!(!block_on(p.scan(&mut state)));
    let (_, at) = retry_of(&state, WORLD).unwrap();
    *p.clock.0.lock().unwrap() = at;
    store.lock().unwrap().demand = true;
    block_on(p.retry_scan(&mut state));
    // Nothing was sent, and the World is not due again at once.
    assert_eq!(p.sink.operations().len(), 1);
    let (backoff, next) = retry_of(&state, WORLD).unwrap();
    assert_eq!(backoff, BACKOFF_MIN * 2);
    assert!(next > p.clock.elapsed());
}

#[test]
fn projection_names_refuse_slash_dot_and_dot_dot() {
    for name in ["/", "Al/dric", ".", ".."] {
        assert!(!projection_name_valid(name), "{name:?}");
        // The account-characters rule keeps its own grammar.
        assert!(name_valid(name), "{name:?}");
        assert!(
            encode_profile(
                AUTHORITY,
                &profile(Some(ProfileEntry {
                    name: name.into(),
                    ..entry(None)
                }))
            )
            .is_err()
        );
        assert!(encode_online(AUTHORITY, &online(vec![online_entry(name)])).is_err());
        assert!(encode_highscores(AUTHORITY, &highscores(vec![row(1, 1, name)])).is_err());
    }
    for name in ["Al Dric", "...", "Al.dric", ".a"] {
        assert!(projection_name_valid(name), "{name:?}");
    }
}
