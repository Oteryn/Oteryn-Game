use super::*;
use oteryn_client_ui::boss_encounter::{BossCategory, Intent};
#[test]
fn boss_gallery_real_caller_carries_filters_portraits_and_all_stages() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Error> {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let commands = Arc::new(Mutex::new(Vec::new()));
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(
                    Peer {
                        commands: Arc::clone(&commands),
                    },
                    incoming,
                    outbox,
                ));
                ready(&link, false).await?;
                {
                    let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
                    mailbox.boss_encounter.browser_category = BossCategory::Archfoe;
                    mailbox.boss_encounter.browser_search = "Oberon".into();
                }
                link.boss_encounter_request(Intent::Catalogue);
                ready(&link, false).await?;
                let panel = link.boss_encounter_panel();
                let text = panel_text(&panel);
                assert!(text.contains("Grand Master Oberon — Archfoe"));
                assert!(text.contains("Total boss points: 140"));
                assert!(text.contains("Prowess: 5/5 | 10 total points | complete"));
                assert!(text.contains("Expertise: 20/20 | 40 total points | complete"));
                assert!(text.contains("Mastery: 20/60 | 100 total points | in progress"));
                let cards = oteryn_client_ui::boss_encounter::catalogue_cards(&panel);
                assert_eq!(cards.len(), 1);
                assert_eq!(cards[0].look, 1250);
                assert!(!cards[0].object);
                assert_eq!(panel.fields[0], cards[0].key);
                link.boss_encounter_request(Intent::CatalogueNext);
                ready(&link, false).await?;
                assert!(
                    panel_text(&link.boss_encounter_panel())
                        .contains("Mastery: 60/60 | 100 total points | complete")
                );
                let sent = commands
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone();
                assert_eq!(sent.len(), 2);
                let queries: Vec<_> = sent
                    .iter()
                    .filter_map(|(kind, payload)| {
                        assert_eq!(*kind, 133);
                        oteryn_client_runtime::boss_encounter::catalogue::Query::decode(payload)
                    })
                    .collect();
                assert_eq!(queries.len(), 2);
                assert_eq!(queries[0].category, 2);
                assert_eq!(queries[0].search, "Oberon");
                assert_eq!(queries[1].after, 64);
                {
                    let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
                    mailbox.cancel_activity_requests();
                    assert!(mailbox.boss_encounter.catalogue.is_none());
                    assert!(mailbox.boss_encounter.catalogue_query.is_none());
                    assert!(mailbox.boss_encounter.pending_catalogue.is_none());
                }
                drop(link);
                task.await?;
                Ok(())
            })?
        })
        .map_err(|e| -> Error { Box::new(e) })?
        .join()
        .map_err(|_| "boss gallery test thread panicked")?
}
