use super::*;
use oteryn_client_runtime::cyclopedia_map::raid_warnings::encode;

#[test]
fn raid_warning_reducer_area_subarea_and_clear() -> Result<(), MapWireError> {
    let mut warnings = Warnings::default();
    let area = Warning {
        area: 23,
        subarea: None,
    };
    let subarea = Warning {
        area: 23,
        subarea: Some(109),
    };

    apply(&mut warnings, &encode(&[area])?)?;
    assert!(warnings.area_warned(23));
    assert!(!warnings.subarea_warned(23, 109));
    apply(&mut warnings, &encode(&[subarea])?)?;
    assert!(warnings.area_warned(23));
    assert!(warnings.subarea_warned(23, 109));
    assert!(!warnings.area_warned(24));
    assert!(!warnings.subarea_warned(24, 109));

    for malformed in [
        &[][..],
        &[1, 0][..],
        &[0, 0, 0][..],
        &[1, 0, 0, 0, 0, 0, 109, 0, 0, 0][..],
    ] {
        assert!(apply(&mut warnings, malformed).is_err());
        assert!(warnings.area_warned(23));
        assert!(warnings.subarea_warned(23, 109));
    }

    apply(&mut warnings, &encode(&[])?)?;
    assert!(!warnings.area_warned(23));
    assert!(!warnings.subarea_warned(23, 109));
    Ok(())
}
