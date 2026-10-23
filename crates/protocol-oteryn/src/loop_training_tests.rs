use super::*;
#[test]
fn loop_training_decoders_refuse_duplicates_unknowns_overflow_and_cross_action_fields() {
    assert_eq!(decode_offline_training(&[8, 1]).map(|h| h.get()), Ok(1));
    for bytes in [&[][..], &[8, 0], &[8, 1, 8, 2], &[16, 1], &[8, 1, 0]] {
        assert!(decode_offline_training(bytes).is_err());
    }
    for bytes in [
        &[][..],
        &[8, 0],
        &[8, 4],
        &[8, 1],
        &[8, 2, 16, 1],
        &[8, 3, 24, 1],
        &[8, 2, 8, 2],
        &[8, 1, 16, 0, 24, 1],
        &[8, 1, 16, 1, 24, 1, 32, 0],
    ] {
        assert!(decode_exercise_training(bytes).is_err(), "{bytes:?}");
    }
    assert_eq!(
        decode_exercise_training(&[8, 2]),
        Ok(ExerciseIntent::Status)
    );
    assert_eq!(decode_exercise_training(&[8, 3]), Ok(ExerciseIntent::Stop));
    assert!(matches!(
        decode_exercise_training(&[8, 1, 16, 1, 24, 2]),
        Ok(ExerciseIntent::Start { .. })
    ));
    assert!(decode_exercise_training(&[0; 33]).is_err());
    assert!(decode_offline_training(&[8, 255, 255, 255, 255, 255, 255, 255, 255, 255, 2]).is_err());
    assert!(decode_offline_training(&[8, 128]).is_err());
}
#[test]
fn loop_training_result_is_bounded_and_refusal_has_no_durable_claim() {
    assert_eq!(
        encode_training_result(TrainingResult::REFUSED),
        Ok(vec![8, 4, 16, 0, 24, 0, 32, 0])
    );
    assert!(
        encode_training_result(TrainingResult {
            disposition: 1,
            revision: u64::MAX,
            pool_seconds: 43200,
            charges: u32::MAX
        })
        .is_ok_and(|bytes| bytes.len() <= MAX_TRAINING_RESULT_BYTES)
    );
    assert!(
        encode_training_result(TrainingResult {
            revision: 1,
            ..TrainingResult::REFUSED
        })
        .is_err()
    );
}
