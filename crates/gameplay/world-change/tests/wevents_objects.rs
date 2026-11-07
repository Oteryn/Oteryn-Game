use oteryn_gameplay_world_change::objects::{ObjectStatus, SharedObjectState};

#[test]
fn wevents_shared_objects_hatch_once_and_reset_without_leaking_water_or_fire()
-> Result<(), &'static str> {
    let mut object = SharedObjectState::default();
    assert!(object.add(2, 3)?);
    assert!(!object.add(2, 3)?);
    assert!(object.ignite()?);
    assert!(!object.ignite()?);
    assert!(object.begin(1_000, 60_000)?);
    assert!(!object.begin(1_001, 60_000)?);
    assert_eq!(object.status(60_999)?, ObjectStatus::Waiting);
    assert!(!object.claim(60_999)?);
    assert_eq!(object.status(61_000)?, ObjectStatus::Ready);
    assert!(object.claim(61_000)?);
    assert!(!object.claim(61_001)?);
    assert!(object.take(1)?);
    let previous = object.revision();
    object.reset()?;
    assert!(object.revision() > previous);
    assert_eq!(object.units(), 0);
    assert!(!object.lit());
    assert_eq!(object.status(62_000)?, ObjectStatus::Dormant);
    assert!(object.begin(62_000, 0)?);
    assert!(object.claim(62_000)?);
    Ok(())
}

#[test]
fn wevents_object_invalid_time_or_overflow_never_partially_mutates_state()
-> Result<(), &'static str> {
    let mut object = SharedObjectState::default();
    let before = object.clone();
    assert!(object.begin(-1, 1).is_err());
    assert!(object.begin(1, -1).is_err());
    assert!(object.begin(i64::MAX, 1).is_err());
    assert_eq!(object, before);
    assert!(object.add(u16::MAX, u16::MAX)?);
    let full = object.clone();
    assert!(!object.add(1, u16::MAX)?);
    assert_eq!(object, full);
    assert!(object.add(1, 1).is_err());
    assert_eq!(object, full);
    Ok(())
}
