use super::{tests::Peer, *};
use crate::play::play_channel;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn shared_modal_answers_actual_session_and_rejects_duplicate_and_closed_bridge() -> Result {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (link, _commands, outbox) = play_channel();
            link.use_shared_frontend();
            let mut peer = Peer::default();
            let mut driver = Driver::new(true).with_outbox(&outbox);
            driver.poll(&mut peer).await?;
            let projected = link.server_modal_panel().ok_or("projected server modal")?;
            let answer = Answer {
                dialog_id: 73,
                button_id: 4,
                choice_id: Some(9),
            };
            assert!(!link.server_modal_submit(projected.revision + 1, answer, Entry::default()));
            assert!(!link.server_modal_submit(
                projected.revision,
                Answer {
                    choice_id: Some(10),
                    ..answer
                },
                Entry::default()
            ));
            assert!(link.server_modal_submit(projected.revision, answer, Entry::default()));
            assert!(!link.server_modal_submit(projected.revision, answer, Entry::default()));
            assert!(link.server_modal_panel().ok_or("queued")?.queued);
            assert_eq!(
                driver.poll(&mut peer).await?,
                Some(Completion::Answered(answer))
            );
            assert_eq!(peer.answers, 1);
            assert!(link.server_modal_panel().is_none());
            drop(driver);
            assert!(!link.server_modal_submit(projected.revision, answer, Entry::default()));
            Ok(())
        })
}

#[test]
fn shared_input_submission_validates_and_keeps_the_first_queued_entry() -> Result {
    use super::tests::AmountPeer;
    use oteryn_client_runtime::server_modal::input::{InputAnswer, InputValue};
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (link, _commands, outbox) = play_channel();
            link.use_shared_frontend();
            let mut peer = AmountPeer::default();
            let mut driver = Driver::new(true).with_input(true).with_outbox(&outbox);
            driver.poll(&mut peer).await?;
            let projected = link.server_modal_panel().ok_or("amount modal")?;
            let answer = Answer {
                dialog_id: 31,
                button_id: 1,
                choice_id: None,
            };
            let entry = Entry {
                value: Some(InputValue::Amount(12)),
                dont_show_again: true,
            };
            assert!(!link.server_modal_submit(
                projected.revision,
                answer,
                Entry {
                    value: Some(InputValue::Amount(21)),
                    dont_show_again: false
                }
            ));
            assert!(link.server_modal_submit(projected.revision, answer, entry));
            assert!(!link.server_modal_submit(
                projected.revision,
                answer,
                Entry {
                    value: Some(InputValue::Amount(15)),
                    dont_show_again: false
                }
            ));
            driver.poll(&mut peer).await?;
            let sent = InputAnswer::decode(&peer.answer.ok_or("sent input")?)
                .map_err(|_| "input payload")?;
            assert_eq!(sent.value, Some(InputValue::Amount(12)));
            assert!(sent.dont_show_again);
            Ok(())
        })
}

#[test]
fn a_new_driver_rejects_the_old_revision_even_when_server_reuses_dialog_id() -> Result {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (link, _commands, outbox) = play_channel();
            link.use_shared_frontend();
            let mut first = Driver::new(true).with_outbox(&outbox);
            first.poll(&mut Peer::default()).await?;
            let old = link.server_modal_panel().ok_or("first descriptor")?;
            drop(first);
            let mut next = Driver::new(true).with_outbox(&outbox);
            let mut peer = Peer::default();
            next.poll(&mut peer).await?;
            let current = link.server_modal_panel().ok_or("new descriptor")?;
            assert!(current.revision > old.revision);
            let answer = Answer {
                dialog_id: 73,
                button_id: 4,
                choice_id: Some(9),
            };
            assert!(!link.server_modal_submit(old.revision, answer, Entry::default()));
            assert!(link.server_modal_submit(current.revision, answer, Entry::default()));
            next.poll(&mut peer).await?;
            assert_eq!(peer.answers, 1);
            Ok(())
        })
}
