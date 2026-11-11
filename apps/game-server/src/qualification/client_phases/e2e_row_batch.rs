// Ordinary node-boot jobs must select every row by default. Explicit ranges remain
// available to the all-batches runner for bounded local qualification.
const E2E_ROW_BATCH_DEFAULT: (usize, usize) = (0, usize::MAX);

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
    e2e_row_in_batch(index, start, len)
}

fn e2e_row_in_batch(index: usize, start: usize, len: usize) -> bool {
    // Captures need the shared journey's state in the same node boot. Assign
    // them to its batch so neither the setup nor its verdicts are repeated.
    let index = match index {
        103 | 104 => 231,
        110 | 111 => 230,
        _ => index,
    };
    index >= start && index - start < len
}

// A failed row can leave its socket open or its session in loss/grace; the
// next row must not inherit that state.
async fn release_row_characters(clients: &SeamClients<'_>) {
    for index in 0..clients.accounts.len() {
        if let Err(error) = wait_character_released(clients, index).await {
            evidence(&format!("e2e_row_release account={index} error={error}"));
        }
    }
}

#[cfg(test)]
mod e2e_row_batch_tests {
    use super::*;

    #[test]
    fn full_row_batch_parses_default_all_and_ranges() -> TestResult {
        assert_eq!(e2e_row_batch(None), Some((0, usize::MAX)));
        let (start, len) = e2e_row_batch(None).ok_or("missing default batch")?;
        for index in 0..308 {
            assert!(e2e_row_in_batch(index, start, len), "row {index}");
        }
        assert_eq!(e2e_row_batch(Some("all")), Some((0, usize::MAX)));
        assert_eq!(e2e_row_batch(Some("20:20")), Some((20, 20)));
        assert_eq!(e2e_row_batch(Some("20")), None);
        assert_eq!(e2e_row_batch(Some("x:1")), None);
        Ok(())
    }

    #[test]
    fn full_row_batches_run_every_index_once_with_its_shared_journey() {
        for index in 0..308 {
            let batches: Vec<_> = (0..308)
                .step_by(15)
                .filter(|start| e2e_row_in_batch(index, *start, 15))
                .collect();
            assert_eq!(batches.len(), 1, "row {index}");
            let owner = match index {
                103 | 104 => 231,
                110 | 111 => 230,
                _ => index,
            };
            assert!(e2e_row_in_batch(owner, batches[0], 15));
        }
    }
}
