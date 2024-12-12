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

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SimpleColor>>,
) {
    let main_material = materials.add(SimpleColor {
        color: WHITE.into(),
    });
    let sphere_mesh = meshes.add(Sphere::default());

    // create a simple Perf UI with default settings
    // and all entries provided by the crate:
    commands.spawn(PerfUiDefaultEntries::default());

    for i in 1..=25 {
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
    query: Query<(Entity, &Birth, &Mesh3d, &MeshMaterial3d<TrailMaterial>)>,
    mut commands: Commands,
    mut materials: ResMut<Assets<TrailMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    time: Res<Time>,
) {
    query
        .iter()
        .for_each(|(entity, birth, mesh_3d, mesh_material)| {
            if time.elapsed_secs() - birth.0 > LIFETIME {
                commands.entity(entity).despawn();
                meshes.remove(mesh_3d);
                materials.remove(mesh_material);
            }
        });
}

fn update_position(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TrailMaterial>>,
    time: Res<Time>,
    mut query: Query<&mut Transform, With<End>>,
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

        let material = materials.add(TrailMaterial {
            alpha_mode: AlphaMode::Blend,
            color: GREY.into(),
            birth_time: time.elapsed_secs(),
            lifetime: LIFETIME,
        });

        commands.spawn((
            Mesh3d(
                meshes.add(
                    CylinderMeshBuilder::new(0.1, delta.length(), 4)
                        .anchor(CylinderAnchor::Bottom)
                        .without_caps()
                        .build(),
                ),
            ),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(old_translation).aligned_by(
                Dir3::Y,
                delta,
                Dir3::X,
                Dir3::X,
            ),
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
    alpha_mode: AlphaMode,
}

impl Material for TrailMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/trail.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
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
