use bevy::{
    prelude::*,
    render::{
        mesh::{CylinderAnchor, CylinderMeshBuilder},
        render_resource::{AsBindGroup, ShaderRef},
    },
};
use bevy_inspector_egui::{prelude::*, quick::ResourceInspectorPlugin};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use iyes_perf_ui::prelude::*;

const NUM_OF_TRAILS: i32 = 10;
const INITIAL_DISTANCE: f32 = 0.01;
const TRAIL_LIFETIME: u16 = 100; // in tenths of a second
const DELTA_T: u8 = 50;

#[derive(Reflect, Resource, Default, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
struct Configuration {
    trail_lifetime: u16, // in tenths of a second
    delta_t: u8,
    physics_refresh_rate: u16,
}

#[derive(Component)]
struct TrailHead;

#[derive(Component)]
struct TrailData {
    mesh: Handle<Mesh>,
    material: Handle<SimpleColorMaterial>,
}

#[derive(Component)]
struct TimeOfBirth(f32);

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MaterialPlugin::<SimpleColorMaterial>::default(),
            PanOrbitCameraPlugin,
        ))
        //
        .add_plugins((
            bevy::diagnostic::FrameTimeDiagnosticsPlugin,
            bevy::diagnostic::EntityCountDiagnosticsPlugin,
            bevy::diagnostic::SystemInformationDiagnosticsPlugin,
        ))
        .add_plugins(PerfUiPlugin)
        //
        .insert_resource(Configuration {
            trail_lifetime: TRAIL_LIFETIME,
            delta_t: DELTA_T,
            physics_refresh_rate: 120,
        })
        .register_type::<Configuration>()
        .add_plugins(ResourceInspectorPlugin::<Configuration>::default())
        //
        .add_systems(Startup, setup)
        .add_systems(Update, check_config_change)
        // .add_systems(Update, rotate_camera)
        .add_systems(FixedUpdate, update_position)
        .add_systems(Update, (remove_old_trail_segments, shrink_trail_segments))
        //
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut simple_color_materials: ResMut<Assets<SimpleColorMaterial>>,
    config: Res<Configuration>,
) {
    commands.insert_resource(Time::<Fixed>::from_hz(config.physics_refresh_rate as f64));

    // create a simple Perf UI with default settings
    // and all entries provided by the crate:
    commands.spawn(PerfUiDefaultEntries::default());

    let head_mesh = meshes.add(Sphere::new(0.3));
    let trail_mesh = meshes.add(
        CylinderMeshBuilder::new(0.12, 1., 32)
            .anchor(CylinderAnchor::Bottom)
            .without_caps()
            .build(),
    );

    for i in 1..=NUM_OF_TRAILS {
        let ratio = i as f32 / NUM_OF_TRAILS as f32;

        let head_color = Hsla::hsl(ratio * 360., 0.7, 0.5);
        let head_material = simple_color_materials.add(SimpleColorMaterial {
            color: head_color.into(),
        });
        let trail_material = simple_color_materials.add(SimpleColorMaterial {
            color: head_color.with_saturation(0.3).into(),
        });

        let initial_pos = i as f32 * INITIAL_DISTANCE;
        commands.spawn((
            TrailHead,
            Mesh3d(head_mesh.clone()),
            MeshMaterial3d(head_material.clone()),
            Transform::from_translation(Vec3::splat(initial_pos)),
            TrailData {
                mesh: trail_mesh.clone(),
                material: trail_material.clone(),
            },
        ));
    }

    commands.spawn((
        Transform::from_translation(Vec3::new(1., 0., 1.) * 80.),
        PanOrbitCamera {
            focus: Vec3::new(0., 0., 30.),
            ..default()
        },
    ));
}

fn check_config_change(config: Res<Configuration>, mut fixed_time: ResMut<Time<Fixed>>) {
    if config.is_changed() {
        fixed_time.set_timestep_hz(std::cmp::max(config.physics_refresh_rate, 1) as f64);
    }
}

// fn rotate_camera(mut query: Query<&mut PanOrbitCamera>) {
//     for mut camera in &mut query {
//         camera.target_yaw += 0.001;
//     }
// }

fn update_position(
    mut query: Query<(&mut Transform, &TrailData), With<TrailHead>>,
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    config: Res<Configuration>,
) {
    for (mut transform, trail_data) in &mut query {
        let old_translation = transform.translation.clone();

        const SIGMA: f32 = 10.;
        const RHO: f32 = 28.;
        const BETA: f32 = 8. / 3.;

        let dx = SIGMA * (old_translation.y - old_translation.x);
        let dy = old_translation.x * (RHO - old_translation.z) - old_translation.y;
        let dz = old_translation.x * old_translation.y - BETA * old_translation.z;
        let dt = config.delta_t as f32 / 10000.;

        let delta = Vec3::new(dx, dy, dz) * dt;
        let new_translation = old_translation + delta;
        transform.translation = new_translation;

        commands.spawn((
            Mesh3d(trail_data.mesh.clone()),
            MeshMaterial3d(trail_data.material.clone()),
            Transform::from_translation(old_translation)
                .with_scale(Vec3::new(1., delta.length(), 1.))
                .aligned_by(Dir3::Y, delta, Dir3::X, Dir3::X),
            TimeOfBirth(time.elapsed_secs()),
        ));
    }
}

fn shrink_trail_segments(
    mut query: Query<(&mut TimeOfBirth, &mut Transform)>,
    time: Res<Time>,
    config: Res<Configuration>,
) {
    query
        .par_iter_mut()
        .for_each(|(mut time_of_birth, mut transform)| {
            let ratio = 1.
                - ((time.elapsed_secs() - time_of_birth.0) / (config.trail_lifetime as f32 / 10.));
            transform.scale.x = ratio;
            transform.scale.z = ratio;
            if ratio < 0. {
                // Set time of birth to 0, so we can clean it up later.
                time_of_birth.0 = 0.
            }
        });
}

fn remove_old_trail_segments(query: Query<(Entity, &TimeOfBirth)>, mut commands: Commands) {
    query.iter().for_each(|(entity, time_of_birth)| {
        if time_of_birth.0 == 0. {
            commands.entity(entity).despawn();
        }
    });
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
