use super::*;
#[test]
fn s17a_native_start_update_stop_use_confirmed_revision() -> Result<(), &'static str> {
    let mut panel = Panel {
        selected: true,
        ..Default::default()
    };
    assert_eq!(panel.delay, 0);
    assert!(!panel.active);
    assert!(matches!(
        panel.command(0)?,
        Message::Start {
            revision: 0,
            delay: 0,
            ..
        }
    ));
    panel.apply(Message::CasterState {
        cast_id: [7; 16],
        revision: 4,
        epoch: 2,
        delay: 30,
        count: 3,
    })?;
    assert!(panel.text().contains("3 viewers"));
    assert!(matches!(
        panel.command(0)?,
        Message::Update { revision: 4, .. }
    ));
    assert_eq!(panel.command(1)?, Message::Stop { revision: 4 });
    panel.delay = 301;
    assert!(panel.command(0).is_err());
    panel.apply(Message::End)?;
    assert!(!panel.active);
    assert_eq!(panel.revision, 0);
    assert!(
        panel
            .apply(Message::Chat("wrong direction".into()))
            .is_err()
    );
    Ok(())
}
