use bevy::{
    prelude::*,
    render::{
        mesh::{CylinderAnchor, CylinderMeshBuilder},
        render_resource::{AsBindGroup, ShaderRef},
    },
};
use bevy_inspector_egui::prelude::*;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use iyes_perf_ui::prelude::*;

const NUM_OF_TRAILS: i32 = 10;
const DIST_BTWN_TRAILS: f32 = 0.01;
const TRAIL_LIFETIME: u16 = 100;
const DELTA_T: u8 = 50;

#[derive(Reflect, Resource, Default, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
struct Configuration {
    trail_lifetime: u16,
    delta_t: u8,
    physics_refresh_rate: u16,
}

/// A marker component for our shapes so we can query them separately from the ground plane
#[derive(Component)]
struct Head;

#[derive(Component)]
struct TrailData {
    mesh: Handle<Mesh>,
    material: Handle<SimpleColor>,
}

#[derive(Component)]
struct Birth(f32);

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MaterialPlugin::<SimpleColor>::default(),
            PanOrbitCameraPlugin,
        ))
        // we want Bevy to measure these values for us:
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugins(bevy::diagnostic::EntityCountDiagnosticsPlugin)
        .add_plugins(bevy::diagnostic::SystemInformationDiagnosticsPlugin)
        // .add_plugins(PerfUiPlugin)
        //
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, update_position)
        .add_systems(Update, (remove_old_ones, shrink_trail))
        .add_systems(Update, check_config_change)
        .add_systems(Update, rotate_camera)
        //
        .insert_resource(Configuration {
            trail_lifetime: TRAIL_LIFETIME,
            delta_t: DELTA_T,
            physics_refresh_rate: 120,
        })
        .register_type::<Configuration>()
        .add_plugins(ResourceInspectorPlugin::<Configuration>::default())
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut simple_color_materials: ResMut<Assets<SimpleColor>>,
    config: Res<Configuration>,
) {
    commands.insert_resource(Time::<Fixed>::from_hz(config.physics_refresh_rate as f64));

    let sphere_mesh = meshes.add(Sphere::new(0.3));
    let trail_mesh = meshes.add(
        CylinderMeshBuilder::new(0.12, 1., 32)
            .anchor(CylinderAnchor::Bottom)
            .without_caps()
            .build(),
    );

    // create a simple Perf UI with default settings
    // and all entries provided by the crate:
    commands.spawn(PerfUiDefaultEntries::default());

    for i in 1..=NUM_OF_TRAILS {
        let ratio = i as f32 / NUM_OF_TRAILS as f32;
        let main_color = Hsla::hsl(ratio * 360., 0.7, 0.5);
        let main_material = simple_color_materials.add(SimpleColor {
            color: main_color.into(),
        });
        let trail_color = simple_color_materials.add(SimpleColor {
            color: main_color.with_saturation(0.3).into(),
        });

        let init_cond = i as f32 * DIST_BTWN_TRAILS;
        commands.spawn((
            Head,
            Mesh3d(sphere_mesh.clone()),
            MeshMaterial3d(main_material.clone()),
            Transform::from_xyz(init_cond, init_cond, init_cond),
            TrailData {
                mesh: trail_mesh.clone(),
                material: trail_color.clone(),
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

fn rotate_camera(mut query: Query<&mut PanOrbitCamera>) {
    for mut camera in &mut query {
        camera.target_yaw += 0.001;
    }
}

fn shrink_trail(
    mut query: Query<(&mut Birth, &mut Transform)>,
    time: Res<Time>,
    config: Res<Configuration>,
) {
    query.par_iter_mut().for_each(|(mut birth, mut transform)| {
        let ratio = 1. - ((time.elapsed_secs() - birth.0) / (config.trail_lifetime as f32 / 10.));
        transform.scale.x = ratio;
        transform.scale.z = ratio;
        if ratio < 0. {
            birth.0 = 0.
        }
    });
}

fn remove_old_ones(query: Query<(Entity, &Birth)>, mut commands: Commands) {
    query.iter().for_each(|(entity, birth)| {
        if birth.0 == 0. {
            commands.entity(entity).despawn();
        }
    });
}

fn update_position(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    mut query: Query<(&mut Transform, &TrailData), With<Head>>,
    config: Res<Configuration>,
) {
    const SIGMA: f32 = 10.;
    const RHO: f32 = 28.;
    const BETA: f32 = 8. / 3.;

    for (mut transform, trail_data) in &mut query {
        let old_translation = transform.translation.clone();

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
            Birth(time.elapsed_secs()),
        ));
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct SimpleColor {
    #[uniform(0)]
    color: LinearRgba,
}

impl Material for SimpleColor {
    fn fragment_shader() -> ShaderRef {
        "shaders/simple_color.wgsl".into()
    }
}
