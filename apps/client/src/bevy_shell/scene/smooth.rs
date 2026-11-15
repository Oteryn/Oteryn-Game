//! Bounded 8-pixel light sampling with GPU interpolation, batched by camera layer.
use super::*;
use bevy::{ecs::system::SystemParam, mesh::PrimitiveTopology};
use oteryn_renderer::lighting::SceneLighting;

#[derive(SystemParam)]
pub(in crate::bevy_shell) struct MeshAssets<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<ColorMaterial>>,
}
#[derive(Default)]
pub(super) struct Cache {
    batches: Vec<(Entity, Handle<Mesh>)>,
    material: Option<Handle<ColorMaterial>>,
}
impl Cache {
    pub fn hide(&self, commands: &mut Commands) {
        for (entity, _) in &self.batches {
            commands.entity(*entity).insert(Visibility::Hidden);
        }
    }

    pub fn draw(
        &mut self,
        commands: &mut Commands,
        assets: &mut MeshAssets,
        image: &Handle<Image>,
        batches: [Vertices; 2],
    ) {
        let (meshes, materials) = (&mut assets.meshes, &mut assets.materials);
        let material = self.material.get_or_insert_with(|| {
            materials.add(ColorMaterial {
                texture: Some(image.clone()),
                alpha_mode: bevy::sprite_render::AlphaMode2d::Blend,
                ..default()
            })
        });
        for (index, vertices) in batches.into_iter().enumerate() {
            let mesh = vertices.mesh();
            if let Some((entity, handle)) = self.batches.get(index) {
                if let Some(mut existing) = meshes.get_mut(handle) {
                    *existing = mesh;
                }
                commands.entity(*entity).insert(Visibility::Visible);
            } else {
                let handle = meshes.add(mesh);
                let entity = commands
                    .spawn((
                        Mesh2d(handle.clone()),
                        MeshMaterial2d(material.clone()),
                        Transform::default(),
                        RenderLayers::layer(index + 1),
                        // Geometry is clipped to its camera viewport before upload.
                        bevy::camera::visibility::NoFrustumCulling,
                    ))
                    .id();
                self.batches.push((entity, handle));
            }
        }
    }
}

#[derive(Default)]
pub(super) struct Vertices {
    positions: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}
impl Vertices {
    pub fn push(
        &mut self,
        lighting: &SceneLighting,
        quad: &QuadInstance,
        index: usize,
        layer: QuadLayer,
        projection: (&Sprite, Transform, QuadViewport),
    ) {
        let (sprite, transform, viewport) = projection;
        let origin = Vec2::from(quad.position) + viewport.scroll;
        let start = origin.max(Vec2::ZERO);
        let end = (origin + Vec2::from(quad.size)).min(viewport.clip_size);
        let size = end - start;
        let columns = (size.x / 8.).ceil() as usize;
        let rows = (size.y / 8.).ceil() as usize;
        // Crop UVs with screen geometry; constant palette UVs stay constant.
        let uv_start = Vec2::from_slice(&quad.uv[..2]);
        let uv_size = Vec2::new(quad.uv[2] - quad.uv[0], quad.uv[3] - quad.uv[1]);
        let base = self.positions.len() as u32;
        for row in 0..=rows {
            for column in 0..=columns {
                let fraction = Vec2::new(column as f32 / columns as f32, row as f32 / rows as f32);
                let screen = start + size * fraction - viewport.scroll;
                let local = (fraction - Vec2::splat(0.5)) * size / viewport.scale;
                self.positions.push(
                    transform
                        .transform_point(Vec3::new(local.x, -local.y, 0.))
                        .to_array(),
                );
                self.uv.push(
                    (uv_start
                        + uv_size * (screen - Vec2::from(quad.position)) / Vec2::from(quad.size))
                    .to_array(),
                );
                let mut tint = lighting.quad_tint_at(quad, index, layer, screen.to_array());
                tint[3] = sprite.color.alpha();
                self.colors.push(tint);
            }
        }
        let stride = (columns + 1) as u32;
        for row in 0..rows {
            for column in 0..columns {
                let top = base + (row * (columns + 1) + column) as u32;
                self.indices.extend([
                    top,
                    top + stride,
                    top + 1,
                    top + 1,
                    top + stride,
                    top + stride + 1,
                ]);
            }
        }
    }

    fn mesh(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uv)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_indices(bevy::mesh::Indices::U32(self.indices))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_renderer::lighting::PointLight;

    #[test]
    fn scrolled_smooth_mesh_crops_uvs_and_samples_light_in_world_coordinates()
    -> Result<(), Box<dyn std::error::Error>> {
        let quad = QuadInstance {
            position: [-8., 0.],
            size: [32.; 2],
            uv: [0., 0., 1., 1.],
        };
        let viewport = QuadViewport {
            scale: 1.,
            size: Vec2::splat(64.),
            offset: Vec2::ZERO,
            clip_size: Vec2::splat(32.),
            scroll: Vec2::new(16., -8.),
        };
        let lighting = SceneLighting {
            ambient: [0; 3],
            sources: vec![PointLight {
                position: [0., 16.],
                radius: 64.,
                color: [255; 3],
            }],
            ..default()
        };
        let atlas = AtlasImage::new(32, 1, 1, vec![255; 32 * 32 * 4])?;
        let (sprite, transform) =
            project_quad(&quad, [1.; 4], 0, &Handle::default(), &atlas, viewport)
                .ok_or("visible quad")?;
        let mut vertices = Vertices::default();
        vertices.push(
            &lighting,
            &quad,
            0,
            QuadLayer::Overlay,
            (&sprite, transform, viewport),
        );
        assert_eq!(vertices.positions.len(), 16);
        assert_eq!(vertices.uv.first(), Some(&[0., 0.25]));
        assert_eq!(vertices.uv.last(), Some(&[0.75, 1.]));
        for (position, color) in vertices.positions.iter().zip(&vertices.colors) {
            let screen = Vec2::new(position[0] + 32., 32. - position[1]);
            assert!((8. ..=32.).contains(&screen.x));
            assert!((0. ..=24.).contains(&screen.y));
            assert_eq!(
                *color,
                lighting.quad_tint_at(
                    &quad,
                    0,
                    QuadLayer::Overlay,
                    (screen - viewport.scroll).to_array(),
                )
            );
        }
        Ok(())
    }

    #[test]
    fn clipped_world_mesh_interpolates_light_and_preserves_uv_order_and_fade()
    -> Result<(), Box<dyn std::error::Error>> {
        let quad = QuadInstance {
            position: [-8., 0.],
            size: [32.; 2],
            uv: [0., 0., 1., 1.],
        };
        let lighting = SceneLighting {
            ambient: [0; 3],
            sources: vec![PointLight {
                position: [0., 16.],
                radius: 64.,
                color: [255; 3],
            }],
            opacity: vec![(0..1, 50)],
            ..default()
        };
        let viewport = QuadViewport {
            scale: 2.,
            size: Vec2::splat(64.),
            offset: Vec2::splat(8.),
            clip_size: Vec2::splat(32.),
            scroll: Vec2::ZERO,
        };
        let atlas = AtlasImage::new(32, 1, 1, vec![255; 32 * 32 * 4])?;
        let (sprite, transform) = project_quad(
            &quad,
            lighting.quad_tint(&quad, 0, QuadLayer::Overlay),
            7,
            &Handle::default(),
            &atlas,
            viewport,
        )
        .ok_or("visible quad")?;
        let mut vertices = Vertices::default();
        vertices.push(
            &lighting,
            &quad,
            0,
            QuadLayer::Overlay,
            (&sprite, transform, viewport),
        );
        assert_eq!(vertices.positions.len(), 4 * 5);
        assert_eq!(vertices.indices.len(), 3 * 4 * 6);
        assert!(
            vertices
                .colors
                .windows(2)
                .any(|pair| pair[0][0] != pair[1][0])
        );
        for ((position, uv), color) in vertices
            .positions
            .iter()
            .zip(&vertices.uv)
            .zip(&vertices.colors)
        {
            assert_eq!(position[2], 0.007);
            assert!((0.25..=1.).contains(&uv[0]));
            assert!((0. ..=1.).contains(&uv[1]));
            assert_eq!(color[3], 0.5);
            let screen = [position[0] * 2. + 24., 24. - position[1] * 2.];
            assert_eq!(
                *color,
                lighting.quad_tint_at(&quad, 0, QuadLayer::Overlay, screen)
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod reachability_tests {
    use super::*;
    use crate::play::{PlayView, Pushed, play_channel};
    use oteryn_renderer::TileCoord;
    use oteryn_session::{
        ActorPosition, EntityDetail, EntityKind, EntityRef, StepDirection, VisibleAppearance,
        WorldSpatialEntity,
    };

    #[test]
    fn received_creature_light_reaches_interpolated_bevy_mesh_and_clears_on_logout()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut view = PlayView::new(TileCoord::new(0, 0), 7, &[])?;
        view.apply_pushed(&Pushed {
            world_clock: Some(oteryn_session::world_clock::WorldClock {
                tibian_minute: 0,
                light_level: 0,
                light_color: 215,
            }),
            appearances: Some(vec![VisibleAppearance {
                entity: WorldSpatialEntity {
                    display_name: None,
                    kind: EntityKind::Player,
                    entity: EntityRef {
                        identity: [1; 16],
                        generation: 1,
                    },
                    position: ActorPosition {
                        x: 0,
                        y: 0,
                        floor: 7,
                    },
                    detail: EntityDetail::Actor {
                        direction: StepDirection::South,
                        appearance_ref: 1,
                        health_percent: 100,
                    },
                },
                look: None,
                light: Some((8, 215)),
                forge_stacks: None,
            }]),
            ..default()
        })?;
        let mut host = HostOwner::new(None)?;
        let (link, _, _) = play_channel();
        host.play = Some(super::super::super::owner::ActivePlay { view, link });
        let mut app = App::new();
        app.insert_non_send(host)
            .init_resource::<SceneCache>()
            .init_resource::<Time>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<ColorMaterial>>()
            .insert_resource(WorldViewport {
                origin: UVec2::ZERO,
                size: UVec2::new(640, 480),
            })
            .add_systems(Update, project);
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.update();
        let cache = &app.world().resource::<SceneCache>().smooth;
        assert_eq!(cache.batches.len(), 2);
        let entity = cache.batches[0].0;
        let handle = cache.batches[0].1.clone();
        let mesh = app
            .world()
            .resource::<Assets<Mesh>>()
            .get(&handle)
            .ok_or("world mesh")?;
        let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
            mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            return Err("missing interpolated vertex colors".into());
        };
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            return Err("missing world positions".into());
        };
        assert!(
            positions
                .windows(2)
                .zip(colors.windows(2))
                .any(|(p, c)| p[0][2] == p[1][2] && c[0][0] != c[1][0])
        );
        app.update();
        assert_eq!(
            app.world().resource::<SceneCache>().smooth.batches[0].0,
            entity
        );
        app.world_mut().non_send_mut::<HostOwner>().play = None;
        app.update();
        assert_eq!(
            app.world().get::<Visibility>(entity),
            Some(&Visibility::Hidden)
        );
        Ok(())
    }
}
