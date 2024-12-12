use bevy::{
    color::palettes::css::{GREY, WHITE},
    prelude::*,
    render::{
        mesh::{CylinderAnchor, CylinderMeshBuilder},
        render_resource::{AsBindGroup, ShaderRef},
    },
};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use iyes_perf_ui::prelude::*;

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
        .add_systems(Update, update_position)
        .add_systems(Update, remove_old_ones)
        .run();
}

/// A marker component for our shapes so we can query them separately from the ground plane
#[derive(Component)]
struct End;

#[derive(Component)]
struct Birth(f32);

#[derive(Default, Resource)]
struct TrailInstance {
    mesh: Handle<Mesh>,
    material: Handle<TrailMaterial>,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut simple_color_materials: ResMut<Assets<SimpleColor>>,
    mut trail_materials: ResMut<Assets<TrailMaterial>>,
) {
    let main_material = simple_color_materials.add(SimpleColor {
        color: WHITE.into(),
    });
    let sphere_mesh = meshes.add(Sphere::default());

    let trail_material = trail_materials.add(TrailMaterial {
        color: GREY.into(),
        birth_time: 0.,
        lifetime: 100000000000.,
    });
    let trail_mesh = meshes.add(
        CylinderMeshBuilder::new(0.1, 1., 4)
            .anchor(CylinderAnchor::Bottom)
            .without_caps()
            .build(),
    );
    commands.insert_resource(TrailInstance {
        material: trail_material,
        mesh: trail_mesh,
    });

    // create a simple Perf UI with default settings
    // and all entries provided by the crate:
    commands.spawn(PerfUiDefaultEntries::default());

    for i in 1..=100 {
        let init_cond = i as f32 * 0.05;
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

const LIFETIME: f32 = 4.;

fn remove_old_ones(
    mut query: Query<(Entity, &Birth, &mut Transform)>,
    mut commands: Commands,
    time: Res<Time>,
) {
    query.iter_mut().for_each(|(entity, birth, mut transform)| {
        let ratio = (time.elapsed_secs() - birth.0) / LIFETIME;
        transform.scale.z = 1. - ratio;
        if ratio > 1. {
            commands.entity(entity).despawn();
        }
    });
}

fn update_position(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<&mut Transform, With<End>>,
    trail_instance: Res<TrailInstance>,
) {
    const SIGMA: f32 = 10.;
    const RHO: f32 = 28.;
    const BETA: f32 = 8. / 3.;

    for mut transform in &mut query {
        let old_translation = transform.translation.clone();

        let dx = SIGMA * (old_translation.y - old_translation.x);
        let dy = old_translation.x * (RHO - old_translation.z) - old_translation.y;
        let dz = old_translation.x * old_translation.y - BETA * old_translation.z;
        let dt = time.delta().as_secs_f32() * 0.5;

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
