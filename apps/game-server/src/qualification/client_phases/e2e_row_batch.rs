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
