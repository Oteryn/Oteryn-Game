//! E2EF7: complete bounded ranking journey using ordinary admitted characters.
//! No privileged provisioning: the retained snapshot must come from the served job.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::highscores::{self, wire};
use std::collections::BTreeSet;

const RANKING_ACTIONS: &[&str] = &[
    "categories",
    "bounded-pages",
    "vocation-filters",
    "own-rank",
    "unsupported-refusal",
    "unknown-snapshot-refusal",
    "relog-snapshot",
];

// Kept outside the timed future so cancellation retains the exact unfinished read.
// These are logical ranking queries (one may retry an empty refusal), not wire counts.
struct RankingProgress {
    stage: &'static str,
    stage_started: tokio::time::Instant,
    query: Option<wire::HighscoresQuery>,
    query_started: tokio::time::Instant,
    issued: u32,
    finished: u32,
    validated: u32,
    in_flight: bool,
}

impl RankingProgress {
    fn new() -> Self {
        let now = tokio::time::Instant::now();
        Self {
            stage: "initial_admission",
            stage_started: now,
            query: None,
            query_started: now,
            issued: 0,
            finished: 0,
            validated: 0,
            in_flight: false,
        }
    }

    fn stage(&mut self, stage: &'static str, query: Option<wire::HighscoresQuery>) {
        self.stage = stage;
        self.stage_started = tokio::time::Instant::now();
        self.query = query;
        self.in_flight = false;
        self.report("start");
    }

    fn begin_query(&mut self, query: wire::HighscoresQuery) {
        self.query = Some(query);
        self.query_started = tokio::time::Instant::now();
        self.issued += 1;
        self.in_flight = true;
    }

    fn finish_query(&mut self, validated: bool) {
        self.finished = self.issued;
        self.validated += u32::from(validated);
        self.in_flight = false;
        if !validated {
            self.report("query_failed");
        }
    }

    fn report(&self, state: &'static str) {
        let query = self.query.map_or_else(String::new, |query| {
            let snapshot = match query.snapshot_id {
                0 => "latest",
                u64::MAX => "out_of_range",
                _ => "retained",
            };
            format!(
                " category={} vocation={} page={} snapshot={snapshot}",
                query.category, query.vocation, query.page,
            )
        });
        evidence(&format!(
            "E2EF7_PROGRESS scenario=S11 stage={} state={state} issued={} finished={} validated={} in_flight={} stage_ms={} query_ms={}{}",
            self.stage,
            self.issued,
            self.finished,
            self.validated,
            self.in_flight,
            self.stage_started.elapsed().as_millis(),
            if self.in_flight {
                self.query_started.elapsed().as_millis()
            } else {
                0
            },
            query,
        ));
    }
}

struct Row {
    required: &'static [&'static str],
    passed: BTreeSet<&'static str>,
    invalid_action: bool,
    started: std::time::Instant,
}

impl Row {
    fn new(required: &'static [&'static str]) -> Self {
        Self {
            required,
            passed: BTreeSet::new(),
            invalid_action: false,
            started: std::time::Instant::now(),
        }
    }

    fn pass(&mut self, action: &'static str) {
        if !self.required.contains(&action) || !self.passed.insert(action) {
            self.invalid_action = true;
            return;
        }
        evidence(&format!(
            "E2EF7_CHECK S11 {action} PASS server-frames-validated"
        ));
    }

    fn complete(&self) -> bool {
        !self.required.is_empty()
            && !self.invalid_action
            && self
                .required
                .iter()
                .all(|action| self.passed.contains(action))
    }

    fn finish(self, result: TestResult) {
        for action in self.required {
            if !self.passed.contains(action) {
                evidence(&format!("E2EF7_GAP S11 {action} unproven"));
            }
        }
        if let Err(error) = &result {
            evidence(&format!("E2EF7_GAP S11 {error}"));
        }
        evidence(&format!(
            "e2e_all scenario=S11.full status={} seconds={:.3}",
            if result.is_ok() && self.complete() {
                "PASS"
            } else {
                "FAIL"
            },
            self.started.elapsed().as_secs_f64(),
        ));
    }
}

pub(super) async fn run(clients: &SeamClients<'_>) {
    let mut row = Row::new(RANKING_ACTIONS);
    let mut progress = RankingProgress::new();
    // Admission and final logout are part of the row, so failure cannot silently
    // omit its report. Other packet mounts can still run after this returns.
    let result = tokio::time::timeout(
        Duration::from_secs(180),
        journey(&mut row, clients, &mut progress),
    )
    .await;
    progress.report(match &result {
        Ok(Ok(())) => "finished",
        Ok(Err(_)) => "failed",
        Err(_) => "deadline",
    });
    row.finish(match result {
        Ok(result) => result,
        Err(_) => Err("ranking journey deadline; remaining actions unproven".into()),
    });
}

async fn exchange(
    client: &mut DevClientSession,
    query: wire::HighscoresQuery,
) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
    let command = highscores::command(query).map_err(|e| format!("ranking input: {e:?}"))?;
    Ok(client
        .session_mut()
        .gated_command(command.capability, command.command_type, &command.payload)
        .await
        .map_err(|error| format!("ranking query {query:?}: {error}"))?)
}

async fn query(
    client: &mut DevClientSession,
    query: wire::HighscoresQuery,
) -> TestResult<wire::HighscoresResult> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        let reply = exchange(client, query).await?;
        if reply.status == CommandStatus::Accepted {
            let page = wire::decode_highscores_result(&reply.payload)
                .map_err(|e| format!("ranking result: {e:?}"))?;
            validate_page(&page, query)?;
            return Ok(page);
        }
        // The shared durable root can be briefly busy while the snapshot job
        // publishes. Retry only explicit empty refusals, with a fixed deadline.
        if reply.status != CommandStatus::Rejected
            || !reply.payload.is_empty()
            || tokio::time::Instant::now() >= deadline
        {
            return Err(format!("ranking query refused: {reply:?}").into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    }
}

async fn observed_query(
    client: &mut DevClientSession,
    request: wire::HighscoresQuery,
    progress: &mut RankingProgress,
) -> TestResult<wire::HighscoresResult> {
    progress.begin_query(request);
    let result = query(client, request).await;
    progress.finish_query(result.is_ok());
    result
}

fn validate_page(page: &wire::HighscoresResult, query: wire::HighscoresQuery) -> TestResult {
    if page.category != query.category
        || page.page != query.page
        || page.snapshot_id == 0
        || (query.snapshot_id != 0 && page.snapshot_id != query.snapshot_id)
        || page.rows.len() > wire::MAX_HIGHSCORES_PAGE_ROWS
        || (page.has_more && (page.rows.len() != wire::MAX_HIGHSCORES_PAGE_ROWS || page.page >= 20))
        || (page.page > 20 && (!page.rows.is_empty() || page.has_more))
    {
        return Err("ranking response identity or page bound mismatch".into());
    }
    if page
        .rows
        .windows(2)
        .any(|pair| pair[0].rank > pair[1].rank || pair[0].value < pair[1].value)
    {
        return Err(
            "ranking page is not ordered by descending value and ascending global rank".into(),
        );
    }
    Ok(())
}

fn request(category: u32, vocation: u32, page: u32, snapshot_id: u64) -> wire::HighscoresQuery {
    wire::HighscoresQuery {
        world: None,
        details: false,
        category,
        vocation,
        page,
        snapshot_id,
    }
}

async fn list(
    client: &mut DevClientSession,
    category: u32,
    vocation: u32,
    snapshot_id: u64,
    progress: &mut RankingProgress,
    stage: &'static str,
) -> TestResult<(wire::HighscoresResult, Vec<wire::HighscoreRow>)> {
    let first_query = request(category, vocation, 1, snapshot_id);
    progress.stage(stage, Some(first_query));
    let first = observed_query(client, first_query, progress).await?;
    let mut rows = first.rows.clone();
    let mut more = first.has_more;
    for number in 2..=20 {
        if !more {
            break;
        }
        let page = observed_query(
            client,
            request(category, vocation, number, first.snapshot_id),
            progress,
        )
        .await?;
        if page.computed_at != first.computed_at || page.own_row != first.own_row {
            return Err("ranking cut or own rank changed between pinned pages".into());
        }
        more = page.has_more;
        rows.extend(page.rows);
    }
    let names: BTreeSet<_> = rows.iter().map(|row| &row.name).collect();
    if more
        || rows.len() > wire::MAX_HIGHSCORES_LIST_ROWS as usize
        || names.len() != rows.len()
        || rows
            .windows(2)
            .any(|pair| pair[0].rank > pair[1].rank || pair[0].value < pair[1].value)
    {
        return Err("ranking list repeats rows, exceeds cap or misorders a page boundary".into());
    }
    let beyond = observed_query(
        client,
        request(category, vocation, u32::MAX, first.snapshot_id),
        progress,
    )
    .await?;
    if !beyond.rows.is_empty()
        || beyond.has_more
        || beyond.own_row != first.own_row
        || beyond.computed_at != first.computed_at
    {
        return Err("beyond-end query lost own rank or returned unbounded rows".into());
    }
    Ok((first, rows))
}

async fn refused(
    client: &mut DevClientSession,
    query: wire::HighscoresQuery,
    progress: &mut RankingProgress,
) -> TestResult {
    progress.begin_query(query);
    let result = async {
        let reply = exchange(client, query).await?;
        // The generic session adapter exposes status/payload, not the terminal code.
        // Do not claim to have checked NOT_SUPPORTED/SNAPSHOT_EXPIRED codes here.
        if reply.status != CommandStatus::Rejected || !reply.payload.is_empty() {
            return Err(
                format!("invalid ranking query not refused without rows: {reply:?}").into(),
            );
        }
        Ok(())
    }
    .await;
    progress.finish_query(result.is_ok());
    result
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let (result, logout, attempts, busy_timed_out) =
        super::logout_retrying_busy(client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("ranking character logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
    }
    Ok(())
}

async fn journey(
    row: &mut Row,
    clients: &SeamClients<'_>,
    progress: &mut RankingProgress,
) -> TestResult {
    progress.stage("initial_admission", None);
    let mut client = connect_world(clients, 0xa0)
        .await
        .map_err(|error| format!("ranking initial admission: {error}"))?;
    let initial_session = client.join_snapshot().game_session_id;
    let mut saved = Vec::new();
    let result: TestResult = async {
        let entities = client.world_entities().ok_or("ranking own entity absent")?;
        let own_name = entities
            .iter()
            .find(|entry| entry.entity.identity == *entities.own_identity())
            .and_then(|entry| entry.display_name)
            .ok_or("ranking own display name absent")?;
        let own_name = own_name
            .as_str()
            .map_err(|e| format!("ranking own name: {e:?}"))?;
        let mut observed_own = false;
        // Charm points (11) have no activated source. Its unsupported response
        // is exercised as a refusal; it must never masquerade as an empty list.
        for category in (1..=10).chain(std::iter::once(12)) {
            let (first, all) = list(&mut client, category, 0, 0, progress, "category_list").await?;
            if let Some(own) = &first.own_row {
                if own.name != own_name {
                    return Err("ranking own row names another character".into());
                }
                if let Some(listed) = all.iter().find(|entry| entry.name == own.name)
                    && listed != own
                {
                    return Err("own rank differs from listed row".into());
                }
                observed_own = true;
            }
            for (vocation, key) in [
                (1, "knight"),
                (2, "paladin"),
                (3, "sorcerer"),
                (4, "druid"),
                (5, "monk"),
            ] {
                let (filtered, entries) = list(
                    &mut client,
                    category,
                    vocation,
                    first.snapshot_id,
                    progress,
                    "vocation_list",
                )
                .await?;
                if entries.iter().chain(filtered.own_row.iter()).any(|entry| {
                    oteryn_gameplay_progression::wheel::base_vocation(&entry.vocation) != Some(key)
                }) || filtered.computed_at != first.computed_at
                {
                    return Err("vocation filter admitted another vocation or cut".into());
                }
                for entry in &entries {
                    if let Some(global) = all.iter().find(|global| global.name == entry.name)
                        && global != entry
                    {
                        return Err("vocation filter reranked a global row".into());
                    }
                }
                if all.len() < wire::MAX_HIGHSCORES_LIST_ROWS as usize {
                    let expected: Vec<_> = all
                        .iter()
                        .filter(|entry| {
                            oteryn_gameplay_progression::wheel::base_vocation(&entry.vocation)
                                == Some(key)
                        })
                        .cloned()
                        .collect();
                    if entries != expected {
                        return Err("vocation filter omitted or invented ranked rows".into());
                    }
                }
                let expected_own = first.own_row.as_ref().filter(|own| {
                    oteryn_gameplay_progression::wheel::base_vocation(&own.vocation) == Some(key)
                });
                if filtered.own_row.as_ref() != expected_own {
                    return Err("vocation-filtered own rank mismatches global own rank".into());
                }
            }
            saved.push((first, all));
        }
        row.pass("categories");
        row.pass("bounded-pages");
        row.pass("vocation-filters");
        if !observed_own {
            return Err("no own ranking row observed; empty lists cannot qualify own rank".into());
        }
        row.pass("own-rank");
        progress.stage("unsupported_refusal", Some(request(u32::MAX, 0, 1, 0)));
        refused(&mut client, request(u32::MAX, 0, 1, 0), progress).await?;
        progress.stage("unsupported_refusal", Some(request(11, 0, 1, 0)));
        refused(&mut client, request(11, 0, 1, 0), progress).await?;
        row.pass("unsupported-refusal");
        // Valid wire token outside the store's positive signed-bigint domain.
        // This is an invalid query, never a minted runtime actor/item handle.
        progress.stage("unknown_snapshot_refusal", Some(request(1, 0, 1, u64::MAX)));
        refused(&mut client, request(1, 0, 1, u64::MAX), progress).await?;
        row.pass("unknown-snapshot-refusal");
        // A successful read after both refusals proves the connection survived.
        progress.stage("retained_read", None);
        for (before, _) in &saved {
            if observed_query(
                &mut client,
                request(before.category, 0, 1, before.snapshot_id),
                progress,
            )
            .await?
                != *before
            {
                return Err("refused query changed retained ranking state".into());
            }
        }
        progress.stage("initial_logout", None);
        logout(&mut client).await?;
        Ok(())
    }
    .await;
    if result.is_err() {
        progress.report("journey_failed");
        progress.stage("failed_journey_logout", None);
        let _ = logout(&mut client).await;
        return result;
    }
    // Preserve the saved pages across a genuinely new admission, not recovery.
    drop(client);
    progress.stage("relog_admission", None);
    let mut fresh = connect_world(clients, 0xa1).await?;
    let result: TestResult = async {
        if fresh.join_snapshot().game_session_id == initial_session {
            return Err("ranking relog reused the old game session".into());
        }
        for (before, all) in &saved {
            let (after, rows) = list(
                &mut fresh,
                before.category,
                0,
                before.snapshot_id,
                progress,
                "relog_list",
            )
            .await?;
            if after != *before || rows != *all {
                return Err("fresh admission lost retained snapshot, rows or own rank".into());
            }
        }
        row.pass("relog-snapshot");
        Ok(())
    }
    .await;
    if result.is_err() {
        progress.report("relog_failed");
    }
    progress.stage("final_logout", None);
    let cleanup = logout(&mut fresh).await;
    result?;
    cleanup?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn ranking_deadline_retains_the_unfinished_query_and_completed_progress() {
        let mut progress = RankingProgress::new();
        let result = tokio::time::timeout(Duration::from_secs(180), async {
            progress.begin_query(request(1, 0, 1, 0));
            progress.finish_query(true);
            progress.begin_query(request(2, 3, 4, 7));
            std::future::pending::<()>().await;
        })
        .await;
        assert!(result.is_err());
        assert_eq!(
            (progress.issued, progress.finished, progress.validated),
            (2, 1, 1)
        );
        assert!(progress.in_flight);
        assert_eq!(
            progress
                .query
                .map(|query| (query.category, query.vocation, query.page)),
            Some((2, 3, 4)),
        );
        assert_eq!(progress.query_started.elapsed(), Duration::from_secs(180));
    }

    #[test]
    fn verdict_requires_every_action_and_rejects_duplicate_or_unknown_actions() {
        let mut row = Row::new(&["first", "second"]);
        row.pass("first");
        assert!(!row.complete());
        row.pass("second");
        assert!(row.complete());
        row.pass("first");
        assert!(!row.complete());
        let mut unknown = Row::new(&["first"]);
        unknown.pass("other");
        unknown.pass("first");
        assert!(!unknown.complete());
        assert!(!Row::new(&[]).complete());
    }

    #[test]
    fn pages_must_echo_the_pinned_cut_and_respect_the_terminal_bound() {
        let query = request(1, 0, 21, 7);
        let mut page = wire::HighscoresResult {
            world: None,
            own_page: None,
            category: 1,
            snapshot_id: 7,
            computed_at: 1,
            page: 21,
            has_more: false,
            rows: Vec::new(),
            own_row: None,
        };
        assert!(validate_page(&page, query).is_ok());
        page.snapshot_id = 8;
        assert!(validate_page(&page, query).is_err());
        page.snapshot_id = 7;
        page.has_more = true;
        assert!(validate_page(&page, query).is_err());
    }

    #[test]
    fn ranking_validation_rejects_misordered_and_oversized_server_pages() {
        let entry = wire::HighscoreRow {
            rank: 1,
            name: "Alice".into(),
            vocation: "knight".into(),
            level: 8,
            value: 100,
        };
        let mut page = wire::HighscoresResult {
            world: None,
            own_page: None,
            category: 1,
            snapshot_id: 7,
            computed_at: 1,
            page: 1,
            has_more: false,
            rows: vec![
                entry.clone(),
                wire::HighscoreRow {
                    rank: 2,
                    name: "Bob".into(),
                    value: 90,
                    ..entry.clone()
                },
            ],
            own_row: None,
        };
        let query = request(1, 0, 1, 7);
        assert!(validate_page(&page, query).is_ok());
        page.rows.swap(0, 1);
        assert!(validate_page(&page, query).is_err());
        page.rows = vec![entry; wire::MAX_HIGHSCORES_PAGE_ROWS + 1];
        assert!(validate_page(&page, query).is_err());
    }
}

// Keep the S11 journey above unchanged. The packet entry point isolates each
// additional row's admission, deadline and verdict from the preceding row.
pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    run(clients).await;
    let mut row = ClockRow::default();
    let mut client = None;
    let result = tokio::time::timeout(
        qualification_deadline(3_750),
        clock_journey(&mut row, &mut client, clients),
    )
    .await;
    let mut result = match result {
        Ok(result) => result,
        Err(_) => Err("world-clock deadline: full cycle not observed within wall-time budget".into()),
    };
    // The session lives outside the timed future: even cancellation attempts an
    // ordinary logout, and cleanup failure prevents a full-row PASS.
    if let Some(client) = client.as_mut() {
        match tokio::time::timeout(Duration::from_secs(25), logout(client)).await {
            Ok(Ok(())) => row.pass("final-logout"),
            cleanup => {
                evidence(&format!("E2EF7_GAP W20a final-logout {cleanup:?}"));
                if result.is_ok() {
                    result = Err(format!("world-clock final logout did not succeed: {cleanup:?}").into());
                }
            }
        }
    }
    drop(client);
    row.finish(result);
    additional_rows(clients).await;
    continuation_rows(clients).await;
    forge_row(clients).await;
}

const CLOCK_ACTIONS: &[&str] = &[
    "snapshot",
    "clock-delta",
    "day",
    "night",
    "sunrise",
    "sunset",
    "midnight-wrap",
    "full-cycle",
    "fresh-admission",
    "final-logout",
];

#[derive(Default)]
struct ClockRow {
    passed: BTreeSet<&'static str>,
    previous: Option<(oteryn_protocol_oteryn::world_clock::WorldClock, Duration)>,
    elapsed_minutes: u32,
    first_observation: Option<Duration>,
    started: Option<std::time::Instant>,
}

impl ClockRow {
    // A cycle can observe the same class of frame repeatedly; only the first
    // proven occurrence emits its bounded check. No action is caller-defined.
    fn pass(&mut self, action: &'static str) {
        if CLOCK_ACTIONS.contains(&action) && self.passed.insert(action) {
            evidence(&format!(
                "E2EF7_CHECK W20a {action} PASS server-frames-validated"
            ));
        }
    }

    fn complete(&self) -> bool {
        CLOCK_ACTIONS
            .iter()
            .all(|action| self.passed.contains(action))
    }

    // Pure validation is shared by epoch attribution and the single committed
    // observation. A rejected candidate cannot add minutes or emit evidence.
    fn validate_observation(
        &self,
        clock: oteryn_protocol_oteryn::world_clock::WorldClock,
        elapsed: Duration,
    ) -> TestResult<u32> {
        let minute = u32::from(clock.tibian_minute);
        if minute >= 1_440
            || clock.light_color != 215
            || clock.light_level != expected_light(minute)
        {
            return Err(format!("world-clock minute/light/color mismatch: {clock:?}").into());
        }
        let mut elapsed_minutes = self.elapsed_minutes;
        if let Some((previous, at)) = self.previous {
            let elapsed_ms = elapsed
                .checked_sub(at)
                .ok_or("world-clock observation time moved backwards")?
                .as_millis();
            let forward = (minute + 1_440 - u32::from(previous.tibian_minute)) % 1_440;
            // Samples are consumed every second, with a new admission at most
            // every 90 seconds. Allow 10s for transport/snapshot pacing, never a
            // reversed, frozen or accelerated clock. Long gaps fail closed.
            let expected_ms = u128::from(forward) * 2_500;
            if elapsed_ms > 180_000 || expected_ms.abs_diff(elapsed_ms) > 10_000 {
                return Err(format!(
                    "world-clock rate/discontinuity: previous={previous:?} current={clock:?} gap_ms={elapsed_ms}"
                )
                .into());
            }
            elapsed_minutes = elapsed_minutes
                .checked_add(forward)
                .ok_or("world-clock cumulative minutes overflow")?;
            let since_first = elapsed
                .checked_sub(
                    self.first_observation
                        .ok_or("world-clock first sample absent")?,
                )
                .ok_or("world-clock cumulative time moved backwards")?;
            if (u128::from(elapsed_minutes) * 2_500).abs_diff(since_first.as_millis()) > 10_000
            {
                return Err("world-clock cumulative rate drift exceeds 10s".into());
            }
        }
        Ok(elapsed_minutes)
    }

    fn observe(
        &mut self,
        clock: oteryn_protocol_oteryn::world_clock::WorldClock,
        elapsed: Duration,
    ) -> TestResult {
        let elapsed_minutes = self.validate_observation(clock, elapsed)?;
        let minute = u32::from(clock.tibian_minute);
        if let Some((previous, _)) = self.previous
            && elapsed_minutes > self.elapsed_minutes
        {
            if minute < u32::from(previous.tibian_minute) {
                self.pass("midnight-wrap");
            }
            // Both endpoints must be in the ramp. Day/night plateaux or
            // reconnecting from night to day cannot stand in for a ramp.
            let prior = u32::from(previous.tibian_minute);
            if (360..480).contains(&prior)
                && (360..480).contains(&minute)
                && clock.light_level > previous.light_level
            {
                self.pass("sunrise");
            }
            if (1_050..1_170).contains(&prior)
                && (1_050..1_170).contains(&minute)
                && clock.light_level < previous.light_level
            {
                self.pass("sunset");
            }
        }
        self.elapsed_minutes = elapsed_minutes;
        if (480..1_050).contains(&minute) && clock.light_level == 250 {
            self.pass("day");
        }
        if !(360..1_170).contains(&minute) && clock.light_level == 40 {
            self.pass("night");
        }
        if self.elapsed_minutes >= 1_440 {
            self.pass("full-cycle");
        }
        self.first_observation.get_or_insert(elapsed);
        self.previous = Some((clock, elapsed));
        Ok(())
    }

    fn finish(self, result: TestResult) {
        for action in CLOCK_ACTIONS {
            if !self.passed.contains(action) {
                evidence(&format!("E2EF7_GAP W20a {action} unproven"));
            }
        }
        if let Err(error) = &result {
            evidence(&format!("E2EF7_GAP W20a {error}"));
        }
        evidence(&format!(
            "e2e_all scenario=W20a.full status={} seconds={:.3}",
            if result.is_ok() && self.complete() {
                "PASS"
            } else {
                "FAIL"
            },
            self.started
                .map_or(0.0, |start| start.elapsed().as_secs_f64()),
        ));
    }
}

// Independent contract oracle: do not call the server's world_clock::light_level
// to validate that same implementation. These are WORLDINT0-RL-16/17 values.
fn expected_light(minute: u32) -> u8 {
    match minute {
        360..480 => u8::try_from((40 + 7 * ((minute - 360) / 4 + 1)).min(250)).unwrap_or(250),
        480..1_050 => 250,
        1_050..1_170 => u8::try_from(
            250_u32
                .saturating_sub(7 * ((minute - 1_050) / 4 + 1))
                .max(40),
        )
        .unwrap_or(40),
        _ => 40,
    }
}

// WORLDINT0-RL-18: a delta is owed at an hour boundary or a light change,
// subject to the ten-second pacing limit. Walk at most one hour of the
// independent light oracle; a known full hour always crosses a boundary.
// Only explicitly injected time is used, so receive latency cannot invent a
// change. A silent advance remains outstanding until a real frame arrives.
fn clock_delta_expected_after_advance(
    clock: oteryn_protocol_oteryn::world_clock::WorldClock,
    advance: Duration,
) -> bool {
    if advance < Duration::from_secs(10) {
        return false;
    }
    (1..=60_u32)
        .take_while(|minutes| u128::from(*minutes) * 2_500 <= advance.as_millis())
        .any(|minutes| {
            let minute = (u32::from(clock.tibian_minute) + minutes) % 1_440;
            minute.is_multiple_of(60) || expected_light(minute) != clock.light_level
        })
}

fn admitted_clock(
    client: &mut DevClientSession,
) -> TestResult<oteryn_protocol_oteryn::world_clock::WorldClock> {
    use oteryn_protocol_oteryn::world_clock::{
        CAPABILITY_WORLD_LIGHT_V1, SNAPSHOT_TYPE_WORLD_CLOCK_V1, STATE_DOMAIN_WORLD_CLOCK,
        decode_world_clock,
    };
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_WORLD_LIGHT_V1)
    {
        return Err("world-clock admission did not select capability 19".into());
    }
    let snapshot = client
        .session_mut()
        .gated_snapshot(STATE_DOMAIN_WORLD_CLOCK)
        .ok_or("world-clock snapshot domain 18 absent")?;
    if snapshot.snapshot_type != SNAPSHOT_TYPE_WORLD_CLOCK_V1 {
        return Err("world-clock snapshot type mismatch".into());
    }
    decode_world_clock(&snapshot.payload)
        .map_err(|error| format!("world-clock snapshot: {error:?}").into())
}

// Advances are explicit and occur only while W20 observes time. Other packet
// rows, immediate cooldown refusals and offline restoration retain real time.
fn qualification_deadline(real_seconds: u64) -> Duration {
    Duration::from_secs(if std::env::var_os("OTERYN_QUALIFICATION_CLOCK_FILE").is_some() {
        120
    } else {
        real_seconds
    })
}

fn qualification_step() -> Duration {
    Duration::from_secs(75)
}

fn qualification_offset() -> Duration {
    #[cfg(feature = "qualification-clock")]
    {
        Duration::from_millis(crate::world_clock::qualification_offset_ms())
    }
    #[cfg(not(feature = "qualification-clock"))]
    {
        Duration::ZERO
    }
}

fn advance_qualification_time(step: Duration) -> TestResult {
    if let Some(path) = std::env::var_os("OTERYN_QUALIFICATION_CLOCK_FILE") {
        if !cfg!(feature = "qualification-clock") {
            return Err("qualification clock requires the disposable build feature".into());
        }
        let path = std::path::PathBuf::from(path);
        let next = qualification_offset().checked_add(step).ok_or("clock overflow")?;
        let temporary = path.with_extension("next");
        fs::write(&temporary, next.as_millis().to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&temporary, fs::Permissions::from_mode(0o644))?;
        }
        fs::rename(temporary, path)?;
    }
    Ok(())
}

fn qualification_elapsed(started: std::time::Instant, offset: Duration) -> Duration {
    started.elapsed().saturating_add(qualification_offset().saturating_sub(offset))
}

// A 90s session can contain its admission epoch and at most two 75s
// advances. Retain silent epochs: a naturally emitted frame can still be in
// the TLS queue when the next explicit advance is applied.
struct ClockEpochs {
    offsets: [Duration; 3],
    len: usize,
    observed: usize,
    origin: Duration,
}

impl ClockEpochs {
    fn new(origin: Duration, admitted_offset: Duration) -> Self {
        Self {
            offsets: [admitted_offset; 3],
            len: 1,
            observed: 0,
            origin,
        }
    }

    fn record(&mut self, offset: Duration) -> TestResult {
        let current = self.offsets[self.len - 1];
        if offset < current {
            return Err("world-clock injected offset moved backwards".into());
        }
        if offset != current {
            if self.len == self.offsets.len() {
                return Err("world-clock session exceeded its bounded offset epochs".into());
            }
            self.offsets[self.len] = offset;
            self.len += 1;
        }
        Ok(())
    }

    fn observe(
        &mut self,
        row: &mut ClockRow,
        clock: oteryn_protocol_oteryn::world_clock::WorldClock,
        received_elapsed: Duration,
    ) -> TestResult {
        let mut matched = None;
        for index in self.observed..self.len {
            // The independent input is real receive time plus a recorded
            // injection, never time reconstructed from the received minute.
            let injected = self.offsets[index]
                .checked_sub(self.origin)
                .ok_or("world-clock offset precedes journey origin")?;
            let elapsed = received_elapsed
                .checked_add(injected)
                .ok_or("world-clock observation time overflow")?;
            if row.validate_observation(clock, elapsed).is_ok() {
                if matched.is_some() {
                    return Err("world-clock frame matches multiple offset epochs".into());
                }
                matched = Some((index, elapsed));
            }
        }
        let (index, elapsed) = matched.ok_or_else(|| {
            format!(
                "world-clock frame matches no retained offset epoch: clock={clock:?} received_ms={} observed_offset_ms={} current_offset_ms={}",
                received_elapsed.as_millis(),
                self.offsets[self.observed].as_millis(),
                self.offsets[self.len - 1].as_millis(),
            )
        })?;
        row.observe(clock, elapsed)?;
        self.observed = index;
        Ok(())
    }

    fn delta_owed(&self, clock: oteryn_protocol_oteryn::world_clock::WorldClock) -> bool {
        clock_delta_expected_after_advance(
            clock,
            self.offsets[self.len - 1] - self.offsets[self.observed],
        )
    }
}

async fn service_clock_advance(
    client: &mut DevClientSession,
    row: &mut ClockRow,
    epochs: &mut ClockEpochs,
    started: std::time::Instant,
    accelerated: bool,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        client.service_liveness(Duration::from_secs(1)).await?;
        // Use actual elapsed time when the receive window completes. Transport
        // and queueing jitter remain subject to the existing 10s rate tolerance.
        let received_elapsed = started.elapsed();
        for event in client.take_events() {
            if let oteryn_dev_client::SessionEvent::WorldClock(delta) = event {
                epochs.observe(row, delta.value, received_elapsed)?;
                row.pass("clock-delta");
            }
        }
        let (clock, _) = row.previous.ok_or("world-clock admission sample absent")?;
        // An old queued frame does not acknowledge the latest advance. It can,
        // however, prove that its remaining 75s stays on the same hour/light
        // plateau. Recompute after the entire batch without renewing the bound.
        if !accelerated || !epochs.delta_owed(clock) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("world-clock advance did not produce a server delta".into());
        }
    }
}

async fn clock_journey(
    row: &mut ClockRow,
    connection: &mut Option<DevClientSession>,
    clients: &SeamClients<'_>,
) -> TestResult {
    let started = std::time::Instant::now();
    row.started = Some(started);
    let offset = qualification_offset();
    let mut previous_session = None;
    // The real-clock mode takes one hour; the disposable build advances facts.
    // Short sessions also avoid idle-removal: each logout is real and each new
    // admission proves the World clock continues independently of the session.
    for tag in 0xb0..=0xdf {
        *connection = Some(connect_world(clients, tag).await?);
        let client = connection.as_mut().ok_or("world-clock admission absent")?;
        let current_session = client.join_snapshot().game_session_id;
        if previous_session == Some(current_session) {
            return Err("world-clock fresh admission reused the old game session".into());
        }
        let clock = admitted_clock(client)?;
        let admitted_offset = qualification_offset();
        let mut epochs = ClockEpochs::new(offset, admitted_offset);
        row.observe(
            clock,
            started.elapsed() + admitted_offset.saturating_sub(offset),
        )?;
        row.pass("snapshot");
        if previous_session.is_some() {
            row.pass("fresh-admission");
        }
        let reconnect_at = qualification_elapsed(started, offset) + Duration::from_secs(90);
        loop {
            advance_qualification_time(qualification_step())?;
            epochs.record(qualification_offset())?;
            service_clock_advance(
                client,
                row,
                &mut epochs,
                started,
                std::env::var_os("OTERYN_QUALIFICATION_CLOCK_FILE").is_some(),
            )
            .await?;
            // Final logout is checked by run_packet even on timeout or error.
            if CLOCK_ACTIONS
                .iter()
                .filter(|action| **action != "final-logout")
                .all(|action| row.passed.contains(action))
            {
                return Ok(());
            }
            if qualification_elapsed(started, offset) >= reconnect_at {
                break;
            }
        }
        logout(client).await?;
        previous_session = Some(current_session);
        *connection = None;
    }
    Err("world-clock admission budget exhausted before a complete natural cycle".into())
}

#[cfg(test)]
mod clock_tests {
    use super::*;
    use oteryn_protocol_oteryn::world_clock::WorldClock;

    fn reading(minute: u16) -> WorldClock {
        WorldClock {
            tibian_minute: minute,
            light_level: expected_light(u32::from(minute)),
            light_color: 215,
        }
    }

    #[test]
    fn accelerated_clock_keeps_all_natural_transition_assertions() -> TestResult {
        let source = crate::world_clock::WorldClockSource::new(0);
        for initial_minute in [0_u64, 359, 479, 1_049, 1_439] {
            let mut row = ClockRow::default();
            for step in 0..=50_u64 {
                let elapsed = qualification_step() * u32::try_from(step)?;
                let now = initial_minute * 2_500 + u64::try_from(elapsed.as_millis())?;
                row.observe(source.reading(now).ok_or("clock reading")?, elapsed)?;
            }
            for action in ["day", "night", "sunrise", "sunset", "midnight-wrap", "full-cycle"] {
                assert!(row.passed.contains(action), "missing {action}");
            }
            assert!(!row.complete(), "transport and relog are still required");
        }
        Ok(())
    }

    #[test]
    fn accelerated_clock_allows_silent_steps_without_losing_required_deltas() -> TestResult {
        let source = crate::world_clock::WorldClockSource::new(0);
        for initial_minute in [0_u64, 359, 479, 1_049, 1_439] {
            let initial_ms = initial_minute * 2_500;
            let mut clock = source.reading(initial_ms).ok_or("clock reading")?;
            let mut pacer = crate::world_clock::WorldClockDeltaPacer::default();
            pacer.reset(clock, initial_ms);
            let mut row = ClockRow::default();
            row.observe(clock, Duration::ZERO)?;
            let mut observed_offset = Duration::ZERO;
            let mut saw_silent_step = false;
            let mut saw_delta = false;
            for step in 1..=50_u32 {
                let elapsed = qualification_step() * step;
                let expects_delta =
                    clock_delta_expected_after_advance(clock, elapsed - observed_offset);
                let delta = pacer.due(&source, initial_ms + u64::try_from(elapsed.as_millis())?);
                assert_eq!(
                    expects_delta,
                    delta.is_some(),
                    "initial={initial_minute} step={step}"
                );
                if let Some(delta) = delta {
                    row.observe(delta, elapsed)?;
                    clock = delta;
                    observed_offset = elapsed;
                    saw_delta = true;
                } else {
                    saw_silent_step = true;
                }
            }
            assert!(saw_silent_step && saw_delta);
            for action in [
                "day",
                "night",
                "sunrise",
                "sunset",
                "midnight-wrap",
                "full-cycle",
            ] {
                assert!(row.passed.contains(action), "missing {action}");
            }
            assert!(!row.complete(), "transport and relog are still required");
        }
        Ok(())
    }

    #[test]
    fn accelerated_personal_light_keeps_decay_and_expiry_assertions() -> TestResult {
        use crate::foundation::condition::{
            ApplicationFacts, ConditionDefinition, ConditionSourceKind, ConditionStore,
        };
        use oteryn_gameplay_domain::domain::condition::ConditionValues;
        use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
        let root = GameplayDecisionRoot::from_bytes([1; 32]);
        for (level, seconds) in [(6_u8, 370_u64), (8, 695), (8, 1_990)] {
            let duration = Duration::from_secs(seconds);
            let definition = ConditionDefinition::new("qualification.light", 1,
                ConditionValues::SpellLight { level, color: 215, duration_ms: u32::try_from(seconds * 1_000)? })
                .ok_or("light definition")?;
            let mut store = ConditionStore::<String>::new();
            store.apply(&definition, None, ConditionSourceKind::SelfUse, &[], &ApplicationFacts {
                now: 0, base_speed: 220, mana_shield_capacity: 0,
                target_reentry_protected: false, source_reentry_protected: false,
                target_is_player: true, decision_root: &root,
                occurrence: DecisionOccurrenceId::from_bytes([2; 16]),
            }).map_err(|error| format!("light application: {error:?}"))?;
            let step = duration / u32::from(level);
            let mut saw_decay = false;
            for index in 0..=u32::from(level) + 1 {
                let elapsed = step * index;
                let light = store.light_at(u64::try_from(elapsed.as_micros())?)
                    .map(|(level, color)| spatial::EntityLight { level, color: color.unwrap_or(0) });
                light_sample(light, level, elapsed, duration)?;
                saw_decay |= light.is_some_and(|light| light.level > 0 && light.level < level);
                if elapsed >= duration {
                    assert!(light.is_none());
                }
            }
            assert!(saw_decay);
        }
        Ok(())
    }

    #[test]
    fn clock_oracle_checks_ramp_edges_and_plateaux() {
        for (minute, level) in [
            (0, 40),
            (359, 40),
            (360, 47),
            (363, 47),
            (364, 54),
            (476, 250),
            (479, 250),
            (480, 250),
            (1_049, 250),
            (1_050, 243),
            (1_166, 40),
            (1_170, 40),
            (1_439, 40),
        ] {
            assert_eq!(expected_light(minute), level);
        }
    }

    #[test]
    fn snapshots_alone_and_partial_cycle_never_qualify_clock_row() -> TestResult {
        let mut row = ClockRow::default();
        for minute in 0..=1_440_u32 {
            row.observe(
                reading(u16::try_from(minute % 1_440)?),
                Duration::from_millis(u64::from(minute) * 2_500),
            )?;
        }
        assert!(!row.complete());
        for action in [
            "day",
            "night",
            "sunrise",
            "sunset",
            "midnight-wrap",
            "full-cycle",
        ] {
            assert!(row.passed.contains(action));
        }
        for action in ["snapshot", "clock-delta", "fresh-admission", "final-logout"] {
            row.pass(action);
        }
        assert!(row.complete());
        row.passed.remove("full-cycle");
        assert!(!row.complete());
        Ok(())
    }

    #[test]
    fn small_per_sample_drift_cannot_accumulate_into_a_fake_day() -> TestResult {
        let mut row = ClockRow::default();
        row.observe(reading(0), Duration::ZERO)?;
        for minute in 1..=4_u16 {
            row.observe(reading(minute), Duration::from_secs(u64::from(minute) * 5))?;
        }
        // Each individual sample is within the transport tolerance, but the
        // cumulative clock is already 12.5 seconds behind real observation time.
        assert!(row.observe(reading(5), Duration::from_secs(25)).is_err());
        assert!(!row.passed.contains("full-cycle"));
        Ok(())
    }

    #[test]
    fn clock_rejects_wrong_light_color_rate_and_backward_time() -> TestResult {
        for bad in [
            WorldClock {
                light_level: 250,
                ..reading(0)
            },
            WorldClock {
                light_color: 0,
                ..reading(0)
            },
            WorldClock {
                tibian_minute: 1_440,
                ..reading(0)
            },
        ] {
            assert!(ClockRow::default().observe(bad, Duration::ZERO).is_err());
        }
        for (minute, elapsed) in [(600, 2_500), (99, 2_500), (100, 60_000)] {
            let mut row = ClockRow::default();
            row.observe(reading(100), Duration::ZERO)?;
            assert!(
                row.observe(reading(minute), Duration::from_millis(elapsed))
                    .is_err()
            );
        }
        let mut row = ClockRow::default();
        row.observe(reading(100), Duration::from_secs(10))?;
        assert!(row.observe(reading(101), Duration::ZERO).is_err());
        Ok(())
    }
}

#[cfg(test)]
#[path = "full_e2ef7_clock_epochs_tests.rs"]
mod clock_epoch_tests;

// Continuation rows keep their own action sets and cleanup outside timed futures.
use native::world_spatial_entities as spatial;
use oteryn_protocol_oteryn::{self as native, actor_spell as casts, party as parties};

struct JourneyRow {
    id: &'static str,
    actions: Row,
}
impl JourneyRow {
    fn new(id: &'static str, required: &'static [&'static str]) -> Self {
        Self {
            id,
            actions: Row::new(required),
        }
    }
    fn pass(&mut self, action: &'static str) {
        if !self.actions.required.contains(&action) || !self.actions.passed.insert(action) {
            self.actions.invalid_action = true;
        } else {
            evidence(&format!(
                "E2EF7_CHECK {} {action} PASS server-frames-validated",
                self.id
            ));
        }
    }
    fn finish(self, result: &TestResult) {
        for action in self.actions.required {
            if !self.actions.passed.contains(action) {
                evidence(&format!("E2EF7_GAP {} {action} unproven", self.id));
            }
        }
        if let Err(error) = result {
            evidence(&format!("E2EF7_GAP {} {error}", self.id));
        }
        evidence(&format!(
            "e2e_all scenario={}.full status={} seconds={:.3}",
            self.id,
            if result.is_ok() && self.actions.complete() {
                "PASS"
            } else {
                "FAIL"
            },
            self.actions.started.elapsed().as_secs_f64()
        ));
    }
}
const LIGHT_ACTIONS: &[&str] = &[
    "admission",
    "light-cast-mana",
    "light-cooldown-refusal",
    "light-relog",
    "light-decay-expiry",
    "great-cast-mana",
    "great-cooldown-refusal",
    "great-relog",
    "great-decay-expiry",
    "ultimate-cast-mana",
    "ultimate-cooldown-refusal",
    "ultimate-relog",
    "ultimate-decay-expiry",
    "fresh-admission-mana",
    "final-logout",
];
const PARTY_GATE_ACTIONS: &[&str] = &[
    "two-admissions",
    "solo-refusal",
    "invite-accept",
    "near-cast",
    "cooldown-refusal",
    "range-36-cast",
    "range-37-refusal",
    "return-cast",
    "leave-refusal",
    "fresh-admission-mana",
    "final-logout",
];
const PARTY_COST_ACTIONS: &[&str] = &[
    "two-admissions",
    "solo-zero-charge",
    "two-member-scaled-charge",
    "cooldown-zero-charge",
    "range-36-same-charge",
    "range-37-zero-charge",
    "return-same-charge",
    "leave-zero-charge",
    "fresh-admission-mana",
    "final-logout",
];

async fn additional_rows(clients: &SeamClients<'_>) {
    let mut row = JourneyRow::new("W20b", LIGHT_ACTIONS);
    let mut light = None;
    let result = tokio::time::timeout(
        qualification_deadline(3_300),
        light_journey(&mut row, &mut light, clients),
    )
    .await;
    let mut result = result.unwrap_or_else(|_| {
        Err("personal-light journey exceeded wall-time deadline".into())
    });
    if let Some(client) = light.as_mut() {
        match tokio::time::timeout(Duration::from_secs(25), client.logout()).await {
            Ok(Ok(())) => row.pass("final-logout"),
            other => {
                result = Err(
                    format!("personal-light cleanup logout: {other:?};journey={result:?}").into(),
                )
            }
        }
    }
    drop(light);
    row.finish(&result);

    let mut gate = JourneyRow::new("S13a", PARTY_GATE_ACTIONS);
    let mut cost = JourneyRow::new("S13b", PARTY_COST_ACTIONS);
    let mut pair = [None, None];
    let world = qualification_world(clients);
    let mut result = match &world {
        Ok(world) => tokio::time::timeout(
            Duration::from_secs(240),
            party_spell_journey(&mut gate, &mut cost, &mut pair, clients, world),
        )
        .await
        .unwrap_or_else(|_| Err("party-spell journey deadline; remaining actions unproven".into())),
        Err(error) => Err(format!("party compiled-world load: {error}").into()),
    };
    let mut cleaned = 0;
    for client in pair.iter_mut().rev().flatten() {
        // A row failure may occur after accepting an invitation. Remove membership
        // before logging out; this prevents poisoning later packet phases.
        let cleanup = async {
            let state = party_request(client, None).await?;
            if !state.members.is_empty() {
                party_request(client, Some(parties::PartyIntent::Leave)).await?;
            }
            let restore: TestResult = async {
                if let Ok(world) = &world {
                    let start = tile(client.join_snapshot().world_spatial.actor_position)?;
                    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
                        at == start
                    })
                    .ok_or("party cleanup original-position route unavailable")?;
                    walk(client, world, &path).await?;
                }
                Ok(())
            }
            .await;
            // Always attempt release even if restoring a route fails.
            let released = logout(client).await;
            restore.and(released)
        };
        match tokio::time::timeout(Duration::from_secs(60), cleanup).await {
            Ok(Ok(())) => cleaned += 1,
            other => {
                result = Err(format!("party-spell cleanup: {other:?};journey={result:?}").into())
            }
        }
    }
    if cleaned == 2 {
        gate.pass("final-logout");
        cost.pass("final-logout");
    }
    gate.finish(&result);
    cost.finish(&result);
}

fn qualification_world(clients: &SeamClients<'_>) -> TestResult<BundleWorld> {
    let bytes = fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )
}
fn qualification_spell_index(
    book: &crate::spell::SpellBook,
    words: &str,
) -> TestResult<std::num::NonZeroU32> {
    let spell = book
        .spoken(words)
        .ok_or_else(|| format!("compiled spell missing: {words}"))?
        .spell;
    (1..=book.source_len())
        .filter_map(|n| u32::try_from(n).ok().and_then(std::num::NonZeroU32::new))
        .find(|n| book.indexed(*n).is_some_and(|entry| entry.key == spell.key))
        .ok_or_else(|| format!("compiled spell has no index: {words}").into())
}

// These ordinary TLS observers include the boot-required capability closure and
// decode its map/item domains as well as their journey-specific observations.
const LIGHT_SUPPORTED_CAPABILITIES: &[u32] = &[4, 6, 11, 13, 15, 18, 19, 21, 30, 64];
const PVP_SUPPORTED_CAPABILITIES: &[u32] =
    &[4, 6, 9, 11, 13, 15, 17, 18, 21, 30, 42, 64, 87, 134, 199];
const FORGE_SUPPORTED_CAPABILITIES: &[u32] = &[
    4, 6, 9, 11, 12, 13, 14, 15, 17, 18, 21, 25, 30, 64, 104, 152, 199,
];

type LightStream = tokio_rustls::client::TlsStream<TcpStream>;
struct LightClient {
    stream: LightStream,
    generation: u64,
    session: native::GameSessionId,
    sequence: u64,
    command: u64,
    probe: u64,
    selected: Vec<u32>,
    revisions: BTreeMap<u32, u64>,
    own: [u8; spatial::ENTITY_IDENTITY_BYTES],
    light: Option<spatial::EntityLight>,
    vitals: casts::ActorVitals,
    position: ActorPosition,
    content: [u8; 32],
    map_header: native::world_map::MapViewHeader,
    own_updates: u64,
    refusal: Option<casts::SpellCastRefusalDetail>,
}
async fn light_read<S: tokio::io::AsyncRead + Unpin>(stream: &mut S) -> TestResult<Vec<u8>> {
    tokio::time::timeout(Duration::from_secs(20), async {
        let size = stream.read_u32().await?;
        if size == 0 || size > native::MAX_WIRE_FRAME_BYTES {
            return Err("personal-light frame length outside native bound".into());
        }
        let mut bytes = vec![0; usize::try_from(size)?];
        stream.read_exact(&mut bytes).await?;
        Ok(bytes)
    })
    .await
    .map_err(|_| "personal-light frame deadline")?
}
async fn light_write(stream: &mut LightStream, bytes: &[u8]) -> TestResult {
    tokio::time::timeout(Duration::from_secs(20), async {
        stream.write_all(&framed(bytes)).await?;
        stream.flush().await?;
        Ok(())
    })
    .await
    .map_err(|_| "personal-light write deadline")?
}
fn post_frame(
    bytes: &[u8],
    generation: u64,
    kind: MessageType,
) -> TestResult<native::WireEnvelopeView<'_>> {
    let frame = native::decode_wire_envelope(bytes)?;
    frame.validate(native::Direction::ServerToClient, true)?;
    if frame.connection_generation() != generation || frame.message_type() != kind {
        return Err(format!(
            "personal-light unexpected native frame: {:?}",
            frame.message_type()
        )
        .into());
    }
    Ok(frame)
}

async fn raw_snapshot<S: tokio::io::AsyncRead + Unpin>(
    stream: &mut S,
    generation: u64,
    bytes: &[u8],
) -> TestResult<(native::SnapshotBeginFields, Vec<u8>)> {
    let frame = post_frame(bytes, generation, MessageType::SnapshotBegin)?;
    let begin = native::decode_snapshot_begin(frame.payload())?;
    let mut body = Vec::new();
    for index in 0..begin.chunk_count {
        let bytes = light_read(stream).await?;
        let frame = post_frame(&bytes, generation, MessageType::SnapshotChunk)?;
        let (seen, chunk) = native::decode_snapshot_chunk_framing(frame.payload())?;
        if native::decode_snapshot_id(frame.payload())? != begin.snapshot_id || seen != index {
            return Err("native observer snapshot chunk correlation mismatch".into());
        }
        if u64::try_from(body.len())?
            .checked_add(u64::try_from(chunk.len())?)
            .is_none_or(|total| total > begin.total_encoded_bytes)
        {
            return Err("native observer snapshot exceeds declared length".into());
        }
        body.extend_from_slice(chunk);
    }
    let bytes = light_read(stream).await?;
    let frame = post_frame(&bytes, generation, MessageType::SnapshotCommit)?;
    if native::decode_snapshot_id(frame.payload())? != begin.snapshot_id
        || u64::try_from(body.len())? != begin.total_encoded_bytes
    {
        return Err("native observer snapshot commit mismatch".into());
    }
    Ok((begin, body))
}

fn raw_map_snapshot(
    domains: &[native::DomainSnapshot<'_>],
    content: [u8; 32],
) -> TestResult<native::world_map::MapViewHeader> {
    let domain = domains
        .iter()
        .find(|domain| domain.domain_id == 17)
        .ok_or("native observer map snapshot absent")?;
    if domain.snapshot_type != 1 || domains.iter().any(|domain| domain.domain_id == 2) {
        return Err("native observer map snapshot type/domain mismatch".into());
    }
    let map = native::world_map::decode_world_map_snapshot(domain.payload)
        .map_err(|error| format!("native observer map snapshot: {error:?}"))?;
    if map.header.content_generation != content {
        return Err("native observer map content generation changed".into());
    }
    Ok(map.header)
}

fn raw_map_delta(
    header: &native::world_map::MapViewHeader,
    payload: &[u8],
) -> TestResult<native::world_map::MapViewHeader> {
    native::world_map::decode_world_map_delta(header, payload)
        .map(|delta| delta.header)
        .map_err(|error| format!("native observer map delta: {error:?}").into())
}

async fn raw_map_replacement<S: tokio::io::AsyncRead + Unpin>(
    stream: &mut S,
    generation: u64,
    bytes: &[u8],
    sequence: u64,
    revision: u64,
    content: [u8; 32],
) -> TestResult<(u64, u64, native::world_map::MapViewHeader)> {
    let frame = post_frame(bytes, generation, MessageType::SnapshotBegin)?;
    let begin = native::decode_snapshot_begin(frame.payload())?;
    if Some(begin.target_server_sequence) != sequence.checked_add(1) {
        return Err("native observer replacement snapshot sequence gap".into());
    }
    let (_, body) = raw_snapshot(stream, generation, bytes).await?;
    let domains = native::decode_snapshot_body(&body)?;
    if domains.len() != 1 || domains[0].domain_id != 17 || domains[0].revision <= revision {
        return Err("native observer replacement map domain/revision mismatch".into());
    }
    let header = raw_map_snapshot(&domains, content)?;
    Ok((begin.target_server_sequence, domains[0].revision, header))
}

impl LightClient {
    async fn connect(clients: &SeamClients<'_>, tag: u8) -> TestResult<Self> {
        let generation = platform_generation(clients.descriptor, &clients.accounts[1]).await?;
        let token = sign_grant(
            &Grant {
                signing: clients.signing,
                key_id: clients.key_id,
                account_id: &clients.accounts[1],
                character_id: clients.characters[1],
                world_id: clients.world,
                channel_id: clients.channel,
                security_generation: generation,
                scope_generation: clients.scope_generation,
                nonce: [tag; 32],
                attempt: v7(tag, 0x05),
            },
            now_seconds()?,
        );
        let connector = connector(clients.certificate, &EXACT)?;
        let mut stream = tokio::time::timeout(Duration::from_secs(20), async {
            let tcp = TcpStream::connect(clients.address).await?;
            Ok::<_, Box<dyn std::error::Error>>(
                connector
                    .connect(ServerName::try_from("localhost")?, tcp)
                    .await?,
            )
        })
        .await
        .map_err(|_| "personal-light TLS deadline")??;
        if stream.get_ref().1.alpn_protocol() != Some(native::ALPN_OTERYN_GAME_V1.as_bytes()) {
            return Err("personal-light ALPN mismatch".into());
        }
        let supported = LIGHT_SUPPORTED_CAPABILITIES;
        light_write(
            &mut stream,
            &native::encode_client_bootstrap(&native::ClientBootstrapValue {
                schema_revision: 1,
                character_id: CharacterId::decode(&clients.characters[1])?,
                admission_material: token.as_bytes(),
                client_build_id: "oteryn-e2e-light",
                supported_capabilities: supported,
            })?,
        )
        .await?;
        let bytes = light_read(&mut stream).await?;
        let frame = native::decode_wire_envelope(&bytes)?;
        frame.validate(native::Direction::ServerToClient, false)?;
        if frame.message_type() != MessageType::ServerAccepted {
            return Err("personal-light ordinary admission refused".into());
        }
        let accepted = native::decode_server_accepted(frame.payload())?;
        if accepted.world_id != WorldId::decode(&clients.world)?
            || accepted.channel_id != ChannelId::decode(&clients.channel)?
            || accepted.schema_revision != 1
            || supported
                .iter()
                .any(|c| !accepted.selected_capabilities.contains(c))
            || accepted
                .selected_capabilities
                .iter()
                .any(|c| !supported.contains(c))
        {
            return Err("personal-light admission world/channel/capabilities mismatch".into());
        }
        let bytes = light_read(&mut stream).await?;
        let (begin, body) =
            raw_snapshot(&mut stream, accepted.connection_generation, &bytes).await?;
        if begin.target_server_sequence < accepted.current_server_sequence {
            return Err("personal-light snapshot commit mismatch".into());
        }
        let domains = native::decode_snapshot_body(&body)?;
        // PRESENTATION_V1 remains unoffered. The light oracle uses spatial light,
        // vitals and cast dispositions, so unselected presentation domains are refused.
        if domains
            .iter()
            .any(|domain| matches!(domain.domain_id, 19 | 20))
        {
            return Err("personal-light unselected presentation snapshot".into());
        }
        for id in [3, 9, 11, 17, 18] {
            let row = domains
                .iter()
                .find(|d| d.domain_id == id)
                .ok_or_else(|| format!("personal-light required snapshot domain {id} absent"))?;
            if row.snapshot_type != 1 {
                return Err(
                    format!("personal-light unregistered snapshot type in domain {id}").into(),
                );
            }
            match id {
                9 => {
                    native::item_view::decode_character_inventory(row.payload)
                        .map_err(|e| format!("personal-light inventory snapshot: {e:?}"))?;
                }
                11 => {
                    native::item_view::decode_open_container(row.payload)
                        .map_err(|e| format!("personal-light container snapshot: {e:?}"))?;
                }
                18 => {
                    native::world_clock::decode_world_clock(row.payload)
                        .map_err(|e| format!("personal-light clock snapshot: {e:?}"))?;
                }
                _ => {} // Vitals and map are decoded below, before constructing the client.
            }
        }
        let domain = domains
            .iter()
            .find(|d| d.domain_id == 1)
            .ok_or("personal-light spatial snapshot absent")?;
        let (view, lights) = spatial::decode_world_spatial_snapshot_view_lit(
            &accepted.selected_capabilities,
            domain.snapshot_type,
            domain.payload,
        )
        .map_err(|e| format!("personal-light spatial snapshot: {e:?}"))?;
        let spatial::WorldSpatialSnapshotView::Entities(view) = view else {
            return Err("personal-light entity snapshot absent".into());
        };
        let map_header = raw_map_snapshot(&domains, view.content_generation)?;
        let vitals = domains
            .iter()
            .find(|d| d.domain_id == 3)
            .ok_or("personal-light vitals snapshot absent")?;
        let vitals = casts::decode_actor_vitals(vitals.payload)
            .map_err(|e| format!("personal-light vitals snapshot: {e:?}"))?;
        Ok(Self {
            stream,
            generation: accepted.connection_generation,
            session: accepted.game_session_id,
            sequence: begin.target_server_sequence,
            command: accepted.next_command_id,
            probe: 0,
            selected: accepted.selected_capabilities,
            revisions: domains.iter().map(|d| (d.domain_id, d.revision)).collect(),
            own: view.own_identity,
            light: lights.get(&view.own_identity).copied(),
            vitals,
            position: view.actor_position,
            content: view.content_generation,
            map_header,
            own_updates: 0,
            refusal: None,
        })
    }
    async fn receive(&mut self) -> TestResult<Option<(u64, CommandStatus, Vec<u8>)>> {
        let bytes = light_read(&mut self.stream).await?;
        let frame = native::decode_wire_envelope(&bytes)?;
        frame.validate(native::Direction::ServerToClient, true)?;
        if frame.connection_generation() != self.generation {
            return Err("personal-light stale frame generation".into());
        }
        if frame.message_type() == MessageType::SnapshotBegin {
            let (sequence, revision, header) = raw_map_replacement(
                &mut self.stream,
                self.generation,
                &bytes,
                self.sequence,
                self.revisions
                    .get(&17)
                    .copied()
                    .ok_or("map revision absent")?,
                self.content,
            )
            .await?;
            self.sequence = sequence;
            self.revisions.insert(17, revision);
            self.map_header = header;
            return Ok(None);
        }
        match frame.message_type() {
            MessageType::LivenessProbe => {
                let probe = native::decode_liveness_probe(frame.payload())?;
                if probe <= self.probe {
                    return Err("personal-light nonmonotonic probe".into());
                }
                self.probe = probe;
                light_write(
                    &mut self.stream,
                    &native::encode_liveness_ack(self.generation, probe, self.sequence)?,
                )
                .await?;
            }
            MessageType::StateDelta | MessageType::CommandResult => {
                if Some(frame.server_sequence()) != self.sequence.checked_add(1) {
                    return Err("personal-light server sequence gap".into());
                }
                self.sequence = frame.server_sequence();
                if frame.message_type() == MessageType::CommandResult {
                    let reply = native::decode_command_result(frame.payload())?;
                    return Ok(Some((
                        reply.command_id,
                        reply.status,
                        reply.payload.to_vec(),
                    )));
                }
                let delta = native::decode_state_delta(frame.payload())?;
                if delta.domain_id != 1 && delta.delta_type != 1 {
                    return Err("personal-light unregistered native delta type".into());
                }
                if delta.base_revision != self.revisions.get(&delta.domain_id).copied().unwrap_or(0)
                {
                    return Err("personal-light domain revision gap".into());
                }
                self.revisions.insert(delta.domain_id, delta.new_revision);
                match delta.domain_id {
                    1 => {
                        let (view, lights) = spatial::decode_world_spatial_delta_view_lit(
                            &self.selected,
                            delta.delta_type,
                            delta.payload,
                        )
                        .map_err(|e| format!("personal-light spatial delta: {e:?}"))?;
                        let spatial::WorldSpatialDeltaView::Entities(view) = view else {
                            return Err("personal-light entity delta absent".into());
                        };
                        if view.content_generation != self.content {
                            return Err("personal-light content generation changed".into());
                        }
                        self.position = view.actor_position;
                        if view
                            .enter
                            .iter()
                            .chain(&view.update)
                            .any(|e| e.entity.identity == self.own)
                        {
                            self.light = lights.get(&self.own).copied();
                            self.own_updates += 1;
                        }
                    }
                    17 => {
                        self.map_header = raw_map_delta(&self.map_header, delta.payload)?;
                    }
                    9 => {
                        native::item_view::decode_character_inventory(delta.payload)
                            .map_err(|e| format!("personal-light inventory delta: {e:?}"))?;
                    }
                    11 => {
                        native::item_view::decode_open_container(delta.payload)
                            .map_err(|e| format!("personal-light container delta: {e:?}"))?;
                    }
                    3 => {
                        if delta.delta_type != 1 {
                            return Err("personal-light vitals delta type mismatch".into());
                        }
                        self.vitals = casts::decode_actor_vitals(delta.payload)
                            .map_err(|e| format!("personal-light vitals delta: {e:?}"))?;
                    }
                    18 => {
                        native::world_clock::decode_world_clock(delta.payload)
                            .map_err(|e| format!("personal-light clock: {e:?}"))?;
                    }
                    _ => {
                        return Err(format!(
                            "personal-light unselected delta domain {}",
                            delta.domain_id
                        )
                        .into());
                    }
                }
            }
            _ => {
                return Err(
                    format!("personal-light unexpected frame {:?}", frame.message_type()).into(),
                );
            }
        }
        Ok(None)
    }
    async fn command(&mut self, kind: u32, payload: &[u8]) -> TestResult<(CommandStatus, Vec<u8>)> {
        let id = self.command;
        self.command = self
            .command
            .checked_add(1)
            .ok_or("personal-light command id overflow")?;
        light_write(
            &mut self.stream,
            &native::encode_client_command(
                self.generation,
                &native::ClientCommandValue {
                    command_id: id,
                    command_type: kind,
                    payload,
                },
            )?,
        )
        .await?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(25);
        loop {
            if tokio::time::Instant::now() >= deadline {
                return Err("personal-light command deadline".into());
            }
            if let Some((seen, status, reply)) = self.receive().await? {
                if seen != id {
                    return Err("personal-light command correlation mismatch".into());
                }
                return Ok((status, reply));
            }
        }
    }
    async fn cast(
        &mut self,
        index: std::num::NonZeroU32,
    ) -> TestResult<casts::SpellCastDisposition> {
        let payload = casts::encode_spell_cast_intent(&casts::SpellCastIntent {
            spell: index,
            target: casts::SpellTarget::None,
            aim_at_target: false,
        });
        let (status, payload) = self
            .command(casts::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT, &payload)
            .await?;
        if status != CommandStatus::Accepted {
            return Err(format!("personal-light spell command refused: {status:?}").into());
        }
        let (disposition, detail) = casts::decode_spell_cast_result_with_detail(&payload)
            .map_err(|e| format!("personal-light cast codec: {e:?}"))?;
        self.refusal = detail;
        Ok(disposition)
    }
    async fn turn(&mut self) -> TestResult {
        let before = self.position;
        let mut payload = Vec::new();
        bytes_field(&mut payload, 3, &encode_step_intent(StepDirection::North));
        let (status, reply) = self
            .command(
                native::target_move::COMMAND_TYPE_TARGET_MOVE_INTENT,
                &payload,
            )
            .await?;
        if status != CommandStatus::Accepted || reply != [8, 1] || before != self.position {
            return Err("personal-light ordinary activity turn refused or moved actor".into());
        }
        Ok(())
    }
    async fn logout(&mut self) -> TestResult {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        let mut attempts = 0;
        loop {
            attempts += 1;
            let command_id = self.command;
            let (status, payload) = self
                .command(native::logout::COMMAND_TYPE_LOGOUT_INTENT, &[])
                .await
                .map_err(|error| format!("personal-light logout attempt {attempts} command_id={command_id} failed: {error}"))?;
            let result = native::logout::decode_logout_result(&payload)
                .map_err(|error| format!("personal-light logout codec: {error:?}; command_id={command_id}; status={status:?}; attempts={attempts}"))?;
            if status == CommandStatus::Accepted
                && result == native::logout::LogoutResult::Accepted
            {
                return Ok(());
            }
            let busy_timed_out = result == native::logout::LogoutResult::Busy
                && tokio::time::Instant::now() >= deadline;
            if status != CommandStatus::Rejected
                || result != native::logout::LogoutResult::Busy
                || busy_timed_out
            {
                return Err(format!("personal-light logout refused: command_id={command_id}; server_sequence={}; status={status:?}; result={result:?}; attempts={attempts}; busy_timed_out={busy_timed_out}", self.sequence).into());
            }
            // Only a fully correlated Busy permits a new command. The next
            // command services probes; never cancel receive() inside a frame.
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }
}

fn light_sample(
    light: Option<spatial::EntityLight>,
    level: u8,
    elapsed: Duration,
    duration: Duration,
) -> TestResult {
    if level == 0 || duration.as_millis() < u128::from(level) {
        return Err("personal-light oracle requires a positive level and decay interval".into());
    }
    let interval_ms = duration.as_millis() / u128::from(level);
    let decayed = elapsed.as_millis() / interval_ms;
    let expected = level.saturating_sub(u8::try_from(decayed).unwrap_or(u8::MAX));
    let observed = light.map_or(0, |l| l.level);
    // Updates are paced to 1s and reads can straddle the cast's owner tick.
    // Only neighboring decay levels are allowed; expiry is checked separately.
    if observed.abs_diff(expected) > 1 || light.is_some_and(|l| l.color != 215 || l.level > level) {
        return Err(format!("personal-light decay/color mismatch elapsed={elapsed:?} expected={expected} observed={light:?}").into());
    }
    Ok(())
}
async fn light_journey(
    row: &mut JourneyRow,
    connection: &mut Option<LightClient>,
    clients: &SeamClients<'_>,
) -> TestResult {
    let book = native_spell_book()?;
    *connection = Some(LightClient::connect(clients, 0xe0).await?);
    row.pass("admission");
    for (words, level, mana, seconds, paid, refused, relog, tag, expired) in [
        (
            "utevo lux",
            6,
            20,
            370,
            "light-cast-mana",
            "light-cooldown-refusal",
            "light-relog",
            0xe6,
            "light-decay-expiry",
        ),
        (
            "utevo gran lux",
            8,
            60,
            695,
            "great-cast-mana",
            "great-cooldown-refusal",
            "great-relog",
            0xe7,
            "great-decay-expiry",
        ),
        (
            "utevo vis lux",
            8,
            140,
            1_990,
            "ultimate-cast-mana",
            "ultimate-cooldown-refusal",
            "ultimate-relog",
            0xe8,
            "ultimate-decay-expiry",
        ),
    ] {
        let client = connection.as_mut().ok_or("personal-light session absent")?;
        if client.light.is_some() {
            return Err("personal-light preexisting light prevents isolated cast assertion".into());
        }
        let index = qualification_spell_index(book, words)?;
        let before = client.vitals;
        let own_updates = client.own_updates;
        let started = std::time::Instant::now();
        let offset = qualification_offset();
        let cast = client.cast(index).await?;
        if cast != casts::SpellCastDisposition::Cast {
            let spell = book
                .indexed(index)
                .ok_or("personal-light compiled index disappeared")?;
            return Err(format!("prerequisite-blocked:words={words:?};observed={cast:?};detail={:?};required-level={};required-vocations={:?};required-premium={};required-mana={mana};served-mana={};no-player-progression-or-vocation-route-completed", client.refusal, spell.level, spell.vocations, spell.premium, before.mana).into());
        }
        // Cast results precede their vitals and visibility deltas. Wait for both.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
        while client.own_updates == own_updates
            || client.light != Some(spatial::EntityLight { level, color: 215 })
            || before.mana.checked_sub(client.vitals.mana) != Some(mana)
        {
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("{words}: cast lacked exact mana/light frames; before={before:?} after={:?} light={:?}", client.vitals, client.light).into());
            }
            if client.receive().await?.is_some() {
                return Err("personal-light unsolicited command result during observation".into());
            }
        }
        row.pass(paid);
        let after = client.vitals;
        let retry = client.cast(index).await?;
        // A subsequent ordinary command is a server-sequenced barrier: a
        // refused cast cannot hide a queued vitals charge after its result.
        client.turn().await?;
        if retry != casts::SpellCastDisposition::CoolingDown || client.vitals != after {
            return Err(
                format!("{words}: immediate retry did not refuse freely: {retry:?}").into(),
            );
        }
        row.pass(refused);
        // LIGHT is a durable finite condition. Fresh admission must retain
        // both its current light and the paid resources, with offline time
        // excluded from the owner's frozen remaining-duration contract.
        let previous_session = client.session;
        let previous_light = client.light;
        let previous_position = client.position;
        client.logout().await?;
        let offline_started = std::time::Instant::now();
        *connection = None;
        let fresh = LightClient::connect(clients, tag).await?;
        let restored = fresh.session != previous_session
            && fresh.light == previous_light
            && fresh.vitals == after
            && fresh.position == previous_position;
        *connection = Some(fresh);
        let offline = offline_started.elapsed();
        if !restored {
            return Err(format!(
                "{words}: fresh admission did not retain active light and paid mana"
            )
            .into());
        }
        row.pass(relog);
        let client = connection
            .as_mut()
            .ok_or("personal-light restored session absent")?;
        let duration = Duration::from_secs(seconds);
        let mut last_level = level;
        let mut saw_decay = false;
        let mut activity = std::time::Instant::now();
        while qualification_elapsed(started, offset).saturating_sub(offline) <= duration + Duration::from_secs(8) {
            advance_qualification_time(duration / u32::from(level))?;
            let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
            loop {
                let accelerated = std::env::var_os("OTERYN_QUALIFICATION_CLOCK_FILE").is_some();
                let result = if accelerated {
                    tokio::time::timeout_at(deadline, client.receive()).await??
                } else {
                    client.receive().await?
                };
                if result.is_some() {
                    return Err("personal-light unsolicited command result during observation".into());
                }
                if !accelerated
                    || client.light.map_or(0, |light| light.level) < last_level
                {
                    break;
                }
            }
            let elapsed = qualification_elapsed(started, offset).saturating_sub(offline);
            light_sample(client.light, level, elapsed, duration)?;
            let current = client.light.map_or(0, |l| l.level);
            if current > last_level {
                return Err("personal-light increased without a new cast".into());
            }
            saw_decay |= current < level && current > 0;
            last_level = current;
            if client.light.is_none() {
                if elapsed + Duration::from_secs(2) < duration || !saw_decay {
                    return Err(
                        "personal-light expired early or no intermediate decay observed".into(),
                    );
                }
                row.pass(expired);
                break;
            }
            if activity.elapsed() >= Duration::from_secs(60) {
                client.turn().await?;
                activity = std::time::Instant::now();
            }
        }
        if client.light.is_some() {
            return Err(format!("{words}: light did not expire naturally").into());
        }
    }
    let client = connection.as_mut().ok_or("personal-light session absent")?;
    let session = client.session;
    client.logout().await?;
    let saved = client.vitals;
    *connection = None;
    let fresh = LightClient::connect(clients, 0xe1).await?;
    let matches = fresh.session != session && fresh.light.is_none() && fresh.vitals == saved;
    *connection = Some(fresh);
    if !matches {
        return Err(
            "personal-light fresh admission lost paid mana/expiry or reused session".into(),
        );
    }
    row.pass("fresh-admission-mana");
    Ok(())
}

async fn party_request(
    client: &mut DevClientSession,
    intent: Option<parties::PartyIntent>,
) -> TestResult<parties::PartyResult> {
    let (kind, payload) = match intent {
        Some(intent) => (
            parties::COMMAND_TYPE_PARTY_INTENT,
            parties::encode_party_intent(intent).map_err(|e| format!("party intent: {e:?}"))?,
        ),
        None => (parties::COMMAND_TYPE_PARTY_QUERY, Vec::new()),
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        let reply = client
            .session_mut()
            .gated_command(parties::CAPABILITY_PARTY_V1, kind, &payload)
            .await?;
        if reply.status != CommandStatus::Accepted {
            return Err(format!("party command refused: {:?}", reply.status).into());
        }
        let state = parties::decode_party_result(&reply.payload)
            .map_err(|e| format!("party result: {e:?}"))?;
        if state.disposition == parties::PartyDisposition::Ok {
            return Ok(state);
        }
        // Retry only the read, never a mutating invitation or acceptance.
        if intent.is_some()
            || state.disposition != parties::PartyDisposition::Unavailable
            || tokio::time::Instant::now() >= deadline
        {
            return Err(format!("party operation refused: {:?}", state.disposition).into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}
fn same_two_member_roster(a: &parties::PartyResult, b: &parties::PartyResult) -> bool {
    let members = |view: &parties::PartyResult| {
        view.members
            .iter()
            .map(|m| (m.name.clone(), m.leader))
            .collect::<BTreeSet<_>>()
    };
    a.members.len() == 2
        && b.members.len() == 2
        && a.members.iter().filter(|m| m.leader).count() == 1
        && members(a).len() == 2
        && members(a) == members(b)
}
fn player_ref(client: &DevClientSession) -> TestResult<spatial::EntityRef> {
    let entities = client.world_entities().ok_or("party own entities absent")?;
    entities
        .iter()
        .find(|e| e.entity.identity == *entities.own_identity())
        .map(|e| e.entity)
        .ok_or_else(|| "party own actor absent".into())
}
fn pair_mut(
    pair: &mut [Option<DevClientSession>; 2],
) -> TestResult<(&mut DevClientSession, &mut DevClientSession)> {
    let [caster, peer] = pair;
    Ok((
        caster.as_mut().ok_or("party caster admission absent")?,
        peer.as_mut().ok_or("party peer admission absent")?,
    ))
}
async fn party_walk(
    caster: &mut DevClientSession,
    peer: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for direction in path {
        // Both sockets remain serviced on every real movement step.
        let (moved, serviced) = join(
            walk(caster, world, &[*direction]),
            peer.service_liveness(Duration::from_millis(10)),
        )
        .await;
        moved?;
        serviced?;
        caster.take_events();
        peer.take_events();
    }
    Ok(())
}
async fn party_cooldown(
    caster: &mut DevClientSession,
    peer: &mut DevClientSession,
    spell: &crate::spell::SpellDefinition,
) -> TestResult {
    let micros = spell
        .groups
        .iter()
        .map(|g| g.cooldown_micros)
        .chain([spell.cooldown_micros])
        .max()
        .unwrap_or(0);
    let until =
        tokio::time::Instant::now() + Duration::from_micros(micros) + Duration::from_millis(100);
    while tokio::time::Instant::now() < until {
        let (a, b) = join(
            caster.service_liveness(Duration::from_millis(25)),
            peer.service_liveness(Duration::from_millis(25)),
        )
        .await;
        a?;
        b?;
        caster.take_events();
        peer.take_events();
    }
    Ok(())
}
// Independent exact two-member oracle: ceil(base * 0.9 * 2). The solo
// case must refuse without spending; it cannot be used as a successful X=1 cast.
fn two_member_charge(base: u32) -> TestResult<u32> {
    u32::try_from((u64::from(base) * 18).div_ceil(10)).map_err(Into::into)
}
fn verify_party_refusal(
    outcome: &oteryn_dev_client::CastOutcome,
    before: casts::ActorVitals,
    after: casts::ActorVitals,
    expected: casts::SpellCastDisposition,
) -> TestResult {
    if outcome.status != CommandStatus::Accepted
        || outcome.disposition != expected
        || outcome.actor_vitals_delta.is_some()
        || before != after
    {
        return Err(format!("party refusal mismatch: expected={expected:?} observed={outcome:?} before={before:?} after={after:?}").into());
    }
    Ok(())
}
async fn party_paid_cast(
    caster: &mut DevClientSession,
    index: std::num::NonZeroU32,
    charge: u32,
) -> TestResult {
    let before = caster
        .actor_vitals()
        .copied()
        .ok_or("party caster vitals absent")?;
    let outcome = caster
        .cast_spell(index, casts::SpellTarget::None, false)
        .await?;
    let after = caster
        .actor_vitals()
        .copied()
        .ok_or("party cast lost vitals")?;
    verify_party_payment(&outcome, before, after, charge)
}
fn verify_party_payment(
    outcome: &oteryn_dev_client::CastOutcome,
    before: casts::ActorVitals,
    after: casts::ActorVitals,
    charge: u32,
) -> TestResult {
    if outcome.status != CommandStatus::Accepted
        || outcome.disposition != casts::SpellCastDisposition::Cast
        || outcome.actor_vitals_delta.as_ref().map(|d| d.value) != Some(after)
        || before.mana.checked_sub(after.mana) != Some(charge)
        || before.health != after.health
        || before.soul != after.soul
    {
        return Err(format!("prerequisite-or-cast-blocked:party observed={outcome:?};required-scaled-mana={charge};before={before:?};after={after:?};need-player-earned-level-32-class-and-premium;no-fixture-provisioning").into());
    }
    Ok(())
}
async fn party_free_refusal(
    caster: &mut DevClientSession,
    index: std::num::NonZeroU32,
    expected: casts::SpellCastDisposition,
) -> TestResult {
    let before = caster
        .actor_vitals()
        .copied()
        .ok_or("party refusal vitals absent")?;
    let outcome = caster
        .cast_spell(index, casts::SpellTarget::None, false)
        .await?;
    // Drain the refused cast's preceding server work before asserting that
    // no late vitals charge arrived. This is a real query, never a sleep.
    party_request(caster, None).await?;
    verify_party_refusal(
        &outcome,
        before,
        caster
            .actor_vitals()
            .copied()
            .ok_or("party refusal lost vitals")?,
        expected,
    )
}
async fn party_spell_journey(
    gate: &mut JourneyRow,
    cost: &mut JourneyRow,
    pair: &mut [Option<DevClientSession>; 2],
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    let book = native_spell_book()?;
    pair[0] = Some(connect_character(clients, 0xe2, 1).await?);
    pair[1] = Some(connect_character(clients, 0xe3, 0).await?);
    let (caster, peer) = pair_mut(pair)?;
    if caster.join_snapshot().game_session_id == peer.join_snapshot().game_session_id
        || player_ref(caster)? == player_ref(peer)?
    {
        return Err("party needs two distinct real admitted characters".into());
    }
    for client in [&mut *caster, &mut *peer] {
        if !party_request(client, None).await?.members.is_empty() {
            return Err("party preexisting membership prevents isolated gate test".into());
        }
    }
    gate.pass("two-admissions");
    cost.pass("two-admissions");
    // The retained server-owned ranking row carries the durable vocation.
    // Read it rather than guessing class from a generic refusal or profile env.
    let observed = query(caster, request(1, 0, 1, 0)).await?;
    let own = observed
        .own_row
        .ok_or("party prerequisite: server ranking has no own build row")?;
    let words = match oteryn_gameplay_progression::wheel::base_vocation(&own.vocation) {
        Some("sorcerer") => "utori mas sio",
        Some("druid") => "utura mas sio",
        Some("paladin") => "utamo mas sio",
        Some("knight") => "utito mas sio",
        Some("monk") => "utevo mas sio",
        _ => return Err(format!("party-prerequisite-blocked:served-vocation={};served-level={};required=class,level-32,premium;ordinary-admission-has-no-party-spell-vocation", own.vocation, own.level).into()),
    };
    let index = qualification_spell_index(book, words)?;
    let spell = book.indexed(index).ok_or("party compiled index absent")?;
    let before = caster
        .actor_vitals()
        .copied()
        .ok_or("party initial vitals absent")?;
    let solo = caster.cast_spell(index, casts::SpellTarget::None, false).await
        .map_err(|e| format!("party solo cast: {e};served-vocation={};served-level={};required-level={},premium={}", own.vocation, own.level, spell.level, spell.premium))?;
    verify_party_refusal(&solo, before, caster.actor_vitals().copied().ok_or("party solo lost vitals")?, casts::SpellCastDisposition::Rejected)
        .map_err(|e| format!("party prerequisite-or-gate-blocked: {e};served-vocation={};served-level={};required-level={},premium={}", own.vocation, own.level, spell.level, spell.premium))?;
    let crate::spell::Execution::PartyBuff(buff) = &spell.execution else {
        return Err("party spell lacks party execution".into());
    };
    let (charge, scaled) = match buff.mana {
        crate::spell::party::PartyMana::Scaled {
            base,
            numerator: 9,
            denominator: 10,
        } => (two_member_charge(base)?, true),
        crate::spell::party::PartyMana::Fixed(value) => (value, false),
        _ => return Err("party mana profile differs from independent oracle".into()),
    };
    let caster_start = tile(caster.world_spatial().actor_position)?;
    let peer_start = tile(peer.world_spatial().actor_position)?;
    let path = route(world, caster_start, |at| {
        at != peer_start
            && at.floor == peer_start.floor
            && at.x.abs_diff(peer_start.x).max(at.y.abs_diff(peer_start.y)) <= 1
    })
    .ok_or("party real-player adjacent route unavailable")?;
    party_walk(caster, peer, world, &path).await?;
    // Drain interest updates before presenting the peer's actual runtime handle.
    caster.service_liveness(Duration::from_millis(100)).await?;
    peer.service_liveness(Duration::from_millis(100)).await?;
    let peer_ref = player_ref(peer)?;
    if !caster
        .world_entities()
        .is_some_and(|view| view.iter().any(|e| e.entity == peer_ref))
    {
        return Err("party peer absent from caster's real visibility frames".into());
    }
    party_request(caster, Some(parties::PartyIntent::Invite(peer_ref))).await?;
    let invitations = party_request(peer, None).await?;
    if invitations.invitations.len() != 1 {
        return Err("party invitation not observed by peer".into());
    }
    party_request(
        peer,
        Some(parties::PartyIntent::Accept(
            invitations.invitations[0].handle,
        )),
    )
    .await?;
    // Mutations acknowledge the operation; each observer must query its own
    // roster. Opaque handles are connection-local and cannot be compared.
    let joined = party_request(peer, None).await?;
    let roster = party_request(caster, None).await?;
    if !same_two_member_roster(&roster, &joined) {
        return Err("party accepted roster does not contain the same two members".into());
    }
    gate.pass("invite-accept");
    // Qualify the earlier no-party refusal only after the same spell and
    // credentials succeed with the observed party, paying exactly its fee.
    party_paid_cast(caster, index, charge).await?;
    gate.pass("solo-refusal");
    cost.pass("solo-zero-charge");
    gate.pass("near-cast");
    if scaled {
        cost.pass("two-member-scaled-charge");
    }
    party_free_refusal(caster, index, casts::SpellCastDisposition::CoolingDown).await?;
    gate.pass("cooldown-refusal");
    cost.pass("cooldown-zero-charge");
    party_cooldown(caster, peer, spell).await?;
    let origin = tile(peer.world_spatial().actor_position)?;
    // Range admission is 3D Chebyshev <=36, independent of the rendered area.
    // Use the player's actual path and demand the endpoint frame exactly.
    for (distance, action, fee_action) in [
        (36, "range-36-cast", "range-36-same-charge"),
        (37, "range-37-refusal", "range-37-zero-charge"),
    ] {
        let from = tile(caster.world_spatial().actor_position)?;
        let path = route(world, from, |at| {
            at != origin
                && at.floor == origin.floor
                && at.x.abs_diff(origin.x).max(at.y.abs_diff(origin.y)) == distance
        })
        .ok_or_else(|| {
            format!(
                "party range-{distance} real walkable endpoint unavailable within town route budget"
            )
        })?;
        party_walk(caster, peer, world, &path).await?;
        let at = tile(caster.world_spatial().actor_position)?;
        if at.floor != origin.floor
            || at.x.abs_diff(origin.x).max(at.y.abs_diff(origin.y)) != distance
        {
            return Err("party range endpoint frame mismatch".into());
        }
        if distance == 36 {
            party_paid_cast(caster, index, charge).await?;
        } else {
            party_free_refusal(caster, index, casts::SpellCastDisposition::Rejected).await?;
        }
        gate.pass(action);
        cost.pass(fee_action);
        party_cooldown(caster, peer, spell).await?;
    }
    let path = route(world, tile(caster.world_spatial().actor_position)?, |at| {
        at != origin
            && at.floor == origin.floor
            && at.x.abs_diff(origin.x).max(at.y.abs_diff(origin.y)) <= 1
    })
    .ok_or("party return to member unavailable")?;
    party_walk(caster, peer, world, &path).await?;
    party_paid_cast(caster, index, charge).await?;
    gate.pass("return-cast");
    cost.pass("return-same-charge");
    party_cooldown(caster, peer, spell).await?;
    party_request(peer, Some(parties::PartyIntent::Leave)).await?;
    if party_request(caster, None).await?.members.len() > 1 {
        return Err("party leave retained a second member".into());
    }
    party_free_refusal(caster, index, casts::SpellCastDisposition::Rejected).await?;
    gate.pass("leave-refusal");
    cost.pass("leave-zero-charge");
    // Release the peer first: fresh admissions can share their start tile,
    // but movement back into a still occupied start must not be assumed legal.
    let peer_saved = peer
        .actor_vitals()
        .copied()
        .ok_or("party saved peer vitals absent")?;
    let sessions = [
        caster.join_snapshot().game_session_id,
        peer.join_snapshot().game_session_id,
    ];
    if !party_request(caster, None).await?.members.is_empty() {
        party_request(caster, Some(parties::PartyIntent::Leave)).await?;
    }
    logout(peer).await?;
    let path = route(world, tile(caster.world_spatial().actor_position)?, |at| {
        at == caster_start
    })
    .ok_or("party origin restoration unavailable")?;
    walk(caster, world, &path).await?;
    let saved = [
        caster
            .actor_vitals()
            .copied()
            .ok_or("party saved caster vitals absent")?,
        peer_saved,
    ];
    logout(caster).await?;
    *pair = [None, None];
    pair[0] = Some(connect_character(clients, 0xe4, 1).await?);
    pair[1] = Some(connect_character(clients, 0xe5, 0).await?);
    let (caster, peer) = pair_mut(pair)?;
    for (client, previous, expected, position) in [
        (caster, sessions[0], saved[0], caster_start),
        (peer, sessions[1], saved[1], peer_start),
    ] {
        if client.join_snapshot().game_session_id == previous
            || client.actor_vitals().copied() != Some(expected)
            || tile(client.world_spatial().actor_position)? != position
            || !party_request(client, None).await?.members.is_empty()
        {
            return Err(
                "party fresh admission lost paid mana/position or retained membership/session"
                    .into(),
            );
        }
    }
    gate.pass("fresh-admission-mana");
    cost.pass("fresh-admission-mana");
    if !scaled {
        evidence(
            "E2EF7_GAP S13b observed-fixed-cost-Enlighten-Party;scaled-class-prerequisite-unmet",
        );
    }
    Ok(())
}

#[cfg(test)]
mod continuation_tests {
    use super::*;
    #[test]
    fn continuation_actions_fail_closed() {
        for required in [LIGHT_ACTIONS, PARTY_GATE_ACTIONS, PARTY_COST_ACTIONS] {
            let mut row = JourneyRow::new("W20b", required);
            assert!(!row.actions.complete());
            for action in &required[..required.len() - 1] {
                row.pass(action);
            }
            assert!(!row.actions.complete());
            row.pass(required[required.len() - 1]);
            assert!(row.actions.complete());
            row.pass(required[0]);
            assert!(!row.actions.complete());
        }
    }
    #[test]
    fn native_observer_rejects_wrong_frame_generation_and_type() -> TestResult {
        let bytes = envelope(MessageType::LivenessProbe as u64, 7, &[8, 1]);
        assert!(post_frame(&bytes, 7, MessageType::LivenessProbe).is_ok());
        assert!(post_frame(&bytes, 6, MessageType::LivenessProbe).is_err());
        assert!(post_frame(&bytes, 7, MessageType::SnapshotCommit).is_err());
        assert!(post_frame(&[], 7, MessageType::LivenessProbe).is_err());
        let mut pair = [None, None];
        assert!(pair_mut(&mut pair).is_err());
        Ok(())
    }
    #[test]
    fn journey_spell_indices_resolve_against_the_pinned_book() -> TestResult {
        let book = native_spell_book()?;
        for words in [
            "utevo lux",
            "utevo gran lux",
            "utevo vis lux",
            "utori mas sio",
            "utura mas sio",
            "utamo mas sio",
            "utito mas sio",
            "utevo mas sio",
        ] {
            let index = qualification_spell_index(book, words)?;
            assert_eq!(
                book.indexed(index).map(|s| &s.key),
                book.spoken(words).map(|s| &s.spell.key)
            );
        }
        assert!(qualification_spell_index(book, "missing-qualification-spell").is_err());
        Ok(())
    }
    #[test]
    fn personal_light_oracle_rejects_wrong_color_level_and_expiry() {
        let duration = Duration::from_secs(370);
        assert!(light_sample(None, 0, Duration::ZERO, duration).is_err());
        assert!(light_sample(None, 6, Duration::ZERO, Duration::ZERO).is_err());
        assert!(
            light_sample(
                Some(spatial::EntityLight {
                    level: 6,
                    color: 215
                }),
                6,
                Duration::ZERO,
                duration
            )
            .is_ok()
        );
        assert!(
            light_sample(
                Some(spatial::EntityLight {
                    level: 3,
                    color: 215
                }),
                6,
                Duration::from_secs(190),
                duration
            )
            .is_ok()
        );
        assert!(
            light_sample(
                Some(spatial::EntityLight {
                    level: 6,
                    color: 215
                }),
                6,
                Duration::from_secs(190),
                duration
            )
            .is_err()
        );
        assert!(
            light_sample(
                Some(spatial::EntityLight { level: 6, color: 1 }),
                6,
                Duration::ZERO,
                duration
            )
            .is_err()
        );
        assert!(light_sample(None, 6, Duration::ZERO, duration).is_err());
        assert!(light_sample(None, 6, duration, duration).is_ok());
    }
    #[test]
    fn scaled_two_member_cost_rounds_up_independently() -> TestResult {
        assert_eq!(two_member_charge(120)?, 216);
        assert_eq!(two_member_charge(90)?, 162);
        assert_eq!(two_member_charge(60)?, 108);
        assert_eq!(two_member_charge(1)?, 2);
        assert!(two_member_charge(u32::MAX).is_err());
        Ok(())
    }
    #[test]
    fn party_roster_compares_members_without_cross_session_handles() {
        let mut a = parties::PartyResult::refused(parties::PartyDisposition::Ok);
        a.members = vec![
            parties::PartyEntry {
                list: Vec::new(),
                handle: 1,
                name: "Caster".into(),
                leader: true,
                expires_at: 0,
                party_spell_remaining_ms: None,
            },
            parties::PartyEntry {
                list: Vec::new(),
                handle: 2,
                name: "Peer".into(),
                leader: false,
                expires_at: 0,
                party_spell_remaining_ms: None,
            },
        ];
        let mut b = a.clone();
        b.members[0].handle = 100;
        b.members[1].handle = 200;
        b.members.reverse();
        assert!(same_two_member_roster(&a, &b));
        b.members[0].name = "Other".into();
        assert!(!same_two_member_roster(&a, &b));
        b = a.clone();
        b.members[1].leader = true;
        assert!(!same_two_member_roster(&a, &b));
    }
    #[test]
    fn party_payment_requires_exact_native_delta() -> TestResult {
        let before = casts::ActorVitals {
            health: 150,
            max_health: 150,
            mana: 300,
            max_mana: 300,
            soul: 100,
            harmony: 0,
            serene: false,
        };
        let after = casts::ActorVitals { mana: 84, ..before };
        let mut outcome = oteryn_dev_client::CastOutcome {
            command_id: 1,
            status: CommandStatus::Accepted,
            disposition: casts::SpellCastDisposition::Cast,
            result_server_sequence: 1,
            actor_vitals_delta: Some(oteryn_dev_client::AppliedDelta {
                server_sequence: 2,
                base_revision: 1,
                new_revision: 2,
                value: after,
            }),
        };
        verify_party_payment(&outcome, before, after, 216)?;
        assert!(verify_party_payment(&outcome, before, after, 240).is_err());
        outcome.actor_vitals_delta = None;
        assert!(verify_party_payment(&outcome, before, after, 216).is_err());
        Ok(())
    }
    #[test]
    fn party_refusal_requires_observed_disposition_and_unchanged_resources() {
        let before = casts::ActorVitals {
            health: 150,
            max_health: 150,
            mana: 100,
            max_mana: 100,
            soul: 100,
            harmony: 0,
            serene: false,
        };
        let mut outcome = oteryn_dev_client::CastOutcome {
            command_id: 1,
            status: CommandStatus::Accepted,
            disposition: casts::SpellCastDisposition::Rejected,
            result_server_sequence: 1,
            actor_vitals_delta: None,
        };
        assert!(
            verify_party_refusal(
                &outcome,
                before,
                before,
                casts::SpellCastDisposition::Rejected
            )
            .is_ok()
        );
        outcome.disposition = casts::SpellCastDisposition::LevelTooLow;
        assert!(
            verify_party_refusal(
                &outcome,
                before,
                before,
                casts::SpellCastDisposition::Rejected
            )
            .is_err()
        );
        outcome.disposition = casts::SpellCastDisposition::Rejected;
        assert!(
            verify_party_refusal(
                &outcome,
                before,
                casts::ActorVitals { mana: 99, ..before },
                casts::SpellCastDisposition::Rejected
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod raw_observer_tests {
    use super::*;
    use native::world_map::{self, MapViewHeader, WorldMapViewDelta, WorldMapViewSnapshot};

    #[test]
    fn raw_observers_support_the_boot_owner_required_capability_closure() -> TestResult {
        use crate::gameplay_transport::world_required_capabilities::{
            BoundRequiredCapabilities, SessionCapabilityError, offered_for_scope_with,
        };
        // The bundle branch in node_boot/run.sh declares these roots. The owner
        // derives the mandatory floor and transitive dependencies from its registry.
        let required = BoundRequiredCapabilities::for_channel_boot(
            &[11, 18, 21, 30],
            RuntimeScopeRefV1::channel(
                WorldId::decode(&v7(0x71, 1))?,
                ChannelId::decode(&v7(0x72, 1))?,
            ),
            1,
            "node-boot-raw-observers",
            offered_for_scope_with(true, None, false),
        )
        .map_err(|error| format!("raw observer boot requirements: {error:?}"))?;
        // Appearance's owner adds 25/104 at session admission when cosmetics are
        // available. They are intentionally absent from the static boot offer.
        for (supported, dynamic_only) in [
            (LIGHT_SUPPORTED_CAPABILITIES, &[][..]),
            (PVP_SUPPORTED_CAPABILITIES, &[][..]),
            (FORGE_SUPPORTED_CAPABILITIES, &[25, 104][..]),
        ] {
            let selected = required
                .authenticate_support(supported)
                .map_err(|error| format!("raw observer admission: {error:?}"))?
                .selected();
            let expected: Vec<_> = supported
                .iter()
                .copied()
                .filter(|id| !dynamic_only.contains(id))
                .collect();
            assert_eq!(selected.as_slice(), expected.as_slice());
            let deferred: Vec<_> = supported
                .iter()
                .copied()
                .filter(|id| !selected.contains(*id))
                .collect();
            assert_eq!(deferred.as_slice(), dynamic_only);
            for domain in [9, 11, 17] {
                assert!(selected.domain_selected(domain));
            }
            assert!(!selected.domain_selected(2));
            for missing in required.effective() {
                let reduced: Vec<_> = supported
                    .iter()
                    .copied()
                    .filter(|id| id != missing)
                    .collect();
                assert!(matches!(
                    required.authenticate_support(&reduced),
                    Err(SessionCapabilityError::MissingSupport(_))
                ));
            }
        }
        Ok(())
    }

    fn header(floor: i16) -> MapViewHeader {
        MapViewHeader {
            content_generation: [0x11; 32],
            bundle_digest: [0x22; 32],
            reset_epoch: 3,
            origin: ActorPosition {
                x: 100,
                y: 100,
                floor,
            },
            first_visible_floor: world_map::first_visible_floor_start(floor),
        }
    }

    fn snapshot(
        generation: u64,
        id: u64,
        sequence: u64,
        revision: u64,
        header: MapViewHeader,
    ) -> TestResult<Vec<Vec<u8>>> {
        let payload = world_map::encode_world_map_snapshot(&WorldMapViewSnapshot {
            header,
            tiles: Vec::new(),
        })
        .map_err(|error| format!("map snapshot fixture: {error:?}"))?;
        Ok(native::encode_snapshot(
            generation,
            id,
            sequence,
            &[native::DomainSnapshot {
                domain_id: 17,
                revision,
                snapshot_type: 1,
                payload: &payload,
            }],
        )?)
    }

    fn tail(frames: &[Vec<u8>]) -> Vec<u8> {
        frames[1..].iter().flat_map(|frame| framed(frame)).collect()
    }

    fn begin_frame(begin: native::SnapshotBeginFields) -> Vec<u8> {
        let mut payload = Vec::new();
        scalar(&mut payload, 1, begin.snapshot_id);
        scalar(&mut payload, 2, u64::from(begin.chunk_count));
        scalar(&mut payload, 3, begin.total_encoded_bytes);
        scalar(&mut payload, 4, begin.target_server_sequence);
        envelope(MessageType::SnapshotBegin as u64, 7, &payload)
    }

    fn chunk_frame(id: u64, index: u64, data: &[u8]) -> Vec<u8> {
        let mut payload = Vec::new();
        scalar(&mut payload, 1, id);
        scalar(&mut payload, 2, index);
        bytes_field(&mut payload, 3, data);
        envelope(MessageType::SnapshotChunk as u64, 7, &payload)
    }

    async fn replacement(frames: &[Vec<u8>]) -> TestResult<(u64, u64, MapViewHeader)> {
        raw_map_replacement(
            &mut tail(frames).as_slice(),
            7,
            &frames[0],
            10,
            5,
            [0x11; 32],
        )
        .await
    }

    #[tokio::test]
    async fn raw_observer_decodes_initial_map_delta_and_floor_replacement() -> TestResult {
        let initial = snapshot(7, 1, 0, 5, header(-7))?;
        let (begin, body) = raw_snapshot(&mut tail(&initial).as_slice(), 7, &initial[0]).await?;
        assert_eq!(begin.target_server_sequence, 0);
        let domains = native::decode_snapshot_body(&body)?;
        let joined = raw_map_snapshot(&domains, [0x11; 32])?;
        assert_eq!(joined, header(-7));
        assert!(raw_map_snapshot(&domains, [0x33; 32]).is_err());
        let mut moved = joined;
        moved.origin.x += 1;
        let payload = world_map::encode_world_map_delta(
            &joined,
            &WorldMapViewDelta {
                header: moved,
                tiles: Vec::new(),
                cleared: Vec::new(),
            },
        )
        .map_err(|error| format!("map delta fixture: {error:?}"))?;
        assert_eq!(raw_map_delta(&joined, &payload)?, moved);
        let mut stale = joined;
        stale.reset_epoch += 1;
        assert!(raw_map_delta(&stale, &payload).is_err());
        assert!(raw_map_delta(&joined, &[]).is_err());

        // Split a real encoded body inside a field to exercise the same bounded
        // multi-chunk transfer used when a replacement outgrows one wire frame.
        let frames = snapshot(7, 11, 11, 6, header(-8))?;
        let mut begin =
            native::decode_snapshot_begin(native::decode_wire_envelope(&frames[0])?.payload())?;
        let chunk = native::decode_wire_envelope(&frames[1])?;
        let (_, body) = native::decode_snapshot_chunk_framing(chunk.payload())?;
        begin.chunk_count = 2;
        let split = vec![
            begin_frame(begin),
            chunk_frame(11, 0, &body[..7]),
            chunk_frame(11, 1, &body[7..]),
            frames[2].clone(),
        ];
        assert_eq!(replacement(&split).await?, (11, 6, header(-8)));
        let mut reversed = split;
        reversed.swap(1, 2);
        assert!(replacement(&reversed).await.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn raw_observer_rejects_unfenced_or_malformed_map_replacements() -> TestResult {
        let valid = snapshot(7, 11, 11, 6, header(-8))?;
        assert_eq!(replacement(&valid).await?, (11, 6, header(-8)));
        for frames in [
            snapshot(8, 11, 11, 6, header(-8))?,
            snapshot(7, 11, 10, 6, header(-8))?,
            snapshot(7, 11, 12, 6, header(-8))?,
            snapshot(7, 11, 11, 5, header(-8))?,
            snapshot(
                7,
                11,
                11,
                6,
                MapViewHeader {
                    content_generation: [0x33; 32],
                    ..header(-8)
                },
            )?,
        ] {
            assert!(replacement(&frames).await.is_err());
        }
        for index in [1, 2] {
            for foreign in [
                snapshot(8, 11, 11, 6, header(-8))?,
                snapshot(7, 12, 11, 6, header(-8))?,
            ] {
                let mut frames = valid.clone();
                frames[index] = foreign[index].clone();
                assert!(replacement(&frames).await.is_err());
            }
        }
        let begin =
            native::decode_snapshot_begin(native::decode_wire_envelope(&valid[0])?.payload())?;
        for total in [
            begin.total_encoded_bytes - 1,
            begin.total_encoded_bytes + 1,
            native::MAX_SNAPSHOT_ASSEMBLED_BYTES + 1,
        ] {
            let mut frames = valid.clone();
            frames[0] = begin_frame(native::SnapshotBeginFields {
                total_encoded_bytes: total,
                ..begin
            });
            assert!(replacement(&frames).await.is_err());
        }
        for (domain_id, snapshot_type, payload) in
            [(2, 1, &[][..]), (17, 2, &[][..]), (17, 1, &[0xff][..])]
        {
            let frames = native::encode_snapshot(
                7,
                11,
                11,
                &[native::DomainSnapshot {
                    domain_id,
                    revision: 6,
                    snapshot_type,
                    payload,
                }],
            )?;
            assert!(replacement(&frames).await.is_err());
        }
        assert!(replacement(&valid[..2]).await.is_err());
        Ok(())
    }
}

// Pass 2: ordinary-player acquisition and complete PvP/imbuement lifecycles.
use native::item_view::{ItemEntry, ItemHandle};
use native::{attack as attacks, cyclopedia_character as character, imbuement as imbues};

const PVP_ACTIONS: &[&str] = &[
    "earned-levels",
    "two-admissions",
    "expert-modes",
    "secure-refusal",
    "peaceful-refusals",
    "damage-white-skull",
    "logout-lock",
    "pz-lock",
    "lethal-frag",
    "kill-lock",
    "fresh-admission-frag-skull",
    "final-logout",
];
const IMBUE_ACTIONS: &[&str] = &[
    "player-acquisition",
    "shrine-apply",
    "occupied-refusal",
    "worn-effect",
    "eligible-duration",
    "unequipped-effect",
    "fresh-admission-active",
    "shrine-clear",
    "empty-clear-refusal",
    "scroll-create",
    "scroll-use",
    "etch-all",
    "fresh-admission-cleared",
    "final-logout",
];

async fn continuation_rows(clients: &SeamClients<'_>) {
    for (id, actions) in [("S08", PVP_ACTIONS), ("P21", IMBUE_ACTIONS)] {
        let mut row = JourneyRow::new(id, actions);
        let mut connections = [None, None];
        let mut pvp_connections = [None, None];
        let result = async {
            let world = qualification_world(clients)?;
            if id == "S08" {
                pvp_journey(
                    &mut row,
                    &mut connections,
                    &mut pvp_connections,
                    clients,
                    &world,
                )
                .await
            } else {
                imbuement_journey(&mut row, &mut connections[0], clients, &world).await
            }
        };
        let mut result = tokio::time::timeout(Duration::from_secs(1_800), result)
            .await
            .unwrap_or_else(|_| {
                Err(format!("{id}: journey deadline; required observations incomplete").into())
            });
        let admitted = connections.iter().flatten().count();
        let mut released = 0;
        for connection in connections.iter_mut().rev().flatten() {
            let cleanup = async {
                connection.session_mut().attack_target(None).await?;
                release_when_unlocked(connection, Duration::from_secs(920)).await
            };
            match tokio::time::timeout(Duration::from_secs(940), cleanup).await {
                Ok(Ok(())) => released += 1,
                other => result = Err(format!("{id}: cleanup={other:?};journey={result:?}").into()),
            }
        }
        let raw_admitted = pvp_connections.iter().flatten().count();
        let [first, second] = &mut pvp_connections;
        let (first, second) = join(cleanup_pvp(first.as_mut()), cleanup_pvp(second.as_mut())).await;
        let mut raw_released = 0;
        for cleanup in [first, second] {
            match cleanup {
                Ok(count) => raw_released += count,
                Err(error) => {
                    result = Err(format!("PvP raw cleanup:{error};journey={result:?}").into())
                }
            }
        }
        if admitted + raw_admitted > 0 && admitted == released && raw_admitted == raw_released {
            row.pass("final-logout");
        }
        row.finish(&result);
    }
}

async fn character_tab(
    client: &mut DevClientSession,
    tab: character::CharacterTab,
) -> TestResult<Vec<u8>> {
    let reply = client
        .session_mut()
        .gated_command(
            character::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            character::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[tab as u8, 0, 0, 50],
        )
        .await?;
    if reply.status != CommandStatus::Accepted || reply.payload.first() != Some(&(tab as u8)) {
        return Err(format!("character-tab {tab:?} refused: {reply:?}").into());
    }
    Ok(reply.payload)
}
async fn player_level(client: &mut DevClientSession) -> TestResult<u32> {
    let bytes = character_tab(client, character::CharacterTab::GeneralStats).await?;
    let level = u32::from_le_bytes(
        bytes
            .get(9..13)
            .ok_or("truncated native GeneralStats")?
            .try_into()?,
    );
    if level == 0 {
        return Err("native GeneralStats has zero level".into());
    }
    Ok(level)
}
async fn fresh_player(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    index: usize,
) -> TestResult {
    let old = client.join_snapshot().game_session_id;
    release_when_unlocked(client, Duration::from_secs(920)).await?;
    *client = connect_character(clients, 0xf0 + u8::try_from(index)?, index).await?;
    if client.join_snapshot().game_session_id == old {
        return Err("fresh admission reused old session".into());
    }
    Ok(())
}
async fn release_when_unlocked(client: &mut DevClientSession, budget: Duration) -> TestResult {
    let deadline = tokio::time::Instant::now() + budget;
    loop {
        let reply = client.session_mut().logout().await?;
        match reply.result {
            Some(native::logout::LogoutResult::Accepted)
                if reply.status == CommandStatus::Accepted =>
            {
                return Ok(());
            }
            Some(
                native::logout::LogoutResult::InFight { .. } | native::logout::LogoutResult::Busy,
            ) if tokio::time::Instant::now() < deadline => {
                // Keep the actor active without resetting its combat lock.
                player_turn(client).await?;
                client.service_liveness(Duration::from_millis(250)).await?;
                client.take_events();
            }
            _ => return Err(format!("ordinary logout did not release: {reply:?}").into()),
        }
    }
}

// Try the real starter's unarmed hunt, never a fixture build. A refused attack,
// no damage/kill, absent spawn, or the bounded XP budget is an explicit prerequisite
// failure. Only served level frames can end this loop successfully.
async fn earn_pvp_level(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(150);
    loop {
        let level = player_level(client).await?;
        if level >= oteryn_gameplay_combat_rules::pvp::PVP_MIN_LEVEL {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("player-level-acquisition-blocked:served-level={level};required=8;ordinary-rat-hunt-budget=150s").into());
        }
        hunt_one(client, world).await.map_err(|e| {
            format!(
                "player-level-acquisition-blocked:served-level={level};required=8;ordinary-hunt={e}"
            )
        })?;
    }
}
async fn hunt_one(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let bytes = fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let meta = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let positions: Vec<_> = meta
        .spawns
        .sources
        .iter()
        .flat_map(|s| s.points.iter().map(move |p| (s.floor, p)))
        .filter(|(_, p)| {
            meta.spawns
                .creatures
                .get(p.creature as usize)
                .is_some_and(|c| c.key == "oteryn:creature.rat")
        })
        .map(|(floor, p)| TilePos {
            x: p.x,
            y: p.y,
            floor,
        })
        .collect();
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|t| !t.protection_zone())
            && positions
                .iter()
                .any(|p| p.floor == at.floor && p.x.abs_diff(at.x).max(p.y.abs_diff(at.y)) <= 1)
    })
    .ok_or("no reachable unprotected rat spawn in compiled world within 160-tile route budget")?;
    walk(client, world, &path).await?;
    let at = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|es| {
            es.iter().find(|e| {
                e.kind == oteryn_dev_client::EntityKind::Creature
                    && e.position.floor == at.floor
                    && e.position.x.abs_diff(at.x).max(e.position.y.abs_diff(at.y)) <= 1
            })
        })
        .copied()
        .ok_or("ordinary rat route reached endpoint but no adjacent live creature frame")?;
    let reply = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if reply.disposition != attacks::AttackIntentDisposition::Ok {
        return Err(format!("starter attack refused: {reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
        let alive = client
            .world_entities()
            .is_some_and(|es| es.iter().any(|e| e.entity == target.entity));
        if !alive {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            client.session_mut().attack_target(None).await?;
            return Err(format!(
                "starter attack produced no kill in 45s;target={:?};vitals={:?}",
                target.entity,
                client.actor_vitals()
            )
            .into());
        }
    }
    client.session_mut().attack_target(None).await?;
    // Loot only a real nearby corpse; custody comes from native MOVE results.
    let corpse = client
        .world_entities()
        .and_then(|es| {
            es.iter().find(|e| {
                e.kind == oteryn_dev_client::EntityKind::Corpse && e.position == target.position
            })
        })
        .and_then(|e| {
            if let oteryn_dev_client::EntityDetail::Object { item_handle, .. } = e.detail {
                item_handle
            } else {
                None
            }
        });
    if let Some(corpse) = corpse {
        client.use_item(corpse).await?;
        let entries = client
            .open_container()
            .ok_or("killed creature corpse has no native container")?
            .entries
            .clone();
        for item in entries {
            let reply = client
                .move_item(&native::item_view::ItemMoveIntent {
                    source: item.handle,
                    destination: native::item_view::ItemMoveDestination::MainBackpack,
                })
                .await?;
            if reply.outcome != native::item_view::ItemMoveOutcome::Moved {
                return Err(format!("ordinary loot refused: {reply:?}").into());
            }
        }
    }
    Ok(())
}

async fn expert(
    client: &mut PvpClient,
    expert: native::pvp_expert::ExpertMode,
    secure: bool,
) -> TestResult {
    let expected = native::pvp_expert::ExpertFightModes {
        modes: attacks::FightModes {
            secure,
            ..attacks::FightModes::DEFAULT
        },
        expert,
    };
    let reply = client
        .session_mut()
        .gated_command(
            native::pvp_expert::CAPABILITY_PVP_EXPERT_V1,
            native::pvp_expert::COMMAND_TYPE_EXPERT_FIGHT_MODES_INTENT,
            &native::pvp_expert::encode_expert_fight_modes(&expected),
        )
        .await?;
    if reply.status != CommandStatus::Accepted
        || attacks::decode_attack_intent_result(&reply.payload)
            .map_err(|e| format!("expert result:{e:?}"))?
            != attacks::AttackIntentDisposition::Ok
    {
        return Err(format!("expert mode refused: {reply:?}").into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(native::pvp_expert::STATE_DOMAIN_EXPERT_FIGHT_MODES, 1)
        .await?;
    let observed = native::pvp_expert::decode_expert_fight_modes(&delta.payload)
        .map_err(|e| format!("expert state:{e:?}"))?;
    if observed != expected {
        return Err(format!("Open-PvP prerequisite:requested={expected:?};served={observed:?};world may fix expert controls").into());
    }
    Ok(())
}
async fn skull(client: &mut PvpClient, target: spatial::EntityRef) -> TestResult<u8> {
    let reply = client
        .session_mut()
        .gated_command(
            native::party_skull::CAPABILITY_PARTY_SKULL_V1,
            native::party_skull::COMMAND_TYPE_PARTY_SKULL_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("skull query refused:{reply:?}").into());
    }
    native::party_skull::decode_party_icons(&reply.payload)
        .map_err(|e| format!("skull codec:{e:?}"))?
        .into_iter()
        .find(|i| i.entity == target)
        .map(|i| i.skull)
        .ok_or_else(|| "observed actor absent from skull result".into())
}
async fn neutral_refusal(a: &mut PvpClient, b: &mut PvpClient) -> TestResult {
    let before = *b.actor_vitals().ok_or("peer vitals absent")?;
    let target = pvp_ref(b)?;
    let reply = a.session_mut().attack_target(Some(&target)).await?;
    pvp_poll(a, b).await?;
    // An ordinary query is the command barrier; absence of a delta alone is insufficient.
    skull(a, target).await?;
    if reply.disposition != attacks::AttackIntentDisposition::Rejected
        || b.actor_vitals() != Some(&before)
    {
        return Err(format!("neutral attack not refused without damage:{reply:?}").into());
    }
    Ok(())
}
fn pz_edge(world: &BundleWorld, at: TilePos) -> Option<StepDirection> {
    [
        StepDirection::North,
        StepDirection::East,
        StepDirection::South,
        StepDirection::West,
    ]
    .into_iter()
    .find(|d| {
        destination(world, at, *d).is_some_and(|p| {
            world
                .base()
                .tile(p.x, p.y, p.floor)
                .is_some_and(|t| t.protection_zone())
        })
    })
}
async fn pvp_journey(
    row: &mut JourneyRow,
    pair: &mut [Option<DevClientSession>; 2],
    raw: &mut [Option<PvpClient>; 2],
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    // Level both through their own playable route before admitting the peer concurrently.
    pair[0] = Some(connect_character(clients, 0xf2, 0).await?);
    earn_pvp_level(pair[0].as_mut().ok_or("first admission")?, world).await?;
    release_when_unlocked(
        pair[0].as_mut().ok_or("first admission")?,
        Duration::from_secs(70),
    )
    .await?;
    pair[0] = None;
    pair[1] = Some(connect_character(clients, 0xf3, 1).await?);
    earn_pvp_level(pair[1].as_mut().ok_or("second admission")?, world).await?;
    row.pass("earned-levels");
    release_when_unlocked(
        pair[1].as_mut().ok_or("second admission")?,
        Duration::from_secs(70),
    )
    .await?;
    pair[1] = None;
    raw[0] = Some(PvpClient::connect(clients, 0xf4, 0).await?);
    raw[1] = Some(PvpClient::connect(clients, 0xf5, 1).await?);
    let [a, b] = raw;
    let (a, b) = (
        a.as_mut().ok_or("PvP attacker admission")?,
        b.as_mut().ok_or("PvP victim admission")?,
    );
    if a.join_snapshot().game_session_id == b.join_snapshot().game_session_id
        || pvp_ref(a)? == pvp_ref(b)?
    {
        return Err("PvP needs distinct real players".into());
    }
    row.pass("two-admissions");
    let path = route(world, tile(a.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|t| !t.protection_zone())
            && pz_edge(world, at).is_some()
    })
    .ok_or("no reachable outside-PZ tile adjacent to real PZ for lock test")?;
    pvp_walk(a, b, world, &path).await?;
    let combat_tile = tile(a.world_spatial().actor_position)?;
    let path = route(world, tile(b.world_spatial().actor_position)?, |at| {
        at != combat_tile
            && at.floor == combat_tile.floor
            && at
                .x
                .abs_diff(combat_tile.x)
                .max(at.y.abs_diff(combat_tile.y))
                <= 1
            && world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|t| !t.protection_zone())
    })
    .ok_or("no unprotected adjacent peer route")?;
    pvp_walk(b, a, world, &path).await?;
    // Natural admission immunity must elapse while both sessions are serviced.
    let until = tokio::time::Instant::now() + Duration::from_secs(11);
    while tokio::time::Instant::now() < until {
        pvp_poll(a, b).await?;
        a.take_events();
        b.take_events();
    }
    let victim_name = pvp_name(b).await?;
    let old_kills = pvp_history(a).await?;
    if skull(b, pvp_ref(a)?).await? != 0 {
        return Err("PvP needs initially neutral attacker; existing skull would invalidate unjustified oracle".into());
    }
    use native::pvp_expert::ExpertMode::*;
    for mode in [WhiteHand, YellowHand, RedFist, Dove] {
        expert(a, mode, false).await?;
    }
    row.pass("expert-modes");
    for mode in [WhiteHand, YellowHand, Dove] {
        expert(a, mode, false).await?;
        neutral_refusal(a, b).await?;
    }
    row.pass("peaceful-refusals");
    expert(a, RedFist, true).await?;
    neutral_refusal(a, b).await?;
    row.pass("secure-refusal");
    expert(a, RedFist, false).await?;
    let before = *b.actor_vitals().ok_or("victim vitals")?;
    let reply = a.session_mut().attack_target(Some(&pvp_ref(b)?)).await?;
    if reply.disposition != attacks::AttackIntentDisposition::Ok {
        return Err(format!(
            "PvP aggression blocked despite earned level/neutral modes:{reply:?};tiles={:?}/{:?}",
            a.world_spatial().actor_position,
            b.world_spatial().actor_position
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    while b.actor_vitals().is_none_or(|v| v.health >= before.health) {
        if tokio::time::Instant::now() >= deadline {
            return Err("PvP accepted target but no victim damage frame within 45s;ordinary weapon/damage prerequisite unproven".into());
        }
        pvp_poll(a, b).await?;
        a.take_events();
        b.take_events();
    }
    a.session_mut().attack_target(None).await?;
    if skull(b, pvp_ref(a)?).await? != 3 {
        return Err("unjustified damage did not produce observer-relative white skull".into());
    }
    row.pass("damage-white-skull");
    for client in [&mut *a, &mut *b] {
        let reply = client.session_mut().logout().await?;
        if !matches!(
            reply.result,
            Some(native::logout::LogoutResult::InFight { .. })
        ) {
            return Err(format!("PvP hit did not lock logout:{reply:?}").into());
        }
    }
    row.pass("logout-lock");
    let before_position = a.world_spatial().actor_position;
    let reply = a
        .step(pz_edge(world, combat_tile).ok_or("PZ edge vanished")?)
        .await?;
    if reply.disposition != StepDisposition::Blocked
        || a.world_spatial().actor_position != before_position
    {
        return Err(format!("PvP PZ lock allowed entry:{reply:?}").into());
    }
    row.pass("pz-lock");
    let death_position = b.world_spatial().actor_position;
    let target = pvp_ref(b)?;
    if a.session_mut()
        .attack_target(Some(&target))
        .await?
        .disposition
        != attacks::AttackIntentDisposition::Ok
    {
        return Err("PvP lethal continuation refused".into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(150);
    loop {
        pvp_poll(a, b).await?;
        a.take_events();
        b.take_events();
        if b.world_spatial().actor_position != death_position
            && b.actor_vitals().is_some_and(|v| v.health == v.max_health)
        {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("PvP damage did not reach real lethal respawn within 150s".into());
        }
    }
    a.session_mut().attack_target(None).await?;
    let new_kills = pvp_history(a).await?;
    let expected = character::pvp_kill_description(&victim_name);
    if !new_kills.iter().any(|k| {
        k.1 == character::KillStatus::Unjustified as u8 && k.2 == expected && !old_kills.contains(k)
    }) {
        return Err(format!("lethal respawn has no new unjustified frag for {victim_name}").into());
    }
    row.pass("lethal-frag");
    let until = tokio::time::Instant::now() + Duration::from_secs(65);
    while tokio::time::Instant::now() < until {
        pvp_poll(a, b).await?;
        a.take_events();
        b.take_events();
    }
    let locked = a.session_mut().logout().await?;
    if !matches!(
        locked.result,
        Some(native::logout::LogoutResult::InFight { .. })
    ) {
        return Err(
            format!("kill lock absent after ordinary 60s hit lock elapsed:{locked:?}").into(),
        );
    }
    row.pass("kill-lock");
    // Release the victim before awaiting the killer's natural 15-minute block.
    let respawn = b.position;
    pvp_release(b, Duration::from_secs(70)).await?;
    pvp_fresh(a, clients, 0).await?;
    if pvp_history(a).await? != new_kills {
        return Err("fresh admission lost durable frag history".into());
    }
    // White expires naturally; the victim must still see the durable orange revenge mark.
    *b = PvpClient::connect(clients, 0xf7, 1).await?;
    if b.position != respawn {
        return Err("fresh admission lost victim respawn position".into());
    }
    let pos = tile(a.world_spatial().actor_position)?;
    let path = route(world, tile(b.world_spatial().actor_position)?, |at| {
        at != pos && at.floor == pos.floor && at.x.abs_diff(pos.x).max(at.y.abs_diff(pos.y)) <= 1
    })
    .ok_or("fresh peer cannot approach durable skull")?;
    pvp_walk(b, a, world, &path).await?;
    if skull(b, pvp_ref(a)?).await? != 6 {
        return Err("fresh admission did not retain victim-relative orange revenge skull".into());
    }
    row.pass("fresh-admission-frag-skull");
    Ok(())
}

fn decode_pvp_history(bytes: &[u8]) -> TestResult<Vec<(i64, u8, String)>> {
    if bytes.first() != Some(&(character::CharacterTab::RecentPvpKills as u8)) {
        return Err("wrong PvP history tab".into());
    }
    let count = u16::from_le_bytes(bytes.get(1..3).ok_or("short history header")?.try_into()?);
    if count > 50 {
        return Err("PvP history page over bound".into());
    }
    let mut at = 3;
    let mut rows = Vec::new();
    for _ in 0..count {
        let timestamp = i64::from_le_bytes(
            bytes
                .get(at..at + 8)
                .ok_or("short history timestamp")?
                .try_into()?,
        );
        at += 8;
        let status = *bytes.get(at).ok_or("short history status")?;
        at += 1;
        if ![1, 2].contains(&status) {
            return Err("invalid history kill status".into());
        }
        let len = usize::from(*bytes.get(at).ok_or("short history text length")?);
        at += 1;
        let text =
            std::str::from_utf8(bytes.get(at..at + len).ok_or("short history text")?)?.to_owned();
        at += len;
        rows.push((timestamp, status, text));
    }
    if at != bytes.len() {
        return Err("trailing PvP history bytes".into());
    }
    Ok(rows)
}
async fn pvp_history(client: &mut PvpClient) -> TestResult<Vec<(i64, u8, String)>> {
    decode_pvp_history(&pvp_tab(client, character::CharacterTab::RecentPvpKills).await?)
}

fn carried(client: &DevClientSession, definition: std::num::NonZeroU32) -> TestResult<ItemEntry> {
    client
        .inventory()
        .and_then(|i| {
            i.entries
                .iter()
                .find(|e| e.item_definition_ref == definition)
        })
        .copied()
        .ok_or_else(|| {
            format!("required direct-backpack item absent:definition-ref={definition}").into()
        })
}
fn units(client: &DevClientSession, definition: std::num::NonZeroU32) -> u64 {
    client.inventory().map_or(0, |i| {
        i.entries
            .iter()
            .chain(i.main_backpack.iter())
            .chain(i.equipment.iter().map(|e| &e.item))
            .filter(|e| e.item_definition_ref == definition)
            .map(|e| u64::from(e.count.get()))
            .sum()
    })
}
async fn wait_units(
    client: &mut DevClientSession,
    definition: std::num::NonZeroU32,
    expected: u64,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while units(client, definition) != expected {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "native custody mismatch:definition={definition};required={expected};served={}",
                units(client, definition)
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
    Ok(())
}
// Content selects a real vendor and route, never supplies evidence or grants an
// item. Every acquisition still needs the served offer, accepted BUY and inventory.
async fn acquire(
    client: &mut DevClientSession,
    world: &BundleWorld,
    definitions: &crate::content::item_ref::ItemDefinitionIndex,
    key: &str,
    count: u32,
) -> TestResult<std::num::NonZeroU32> {
    use native::npc_service as npc;
    use oteryn_dev_client::client_runtime::npc_shop as shop;
    let revision = definitions
        .revision(key)
        .ok_or_else(|| format!("acquisition content definition unavailable:{key}"))?;
    let reference = definitions
        .definition_ref("Item", key, revision)
        .ok_or("acquisition reference absent")?;
    if units(client, reference) >= u64::from(count) {
        return Ok(reference);
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let offers: serde_json::Value = serde_json::from_slice(&fs::read(
        root.join("content/services/trade/trade-00000-00323.json"),
    )?)?;
    let metadata = oteryn_world_bundle::bundle::visit(
        &fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?,
        |_, _| Ok(()),
    )?;
    let mut attempted = Vec::new();
    for record in offers["records"].as_array().ok_or("trade records absent")? {
        let declaration = &record["declaration"];
        if !declaration["offers"].as_array().is_some_and(|os| {
            os.iter()
                .any(|o| o["item"]["key"] == key && o["direction"] == "SellToPlayer")
        }) {
            continue;
        }
        let Some(name) = declaration["identity"]["key"]
            .as_str()
            .and_then(|s| s.strip_prefix("oteryn:service.trade."))
        else {
            continue;
        };
        let Some(vendor) = metadata
            .npcs
            .npcs
            .iter()
            .find(|n| n.key == format!("oteryn:npc.{name}"))
        else {
            continue;
        };
        let Some(_path) = route(world, tile(client.world_spatial().actor_position)?, |at| {
            vendor
                .placements
                .iter()
                .any(|p| p.floor == at.floor && p.x.abs_diff(at.x).max(p.y.abs_diff(at.y)) <= 3)
        }) else {
            attempted.push(format!("{name}:unreachable"));
            continue;
        };
        let planned_price = declaration["offers"]
            .as_array()
            .and_then(|os| {
                os.iter()
                    .find(|o| o["item"]["key"] == key && o["direction"] == "SellToPlayer")
            })
            .and_then(|o| o["unit_price"].as_u64())
            .ok_or("vendor price absent")?;
        earn_gold(
            client,
            world,
            planned_price
                .checked_mul(u64::from(count).saturating_sub(units(client, reference)))
                .ok_or("vendor price overflow")?,
        )
        .await?;
        // Hunting may move the player; resolve the actual route again.
        let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
            vendor
                .placements
                .iter()
                .any(|p| p.floor == at.floor && p.x.abs_diff(at.x).max(p.y.abs_diff(at.y)) <= 3)
        })
        .ok_or("vendor route unavailable after gold hunt")?;
        walk(client, world, &path).await?;
        let actor = client
            .world_entities()
            .and_then(|es| {
                es.iter().find(|e| {
                    e.kind == oteryn_dev_client::EntityKind::Npc
                        && vendor.placements.iter().any(|p| {
                            e.position
                                == ActorPosition {
                                    x: i32::from(p.x),
                                    y: i32::from(p.y),
                                    floor: -i16::from(p.floor),
                                }
                        })
                })
            })
            .map(|e| e.entity)
            .ok_or_else(|| format!("vendor {name} has no served actor at placement"))?;
        for text in ["hi", "trade"] {
            let result = shop::talk(
                client.session_mut(),
                &npc::NpcTalkIntent {
                    npc_actor: actor,
                    text: text.into(),
                },
            )
            .await?;
            if result != npc::NpcIntentDisposition::Ok {
                return Err(format!(
                    "player acquisition {key}:NPC={name};dialogue={text};served={result:?}"
                )
                .into());
            }
            let domain = if text == "hi" {
                npc::STATE_DOMAIN_NPC_CONVERSATION
            } else {
                npc::STATE_DOMAIN_NPC_TRADE_WINDOW
            };
            let delta = client.session_mut().read_gated_delta(domain, 1).await?;
            if text == "hi" {
                if npc::decode_npc_conversation(&delta.payload)
                    .map_err(|e| format!("conversation:{e:?}"))?
                    .open
                    .is_none()
                {
                    return Err("vendor greeting did not open".into());
                }
                continue;
            }
            let window = npc::decode_npc_trade_window(&delta.payload)
                .map_err(|e| format!("trade window:{e:?}"))?;
            let (index, offer) = window
                .offers
                .iter()
                .enumerate()
                .find(|(_, o)| {
                    o.side == npc::NpcTradeSide::Buy && o.item_definition_ref == reference.get()
                })
                .ok_or_else(|| format!("vendor {name} served no BUY offer for {key}"))?;
            let missing = u64::from(count).saturating_sub(units(client, reference));
            let quantity = u32::try_from(missing.div_ceil(u64::from(offer.count)))?;
            let before = units(client, reference);
            let reply = shop::buy(
                client.session_mut(),
                &npc::NpcTradeIntent {
                    npc_actor: actor,
                    catalogue_revision: window.open.ok_or("vendor window closed")?.1,
                    offer_index: u32::try_from(index)?,
                    side: offer.side,
                    quantity,
                    expected_unit_price: offer.unit_price,
                },
            )
            .await?;
            if reply != npc::NpcIntentDisposition::Ok {
                return Err(format!("player-acquisition-blocked:item={key};need={count};served-units={before};NPC={name};price={};quantity={quantity};BUY={reply:?};requires-player-earned-money",offer.unit_price).into());
            }
            wait_units(
                client,
                reference,
                before + u64::from(quantity) * u64::from(offer.count),
            )
            .await?;
        }
        return Ok(reference);
    }
    // Try the starter's actual loot route once before declaring an unmet item
    // prerequisite. This is bounded acquisition, not an assertion of global absence.
    let hunt = hunt_one(client, world).await;
    if units(client, reference) >= u64::from(count) {
        return Ok(reference);
    }
    Err(format!("player-acquisition-blocked:item={key};required={count};served-units={};reachable-vendor-attempts={attempted:?};ordinary-hunt={hunt:?};no-item-grant-or-global-missing-route-claim",units(client,reference)).into())
}
async fn imbue_request(
    client: &mut DevClientSession,
    item: ItemHandle,
    action: imbues::ImbuementAction,
) -> TestResult<imbues::ImbuementReply> {
    let response = oteryn_dev_client::client_runtime::imbue::send(
        client.session_mut(),
        &oteryn_dev_client::client_runtime::imbue::ImbueCommand(imbues::ImbuementRequest {
            item,
            action,
        }),
    )
    .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!("imbuement command refused:{response:?}").into());
    }
    response
        .reply
        .ok_or_else(|| "imbuement command has no native reply".into())
}
async fn imbue_window(
    client: &mut DevClientSession,
    definition: std::num::NonZeroU32,
) -> TestResult<imbues::ImbuementWindow> {
    let reply = imbue_request(
        client,
        carried(client, definition)?.handle,
        imbues::ImbuementAction::Query,
    )
    .await?;
    if reply.result != imbues::ImbuementResult::Ok {
        return Err(format!("imbuement window refused:{reply:?}").into());
    }
    reply
        .window
        .ok_or_else(|| "imbuement query omitted window".into())
}
async fn apply_imbue(
    client: &mut DevClientSession,
    definition: std::num::NonZeroU32,
    family: u8,
    tier: u8,
) -> TestResult<imbues::AppliedImbuement> {
    let window = imbue_window(client, definition).await?;
    let (index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, o)| o.family == family && o.tier == tier)
        .ok_or_else(|| format!("served item has no family={family},tier={tier} offer"))?;
    let reply = imbue_request(
        client,
        carried(client, definition)?.handle,
        imbues::ImbuementAction::Apply {
            slot: 0,
            imbuement: u8::try_from(index)?,
        },
    )
    .await?;
    if reply.result != imbues::ImbuementResult::Ok {
        let recipe = oteryn_gameplay_items::imbuement::imbuement_recipe(family, tier)
            .ok_or("recipe unavailable")?;
        return Err(format!("imbuement-apply-blocked:served={:?};family={family};tier={tier};gold={};premium={};materials={:?}",reply.result,offer.gold,offer.requires_premium,recipe.materials).into());
    }
    let current = imbue_window(client, definition).await?;
    current
        .applied
        .into_iter()
        .find(|a| {
            a.slot == 0 && a.family == family && a.tier == tier && a.remaining_ms == 72_000_000
        })
        .ok_or_else(|| "accepted apply did not produce exact fresh 20-hour slot".into())
}
async fn equipment_move(
    client: &mut DevClientSession,
    definition: std::num::NonZeroU32,
    equip: bool,
) -> TestResult {
    let already = client.inventory().is_some_and(|i| {
        i.equipment
            .iter()
            .any(|e| e.item.item_definition_ref == definition)
    });
    if already == equip {
        return Err("equipment toggle precondition mismatch".into());
    }
    let result = client
        .session_mut()
        .equip_item_definition(definition)
        .await?;
    if result.outcome != native::item_view::ItemMoveOutcome::Moved {
        return Err(format!("ordinary equipment toggle refused:{result:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let equipped = client.inventory().is_some_and(|i| {
            i.equipment
                .iter()
                .any(|e| e.item.item_definition_ref == definition)
        });
        if equipped == equip && (equip || carried(client, definition).is_ok()) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("equipment move has no confirming native inventory frame".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
}
async fn strike_effect(client: &mut DevClientSession) -> TestResult<i64> {
    let bytes = character_tab(client, character::CharacterTab::Offence).await?;
    character::combat::CombatStats::decode(&bytes)
        .map_err(|e| format!("combat stats:{e:?}"))?
        .get(character::combat::CombatStat::CriticalChanceImbuement)
        .ok_or_else(|| "server combat tab reports imbuement critical chance unknown".into())
}
async fn shrine(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    // Shrine locations come from the production bundle's item palette and sectors.
    let bytes = fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let mut positions = BTreeSet::new();
    let mut shrine_refs = BTreeSet::new();
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    for (index, entry) in metadata.manifest.palette.iter().enumerate() {
        if [
            24964, 25060, 25061, 25103, 25104, 25174, 25175, 25182, 25183, 25202,
        ]
        .iter()
        .any(|id| entry.key == format!("oteryn:item.tibia.i{id}"))
        {
            shrine_refs.insert(u32::try_from(index)?);
        }
    }
    oteryn_world_bundle::bundle::visit(&bytes, |_, sector| {
        for tile in &sector.tiles {
            if tile.items.iter().any(|e| shrine_refs.contains(&e.palette)) {
                positions.insert(TilePos {
                    x: tile.x,
                    y: tile.y,
                    floor: sector.floor,
                });
            }
        }
        Ok(())
    })?;
    let path=route(world,tile(client.world_spatial().actor_position)?,|at|positions.iter().any(|p|p.floor==at.floor && p.x.abs_diff(at.x).max(p.y.abs_diff(at.y))<=1)).ok_or_else(||format!("shrine-prerequisite-blocked:compiled-shrines={};none reachable within town route budget",positions.len()))?;
    walk(client, world, &path).await
}
async fn use_on(
    client: &mut DevClientSession,
    source: ItemHandle,
    target: ItemHandle,
) -> TestResult {
    let payload = native::world_object::encode_item_use_intent(&native::world_object::UseIntent {
        target: native::world_object::UseTarget::Item(source),
        use_with: None,
        use_with_map_item: Some(native::world_object::MapUseTarget::Item(target)),
        use_with_position: None,
    })
    .map_err(|e| format!("use-with codec:{e:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    if reply.status != CommandStatus::Accepted
        || native::world_object::decode_use_result_with_gate(&reply.payload)
            .map_err(|e| format!("use-with reply:{e:?}"))?
            .0
            != native::world_object::UseDisposition::Committed
    {
        return Err(format!("player use-with refused:{reply:?}").into());
    }
    Ok(())
}
async fn imbuement_journey(
    row: &mut JourneyRow,
    connection: &mut Option<DevClientSession>,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    *connection = Some(connect_character(clients, 0xf6, 1).await?);
    let client = connection.as_mut().ok_or("imbuement admission absent")?;
    let definitions = provisioning::item_index()?;
    let input = native_input()?;
    let profiles: crate::content::native_gameplay::ItemProfilesDocument = serde_json::from_slice(
        &input
            .item_profiles
            .as_ref()
            .ok_or("pinned item profiles absent")?
            .bytes,
    )?;
    use crate::content::ReferenceItemField::Known;
    let candidate=profiles.records.iter().filter(|r|r.admission.materializable && matches!(&r.semantics.imbuement,Known(i) if matches!(i.slot_count,Known(1..=3)) && matches!(&i.allowed_family_tiers,Known(fs) if fs.iter().any(|f|f.family.wire()==1))))
        .min_by_key(|r| {
            let key=&r.production_definition.production_key;
            let present=definitions.revision(key).and_then(|rev|definitions.definition_ref("Item",key,rev)).is_some_and(|d|carried(client,d).is_ok());
            (!present,key!="oteryn:item.tibia.i3264",key.clone())
        }).ok_or("no materializable strike-capable item in pinned native policy")?;
    let target = acquire(
        client,
        world,
        &definitions,
        &candidate.production_definition.production_key,
        1,
    )
    .await?;
    let basic = oteryn_gameplay_items::imbuement::imbuement_recipe(1, 1)
        .ok_or("basic Strike recipe absent")?;
    for &(key, count) in basic.materials {
        acquire(client, world, &definitions, key, count).await?;
    }
    row.pass("player-acquisition");
    earn_gold(client, world, basic.apply_gold + 15_000).await?;
    shrine(client, world).await?;
    let baseline = strike_effect(client).await?;
    let payment_before = player_gold(client).await?;
    let materials_before = inventory_counts(client)?;
    let active = apply_imbue(client, target, 1, 1).await?;
    verify_recipe_consumption(
        &definitions,
        &materials_before,
        &inventory_counts(client)?,
        basic.materials,
    )?;
    verify_gold_payment(payment_before, player_gold(client).await?, basic.apply_gold)?;
    row.pass("shrine-apply");
    let refusal_inventory = inventory_counts(client)?;
    let refusal_gold = player_gold(client).await?;
    let reply = imbue_request(
        client,
        carried(client, target)?.handle,
        imbues::ImbuementAction::Apply {
            slot: 0,
            imbuement: 0,
        },
    )
    .await?;
    if reply.result != imbues::ImbuementResult::SlotOccupied
        || imbue_window(client, target).await?.applied != [active]
    {
        return Err(format!("occupied apply refusal changed slot:{reply:?}").into());
    }
    if inventory_counts(client)? != refusal_inventory || player_gold(client).await? != refusal_gold
    {
        return Err("occupied refusal charged resources".into());
    }
    row.pass("occupied-refusal");
    equipment_move(client, target, true).await?;
    if strike_effect(client).await? != baseline + 100_000 {
        return Err("worn basic Strike did not add exact 10% critical chance".into());
    }
    row.pass("worn-effect");
    // Combat is a real eligible interval for Strike; no clock override. Removing
    // equipment checkpoints the elapsed duration before querying the carried item.
    hunt_one(client, world).await?;
    equipment_move(client, target, false).await?;
    let duration = imbue_window(client, target).await?.applied;
    if duration.len() != 1
        || duration[0].remaining_ms == 0
        || duration[0].remaining_ms >= active.remaining_ms
        || duration[0].family != active.family
        || duration[0].tier != active.tier
    {
        return Err(
            format!("eligible worn combat did not decrement only duration:{duration:?}").into(),
        );
    }
    row.pass("eligible-duration");
    if strike_effect(client).await? != baseline {
        return Err("unequipping did not remove Strike effect".into());
    }
    row.pass("unequipped-effect");
    fresh_player(client, clients, 1).await?;
    if imbue_window(client, target).await?.applied != duration
        || strike_effect(client).await? != baseline
    {
        return Err("fresh admission changed unequipped duration/family/tier/effect".into());
    }
    row.pass("fresh-admission-active");
    shrine(client, world).await?;
    let clear_gold_before = player_gold(client).await?;
    let reply = imbue_request(
        client,
        carried(client, target)?.handle,
        imbues::ImbuementAction::Clear { slot: 0 },
    )
    .await?;
    if reply.result != imbues::ImbuementResult::Ok
        || !imbue_window(client, target).await?.applied.is_empty()
    {
        return Err(
            format!("shrine clear blocked:{reply:?};required-player-earned-gold=15000").into(),
        );
    }
    verify_gold_payment(clear_gold_before, player_gold(client).await?, 15_000)?;
    row.pass("shrine-clear");
    let reply = imbue_request(
        client,
        carried(client, target)?.handle,
        imbues::ImbuementAction::Clear { slot: 0 },
    )
    .await?;
    if reply.result != imbues::ImbuementResult::NotAllowed
        || !imbue_window(client, target).await?.applied.is_empty()
    {
        return Err("empty-slot clear did not refuse without changing item".into());
    }
    verify_gold_payment(clear_gold_before, player_gold(client).await?, 15_000)?;
    row.pass("empty-clear-refusal");
    // Inscribe an actual blank, then consume the newly produced scroll on the
    // cleared item. Filled scroll custody is observed, never supplied by a fixture.
    let blank = acquire(client, world, &definitions, "oteryn:item.tibia.i51442", 1).await?;
    let etcher = acquire(client, world, &definitions, "oteryn:item.tibia.i51443", 1).await?;
    let strong = oteryn_gameplay_items::imbuement::imbuement_recipe(1, 2)
        .ok_or("intricate Strike recipe absent")?;
    for &(key, count) in strong.materials {
        acquire(client, world, &definitions, key, count).await?;
    }
    earn_gold(client, world, strong.apply_gold).await?;
    shrine(client, world).await?;
    let window = imbue_window(client, blank).await?;
    let index = window
        .offers
        .iter()
        .position(|o| o.family == 1 && o.tier == 2)
        .ok_or("blank has no intricate Strike inscription offer")?;
    let scroll = oteryn_gameplay_items::imbuement::scroll_for_imbuement(1, 2)
        .ok_or("intricate Strike scroll mapping absent")?;
    let scroll_ref = provisioning::item_reference(&definitions, scroll.item_id)?;
    let inscription_gold = player_gold(client).await?;
    let inscription_materials = inventory_counts(client)?;
    let before_blank = units(client, blank);
    let before_scroll = units(client, scroll_ref);
    let reply = imbue_request(
        client,
        carried(client, blank)?.handle,
        imbues::ImbuementAction::Apply {
            slot: 0,
            imbuement: u8::try_from(index)?,
        },
    )
    .await?;
    if reply.result != imbues::ImbuementResult::Ok {
        return Err(format!(
            "scroll inscription blocked:{reply:?};requires-premium=true;gold={};materials={:?}",
            strong.apply_gold, strong.materials
        )
        .into());
    }
    wait_units(
        client,
        blank,
        before_blank.checked_sub(1).ok_or("blank count underflow")?,
    )
    .await?;
    wait_units(client, scroll_ref, before_scroll + 1).await?;
    verify_gold_payment(
        inscription_gold,
        player_gold(client).await?,
        strong.apply_gold,
    )?;
    verify_recipe_consumption(
        &definitions,
        &inscription_materials,
        &inventory_counts(client)?,
        strong.materials,
    )?;
    row.pass("scroll-create");
    use_on(
        client,
        carried(client, scroll_ref)?.handle,
        carried(client, target)?.handle,
    )
    .await?;
    wait_units(client, scroll_ref, before_scroll).await?;
    let applied = imbue_window(client, target).await?.applied;
    if applied.len() != 1
        || applied[0].family != 1
        || applied[0].tier != 2
        || applied[0].remaining_ms != 72_000_000
    {
        return Err(format!("scroll consumption produced wrong item slots:{applied:?}").into());
    }
    row.pass("scroll-use");
    let before_etcher = units(client, etcher);
    use_on(
        client,
        carried(client, etcher)?.handle,
        carried(client, target)?.handle,
    )
    .await?;
    wait_units(
        client,
        etcher,
        before_etcher
            .checked_sub(1)
            .ok_or("etcher count underflow")?,
    )
    .await?;
    if !imbue_window(client, target).await?.applied.is_empty() {
        return Err("etcher consumption did not clear all applied slots".into());
    }
    row.pass("etch-all");
    let inventory = inventory_counts(client)?;
    fresh_player(client, clients, 1).await?;
    if !imbue_window(client, target).await?.applied.is_empty()
        || inventory_counts(client)? != inventory
        || strike_effect(client).await? != baseline
    {
        return Err(
            "fresh admission lost cleared slots/consumed sources or restored an effect".into(),
        );
    }
    row.pass("fresh-admission-cleared");
    Ok(())
}
fn inventory_counts(client: &DevClientSession) -> TestResult<BTreeMap<u32, u64>> {
    let inventory = client.inventory().ok_or("native inventory absent")?;
    let mut counts = BTreeMap::new();
    for e in inventory
        .entries
        .iter()
        .chain(inventory.main_backpack.iter())
        .chain(inventory.equipment.iter().map(|e| &e.item))
    {
        *counts.entry(e.item_definition_ref.get()).or_insert(0) += u64::from(e.count.get());
    }
    Ok(counts)
}

async fn player_turn(client: &mut DevClientSession) -> TestResult {
    let mut payload = Vec::new();
    bytes_field(&mut payload, 3, &encode_step_intent(StepDirection::North));
    let result = client
        .session_mut()
        .gated_command(
            native::target_move::CAPABILITY_TARGET_MOVE_V1,
            native::target_move::COMMAND_TYPE_TARGET_MOVE_INTENT,
            &payload,
        )
        .await?;
    if result.status != CommandStatus::Accepted || result.payload != [8, 1] {
        return Err(format!("ordinary activity turn refused:{result:?}").into());
    }
    Ok(())
}

// The shipped DevClientSession does not advertise expert or skull commands.
// This test-only observer sends their production native frames on ordinary TLS
// admissions; no production client or server route is changed.
struct PvpClient {
    stream: LightStream,
    generation: u64,
    session: native::GameSessionId,
    sequence: u64,
    command: u64,
    probe: u64,
    selected: Vec<u32>,
    revisions: BTreeMap<u32, u64>,
    own: spatial::EntityRef,
    vitals: casts::ActorVitals,
    position: ActorPosition,
    content: [u8; 32],
    map_header: native::world_map::MapViewHeader,
    expert: native::pvp_expert::ExpertFightModes,
    expert_delta: Option<Vec<u8>>,
    released: bool,
}
#[derive(Debug)]
struct PvpReply {
    status: CommandStatus,
    payload: Vec<u8>,
}
#[derive(Debug)]
struct PvpAttack {
    disposition: attacks::AttackIntentDisposition,
}
#[derive(Debug)]
struct PvpLogout {
    status: CommandStatus,
    result: Option<native::logout::LogoutResult>,
}
#[derive(Debug)]
struct PvpStep {
    disposition: StepDisposition,
}
impl PvpClient {
    async fn connect(clients: &SeamClients<'_>, tag: u8, index: usize) -> TestResult<Self> {
        let generation = platform_generation(clients.descriptor, &clients.accounts[index]).await?;
        let token = sign_grant(
            &Grant {
                signing: clients.signing,
                key_id: clients.key_id,
                account_id: &clients.accounts[index],
                character_id: clients.characters[index],
                world_id: clients.world,
                channel_id: clients.channel,
                security_generation: generation,
                scope_generation: clients.scope_generation,
                nonce: [tag; 32],
                attempt: v7(tag, 0x05),
            },
            now_seconds()?,
        );
        let connector = connector(clients.certificate, &EXACT)?;
        let mut stream = tokio::time::timeout(Duration::from_secs(20), async {
            let tcp = TcpStream::connect(clients.address).await?;
            Ok::<_, Box<dyn std::error::Error>>(
                connector
                    .connect(ServerName::try_from("localhost")?, tcp)
                    .await?,
            )
        })
        .await
        .map_err(|_| "pvp-native TLS deadline")??;
        if stream.get_ref().1.alpn_protocol() != Some(native::ALPN_OTERYN_GAME_V1.as_bytes()) {
            return Err("pvp-native ALPN mismatch".into());
        }
        let supported = PVP_SUPPORTED_CAPABILITIES;
        light_write(
            &mut stream,
            &native::encode_client_bootstrap(&native::ClientBootstrapValue {
                schema_revision: 1,
                character_id: CharacterId::decode(&clients.characters[index])?,
                admission_material: token.as_bytes(),
                client_build_id: "oteryn-e2e-pvp",
                supported_capabilities: supported,
            })?,
        )
        .await?;
        let bytes = light_read(&mut stream).await?;
        let frame = native::decode_wire_envelope(&bytes)?;
        frame.validate(native::Direction::ServerToClient, false)?;
        if frame.message_type() != MessageType::ServerAccepted {
            return Err("pvp-native ordinary admission refused".into());
        }
        let accepted = native::decode_server_accepted(frame.payload())?;
        if accepted.world_id != WorldId::decode(&clients.world)?
            || accepted.channel_id != ChannelId::decode(&clients.channel)?
            || accepted.schema_revision != 1
            || supported
                .iter()
                .any(|c| !accepted.selected_capabilities.contains(c))
            || accepted
                .selected_capabilities
                .iter()
                .any(|c| !supported.contains(c))
        {
            return Err("pvp-native admission world/channel/capabilities mismatch".into());
        }
        let bytes = light_read(&mut stream).await?;
        let (begin, body) =
            raw_snapshot(&mut stream, accepted.connection_generation, &bytes).await?;
        if begin.target_server_sequence < accepted.current_server_sequence {
            return Err("pvp-native snapshot commit mismatch".into());
        }
        let domains = native::decode_snapshot_body(&body)?;
        for id in [1, 3, 9, 10, 11, 17, 52] {
            let domain = domains
                .iter()
                .find(|d| d.domain_id == id)
                .ok_or("PvP required snapshot domain absent")?;
            if id != 1 && domain.snapshot_type != 1 {
                return Err("PvP snapshot type mismatch".into());
            }
            match id {
                9 => {
                    native::item_view::decode_character_inventory(domain.payload)
                        .map_err(|e| format!("PvP inventory snapshot: {e:?}"))?;
                }
                11 => {
                    native::item_view::decode_open_container(domain.payload)
                        .map_err(|e| format!("PvP container snapshot: {e:?}"))?;
                }
                _ => {}
            }
        }
        let domain = domains
            .iter()
            .find(|d| d.domain_id == 1)
            .ok_or("pvp-native spatial snapshot absent")?;
        let (view, _) = spatial::decode_world_spatial_snapshot_view_lit(
            &accepted.selected_capabilities,
            domain.snapshot_type,
            domain.payload,
        )
        .map_err(|e| format!("pvp-native spatial snapshot: {e:?}"))?;
        let spatial::WorldSpatialSnapshotView::Entities(view) = view else {
            return Err("pvp-native entity snapshot absent".into());
        };
        let map_header = raw_map_snapshot(&domains, view.content_generation)?;
        let vitals = domains
            .iter()
            .find(|d| d.domain_id == 3)
            .ok_or("pvp-native vitals snapshot absent")?;
        let vitals = casts::decode_actor_vitals(vitals.payload)
            .map_err(|e| format!("pvp-native vitals snapshot: {e:?}"))?;
        let own = view
            .entities
            .iter()
            .find(|e| e.entity.identity == view.own_identity)
            .ok_or("PvP own actor absent")?
            .entity;
        let expert = native::pvp_expert::decode_expert_fight_modes(
            domains
                .iter()
                .find(|d| d.domain_id == 52)
                .ok_or("expert snapshot absent")?
                .payload,
        )
        .map_err(|e| format!("expert snapshot:{e:?}"))?;
        Ok(Self {
            stream,
            generation: accepted.connection_generation,
            session: accepted.game_session_id,
            sequence: begin.target_server_sequence,
            command: accepted.next_command_id,
            probe: 0,
            selected: accepted.selected_capabilities,
            revisions: domains.iter().map(|d| (d.domain_id, d.revision)).collect(),
            own,
            vitals,
            position: view.actor_position,
            content: view.content_generation,
            map_header,
            expert,
            expert_delta: None,
            released: false,
        })
    }

    fn session_mut(&mut self) -> &mut Self {
        self
    }
    fn join_snapshot(&self) -> PvpSnapshot {
        PvpSnapshot {
            game_session_id: self.session,
        }
    }
    fn world_spatial(&self) -> PvpPosition {
        PvpPosition {
            actor_position: self.position,
        }
    }
    fn actor_vitals(&self) -> Option<&casts::ActorVitals> {
        Some(&self.vitals)
    }
    fn take_events(&mut self) {}
    async fn receive(&mut self) -> TestResult<Option<(u64, CommandStatus, Vec<u8>)>> {
        let bytes = light_read(&mut self.stream).await?;
        let frame = native::decode_wire_envelope(&bytes)?;
        frame.validate(native::Direction::ServerToClient, true)?;
        if frame.connection_generation() != self.generation {
            return Err("PvP stale frame generation".into());
        }
        if frame.message_type() == MessageType::SnapshotBegin {
            let (sequence, revision, header) = raw_map_replacement(
                &mut self.stream,
                self.generation,
                &bytes,
                self.sequence,
                self.revisions
                    .get(&17)
                    .copied()
                    .ok_or("map revision absent")?,
                self.content,
            )
            .await?;
            self.sequence = sequence;
            self.revisions.insert(17, revision);
            self.map_header = header;
            return Ok(None);
        }
        if frame.message_type() == MessageType::LivenessProbe {
            let probe = native::decode_liveness_probe(frame.payload())?;
            if probe <= self.probe {
                return Err("PvP nonmonotonic probe".into());
            }
            self.probe = probe;
            light_write(
                &mut self.stream,
                &native::encode_liveness_ack(self.generation, probe, self.sequence)?,
            )
            .await?;
            return Ok(None);
        }
        if !matches!(
            frame.message_type(),
            MessageType::StateDelta | MessageType::CommandResult
        ) || Some(frame.server_sequence()) != self.sequence.checked_add(1)
        {
            return Err("PvP frame sequence/type mismatch".into());
        }
        self.sequence = frame.server_sequence();
        if frame.message_type() == MessageType::CommandResult {
            let r = native::decode_command_result(frame.payload())?;
            return Ok(Some((r.command_id, r.status, r.payload.to_vec())));
        }
        let delta = native::decode_state_delta(frame.payload())?;
        if delta.base_revision != self.revisions.get(&delta.domain_id).copied().unwrap_or(0)
            || delta.new_revision <= delta.base_revision
        {
            return Err("PvP domain revision mismatch".into());
        }
        if delta.domain_id != 1 && delta.delta_type != 1 {
            return Err("PvP unregistered delta type".into());
        }
        match delta.domain_id {
            1 => {
                let (view, _) = spatial::decode_world_spatial_delta_view_lit(
                    &self.selected,
                    delta.delta_type,
                    delta.payload,
                )
                .map_err(|e| format!("PvP spatial:{e:?}"))?;
                let spatial::WorldSpatialDeltaView::Entities(view) = view else {
                    return Err("PvP entity delta absent".into());
                };
                if view.content_generation != self.content {
                    return Err("PvP content generation changed".into());
                }
                self.position = view.actor_position;
            }
            17 => {
                self.map_header = raw_map_delta(&self.map_header, delta.payload)?;
            }
            9 => {
                native::item_view::decode_character_inventory(delta.payload)
                    .map_err(|e| format!("PvP inventory delta: {e:?}"))?;
            }
            11 => {
                native::item_view::decode_open_container(delta.payload)
                    .map_err(|e| format!("PvP container delta: {e:?}"))?;
            }
            3 => {
                self.vitals = casts::decode_actor_vitals(delta.payload)
                    .map_err(|e| format!("PvP vitals:{e:?}"))?;
            }
            10 => {
                attacks::decode_actor_combat_state(delta.payload)
                    .map_err(|e| format!("PvP combat:{e:?}"))?;
            }
            52 => {
                self.expert = native::pvp_expert::decode_expert_fight_modes(delta.payload)
                    .map_err(|e| format!("PvP expert:{e:?}"))?;
                self.expert_delta = Some(delta.payload.to_vec());
            }
            _ => return Err(format!("PvP unexpected domain:{}", delta.domain_id).into()),
        }
        self.revisions.insert(delta.domain_id, delta.new_revision);
        Ok(None)
    }
    async fn gated_command(
        &mut self,
        capability: u32,
        kind: u32,
        payload: &[u8],
    ) -> TestResult<PvpReply> {
        if !self.selected.contains(&capability) {
            return Err(format!("PvP capability {capability} not selected").into());
        }
        let id = self.command;
        self.command = self.command.checked_add(1).ok_or("PvP command overflow")?;
        light_write(
            &mut self.stream,
            &native::encode_client_command(
                self.generation,
                &native::ClientCommandValue {
                    command_id: id,
                    command_type: kind,
                    payload,
                },
            )?,
        )
        .await?;
        loop {
            if let Some((seen, status, payload)) = self.receive().await? {
                if seen != id {
                    return Err("PvP command result correlation mismatch".into());
                }
                return Ok(PvpReply { status, payload });
            }
        }
    }
    async fn read_gated_delta(&mut self, domain: u32, kind: u32) -> TestResult<PvpReply> {
        if domain != 52 || kind != 1 {
            return Err("PvP unexpected requested delta".into());
        }
        loop {
            if let Some(payload) = self.expert_delta.take() {
                return Ok(PvpReply {
                    status: CommandStatus::Accepted,
                    payload,
                });
            }
            if self.receive().await?.is_some() {
                return Err("PvP unsolicited result while waiting expert state".into());
            }
        }
    }
    async fn service_liveness(&mut self, duration: Duration) -> TestResult {
        let until = tokio::time::Instant::now() + duration;
        loop {
            if tokio::time::timeout_at(until, self.stream.get_ref().0.readable())
                .await
                .is_err()
            {
                return Ok(());
            }
            if self.receive().await?.is_some() {
                return Err("PvP unsolicited command result".into());
            }
        }
    }
    async fn attack_target(
        &mut self,
        target: Option<&spatial::EntityRef>,
    ) -> TestResult<PvpAttack> {
        let payload = attacks::encode_attack_target_intent(target)
            .map_err(|e| format!("PvP attack:{e:?}"))?;
        let reply = self
            .gated_command(17, attacks::COMMAND_TYPE_ATTACK_TARGET_INTENT, &payload)
            .await?;
        let disposition = attacks::decode_attack_intent_result(&reply.payload)
            .map_err(|e| format!("PvP attack result:{e:?}"))?;
        if (reply.status == CommandStatus::Accepted)
            != (disposition != attacks::AttackIntentDisposition::Rejected)
        {
            return Err("PvP attack result status mismatch".into());
        }
        Ok(PvpAttack { disposition })
    }
    async fn logout(&mut self) -> TestResult<PvpLogout> {
        let reply = self
            .gated_command(21, native::logout::COMMAND_TYPE_LOGOUT_INTENT, &[])
            .await?;
        let result = if reply.payload.is_empty() {
            None
        } else {
            Some(
                native::logout::decode_logout_result(&reply.payload)
                    .map_err(|e| format!("PvP logout:{e:?}"))?,
            )
        };
        if result.is_some_and(|r| r.is_rejected() != (reply.status == CommandStatus::Rejected)) {
            return Err(format!("PvP logout status mismatch: status={:?}; result={result:?}", reply.status).into());
        }
        self.released = result == Some(native::logout::LogoutResult::Accepted);
        Ok(PvpLogout { status: reply.status, result })
    }
    async fn step(&mut self, direction: StepDirection) -> TestResult<PvpStep> {
        let reply = self
            .gated_command(6, 1, &encode_step_intent(direction))
            .await?;
        let disposition = native::world_spatial::decode_step_result_paced(&reply.payload, true)
            .map_err(|e| format!("PvP step:{e:?}"))?;
        Ok(PvpStep { disposition })
    }
}
struct PvpSnapshot {
    game_session_id: native::GameSessionId,
}
struct PvpPosition {
    actor_position: ActorPosition,
}
fn pvp_ref(client: &PvpClient) -> TestResult<spatial::EntityRef> {
    Ok(client.own)
}
async fn pvp_poll(a: &mut PvpClient, b: &mut PvpClient) -> TestResult {
    let (a, b) = join(
        a.service_liveness(Duration::from_millis(25)),
        b.service_liveness(Duration::from_millis(25)),
    )
    .await;
    a?;
    b?;
    Ok(())
}
async fn pvp_walk(
    a: &mut PvpClient,
    b: &mut PvpClient,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        let before = tile(a.position)?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            let result = a.step(direction).await?;
            if result.disposition == StepDisposition::Moved {
                if Some(tile(a.position)?) != destination(world, before, direction) {
                    return Err("PvP real STEP endpoint mismatch".into());
                }
                break;
            }
            if result.disposition != StepDisposition::TooEarly
                || tokio::time::Instant::now() >= deadline
            {
                return Err(format!("PvP route refused:{:?}", result.disposition).into());
            }
            pvp_poll(a, b).await?;
        }
        b.service_liveness(Duration::from_millis(10)).await?;
    }
    Ok(())
}
async fn pvp_release(client: &mut PvpClient, budget: Duration) -> TestResult {
    let deadline = tokio::time::Instant::now() + budget;
    loop {
        let reply = client.logout().await?;
        let wait = match reply.result {
            Some(native::logout::LogoutResult::Accepted) => return Ok(()),
            Some(native::logout::LogoutResult::InFight { retry_after_ms }) => {
                Duration::from_millis(u64::from(retry_after_ms)).min(Duration::from_secs(30))
            }
            Some(native::logout::LogoutResult::Busy) => Duration::from_millis(250),
            _ => return Err(format!("PvP ordinary release blocked: status={:?}; result={:?}", reply.status, reply.result).into()),
        };
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("PvP ordinary release deadline: status={:?}; result={:?}", reply.status, reply.result).into());
        }
        let until = (tokio::time::Instant::now() + wait).min(deadline);
        while tokio::time::Instant::now() < until {
            client.service_liveness(Duration::from_millis(50)).await?;
        }
        let mut payload = Vec::new();
        bytes_field(&mut payload, 3, &encode_step_intent(StepDirection::North));
        let turn = client.gated_command(64, 81, &payload).await?;
        if turn.status != CommandStatus::Accepted || turn.payload != [8, 1] {
            return Err(format!("PvP lock-wait activity refused:{turn:?}").into());
        }
    }
}
async fn pvp_fresh(client: &mut PvpClient, clients: &SeamClients<'_>, index: usize) -> TestResult {
    let old = client.session;
    pvp_release(client, Duration::from_secs(920)).await?;
    *client = PvpClient::connect(clients, 0xf8 + u8::try_from(index)?, index).await?;
    if client.session == old {
        return Err("PvP fresh admission reused session".into());
    }
    Ok(())
}
async fn pvp_tab(client: &mut PvpClient, tab: character::CharacterTab) -> TestResult<Vec<u8>> {
    let reply = client
        .gated_command(199, 388, &[tab as u8, 0, 0, 50])
        .await?;
    if reply.status != CommandStatus::Accepted || reply.payload.first() != Some(&(tab as u8)) {
        return Err(format!("PvP character tab refused:{reply:?}").into());
    }
    Ok(reply.payload)
}
async fn pvp_name(client: &mut PvpClient) -> TestResult<String> {
    let command =
        highscores::command(request(1, 0, 1, 0)).map_err(|e| format!("PvP name request:{e:?}"))?;
    let reply = client
        .gated_command(command.capability, command.command_type, &command.payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err("PvP victim ranking/name refused".into());
    }
    wire::decode_highscores_result(&reply.payload)
        .map_err(|e| format!("PvP name:{e:?}"))?
        .own_row
        .map(|r| r.name)
        .ok_or_else(|| "PvP victim ranking omitted own name".into())
}

async fn player_gold(client: &mut DevClientSession) -> TestResult<u64> {
    use native::atelier::resources as money;
    let reply = client
        .session_mut()
        .gated_command(
            money::CAPABILITY_ATELIER_RESOURCES_V1,
            money::COMMAND_TYPE_ATELIER_RESOURCES_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("native player balances refused:{reply:?}").into());
    }
    let balances =
        money::decode_reply(&reply.payload).map_err(|e| format!("native balances:{e:?}"))?;
    balances
        .carried_gold
        .checked_add(balances.bank_gold)
        .ok_or_else(|| "native gold total overflow".into())
}
async fn earn_gold(
    client: &mut DevClientSession,
    world: &BundleWorld,
    required: u64,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(150);
    loop {
        let served = player_gold(client).await?;
        if served >= required {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("player-money-acquisition-blocked:required-gold={required};served-bank-plus-carried={served};ordinary-hunt-budget=150s").into());
        }
        hunt_one(client,world).await.map_err(|e|format!("player-money-acquisition-blocked:required-gold={required};served-bank-plus-carried={served};ordinary-hunt={e}"))?;
    }
}
fn verify_gold_payment(before: u64, after: u64, charge: u64) -> TestResult {
    if before.checked_sub(after) != Some(charge) {
        return Err(format!(
            "native payment mismatch:before={before};after={after};required-charge={charge}"
        )
        .into());
    }
    Ok(())
}
fn verify_recipe_consumption(
    definitions: &crate::content::item_ref::ItemDefinitionIndex,
    before: &BTreeMap<u32, u64>,
    after: &BTreeMap<u32, u64>,
    materials: &[(&str, u32)],
) -> TestResult {
    for &(key, count) in materials {
        let revision = definitions
            .revision(key)
            .ok_or("material definition missing")?;
        let reference = definitions
            .definition_ref("Item", key, revision)
            .ok_or("material reference missing")?
            .get();
        if before
            .get(&reference)
            .copied()
            .unwrap_or(0)
            .checked_sub(after.get(&reference).copied().unwrap_or(0))
            != Some(u64::from(count))
        {
            return Err(format!(
                "native recipe material consumption mismatch:{key};required={count}"
            )
            .into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod pass_two_tests {
    use super::*;
    #[test]
    fn pvp_history_decodes_only_complete_bounded_native_pages() -> TestResult {
        let bytes = character::TabView::RecentPvpKills(vec![(
            123,
            character::KillStatus::Unjustified,
            "Killed Peer.".into(),
        )])
        .encode()
        .map_err(|e| format!("history fixture:{e:?}"))?;
        assert_eq!(
            decode_pvp_history(&bytes)?,
            vec![(123, 1, "Killed Peer.".into())]
        );
        for len in 0..bytes.len() {
            assert!(decode_pvp_history(&bytes[..len]).is_err());
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(decode_pvp_history(&trailing).is_err());
        let mut bad_status = bytes;
        bad_status[11] = 3;
        assert!(decode_pvp_history(&bad_status).is_err());
        assert!(decode_pvp_history(&[3, 51, 0]).is_err());
        assert!(decode_pvp_history(&[2, 0, 0]).is_err());
        assert_eq!(decode_pvp_history(&[3, 0, 0])?, Vec::new());
        Ok(())
    }
    #[test]
    fn new_rows_require_every_action_including_persistence_and_release() {
        for (id, actions) in [("S08", PVP_ACTIONS), ("P21", IMBUE_ACTIONS)] {
            let mut row = JourneyRow::new(id, actions);
            for &action in &actions[..actions.len() - 1] {
                row.pass(action);
            }
            assert!(!row.actions.complete());
            row.pass("final-logout");
            assert!(row.actions.complete());
            row.pass("final-logout");
            assert!(!row.actions.complete());
        }
    }
    #[test]
    fn material_oracle_requires_exact_native_custody_decrements() -> TestResult {
        let definitions=crate::content::item_ref::ItemDefinitionIndex::decode(br#"{"schema":"OTERYN_NATIVE_ITEM_KEYS/v1","records":[["oteryn:item.tibia.i70001","definition-r1"]]}"#)?;
        let before = BTreeMap::from([(1, 25)]);
        assert!(
            verify_recipe_consumption(
                &definitions,
                &before,
                &BTreeMap::from([(1, 5)]),
                &[("oteryn:item.tibia.i70001", 20)]
            )
            .is_ok()
        );
        for quantity in [0, 4, 6, 25, 26] {
            assert!(
                verify_recipe_consumption(
                    &definitions,
                    &before,
                    &BTreeMap::from([(1, quantity)]),
                    &[("oteryn:item.tibia.i70001", 20)]
                )
                .is_err()
            );
        }
        assert!(
            verify_recipe_consumption(
                &definitions,
                &before,
                &BTreeMap::new(),
                &[("oteryn:item.tibia.i70002", 20)]
            )
            .is_err()
        );
        Ok(())
    }
    #[test]
    fn payment_oracle_refuses_undercharge_overcharge_and_increase() {
        assert!(verify_gold_payment(30_000, 22_500, 7_500).is_ok());
        for after in [30_000, 22_501, 22_499, 30_001] {
            assert!(verify_gold_payment(30_000, after, 7_500).is_err());
        }
        assert!(verify_gold_payment(0, u64::MAX, 1).is_err());
    }
}

async fn cleanup_pvp(peer: Option<&mut PvpClient>) -> TestResult<usize> {
    let Some(peer) = peer else {
        return Ok(0);
    };
    if peer.released {
        return Ok(1);
    }
    tokio::time::timeout(Duration::from_secs(940), async {
        peer.attack_target(None).await?;
        pvp_release(peer, Duration::from_secs(920)).await?;
        Ok::<_, Box<dyn std::error::Error>>(1)
    })
    .await
    .map_err(|_| "PvP cleanup deadline")?
}

// P19 observer retains the native influence, inventory and corpse frames.
struct ForgeClient {
    stream: LightStream,
    generation: u64,
    session: native::GameSessionId,
    sequence: u64,
    command: u64,
    probe: u64,
    selected: Vec<u32>,
    revisions: BTreeMap<u32, u64>,
    vitals: casts::ActorVitals,
    position: ActorPosition,
    content: [u8; 32],
    map_header: native::world_map::MapViewHeader,
    entities: Vec<spatial::WorldSpatialEntity>,
    bindings: Vec<native::appearance::Binding>,
    inventory: native::item_view::CharacterInventory,
    container: native::item_view::OpenContainer,
    released: bool,
    tree: native::container_tree::ContainerViews,
}
#[derive(Debug)]
struct ForgeReply {
    status: CommandStatus,
    payload: Vec<u8>,
}
#[derive(Debug)]
struct ForgeAttack {
    disposition: attacks::AttackIntentDisposition,
}
#[derive(Debug)]
struct ForgeLogout {
    status: CommandStatus,
    result: Option<native::logout::LogoutResult>,
}
#[derive(Debug)]
struct ForgeStep {
    disposition: StepDisposition,
}
impl ForgeClient {
    async fn connect(clients: &SeamClients<'_>, tag: u8, index: usize) -> TestResult<Self> {
        let generation = platform_generation(clients.descriptor, &clients.accounts[index]).await?;
        let token = sign_grant(
            &Grant {
                signing: clients.signing,
                key_id: clients.key_id,
                account_id: &clients.accounts[index],
                character_id: clients.characters[index],
                world_id: clients.world,
                channel_id: clients.channel,
                security_generation: generation,
                scope_generation: clients.scope_generation,
                nonce: [tag; 32],
                attempt: v7(tag, 0x05),
            },
            now_seconds()?,
        );
        let connector = connector(clients.certificate, &EXACT)?;
        let mut stream = tokio::time::timeout(Duration::from_secs(20), async {
            let tcp = TcpStream::connect(clients.address).await?;
            Ok::<_, Box<dyn std::error::Error>>(
                connector
                    .connect(ServerName::try_from("localhost")?, tcp)
                    .await?,
            )
        })
        .await
        .map_err(|_| "forge-native TLS deadline")??;
        if stream.get_ref().1.alpn_protocol() != Some(native::ALPN_OTERYN_GAME_V1.as_bytes()) {
            return Err("forge-native ALPN mismatch".into());
        }
        let supported = FORGE_SUPPORTED_CAPABILITIES;
        light_write(
            &mut stream,
            &native::encode_client_bootstrap(&native::ClientBootstrapValue {
                schema_revision: 1,
                character_id: CharacterId::decode(&clients.characters[index])?,
                admission_material: token.as_bytes(),
                client_build_id: "oteryn-e2e-forge",
                supported_capabilities: supported,
            })?,
        )
        .await?;
        let bytes = light_read(&mut stream).await?;
        let frame = native::decode_wire_envelope(&bytes)?;
        frame.validate(native::Direction::ServerToClient, false)?;
        if frame.message_type() != MessageType::ServerAccepted {
            return Err("forge-native ordinary admission refused".into());
        }
        let accepted = native::decode_server_accepted(frame.payload())?;
        if accepted.world_id != WorldId::decode(&clients.world)?
            || accepted.channel_id != ChannelId::decode(&clients.channel)?
            || accepted.schema_revision != 1
            || supported
                .iter()
                .any(|c| !accepted.selected_capabilities.contains(c))
            || accepted
                .selected_capabilities
                .iter()
                .any(|c| !supported.contains(c))
        {
            return Err("forge-native admission world/channel/capabilities mismatch".into());
        }
        let bytes = light_read(&mut stream).await?;
        let (begin, body) =
            raw_snapshot(&mut stream, accepted.connection_generation, &bytes).await?;
        if begin.target_server_sequence < accepted.current_server_sequence {
            return Err("forge-native snapshot commit mismatch".into());
        }
        let domains = native::decode_snapshot_body(&body)?;
        for id in [1, 3, 9, 10, 11, 14, 17, 21] {
            let domain = domains
                .iter()
                .find(|d| d.domain_id == id)
                .ok_or("Forge required snapshot domain absent")?;
            if id != 1 && domain.snapshot_type != 1 {
                return Err("Forge snapshot type mismatch".into());
            }
        }
        let domain = domains
            .iter()
            .find(|d| d.domain_id == 1)
            .ok_or("forge-native spatial snapshot absent")?;
        let (view, _) = spatial::decode_world_spatial_snapshot_view_lit(
            &accepted.selected_capabilities,
            domain.snapshot_type,
            domain.payload,
        )
        .map_err(|e| format!("forge-native spatial snapshot: {e:?}"))?;
        let spatial::WorldSpatialSnapshotView::Entities(view) = view else {
            return Err("forge-native entity snapshot absent".into());
        };
        let map_header = raw_map_snapshot(&domains, view.content_generation)?;
        let vitals = domains
            .iter()
            .find(|d| d.domain_id == 3)
            .ok_or("forge-native vitals snapshot absent")?;
        let vitals = casts::decode_actor_vitals(vitals.payload)
            .map_err(|e| format!("forge-native vitals snapshot: {e:?}"))?;
        let appearance = native::appearance::Snapshot::decode(
            domains
                .iter()
                .find(|d| d.domain_id == 21)
                .ok_or("forge appearance absent")?
                .payload,
        )
        .map_err(|e| format!("forge appearance:{e:?}"))?;
        if appearance.catalogue.context.content_generation != view.content_generation {
            return Err("forge appearance content mismatch".into());
        }
        let inventory = native::item_view::decode_character_inventory_with_equip_drop(
            domains
                .iter()
                .find(|d| d.domain_id == 9)
                .ok_or("forge inventory absent")?
                .payload,
            true,
        )
        .map_err(|e| format!("forge inventory:{e:?}"))?;
        let container = native::item_view::decode_open_container(
            domains
                .iter()
                .find(|d| d.domain_id == 11)
                .ok_or("forge container absent")?
                .payload,
        )
        .map_err(|e| format!("forge container:{e:?}"))?;
        native::container_tree::decode_container_views(
            domains
                .iter()
                .find(|d| d.domain_id == 14)
                .ok_or("forge tree absent")?
                .payload,
        )
        .map_err(|e| format!("forge tree:{e:?}"))?;
        Ok(Self {
            stream,
            generation: accepted.connection_generation,
            session: accepted.game_session_id,
            sequence: begin.target_server_sequence,
            command: accepted.next_command_id,
            probe: 0,
            selected: accepted.selected_capabilities,
            revisions: domains.iter().map(|d| (d.domain_id, d.revision)).collect(),
            vitals,
            position: view.actor_position,
            content: view.content_generation,
            map_header,
            entities: view.entities,
            bindings: appearance.bindings,
            inventory,
            container,
            released: false,
            tree: native::container_tree::decode_container_views(
                domains
                    .iter()
                    .find(|d| d.domain_id == 14)
                    .ok_or("forge tree absent")?
                    .payload,
            )
            .map_err(|e| format!("forge tree:{e:?}"))?,
        })
    }

    async fn receive(&mut self) -> TestResult<Option<(u64, CommandStatus, Vec<u8>)>> {
        let bytes = light_read(&mut self.stream).await?;
        let frame = native::decode_wire_envelope(&bytes)?;
        frame.validate(native::Direction::ServerToClient, true)?;
        if frame.connection_generation() != self.generation {
            return Err("Forge stale frame generation".into());
        }
        if frame.message_type() == MessageType::SnapshotBegin {
            let (sequence, revision, header) = raw_map_replacement(
                &mut self.stream,
                self.generation,
                &bytes,
                self.sequence,
                self.revisions
                    .get(&17)
                    .copied()
                    .ok_or("map revision absent")?,
                self.content,
            )
            .await?;
            self.sequence = sequence;
            self.revisions.insert(17, revision);
            self.map_header = header;
            return Ok(None);
        }
        if frame.message_type() == MessageType::LivenessProbe {
            let probe = native::decode_liveness_probe(frame.payload())?;
            if probe <= self.probe {
                return Err("Forge nonmonotonic probe".into());
            }
            self.probe = probe;
            light_write(
                &mut self.stream,
                &native::encode_liveness_ack(self.generation, probe, self.sequence)?,
            )
            .await?;
            return Ok(None);
        }
        if !matches!(
            frame.message_type(),
            MessageType::StateDelta | MessageType::CommandResult
        ) || Some(frame.server_sequence()) != self.sequence.checked_add(1)
        {
            return Err("Forge frame sequence/type mismatch".into());
        }
        self.sequence = frame.server_sequence();
        if frame.message_type() == MessageType::CommandResult {
            let r = native::decode_command_result(frame.payload())?;
            return Ok(Some((r.command_id, r.status, r.payload.to_vec())));
        }
        let delta = native::decode_state_delta(frame.payload())?;
        if delta.base_revision != self.revisions.get(&delta.domain_id).copied().unwrap_or(0)
            || delta.new_revision <= delta.base_revision
        {
            return Err("Forge domain revision mismatch".into());
        }
        if !matches!(delta.domain_id, 1 | 21) && delta.delta_type != 1 {
            return Err("Forge unregistered delta type".into());
        }
        match delta.domain_id {
            1 => {
                let (view, _) = spatial::decode_world_spatial_delta_view_lit(
                    &self.selected,
                    delta.delta_type,
                    delta.payload,
                )
                .map_err(|e| format!("Forge spatial:{e:?}"))?;
                let spatial::WorldSpatialDeltaView::Entities(view) = view else {
                    return Err("Forge entity delta absent".into());
                };
                if view.content_generation != self.content {
                    return Err("Forge content generation changed".into());
                }
                self.position = view.actor_position;
                self.entities.retain(|e| !view.leave.contains(&e.entity));
                for entity in view.enter.into_iter().chain(view.update) {
                    self.entities.retain(|e| e.entity != entity.entity);
                    self.entities.push(entity);
                }
            }
            17 => {
                self.map_header = raw_map_delta(&self.map_header, delta.payload)?;
            }
            3 => {
                self.vitals = casts::decode_actor_vitals(delta.payload)
                    .map_err(|e| format!("Forge vitals:{e:?}"))?;
            }
            10 => {
                attacks::decode_actor_combat_state(delta.payload)
                    .map_err(|e| format!("Forge combat:{e:?}"))?;
            }
            9 => {
                self.inventory = native::item_view::decode_character_inventory_with_equip_drop(
                    delta.payload,
                    true,
                )
                .map_err(|e| format!("forge inventory:{e:?}"))?
            }
            11 => {
                self.container = native::item_view::decode_open_container(delta.payload)
                    .map_err(|e| format!("forge container:{e:?}"))?
            }
            14 => {
                self.tree = native::container_tree::decode_container_views(delta.payload)
                    .map_err(|e| format!("forge tree:{e:?}"))?
            }
            21 => match delta.delta_type {
                1 => {
                    let mappings = native::appearance::Mappings::decode(delta.payload)
                        .map_err(|e| format!("forge mappings:{e:?}"))?;
                    if mappings.context.content_generation != self.content {
                        return Err("forge mappings content changed".into());
                    }
                    self.bindings
                        .retain(|b| !mappings.forgets.contains(&b.entity));
                    for binding in mappings.upserts {
                        self.bindings.retain(|b| b.entity != binding.entity);
                        self.bindings.push(binding);
                    }
                }
                2 => {
                    let catalogue = native::appearance::Catalogue::decode(delta.payload)
                        .map_err(|e| format!("forge catalogue:{e:?}"))?;
                    if catalogue.context.content_generation != self.content {
                        return Err("forge catalogue content changed".into());
                    }
                }
                _ => return Err("forge appearance delta type".into()),
            },
            _ => return Err(format!("Forge unexpected domain:{}", delta.domain_id).into()),
        }
        self.revisions.insert(delta.domain_id, delta.new_revision);
        Ok(None)
    }
    async fn gated_command(
        &mut self,
        capability: u32,
        kind: u32,
        payload: &[u8],
    ) -> TestResult<ForgeReply> {
        if !self.selected.contains(&capability) {
            return Err(format!("Forge capability {capability} not selected").into());
        }
        let id = self.command;
        self.command = self
            .command
            .checked_add(1)
            .ok_or("Forge command overflow")?;
        light_write(
            &mut self.stream,
            &native::encode_client_command(
                self.generation,
                &native::ClientCommandValue {
                    command_id: id,
                    command_type: kind,
                    payload,
                },
            )?,
        )
        .await?;
        loop {
            if let Some((seen, status, payload)) = self.receive().await? {
                if seen != id {
                    return Err("Forge command result correlation mismatch".into());
                }
                return Ok(ForgeReply { status, payload });
            }
        }
    }
    async fn service_liveness(&mut self, duration: Duration) -> TestResult {
        let until = tokio::time::Instant::now() + duration;
        loop {
            if tokio::time::timeout_at(until, self.stream.get_ref().0.readable())
                .await
                .is_err()
            {
                return Ok(());
            }
            if self.receive().await?.is_some() {
                return Err("Forge unsolicited command result".into());
            }
        }
    }
    async fn attack_target(
        &mut self,
        target: Option<&spatial::EntityRef>,
    ) -> TestResult<ForgeAttack> {
        let payload = attacks::encode_attack_target_intent(target)
            .map_err(|e| format!("Forge attack:{e:?}"))?;
        let reply = self
            .gated_command(17, attacks::COMMAND_TYPE_ATTACK_TARGET_INTENT, &payload)
            .await?;
        let disposition = attacks::decode_attack_intent_result(&reply.payload)
            .map_err(|e| format!("Forge attack result:{e:?}"))?;
        if (reply.status == CommandStatus::Accepted)
            != (disposition != attacks::AttackIntentDisposition::Rejected)
        {
            return Err("Forge attack result status mismatch".into());
        }
        Ok(ForgeAttack { disposition })
    }
    async fn logout(&mut self) -> TestResult<ForgeLogout> {
        let reply = self
            .gated_command(21, native::logout::COMMAND_TYPE_LOGOUT_INTENT, &[])
            .await?;
        let result = if reply.payload.is_empty() {
            None
        } else {
            Some(
                native::logout::decode_logout_result(&reply.payload)
                    .map_err(|e| format!("Forge logout:{e:?}"))?,
            )
        };
        if result.is_some_and(|r| r.is_rejected() != (reply.status == CommandStatus::Rejected)) {
            return Err(format!("Forge logout status mismatch: status={:?}; result={result:?}", reply.status).into());
        }
        self.released = result == Some(native::logout::LogoutResult::Accepted);
        Ok(ForgeLogout { status: reply.status, result })
    }
    async fn step(&mut self, direction: StepDirection) -> TestResult<ForgeStep> {
        let reply = self
            .gated_command(6, 1, &encode_step_intent(direction))
            .await?;
        let disposition = native::world_spatial::decode_step_result_paced(&reply.payload, true)
            .map_err(|e| format!("Forge step:{e:?}"))?;
        Ok(ForgeStep { disposition })
    }
}

const FORGE_ACTIONS: &[&str] = &[
    "insufficient-dust-refusal",
    "influenced-selection-combat",
    "fiendish-selection-combat",
    "fiendish-corpse-slivers",
    "retired-target-refusal",
    "earned-dust-conversion",
    "fund-durable-dust",
    "fresh-admission-slivers",
    "fresh-admission-dust",
    "receipt-replay-no-duplicate",
    "final-logout",
];

async fn forge_row(clients: &SeamClients<'_>) {
    let mut row = JourneyRow::new("P19", FORGE_ACTIONS);
    let mut connection = None;
    let result = async {
        let world = qualification_world(clients)?;
        connection = Some(ForgeClient::connect(clients, 0xd1, 0).await?);
        forge_journey(
            &mut row,
            connection.as_mut().ok_or("forge admission absent")?,
            clients,
            &world,
        )
        .await
    };
    let mut result = tokio::time::timeout(Duration::from_secs(1_800), result)
        .await
        .unwrap_or_else(|_| {
            Err(
                "P19: ordinary hunt/dust journey exceeded 1800s; remaining observations unproven"
                    .into(),
            )
        });
    if let Some(client) = connection.as_mut() {
        let cleanup = async {
            client.attack_target(None).await?;
            forge_release(client).await
        };
        match tokio::time::timeout(Duration::from_secs(940), cleanup).await {
            Ok(Ok(())) => row.pass("final-logout"),
            other => result = Err(format!("P19 cleanup={other:?};journey={result:?}").into()),
        }
    }
    row.finish(&result);
}

impl ForgeClient {
    // A native turn is a barrier after command acknowledgements, which may precede
    // their inventory/spatial deltas. No elapsed-time-only success is accepted.
    async fn barrier(&mut self) -> TestResult {
        let mut payload = Vec::new();
        bytes_field(&mut payload, 3, &encode_step_intent(StepDirection::North));
        let reply = self
            .gated_command(
                64,
                native::target_move::COMMAND_TYPE_TARGET_MOVE_INTENT,
                &payload,
            )
            .await?;
        if reply.status != CommandStatus::Accepted || reply.payload != [8, 1] {
            return Err(format!("forge turn refused:{reply:?}").into());
        }
        self.service_liveness(Duration::from_millis(50)).await?;
        Ok(())
    }
    fn slivers(&self, definition: std::num::NonZeroU32) -> u64 {
        self.inventory
            .entries
            .iter()
            .chain(self.inventory.equipment.iter().map(|e| &e.item))
            .filter(|e| e.item_definition_ref == definition)
            .map(|e| u64::from(e.count.get()))
            .sum()
    }
    async fn walk(&mut self, world: &BundleWorld, path: &[StepDirection]) -> TestResult {
        for direction in path {
            let before = tile(self.position)?;
            let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
            loop {
                let reply = self.step(*direction).await?;
                if reply.disposition == StepDisposition::Moved {
                    break;
                }
                if !matches!(reply.disposition, StepDisposition::TooEarly)
                    || tokio::time::Instant::now() >= deadline
                {
                    return Err(format!("forge real route step:{reply:?};at={before:?}").into());
                }
                self.barrier().await?;
            }
            self.barrier().await?;
            if Some(tile(self.position)?) != destination(world, before, *direction) {
                return Err("forge STEP endpoint frame mismatch".into());
            }
        }
        Ok(())
    }
    async fn forge(
        &mut self,
        operation: native::item_forge::Operation,
        occurrence: [u8; 16],
    ) -> TestResult<serde_json::Value> {
        let payload = native::item_forge::Request {
            operation,
            occurrence,
            first_item: None,
            second_item: None,
            boost: false,
            protect: false,
            history_limit: 0,
        }
        .encode()
        .ok_or("forge request invalid")?;
        // Every occurrence has a retained terminal result, including COOLDOWN.
        // Service the connection through the ordinary 490ms action interval before
        // issuing it; a retained refusal must never be retried as a new outcome.
        self.service_liveness(Duration::from_millis(550)).await?;
        let reply = self.gated_command(152, 181, &payload).await?;
        if reply.status != CommandStatus::Accepted || reply.payload.is_empty() {
            return Err(format!("native Forge route refused:{reply:?}").into());
        }
        let value: serde_json::Value = serde_json::from_slice(&reply.payload)?;
        self.barrier().await?;
        Ok(value)
    }
}

async fn forge_release(client: &mut ForgeClient) -> TestResult {
    if client.released {
        return Ok(());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(920);
    loop {
        let reply = client.logout().await?;
        match reply.result {
            Some(native::logout::LogoutResult::Accepted) => return Ok(()),
            Some(
                native::logout::LogoutResult::InFight { .. } | native::logout::LogoutResult::Busy,
            ) if tokio::time::Instant::now() < deadline => client.barrier().await?,
            _ => return Err(format!("forge ordinary logout: status={:?}; result={:?}", reply.status, reply.result).into()),
        }
    }
}

fn forge_dust_result(value: &serde_json::Value, operation: &str) -> TestResult<(u64, u64)> {
    if value["operation"] != operation
        || value["succeeded"] != true
        || value["reason"] != serde_json::Value::Null
    {
        return Err(format!("ordinary dust acquisition/operation blocked:{value}").into());
    }
    let dust = value["result"]["dust"]
        .as_u64()
        .ok_or("forge result omitted dust")?;
    let limit = value["result"]["dust_limit"]
        .as_u64()
        .ok_or("forge result omitted dust limit")?;
    if !(100..=4600).contains(&limit) || (limit - 100) % 20 != 0 || dust > limit {
        return Err("native forge dust bounds invalid".into());
    }
    Ok((dust, limit))
}
fn forge_dust_interval(before: (u64, u64), stacks: u8, limit: u64) -> TestResult<(u64, u64)> {
    if !matches!(stacks, 1..=5 | 15)
        || before.0 > before.1
        || before.1 > limit
        || !(100..=4600).contains(&limit)
    {
        return Err("invalid observed forge dust interval".into());
    }
    Ok((
        (before.0 + u64::from(stacks)).min(limit),
        (before.1 + 3 * u64::from(stacks)).min(limit),
    ))
}
fn forge_conversion(
    value: &serde_json::Value,
    before_slivers: u64,
    after_slivers: u64,
) -> TestResult<(u64, u64)> {
    let dust = forge_dust_result(value, "dust_to_slivers")?;
    if value["applied"] != true
        || value["result"]["slivers_created"] != 3
        || value["result"]["slivers_spent"] != 0
        || value["result"]["gold_spent"] != 0
        || value["result"]["cores_spent"] != 0
        || before_slivers.checked_add(3) != Some(after_slivers)
    {
        return Err(format!("forge conversion custody/fee mismatch:{value};slivers={before_slivers}->{after_slivers}").into());
    }
    Ok(dust)
}

// Visit actual compiled spawn approaches and select only a served appearance marker.
// The native population owner chooses species/stacks; the test never classifies by name.
async fn forge_hunt(
    client: &mut ForgeClient,
    world: &BundleWorld,
    fiendish: bool,
    sliver: std::num::NonZeroU32,
) -> TestResult<(u8, spatial::EntityRef)> {
    let bytes = fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let meta = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let mut points: Vec<_> = meta
        .spawns
        .sources
        .iter()
        .flat_map(|s| {
            s.points.iter().map(move |p| TilePos {
                x: p.x,
                y: p.y,
                floor: s.floor,
            })
        })
        .collect();
    points.sort_by_key(|p| (p.floor, p.x, p.y));
    points.dedup();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(330);
    let mut visits = 0;
    let target = loop {
        client.barrier().await?;
        let found = client.entities.iter().find_map(|entity| {
            if entity.kind != spatial::EntityKind::Creature {
                return None;
            }
            let spatial::EntityDetail::Actor { appearance_ref, .. } = entity.detail else {
                return None;
            };
            let stacks = client
                .bindings
                .iter()
                .find(|b| b.entity == entity.entity && b.appearance_ref == appearance_ref)?
                .forge_stacks?;
            ((fiendish && stacks == 15) || (!fiendish && (1..=5).contains(&stacks)))
                .then_some((*entity, stacks))
        });
        if let Some(found) = found {
            break found;
        }
        if tokio::time::Instant::now() >= deadline || points.is_empty() {
            return Err(format!("forge-selection-blocked:required={};served-marker-absent;reachable-spawn-visits={visits};budget=330s,160-tiles;no-global-absence-inferred",if fiendish{"fiendish-15"}else{"influenced-1..5"}).into());
        }
        let point = points[visits % points.len()];
        visits += 1;
        let path = route(world, tile(client.position)?, |at| {
            at.floor == point.floor
                && at.x.abs_diff(point.x).max(at.y.abs_diff(point.y)) <= 1
                && world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|t| !t.protection_zone())
        });
        if let Some(path) = path {
            client.walk(world, &path).await?;
        }
    };
    let (entity, stacks) = target;
    let position = tile(entity.position)?;
    let path = route(world, tile(client.position)?, |at| {
        at.floor == position.floor
            && at.x.abs_diff(position.x).max(at.y.abs_diff(position.y)) <= 1
            && world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|t| !t.protection_zone())
    })
    .ok_or("observed forge creature has no reachable unprotected melee approach")?;
    client.walk(world, &path).await?;
    let xp_before = client.experience().await?;
    let attack = client.attack_target(Some(&entity.entity)).await?;
    if attack.disposition != attacks::AttackIntentDisposition::Ok {
        return Err(format!(
            "forge ordinary combat blocked:stacks={stacks};attack={attack:?};vitals={:?}",
            client.vitals
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let mut damaged = false;
    let mut death_position = entity.position;
    loop {
        client.barrier().await?;
        if let Some(live) = client.entities.iter().find(|e| e.entity == entity.entity) {
            death_position = live.position;
            if let (
                spatial::EntityDetail::Actor {
                    health_percent: before,
                    ..
                },
                spatial::EntityDetail::Actor {
                    health_percent: now,
                    ..
                },
            ) = (entity.detail, live.detail)
            {
                damaged |= now < before;
            }
        } else {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "forge ordinary kill blocked:stacks={stacks};damage-frame={damaged};vitals={:?}",
                client.vitals
            )
            .into());
        }
    }
    client.attack_target(None).await?;
    client.barrier().await?;
    if client.experience().await? <= xp_before {
        return Err("forge target vanished without earned experience".into());
    }
    let corpse = client
        .entities
        .iter()
        .find_map(|e| {
            if e.kind == spatial::EntityKind::Corpse
                && e.position == death_position
                && let spatial::EntityDetail::Object { item_handle, .. } = e.detail
            {
                return item_handle;
            }
            None
        })
        .ok_or("forge death observed without matching native corpse")?;
    if fiendish {
        use native::container_tree as tree;
        let payload = tree::encode_container_view_intent(&tree::ContainerViewIntent::Open {
            handle: corpse,
            replace_view: Some(0),
        })
        .map_err(|e| format!("forge corpse input:{e:?}"))?;
        let reply = client.gated_command(14, 21, &payload).await?;
        if reply.status != CommandStatus::Accepted
            || tree::decode_container_view_result(&reply.payload)
                .map_err(|e| format!("forge corpse:{e:?}"))?
                != tree::ContainerViewOutcome::Opened
        {
            return Err(format!("forge corpse open refused:{reply:?}").into());
        }
        client.barrier().await?;
        let view = client
            .tree
            .views
            .iter()
            .find(|v| v.container_handle == corpse)
            .ok_or("forge corpse view frame absent")?;
        let loot: Vec<_> = view
            .entries
            .iter()
            .filter(|e| e.item_definition_ref == sliver)
            .copied()
            .collect();
        let amount: u64 = loot.iter().map(|e| u64::from(e.count.get())).sum();
        if !(3..=7).contains(&amount) {
            return Err(format!("fiendish corpse slivers expected 3..7,served={amount}").into());
        }
        let before = client.slivers(sliver);
        for item in loot {
            let payload =
                native::item_view::encode_item_move_intent(&native::item_view::ItemMoveIntent {
                    source: item.handle,
                    destination: native::item_view::ItemMoveDestination::MainBackpack,
                })
                .map_err(|e| format!("forge loot input:{e:?}"))?;
            let reply = client.gated_command(4, 9, &payload).await?;
            if reply.status != CommandStatus::Accepted
                || native::item_view::decode_item_move_result_with_equip_drop(&reply.payload, true)
                    .map_err(|e| format!("forge loot:{e:?}"))?
                    != native::item_view::ItemMoveOutcome::Moved
            {
                return Err(format!("fiendish sliver loot refused:{reply:?}").into());
            }
        }
        client.barrier().await?;
        if client.slivers(sliver) != before + amount {
            return Err("fiendish corpse/inventory sliver delta mismatch".into());
        }
        let payload =
            tree::encode_container_view_intent(&tree::ContainerViewIntent::Close { view_id: 0 })
                .map_err(|e| format!("forge close:{e:?}"))?;
        let reply = client.gated_command(14, 21, &payload).await?;
        if reply.status != CommandStatus::Accepted
            || tree::decode_container_view_result(&reply.payload)
                .map_err(|e| format!("forge close:{e:?}"))?
                != tree::ContainerViewOutcome::Closed
        {
            return Err("forge corpse close refused".into());
        }
    }
    Ok((stacks, entity.entity))
}

async fn forge_journey(
    row: &mut JourneyRow,
    client: &mut ForgeClient,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    use native::item_forge::Operation;
    let index = provisioning::item_index()?;
    let sliver = provisioning::item_reference(&index, 37109)?;
    let mut ordinal = 0u8;
    let occurrence_session = client.session;
    // Consume only genuinely held dust via the ordinary recipe. Establish <60
    // before combat, so a later successful recipe proves newly earned dust.
    loop {
        ordinal = ordinal
            .checked_add(1)
            .ok_or("forge baseline budget exhausted")?;
        let before = client.slivers(sliver);
        let value = client
            .forge(
                Operation::DustToSlivers,
                forge_occurrence(occurrence_session, ordinal),
            )
            .await?;
        if value["reason"] == "INSUFFICIENT_DUST" {
            if value["applied"] != false
                || value["succeeded"] != false
                || value["result"] != serde_json::Value::Null
                || client.slivers(sliver) != before
            {
                return Err("insufficient dust refusal changed custody".into());
            }
            row.pass("insufficient-dust-refusal");
            break;
        }
        forge_conversion(&value, before, client.slivers(sliver))?;
        if ordinal >= 80 {
            return Err("forge baseline drain exceeded 80 ordinary conversions".into());
        }
    }
    forge_hunt(client, world, false, sliver).await?;
    row.pass("influenced-selection-combat");
    let (_, retired) = forge_hunt(client, world, true, sliver).await?;
    row.pass("fiendish-selection-combat");
    row.pass("fiendish-corpse-slivers");
    let before = client.slivers(sliver);
    let xp = client.experience().await?;
    let attack = client.attack_target(Some(&retired)).await?;
    client.barrier().await?;
    if attack.disposition != attacks::AttackIntentDisposition::TargetNotVisible
        || client.slivers(sliver) != before
        || client.experience().await? != xp
    {
        return Err(format!("retired forge target not refused without reward:{attack:?}").into());
    }
    row.pass("retired-target-refusal");
    let mut attempted_kills = 0u8;
    let (mut dust, limit) = loop {
        ordinal = ordinal
            .checked_add(1)
            .ok_or("forge hunt occurrence budget exhausted")?;
        let before = client.slivers(sliver);
        let value = client
            .forge(
                Operation::DustToSlivers,
                forge_occurrence(occurrence_session, ordinal),
            )
            .await?;
        if value["reason"] == "INSUFFICIENT_DUST" {
            if value["applied"] != false || client.slivers(sliver) != before {
                return Err("dust acquisition refusal changed slivers".into());
            }
            attempted_kills += 1;
            if attempted_kills > 60 {
                return Err("earned-dust-acquisition-blocked:required=60;native-reason=INSUFFICIENT_DUST;ordinary-influenced-kills=60;premium-only-dust-gate=source:crates/gameplay/domain/src/domain/forge_creature.rs:93;no-premium-grant-attempted".into());
            }
            evidence(
                "E2EF7_GAP P19 dust-acquisition pending required=60 native-reason=INSUFFICIENT_DUST; continuing ordinary influenced hunts",
            );
            forge_hunt(client,world,false,sliver).await.map_err(|e|format!("earned-dust-acquisition-blocked:required=60;ordinary-influenced-hunt={e};premium-dust-gate=crates/gameplay/domain/src/domain/forge_creature.rs:93"))?;
            continue;
        }
        break forge_conversion(&value, before, client.slivers(sliver))?;
    };
    row.pass("earned-dust-conversion");
    // A fresh recipe after re-admission observes the actual current dust balance.
    // At the initial cap an ordinary limit upgrade leaves >=60; at later caps
    // an ordinary conversion leaves >=60. Receipt replay alone proves no balance.
    let upgrade = limit == 100;
    let threshold = if upgrade { 85 } else { 120 };
    let mut bounds = (dust, dust);
    while bounds.0 < threshold {
        let (stacks, _) = forge_hunt(client, world, false, sliver).await?;
        bounds = forge_dust_interval(bounds, stacks, limit)?;
    }
    ordinal = ordinal
        .checked_add(1)
        .ok_or("forge operation budget exhausted")?;
    let before = client.slivers(sliver);
    let value = client
        .forge(
            if upgrade {
                Operation::DustLimit
            } else {
                Operation::DustToSlivers
            },
            forge_occurrence(occurrence_session, ordinal),
        )
        .await?;
    let (served, new_limit) = if upgrade {
        forge_dust_result(&value, "dust_limit")?
    } else {
        forge_conversion(&value, before, client.slivers(sliver))?
    };
    let charge = if upgrade { 25 } else { 60 };
    if served + charge < bounds.0
        || served + charge > bounds.1
        || new_limit != limit + if upgrade { 20 } else { 0 }
        || (upgrade && client.slivers(sliver) != before)
    {
        return Err(format!(
            "earned dust bounds/cap/payment mismatch:before={bounds:?};reply={value}"
        )
        .into());
    }
    dust = served;
    if dust < 60 {
        return Err("ordinary dust funding left fewer than 60 for persistence assertion".into());
    }
    row.pass("fund-durable-dust");
    let held = client.slivers(sliver);
    let xp = client.experience().await?;
    let position = client.position;
    let old = client.session;
    forge_release(client).await?;
    *client = ForgeClient::connect(clients, 0xd3, 0).await?;
    if client.session == old
        || client.position != position
        || client.slivers(sliver) != held
        || client.experience().await? != xp
    {
        return Err("fresh Forge admission did not retain sliver custody/position".into());
    }
    row.pass("fresh-admission-slivers");
    let occurrence = forge_occurrence(client.session, 1);
    let value = client.forge(Operation::DustToSlivers, occurrence).await?;
    let (after, after_limit) = forge_conversion(&value, held, client.slivers(sliver))?;
    if after + 60 != dust || after_limit != new_limit {
        return Err("fresh Forge admission lost/duplicated earned dust or limit".into());
    }
    row.pass("fresh-admission-dust");
    let held = client.slivers(sliver);
    let old = client.session;
    forge_release(client).await?;
    *client = ForgeClient::connect(clients, 0xd5, 0).await?;
    if client.session == old || client.slivers(sliver) != held {
        return Err("second Forge admission lost conversion slivers".into());
    }
    let replay = client.forge(Operation::DustToSlivers, occurrence).await?;
    if replay != value || client.slivers(sliver) != held {
        return Err("Forge receipt replay duplicated reward or changed retained result".into());
    }
    row.pass("receipt-replay-no-duplicate");
    Ok(())
}

fn forge_occurrence(session: native::GameSessionId, ordinal: u8) -> [u8; 16] {
    use sha2::Digest;
    let digest = sha2::Sha256::new()
        .chain_update(b"E2EF7/P19/occurrence")
        .chain_update(session.as_bytes())
        .chain_update([ordinal])
        .finalize();
    let mut id = [0u8; 16];
    id.copy_from_slice(&digest[..16]);
    id[6] = (id[6] & 15) | 0x70;
    id[8] = (id[8] & 63) | 0x80;
    id
}
impl ForgeClient {
    async fn experience(&mut self) -> TestResult<u64> {
        let reply = self
            .gated_command(
                199,
                character::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
                &[character::CharacterTab::GeneralStats as u8, 0, 0, 50],
            )
            .await?;
        if reply.status != CommandStatus::Accepted
            || reply.payload.first() != Some(&(character::CharacterTab::GeneralStats as u8))
        {
            return Err(format!("forge experience read refused:{reply:?}").into());
        }
        Ok(u64::from_le_bytes(
            reply
                .payload
                .get(1..9)
                .ok_or("truncated forge GeneralStats")?
                .try_into()?,
        ))
    }
}

#[cfg(test)]
mod forge_tests {
    use super::*;
    #[test]
    fn forge_intervals_follow_stacks_and_saturate_at_cap() -> TestResult {
        assert_eq!(forge_dust_interval((0, 59), 5, 100)?, (5, 74));
        assert_eq!(forge_dust_interval((90, 99), 15, 100)?, (100, 100));
        assert!(forge_dust_interval((0, 0), 0, 100).is_err());
        assert!(forge_dust_interval((10, 9), 5, 100).is_err());
        assert!(forge_dust_interval((0, 101), 5, 100).is_err());
        Ok(())
    }
    #[test]
    fn forge_conversion_requires_exact_custody_fees_and_success() -> TestResult {
        let value = serde_json::json!({"operation":"dust_to_slivers","succeeded":true,"applied":true,"reason":null,"result":{"dust":24,"dust_limit":100,"slivers_created":3,"slivers_spent":0,"gold_spent":0,"cores_spent":0}});
        assert_eq!(forge_conversion(&value, 7, 10)?, (24, 100));
        assert!(forge_conversion(&value, 7, 9).is_err());
        for (field, wrong) in [
            ("dust", 101),
            ("dust_limit", 101),
            ("slivers_created", 2),
            ("gold_spent", 1),
            ("cores_spent", 1),
            ("slivers_spent", 1),
        ] {
            let mut bad = value.clone();
            bad["result"][field] = wrong.into();
            assert!(forge_conversion(&bad, 7, 10).is_err(), "{field}");
        }
        let mut bad = value.clone();
        bad["succeeded"] = false.into();
        assert!(forge_conversion(&bad, 7, 10).is_err());
        assert!(forge_dust_result(&value, "dust_limit").is_err());
        Ok(())
    }
    #[test]
    fn forge_action_set_requires_persistence_replay_and_cleanup() {
        let mut row = JourneyRow::new("P19", FORGE_ACTIONS);
        for action in FORGE_ACTIONS.iter().take(FORGE_ACTIONS.len() - 1) {
            row.pass(action);
        }
        assert!(!row.actions.complete());
        row.pass("final-logout");
        assert!(row.actions.complete());
        row.pass("final-logout");
        assert!(!row.actions.complete());
        let mut row = JourneyRow::new("P19", FORGE_ACTIONS);
        row.pass("probe");
        assert!(!row.actions.complete());
    }
    #[test]
    fn forge_occurrences_are_session_scoped_and_valid_v7() -> TestResult {
        let first = native::GameSessionId::decode(&v7(1, 1))?;
        let second = native::GameSessionId::decode(&v7(2, 1))?;
        assert_eq!(forge_occurrence(first, 1), forge_occurrence(first, 1));
        assert_ne!(forge_occurrence(first, 1), forge_occurrence(first, 2));
        assert_ne!(forge_occurrence(first, 1), forge_occurrence(second, 1));
        native::GameSessionId::decode(&forge_occurrence(first, 1))?;
        Ok(())
    }
}
