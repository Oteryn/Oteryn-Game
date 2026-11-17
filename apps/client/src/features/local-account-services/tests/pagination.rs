use super::*;

#[test]
fn bazaar_next_page_encodes_opaque_cursor_and_preserves_own_view() -> TestResult {
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        mock.authority.lock().map_err(|_| "lock")?.paginate = true;
        mock.runtime.block_on(async {
            let mut view = View::default();
            controller::execute(
                &mut view,
                Action::Refresh(AuctionView::Own),
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert!(view.page.as_ref().is_some_and(|p| p.next_cursor.is_some()));
            controller::execute(
                &mut view,
                Action::Next,
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert_eq!(view.selected_view, AuctionView::Own);
            assert_eq!(
                view.page
                    .as_ref()
                    .map(|p| p.auctions[0].character_name.as_str()),
                Some("Next character")
            );
            assert!(view.page.as_ref().is_some_and(|p| p.next_cursor.is_none()));
            let state = mock.authority.lock().map_err(|_| "lock")?;
            assert!(state.requests.iter().any(|r| {
                r.contains("view=own&limit=50&cursor=next%2Fpage%3Fview%3Dhistory%26limit%3D1")
            }));
            Ok::<_, Box<dyn std::error::Error>>(())
        })
    })
}
