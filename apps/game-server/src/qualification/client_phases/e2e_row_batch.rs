// One node-boot job cannot run every row journey inside its timeout (each row
// needs about 80 s), so CI runs one batch per head. `OTERYN_E2E_ROW_BATCH` is
// `start:len` over the row indices passed to `e2e_row_selected` in `journeys`;
// `all` runs every row.
const E2E_ROW_BATCH_DEFAULT: (usize, usize) = (1, 15);

fn e2e_row_batch(value: Option<&str>) -> Option<(usize, usize)> {
    match value {
        None => Some(E2E_ROW_BATCH_DEFAULT),
        Some("all") => Some((0, usize::MAX)),
        Some(text) => {
            let (start, len) = text.split_once(':')?;
            let len = len.parse().ok()?;
            (len > 0).then_some((start.parse().ok()?, len))
        }
    }
}

// A failed row can leave its socket open or its session in loss/grace; the
// next row must not inherit that state.
async fn release_row_characters(clients: &SeamClients<'_>) -> TestResult {
    let mut failure = None;
    for index in 0..clients.accounts.len() {
        if let Err(error) = wait_character_released(clients, index).await {
            evidence(&format!("e2e_row_release account={index} error={error}"));
            if failure.is_none() {
                failure = Some(error);
            }
        }
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

#[cfg(test)]
mod e2e_row_batch_tests {
    use super::*;

    #[test]
    fn e2e_row_batch_starts_at_first_live_index_and_rejects_zero_length() {
        assert_eq!(e2e_row_batch(None), Some((1, 15)));
        assert_eq!(e2e_row_batch(Some("1:0")), None);
    }

    #[test]
    fn e2e_row_batch_parses_default_all_and_ranges() {
        assert_eq!(e2e_row_batch(None), Some(E2E_ROW_BATCH_DEFAULT));
        assert_eq!(e2e_row_batch(Some("all")), Some((0, usize::MAX)));
        assert_eq!(e2e_row_batch(Some("20:20")), Some((20, 20)));
        assert_eq!(e2e_row_batch(Some("20")), None);
        assert_eq!(e2e_row_batch(Some("x:1")), None);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum E2eMode {
    #[default]
    Journey,
    Isolated,
}

impl E2eMode {
    fn parse(value: Option<&str>) -> TestResult<Self> {
        match value {
            None | Some("journey") => Ok(Self::Journey),
            Some("isolated") => Ok(Self::Isolated),
            _ => Err("invalid OTERYN_E2E_MODE (expected journey or isolated)".into()),
        }
    }

    fn from_env() -> TestResult<Self> {
        match std::env::var("OTERYN_E2E_MODE") {
            Ok(value) => Self::parse(Some(&value)),
            Err(std::env::VarError::NotPresent) => Self::parse(None),
            Err(error) => Err(error.into()),
        }
    }

    fn permits_profile(self, profile: &std::ffi::OsStr) -> bool {
        // Channel ownership setup carries no character grant.
        profile == "private-channel"
            || (self == Self::Isolated
                && profile.to_str().is_some_and(|profile| profile.starts_with("row-")))
    }

    fn name(self) -> &'static str {
        match self {
            Self::Journey => "journey",
            Self::Isolated => "isolated",
        }
    }

    async fn setup<T>(
        self,
        fresh: impl Future<Output = TestResult<T>>,
    ) -> TestResult<Option<T>> {
        match self {
            Self::Journey => Ok(None),
            Self::Isolated => fresh.await.map(Some),
        }
    }
}

struct IsolatedRow {
    accounts: [String; 2],
    characters: [[u8; 16]; 2],
}

async fn isolated_row_characters(
    clients: &SeamClients<'_>,
    scenario: &str,
) -> TestResult<IsolatedRow> {
    let row = scenario.split('.').next().ok_or("row ID absent")?;
    let requirements = crate::qualification_requirements::RowRequirements::load(row)
        .ok_or("row requirements missing")?;
    let index = usize::from(row != "C17d");
    let (vocation, level, premium) = requirements.build(native_spell_book()?)?;
    let (first_account, first_character) =
        full_e2ef5::fresh_row_character(clients, 0, premium && index == 0).await?;
    let (second_account, second_character) =
        full_e2ef5::fresh_row_character(clients, 1, premium && index == 1).await?;
    let isolated = IsolatedRow {
        accounts: [first_account, second_account],
        characters: [first_character, second_character],
    };
    if requirements.level.is_none()
        && requirements.vocation.is_none()
        && requirements.spells.is_empty()
        && requirements.items.is_empty()
        && requirements.equipment.is_empty()
    {
        evidence(&format!("e2e_row scenario={scenario} granted: empty"));
        return Ok(isolated);
    }
    let row_clients = SeamClients {
        accounts: &isolated.accounts,
        characters: &isolated.characters,
        ..*clients
    };
    if matches!(row, "C20c" | "P16") {
        let mut peer = connect_character(&row_clients, 0x71, 0).await?;
        full_e2ef5::apply_fixture(&mut peer, &format!("row-{row}")).await?;
        full_e2ef5::logout(&mut peer).await?;
    }
    let mut client = connect_character(&row_clients, 0x71, index).await?;
    full_e2ef5::apply_fixture(&mut client, &format!("row-{row}")).await?;
    full_e2ef5::logout(&mut client).await?;
    drop(client);
    let mut client = connect_character(&row_clients, 0x71, index).await?;
    if full_e2ef5::level(&mut client).await? != level {
        return Err("fresh admission did not load the row's level".into());
    }
    if !requirements.items.is_empty() || !requirements.equipment.is_empty() {
        let definitions = provisioning::item_index()?;
        for (&id, &quantity) in &requirements.items {
            let reference = provisioning::item_reference(&definitions, id)?;
            let inventory = client.inventory().ok_or("row inventory absent")?;
            let observed: u64 = inventory.entries.iter().chain(inventory.main_backpack.iter())
                .filter(|entry| entry.item_definition_ref == reference)
                .map(|entry| u64::from(entry.count.get())).sum();
            if observed != u64::from(quantity) {
                return Err(format!("row item {id} quantity={observed} expected={quantity}").into());
            }
        }
        for id in &requirements.equipment {
            let reference = provisioning::item_reference(&definitions, *id)?;
            let response = client.session_mut().equip_item_definition(reference).await?;
            if response.status != CommandStatus::Accepted
                || response.outcome != oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved
            {
                return Err(format!("row equipment {id} equip refused").into());
            }
        }
    }
    full_e2ef5::logout(&mut client).await?;
    let items = requirements.items.iter().map(|(id, quantity)| format!("{id}x{quantity}")).collect::<Vec<_>>().join(",");
    let equipment = requirements.equipment.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
    evidence(&format!(
        "e2e_row scenario={scenario} granted: level={level} vocation={vocation} premium={premium} items={items} equipment={equipment}",
    ));
    Ok(isolated)
}

macro_rules! row_clients {
    ($mode:expr, $clients:ident, $scenario:expr) => {
        let isolated = $mode
            .setup(isolated_row_characters($clients, $scenario))
            .await?;
        let isolated_clients = isolated.as_ref().map(|row| SeamClients {
            accounts: &row.accounts,
            characters: &row.characters,
            ..*$clients
        });
        let $clients = isolated_clients.as_ref().unwrap_or($clients);
    };
}

#[tokio::test]
async fn qualification_journey_mode_never_polls_the_grant_fixture() -> TestResult {
    use std::cell::Cell;
    let grants = Cell::new(0);
    let grant = async {
        grants.set(grants.get() + 1);
        Ok(())
    };
    assert_eq!(E2eMode::parse(None)?.setup(grant).await?, None);
    assert_eq!(grants.get(), 0);
    assert!(!E2eMode::Journey.permits_profile("row-C17a".as_ref()));
    assert!(!E2eMode::Journey.permits_profile("c17a".as_ref()));
    assert!(!E2eMode::Isolated.permits_profile("c17a".as_ref()));
    assert!(E2eMode::Isolated.permits_profile("row-C17a".as_ref()));
    assert_eq!(
        E2eMode::parse(Some("isolated"))?
            .setup(async {
                grants.set(grants.get() + 1);
                Ok(())
            })
            .await?,
        Some(())
    );
    assert_eq!(grants.get(), 1);
    assert!(E2eMode::parse(Some("")).is_err());
    Ok(())
}
