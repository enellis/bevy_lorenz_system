use bevy::{
    color::palettes::css::{GREY, WHITE},
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

const NUM_OF_TRAILS: i32 = 5;
const DIST_BTWN_TRAILS: f32 = 0.01;
const TRAIL_LIFETIME: u16 = 50;
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
struct End;

#[derive(Component)]
struct Birth(f32);

#[derive(Default, Resource)]
struct TrailInstance {
    mesh: Handle<Mesh>,
    material: Handle<SimpleColor>,
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MaterialPlugin::<SimpleColor>::default(),
            MaterialPlugin::<TrailMaterial>::default(),
            PanOrbitCameraPlugin,
        ))
        // we want Bevy to measure these values for us:
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugins(bevy::diagnostic::EntityCountDiagnosticsPlugin)
        .add_plugins(bevy::diagnostic::SystemInformationDiagnosticsPlugin)
        .add_plugins(PerfUiPlugin)
        //
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, update_position)
        .add_systems(Update, (remove_old_ones, shrink_trail))
        .add_systems(Update, check_config_change)
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
    let main_material = simple_color_materials.add(SimpleColor {
        color: WHITE.into(),
    });
    let sphere_mesh = meshes.add(Sphere::new(0.3));

    let trail_material = simple_color_materials.add(SimpleColor { color: GREY.into() });
    let trail_mesh = meshes.add(
        CylinderMeshBuilder::new(0.12, 1., 32)
            .anchor(CylinderAnchor::Bottom)
            .without_caps()
            .build(),
    );
    commands.insert_resource(TrailInstance {
        material: trail_material,
        mesh: trail_mesh,
    });

    commands.insert_resource(Time::<Fixed>::from_hz(config.physics_refresh_rate as f64));

    // create a simple Perf UI with default settings
    // and all entries provided by the crate:
    commands.spawn(PerfUiDefaultEntries::default());

    for i in 1..=NUM_OF_TRAILS {
        let init_cond = i as f32 * DIST_BTWN_TRAILS;
        commands.spawn((
            Mesh3d(sphere_mesh.clone()),
            MeshMaterial3d(main_material.clone()),
            Transform::from_xyz(init_cond, init_cond, init_cond),
            End,
        ));
    }

    commands.spawn((
        Transform::from_translation(Vec3::new(1., 0., 0.) * 100.)
            .looking_at(Vec3::new(1., 1., 1.), Vec3::Y),
        PanOrbitCamera::default(),
    ));
}

fn check_config_change(config: Res<Configuration>, mut fixed_time: ResMut<Time<Fixed>>) {
    if config.is_changed() {
        *fixed_time = Time::<Fixed>::from_hz(std::cmp::max(config.physics_refresh_rate, 1) as f64);
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
    mut query: Query<&mut Transform, With<End>>,
    trail_instance: Res<TrailInstance>,
    config: Res<Configuration>,
) {
    const SIGMA: f32 = 10.;
    const RHO: f32 = 28.;
    const BETA: f32 = 8. / 3.;

    for mut transform in &mut query {
        let old_translation = transform.translation.clone();

        let dx = SIGMA * (old_translation.y - old_translation.x);
        let dy = old_translation.x * (RHO - old_translation.z) - old_translation.y;
        let dz = old_translation.x * old_translation.y - BETA * old_translation.z;
        let dt = config.delta_t as f32 / 10000.;

        let delta = Vec3::new(dx, dy, dz) * dt;
        let new_translation = old_translation + delta;
        transform.translation = new_translation;

        commands.spawn((
            Mesh3d(trail_instance.mesh.clone()),
            MeshMaterial3d(trail_instance.material.clone()),
            Transform::from_translation(old_translation)
                .with_scale(Vec3::new(1., delta.length(), 1.))
                .aligned_by(Dir3::Y, delta, Dir3::X, Dir3::X),
            Birth(time.elapsed_secs()),
        ));
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct TrailMaterial {
    #[uniform(0)]
    color: LinearRgba,
    #[uniform(1)]
    birth_time: f32,
    #[uniform(2)]
    lifetime: f32,
}

impl Material for TrailMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/trail.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
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
