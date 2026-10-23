use super::*;

#[test]
fn tutorial_notices_golden_vectors_and_strict_bounds() -> WireResult<()> {
    assert_eq!(TUTORIAL_NOTICES_SNAPSHOT, [8, 1]);
    for (notice, bytes) in [
        (TutorialNotice::Hint(2), vec![8, 2]),
        (TutorialNotice::Hint(255), vec![8, 255, 1]),
        (
            TutorialNotice::StatusText("é".into()),
            vec![18, 2, 195, 169],
        ),
    ] {
        assert_eq!(encode_tutorial_notice(&notice)?, bytes);
        assert_eq!(decode_tutorial_notice(&bytes)?, notice);
    }
    let largest = TutorialNotice::StatusText("a".repeat(255));
    let bytes = encode_tutorial_notice(&largest)?;
    assert_eq!(bytes.len(), 258);
    assert_eq!(decode_tutorial_notice(&bytes)?, largest);
    for bytes in [
        vec![],
        vec![8, 0],
        vec![8, 128, 2],
        vec![8, 1, 8, 2],
        vec![18, 0],
        vec![18, 1, 255],
        vec![16, 1],
        vec![0; 261],
    ] {
        assert!(decode_tutorial_notice(&bytes).is_err());
    }
    assert!(encode_tutorial_notice(&TutorialNotice::Hint(0)).is_err());
    assert!(encode_tutorial_notice(&TutorialNotice::StatusText("a".repeat(256))).is_err());
    Ok(())
}
