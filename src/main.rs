mod gui;
mod trails;

use std::collections::VecDeque;

use bevy::{
    prelude::*,
    render::{
        mesh::{CylinderAnchor, CylinderMeshBuilder},
        render_resource::{AsBindGroup, ShaderRef},
        view::NoFrustumCulling,
    },
};
use bevy_inspector_egui::{prelude::*, quick::ResourceInspectorPlugin};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use gui::ControlUIPlugin;
use iyes_perf_ui::prelude::*;
use trails::{TrailMaterialPlugin, TrailSegment, Trails};

const NUM_OF_TRAILS: u16 = 10;
const INITIAL_DISTANCE: f32 = 0.01;
const TRAIL_LIFETIME: u16 = 100; // in tenths of a second
const DELTA_T: u8 = 50;

#[derive(Reflect, Resource, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
struct Configuration {
    show_diagnostics: bool,
    rotate_camera: bool,
    camera_speed: i32,
    physics_refresh_rate: u16,
    trail_lifetime: u16, // in tenths of a second
    num_of_trails: u16,
    initial_distance: f32,
    delta_t: u8,
    sigma: f32,
    rho: f32,
    beta: f32,
    trail_segment_count: usize,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            show_diagnostics: false,
            rotate_camera: false,
            camera_speed: 10,
            physics_refresh_rate: 120,
            trail_lifetime: TRAIL_LIFETIME,
            num_of_trails: NUM_OF_TRAILS,
            initial_distance: INITIAL_DISTANCE,
            delta_t: DELTA_T,
            sigma: 10.,
            rho: 28.,
            beta: 8. / 3.,
            trail_segment_count: 0,
        }
    }
}

#[derive(Component)]
struct TrailHead;

#[derive(Component)]
struct TrailData {
    color: LinearRgba,
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            ControlUIPlugin,
            MaterialPlugin::<SimpleColorMaterial>::default(),
            TrailMaterialPlugin,
            PanOrbitCameraPlugin,
        ))
        //
        .add_plugins((
            bevy::diagnostic::FrameTimeDiagnosticsPlugin,
            bevy::diagnostic::EntityCountDiagnosticsPlugin,
            bevy::diagnostic::SystemInformationDiagnosticsPlugin,
        ))
        .add_plugins(PerfUiPlugin)
        .add_systems(
            Update,
            toggle_diagnostics
                .before(iyes_perf_ui::PerfUiSet::Setup)
                .run_if(|config: Res<Configuration>| config.is_changed()),
        )
        //
        .insert_resource(Configuration::default())
        .register_type::<Configuration>()
        .add_plugins(ResourceInspectorPlugin::<Configuration>::default())
        //
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (apply_new_lifetime, apply_physics_refresh_rate)
                .run_if(|config: Res<Configuration>| config.is_changed()),
        )
        .add_systems(
            Update,
            rotate_camera.run_if(|config: Res<Configuration>| config.rotate_camera),
        )
        .add_systems(FixedUpdate, update_position)
        //
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    simple_color_materials: ResMut<Assets<SimpleColorMaterial>>,
    config: Res<Configuration>,
) {
    commands.insert_resource(Time::<Fixed>::from_hz(config.physics_refresh_rate as f64));

    let mut segments_data = VecDeque::with_capacity(16384);
    // Segments data must not be empty
    segments_data.push_back(TrailSegment::default());

    commands.spawn((
        Mesh3d(
            meshes.add(
                CylinderMeshBuilder::new(0.12, 1., 32)
                    .anchor(CylinderAnchor::Bottom)
                    .without_caps()
                    .build(),
            ),
        ),
        Trails {
            segments: segments_data,
        },
        NoFrustumCulling,
    ));

    spawn_trail_heads(&mut commands, meshes, simple_color_materials, config);

    commands.spawn((
        Transform::from_translation(Vec3::new(1., 0., 1.) * 80.),
        PanOrbitCamera {
            focus: Vec3::new(0., 0., 30.),
            ..default()
        },
    ));
}

fn spawn_trail_heads(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut simple_color_materials: ResMut<Assets<SimpleColorMaterial>>,
    config: Res<Configuration>,
) {
    let head_mesh = meshes.add(Sphere::new(0.3));

    for i in 1..=config.num_of_trails {
        let ratio = i as f32 / NUM_OF_TRAILS as f32;

        let head_color = Hsla::hsl(ratio * 360., 0.7, 0.5);
        let head_material = simple_color_materials.add(SimpleColorMaterial {
            color: head_color.into(),
        });

        let initial_pos = i as f32 * config.initial_distance;
        commands.spawn((
            TrailHead,
            Mesh3d(head_mesh.clone()),
            MeshMaterial3d(head_material.clone()),
            Transform::from_translation(Vec3::splat(initial_pos)),
            TrailData {
                color: head_color.with_saturation(0.3).into(),
            },
        ));
    }
}

fn apply_new_lifetime(mut query: Query<&mut Trails>, config: Res<Configuration>) {
    let mut trails = query.single_mut();
    let new_lifetime = config.trail_lifetime as f32 / 10.;
    if trails
        .segments
        .front()
        .is_some_and(|segment| segment.lifetime != new_lifetime)
    {
        trails
            .segments
            .iter_mut()
            .for_each(|segment| segment.lifetime = new_lifetime);
    }
}

fn apply_physics_refresh_rate(config: Res<Configuration>, mut fixed_time: ResMut<Time<Fixed>>) {
    fixed_time.set_timestep_hz(std::cmp::max(config.physics_refresh_rate, 1) as f64);
}

fn toggle_diagnostics(
    mut commands: Commands,
    q_root: Query<Entity, With<PerfUiRoot>>,
    config: Res<Configuration>,
) {
    if config.show_diagnostics {
        if q_root.get_single().is_err() {
            commands.spawn(PerfUiDefaultEntries::default());
        }
    } else {
        if let Ok(e) = q_root.get_single() {
            commands.entity(e).despawn_recursive();
        }
    }
}

fn rotate_camera(mut query: Query<&mut PanOrbitCamera>, config: Res<Configuration>) {
    for mut camera in &mut query {
        camera.target_yaw += config.camera_speed as f32 / 10_000.;
    }
}

fn update_position(
    mut q_heads: Query<(&mut Transform, &TrailData)>,
    mut q_trails: Query<&mut Trails>,
    time: Res<Time<Virtual>>,
    mut config: ResMut<Configuration>,
) {
    let mut trails = q_trails.single_mut();

    // Delete old segments
    if let Some(index) = trails.segments.iter().position(|segment| {
        time.elapsed_secs() - segment.birth_time < config.trail_lifetime as f32 / 10.
    }) {
        trails.segments.drain(..index);
    };

    for (mut transform, trail_data) in &mut q_heads {
        let old_translation = transform.translation.clone();

        let dx = config.sigma * (old_translation.y - old_translation.x);
        let dy = old_translation.x * (config.rho - old_translation.z) - old_translation.y;
        let dz = old_translation.x * old_translation.y - config.beta * old_translation.z;
        let dt = config.delta_t as f32 / 10000.;

        let delta = Vec3::new(dx, dy, dz) * dt;
        let new_translation = old_translation + delta;
        transform.translation = new_translation;

        trails.segments.push_back(TrailSegment {
            position: old_translation,
            length: delta.length(),
            rotation: Quat::from_rotation_arc(Vec3::Y, delta.normalize()).to_array(),
            color: trail_data.color.to_vec3(),
            birth_time: time.elapsed_secs(),
            lifetime: config.trail_lifetime as f32 / 10.,
        });
    }

    config.trail_segment_count = trails.segments.len();
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct SimpleColorMaterial {
    #[uniform(0)]
    color: LinearRgba,
}

impl Material for SimpleColorMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/simple_color.wgsl".into()
    }
}
