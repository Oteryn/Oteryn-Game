//! Fixed-width source crops. Bevy 0.20.1's Sliced mode scales borders down
//! with min(target/source), even when max_corner_scale=1. Tiled crops do not.
use bevy::{
    prelude::*,
    ui::{ComputedNode, VisualBox, widget::NodeImageMode},
};

/// Left/top/right/bottom source-pixel insets. The parent ImageNode holds the
/// source handle (including a button's current up/down state), not a drawn quad.
#[derive(Component, Clone, Copy)]
pub struct Frame {
    pub insets: [f32; 4],
}

/// Insets remain fixed when shrinking a large skin to a small control. Supply
/// a full image; crop/atlas/flipping on the source-holder node are not supported.
pub fn frame(image: Handle<Image>, insets: [f32; 4]) -> impl Bundle {
    (
        Frame { insets },
        ImageNode {
            image,
            color: Color::NONE,
            visual_box: VisualBox::BorderBox,
            // The hidden source must not contribute intrinsic image sizing.
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
    )
}

#[derive(PartialEq)]
struct Stamp {
    source: AssetId<Image>,
    source_size: Vec2,
    target: Vec2,
    border_origin: Vec2,
    insets: [f32; 4],
}

#[derive(Component)]
pub(super) struct Pieces {
    entities: [Entity; 9],
    stamp: Option<Stamp>,
}

// Nine entities per live frame, reused on source/size changes and recursively
// despawned with their owner. No generated textures or size-keyed cache.
pub(super) fn project(
    mut commands: Commands,
    assets: Res<Assets<Image>>,
    mut frames: Query<(
        Entity,
        &Frame,
        &ComputedNode,
        &mut ImageNode,
        Option<&mut Pieces>,
    )>,
    mut pieces: Query<(&mut Node, &mut ImageNode, &mut Visibility), Without<Frame>>,
) {
    for (entity, frame, computed, mut image, mut state) in &mut frames {
        if image.color != Color::NONE {
            image.color = Color::NONE;
        }
        let Some(source) = assets.get(&image.image) else {
            if let Some(state) = state.as_mut() {
                state.stamp = None;
                for child in state.entities {
                    if let Ok((_, _, mut visible)) = pieces.get_mut(child) {
                        *visible = Visibility::Hidden;
                    }
                }
            }
            continue;
        };
        let stamp = Stamp {
            source: image.image.id(),
            source_size: source.size_f32(),
            target: computed.size * computed.inverse_scale_factor,
            // Absolute children are positioned from the parent's padding box.
            // Subtract its border to cover the complete BorderBox, not ContentBox.
            border_origin: computed.border.min_inset * computed.inverse_scale_factor,
            insets: frame.insets,
        };
        if !stamp.target.is_finite()
            || !stamp.target.cmpgt(Vec2::ZERO).all()
            || frame.insets.iter().any(|v| !v.is_finite() || *v < 0.)
        {
            continue;
        }
        if state
            .as_ref()
            .is_some_and(|s| s.stamp.as_ref() == Some(&stamp))
        {
            continue;
        }
        let geometry = geometry(&stamp);
        if let Some(state) = state.as_mut() {
            for (index, child) in state.entities.iter().enumerate() {
                if let Ok((mut node, mut child_image, mut visible)) = pieces.get_mut(*child) {
                    *node = geometry[index].0.clone();
                    *child_image = crop(image.image.clone(), geometry[index].1);
                    *visible = Visibility::Inherited;
                }
            }
            state.stamp = Some(stamp);
        } else {
            let entities = std::array::from_fn(|index| {
                commands
                    .spawn((
                        geometry[index].0.clone(),
                        crop(image.image.clone(), geometry[index].1),
                        Pickable::IGNORE,
                        ZIndex(-1),
                    ))
                    .id()
            });
            commands
                .entity(entity)
                .add_children(&entities)
                .insert(Pieces {
                    entities,
                    stamp: Some(stamp),
                });
        }
    }
}

fn crop(image: Handle<Image>, rect: Rect) -> ImageNode {
    ImageNode {
        image,
        rect: Some(rect),
        visual_box: VisualBox::BorderBox,
        image_mode: NodeImageMode::Tiled {
            tile_x: true,
            tile_y: true,
            stretch_value: 1.,
        },
        ..default()
    }
}

fn geometry(stamp: &Stamp) -> [(Node, Rect); 9] {
    let [left, top, right, bottom] = stamp.insets;
    let source = stamp.source_size;
    let target = stamp.target;
    // A too-small target clips an edge rather than reducing it to subpixels.
    let l = left.min(target.x);
    let t = top.min(target.y);
    let r = right.min(target.x - l);
    let b = bottom.min(target.y - t);
    let xs = [0., left, source.x - right, source.x];
    let ys = [0., top, source.y - bottom, source.y];
    let xt = [0., l, target.x - r, target.x];
    let yt = [0., t, target.y - b, target.y];
    std::array::from_fn(|index| {
        let col = index % 3;
        let row = index / 3;
        let width = xt[col + 1] - xt[col];
        let height = yt[row + 1] - yt[row];
        let valid = width > 0. && height > 0. && xs[col + 1] > xs[col] && ys[row + 1] > ys[row];
        (
            Node {
                position_type: PositionType::Absolute,
                left: px(xt[col] - stamp.border_origin.x),
                top: px(yt[row] - stamp.border_origin.y),
                width: px(width),
                height: px(height),
                display: if valid { Display::Flex } else { Display::None },
                ..default()
            },
            Rect::new(xs[col], ys[row], xs[col + 1], ys[row + 1]),
        )
    })
}
