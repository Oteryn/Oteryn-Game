//! Per-module content scale (zoom) on top of the global `UiScale`. Every fixed size of a
//! module is stored once as its 100% value and multiplied here, so the dock layout state can
//! later own the factor without touching the widgets.
use bevy::prelude::*;

/// The modules with their own content scale. Context menus follow the global scale only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::bevy_shell) enum Module {
    Chat,
    ActionBars,
}

/// The content scale of each module; 1.0 is 100%.
#[derive(Resource, Clone, Debug, PartialEq)]
pub(in crate::bevy_shell) struct ModuleScale {
    chat: f32,
    bars: f32,
}
impl Default for ModuleScale {
    fn default() -> Self {
        Self { chat: 1., bars: 1. }
    }
}
impl ModuleScale {
    /// The same bounds as a window's zoom.
    pub(in crate::bevy_shell) const MIN: f32 = crate::bevy_shell::ui::window_layout::MIN_ZOOM;
    pub(in crate::bevy_shell) const MAX: f32 = crate::bevy_shell::ui::window_layout::MAX_ZOOM;
    pub(in crate::bevy_shell) fn get(&self, module: Module) -> f32 {
        match module {
            Module::Chat => self.chat,
            Module::ActionBars => self.bars,
        }
    }
    /// Sets a module's scale, kept within 75% to 200%; a non-finite value resets it.
    pub(in crate::bevy_shell) fn set(&mut self, module: Module, value: f32) {
        let value = if value.is_finite() {
            value.clamp(Self::MIN, Self::MAX)
        } else {
            1.
        };
        match module {
            Module::Chat => self.chat = value,
            Module::ActionBars => self.bars = value,
        }
    }
}

/// A node whose `size` (logical pixels at 100%) follows its module's scale. A zero axis is
/// left to the layout.
#[derive(Component, Clone, Copy, Debug)]
pub(in crate::bevy_shell) struct Scaled {
    pub(in crate::bevy_shell) module: Module,
    pub(in crate::bevy_shell) size: Vec2,
}
/// A text whose font size (pixels at 100%) follows its module's scale.
#[derive(Component, Clone, Copy, Debug)]
pub(in crate::bevy_shell) struct ScaledFont {
    pub(in crate::bevy_shell) module: Module,
    pub(in crate::bevy_shell) size: f32,
}

/// Applies each module's scale to its tagged nodes and texts.
pub(in crate::bevy_shell) fn apply(
    scale: Option<Res<ModuleScale>>,
    mut nodes: Query<(&Scaled, &mut Node)>,
    mut fonts: Query<(&ScaledFont, &mut TextFont)>,
) {
    let default = ModuleScale::default();
    let scale = scale.as_deref().unwrap_or(&default);
    for (scaled, mut node) in &mut nodes {
        let factor = scale.get(scaled.module);
        if scaled.size.x > 0. {
            let width = px(scaled.size.x * factor);
            if node.width != width || node.min_width != width {
                node.width = width;
                node.min_width = width;
            }
        }
        if scaled.size.y > 0. {
            let height = px(scaled.size.y * factor);
            if node.height != height || node.min_height != height {
                node.height = height;
                node.min_height = height;
            }
        }
    }
    for (scaled, mut font) in &mut fonts {
        let size = bevy::text::FontSize::Px(scaled.size * scale.get(scaled.module));
        if font.font_size != size {
            font.font_size = size;
        }
    }
}

/// The root of a module: Ctrl+mouse wheel over it zooms its content, Ctrl+0 resets it.
#[derive(Component, Clone, Copy, Debug)]
#[require(bevy::picking::hover::Hovered)]
pub(in crate::bevy_shell) struct ModuleRoot(pub(in crate::bevy_shell) Module);

/// One wheel notch changes the zoom by 10%.
pub(in crate::bevy_shell) const ZOOM_STEP: f32 = 0.1;

/// The zoom after `notches` Ctrl+wheel notches from `current`, on whole 10% steps.
pub(in crate::bevy_shell) fn zoomed(current: f32, notches: f32) -> f32 {
    ((current + notches.signum() * ZOOM_STEP) * 10.).round() / 10.
}

pub(in crate::bevy_shell) fn zoom(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    scroll: Option<Res<bevy::input::mouse::AccumulatedMouseScroll>>,
    roots: Query<(&ModuleRoot, &bevy::picking::hover::Hovered)>,
    mut scale: ResMut<ModuleScale>,
) {
    let Some(keys) = keys else {
        return;
    };
    if !keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
        return;
    }
    let notches = scroll.map_or(0., |scroll| scroll.delta.y);
    let reset = keys.any_just_pressed([KeyCode::Digit0, KeyCode::Numpad0]);
    for (root, hovered) in &roots {
        if !hovered.get() {
            continue;
        }
        if reset {
            scale.set(root.0, 1.);
        } else if notches != 0. {
            let next = zoomed(scale.get(root.0), notches);
            scale.set(root.0, next);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_is_per_module_and_bounded() {
        let mut scale = ModuleScale::default();
        scale.set(Module::Chat, 1.5);
        assert_eq!(scale.get(Module::Chat), 1.5);
        assert_eq!(scale.get(Module::ActionBars), 1.);
        scale.set(Module::ActionBars, 9.);
        assert_eq!(scale.get(Module::ActionBars), ModuleScale::MAX);
        scale.set(Module::Chat, 0.5);
        assert_eq!(scale.get(Module::Chat), 0.75);
        scale.set(Module::ActionBars, 3.);
        assert_eq!(scale.get(Module::ActionBars), 2.);
        scale.set(Module::Chat, f32::NAN);
        assert_eq!(scale.get(Module::Chat), 1.);
        assert_eq!(zoomed(1., 3.), 1.1);
        assert_eq!(zoomed(1.5, -1.), 1.4);
        const { assert!(ModuleScale::MIN == 0.75 && ModuleScale::MAX == 2.) };
    }

    #[test]
    fn ctrl_wheel_stops_at_75_and_200_percent_and_ctrl_0_resets() {
        let mut app = App::new();
        app.init_resource::<ModuleScale>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<bevy::input::mouse::AccumulatedMouseScroll>()
            .add_systems(Update, zoom);
        app.world_mut().spawn((
            ModuleRoot(Module::Chat),
            bevy::picking::hover::Hovered(true),
        ));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ControlLeft);
        for (delta, limit) in [(1., 2.), (-1., 0.75)] {
            app.world_mut()
                .resource_mut::<bevy::input::mouse::AccumulatedMouseScroll>()
                .delta
                .y = delta;
            for _ in 0..20 {
                app.update();
            }
            assert_eq!(
                app.world().resource::<ModuleScale>().get(Module::Chat),
                limit
            );
        }
        app.world_mut()
            .resource_mut::<bevy::input::mouse::AccumulatedMouseScroll>()
            .delta
            .y = 0.;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Digit0);
        app.update();
        assert_eq!(app.world().resource::<ModuleScale>().get(Module::Chat), 1.);
    }
}
