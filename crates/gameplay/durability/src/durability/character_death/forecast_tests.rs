use super::*;
fn facts() -> Facts {
    Facts {
        observed: 1,
        level: 100,
        experience: 1_000_000,
        regular: 1,
        twist: true,
        promoted: false,
        skulled: false,
        has_vocation: true,
        amulet: false,
        adventurer: false,
        pvp_available: true,
    }
}
#[test]
fn forecast_matches_death_ladders_skull_aol_and_adventurer_precedence()
-> Result<(), DurabilityError> {
    let f = facts();
    let value = project(f)?;
    assert!(value.pve.experience_lost > 0);
    assert_eq!(
        value.pve.experience_lost,
        value
            .pvp
            .ok_or(DurabilityError::InvalidStoredState)?
            .experience_lost
    );
    assert_eq!(
        (value.pve.equipment_per_mille, value.pve.container_per_mille),
        (70, 700)
    );
    assert_eq!(
        value
            .pvp
            .ok_or(DurabilityError::InvalidStoredState)?
            .consumption,
        Consumption::Twist
    );
    let value = project(Facts { amulet: true, ..f })?;
    assert_eq!(
        (value.pve.equipment_per_mille, value.pve.container_per_mille),
        (0, 0)
    );
    assert!(value.pve.amulet_consumed);
    let value = project(Facts {
        skulled: true,
        amulet: true,
        ..f
    })?;
    assert_eq!(
        (
            value
                .pvp
                .ok_or(DurabilityError::InvalidStoredState)?
                .equipment_per_mille,
            value
                .pvp
                .ok_or(DurabilityError::InvalidStoredState)?
                .container_per_mille
        ),
        (1000, 1000)
    );
    assert!(
        !value
            .pvp
            .ok_or(DurabilityError::InvalidStoredState)?
            .amulet_consumed
    );
    assert_eq!(
        value
            .pvp
            .ok_or(DurabilityError::InvalidStoredState)?
            .consumption,
        Consumption::Regular
    );
    let value = project(Facts {
        adventurer: true,
        amulet: true,
        ..f
    })?;
    assert_eq!(
        value
            .pvp
            .ok_or(DurabilityError::InvalidStoredState)?
            .experience_lost,
        0
    );
    assert_eq!(
        value
            .pvp
            .ok_or(DurabilityError::InvalidStoredState)?
            .consumption,
        Consumption::Keep
    );
    assert!(
        !value
            .pvp
            .ok_or(DurabilityError::InvalidStoredState)?
            .amulet_consumed
    );
    assert!(value.pve.amulet_consumed);
    for f in [
        Facts { regular: 5, ..f },
        Facts { level: 8, ..f },
        Facts {
            has_vocation: false,
            ..f
        },
    ] {
        let v = project(f)?;
        assert_eq!(
            (v.pve.equipment_per_mille, v.pve.container_per_mille),
            (0, 0)
        );
    }
    assert!(project(Facts { regular: 8, ..f }).is_err());
    assert!(
        project(Facts {
            pvp_available: false,
            ..f
        })?
        .pvp
        .is_none()
    );
    Ok(())
}
