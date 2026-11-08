use super::*;
#[test]
fn rich_pages_are_opt_in_bounded_and_reject_truncation() -> Result<(), Error> {
    for target in [None, Some([7; 16])] {
        let request = AchievementRequest { page: 255, target };
        assert_eq!(decode_request(&request.encode())?, request);
        assert!(super::super::decode_tab_request(&request.encode()).is_err());
        let mut invalid = request.encode();
        invalid.push(0);
        assert!(decode_request(&invalid).is_err());
    }
    let page = AchievementPage {
        character: [7; 16],
        name: "Test Hero".into(),
        page: AccountAchievementsResult::default(),
    };
    let bytes = page.encode()?;
    assert_eq!(AchievementPage::decode(&bytes)?, page);
    for end in 0..bytes.len() {
        assert!(AchievementPage::decode(&bytes[..end]).is_err());
    }
    let mut invalid = bytes;
    invalid.push(0);
    assert!(AchievementPage::decode(&invalid).is_err());
    Ok(())
}
