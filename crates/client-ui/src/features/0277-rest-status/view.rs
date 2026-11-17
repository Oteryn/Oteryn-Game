use super::RestStatusPanel;

pub fn panel_text(panel: &RestStatusPanel) -> String {
    if !panel.selected {
        return tr!("Rest status unavailable in this session").into();
    }
    if let Some(error) = panel.error {
        return error.into();
    }
    let Some(status) = panel.snapshot else {
        return tr!("Loading rest status…").into();
    };
    let area = if status.inside_resting_area {
        tr!("Inside resting area")
    } else {
        tr!("Outside resting area")
    };
    let food = match status.food_remaining_ms {
        -1 => tr!("Food: unlimited").to_owned(),
        0 => tr!("Food: hungry").to_owned(),
        remaining => lfmt!(
            "Food: {} s",
            remaining.div_euclid(1_000) + i32::from(remaining % 1_000 != 0)
        ),
    };
    let bonus = |multiplier| match multiplier {
        1 => "normal",
        2 => "double",
        _ => tr!("inactive"),
    };
    lfmt!(
        "{area}\r\nDaily streak: {}\r\n{food}\r\nRest health bonus: {}\r\nRest mana bonus: {}\r\nStamina regeneration: {}\r\nSoul regeneration: {}",
        status.streak,
        bonus(status.health_multiplier),
        bonus(status.mana_multiplier),
        if status.stamina {
            tr!("active")
        } else {
            tr!("inactive")
        },
        if status.soul {
            tr!("active")
        } else {
            tr!("inactive")
        },
        area = area,
        food = food
    )
}
