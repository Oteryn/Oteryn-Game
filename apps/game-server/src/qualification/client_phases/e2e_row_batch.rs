// One node-boot job cannot run every row journey inside its timeout (each row
// needs about 80 s), so CI runs one batch per head. `OTERYN_E2E_ROW_BATCH` is
// `start:len` over the row indices passed to `e2e_row_selected` in `journeys`;
// `all` runs every row.
const E2E_ROW_BATCH_DEFAULT: (usize, usize) = (0, 15);

fn e2e_row_batch(value: Option<&str>) -> Option<(usize, usize)> {
    match value {
        None => Some(E2E_ROW_BATCH_DEFAULT),
        Some("all") => Some((0, usize::MAX)),
        Some(text) => {
            let (start, len) = text.split_once(':')?;
            Some((start.parse().ok()?, len.parse().ok()?))
        }
    }
}

fn e2e_row_selected(index: usize) -> bool {
    let value = std::env::var("OTERYN_E2E_ROW_BATCH").ok();
    let Some((start, len)) = e2e_row_batch(value.as_deref()) else {
        evidence("e2e_row_batch invalid OTERYN_E2E_ROW_BATCH; running no rows");
        return false;
    };
    index >= start && index - start < len
}

// A failed row can leave its socket open or its session in loss/grace; the
// next row must not inherit that state.
async fn release_row_characters(clients: &SeamClients<'_>) -> TestResult {
    let harness = verdict::Harness::global();
    let mut failure = None;
    let mut connection = None;
    let mut deadline = tokio::time::Instant::now();
    for index in 0..clients.accounts.len() {
        let need = character_release_need(clients, index);
        let result = async {
            harness.fixture_gate(need, index)?;
            deadline = fixture_release_deadline();
            row_timing::wait(wait_fixture_released_on(clients, index, deadline, &mut connection))
                .await
                .map_err(|error| harness.fixture_failed(need, index, error.as_ref()))
        }
        .await;
        if let Err(error) = result {
            // The next fixture gets its own deadline and a fresh transport after an error.
            drop(connection.take());
            evidence(&format!("e2e_row_release account={index} error={error}"));
            if failure.is_none() {
                failure = Some(error);
            }
        }
    }
    let closed = row_timing::wait(close_release_connection(connection, deadline)).await;
    match failure {
        Some(error) => Err(error),
        None => closed,
    }
}

#[cfg(test)]
mod e2e_row_batch_tests {
    use super::*;

    #[test]
    fn e2e_row_batch_parses_default_all_and_ranges() {
        assert_eq!(e2e_row_batch(None), Some(E2E_ROW_BATCH_DEFAULT));
        assert_eq!(e2e_row_batch(Some("all")), Some((0, usize::MAX)));
        assert_eq!(e2e_row_batch(Some("20:20")), Some((20, 20)));
        assert_eq!(e2e_row_batch(Some("20")), None);
        assert_eq!(e2e_row_batch(Some("x:1")), None);
    }
}
