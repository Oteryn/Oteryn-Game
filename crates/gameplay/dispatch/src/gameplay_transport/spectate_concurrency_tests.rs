use super::*;

#[test]
fn spectate_concurrent_grant_writers_have_one_typed_loser() {
    let sessions = Arc::new(SpectateSession::from(limits(10, 40)));
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let writers: Vec<_> = (0..2)
        .map(|_| {
            let sessions = Arc::clone(&sessions);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                sessions.dispatch(SpectateDispatch::Owner(Box::new(Request::GrantSet {
                    facts: facts(1, 1, 1),
                    expected_revision: 0,
                    audience: Audience::Code,
                })))
            })
        })
        .collect();
    barrier.wait();
    let replies: Vec<_> = writers
        .into_iter()
        .map(|writer| writer.join().expect("writer"))
        .collect();
    assert_eq!(
        replies
            .iter()
            .filter(|reply| matches!(reply, Reply::Grant { .. }))
            .count(),
        1
    );
    assert_eq!(
        replies
            .iter()
            .filter(|reply| **reply == Reply::StaleRevision)
            .count(),
        1
    );
}
