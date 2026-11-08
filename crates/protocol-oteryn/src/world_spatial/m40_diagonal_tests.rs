use super::*;

#[test]
fn m40_diagonal_direction_codes_round_trip_without_changing_cardinals() -> Result<(), String> {
    for value in 1..=8 {
        let payload = [8, value];
        let direction = decode_step_intent(&payload)
            .map_err(|error| format!("native direction {value}: {error:?}"))?;
        assert_eq!(encode_step_intent(direction), payload);
    }
    for value in [0, 9, 127, 255] {
        let mut payload = vec![8];
        push_varint(&mut payload, value);
        assert_eq!(
            decode_step_intent(&payload),
            Err(WorldSpatialError::Malformed)
        );
    }
    assert_eq!(
        decode_step_intent(&[8, 5, 8, 6]),
        Err(WorldSpatialError::Malformed)
    );
    Ok(())
}

#[test]
fn m40_larger_pacing_receipts_require_both_capabilities() -> Result<(), String> {
    let result = StepResult {
        disposition: StepDisposition::Moved,
        step_wait_ms: MAX_DIAGONAL_STEP_WAIT_MS,
    };
    assert!(encode_step_result_paced(result, true).is_err());
    assert!(encode_step_result_with_diagonal(result, false, true).is_err());
    let bytes =
        encode_step_result_with_diagonal(result, true, true).map_err(|e| format!("{e:?}"))?;
    assert!(decode_step_result_receipt(&bytes, true).is_err());
    assert_eq!(
        decode_step_result_receipt_with_diagonal(&bytes, true, true),
        Ok(result)
    );
    assert!(
        encode_step_result_with_diagonal(
            StepResult {
                step_wait_ms: MAX_DIAGONAL_STEP_WAIT_MS + 50,
                ..result
            },
            true,
            true
        )
        .is_err()
    );
    Ok(())
}
