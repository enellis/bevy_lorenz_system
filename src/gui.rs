use bevy::{ecs::system::SystemState, prelude::*, window::PrimaryWindow};
use bevy_egui::{egui, EguiContext, EguiPlugin};

use crate::{spawn_trail_heads, Configuration, SimpleColorMaterial, TimeOfBirth, TrailHead};

pub struct ControlUIPlugin;

impl Plugin for ControlUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin).add_systems(Update, control_ui);
    }
}

fn control_ui(world: &mut World) {
    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .get_single(world)
    else {
        return;
    };
    let mut egui_context = egui_context.clone();

    egui::Window::new("Control").show(egui_context.get_mut(), |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            if ui.button("Clear").clicked() {
                clear(world);
            };

            if ui.button("Start").clicked() {
                clear(world);
                start(world);
            };
        });
    });
}

fn clear(world: &mut World) {
    let mut system_state: SystemState<(
        Query<
            (Entity, &Mesh3d, &MeshMaterial3d<SimpleColorMaterial>),
            Or<(With<TrailHead>, With<TimeOfBirth>)>,
        >,
        ResMut<Assets<Mesh>>,
        ResMut<Assets<SimpleColorMaterial>>,
        Commands,
    )> = SystemState::new(world);

    let (mut query, mut meshes, mut simple_color_materials, mut commands) =
        system_state.get_mut(world);

    query.iter_mut().for_each(|(entity, mesh, material)| {
        commands.entity(entity).despawn_recursive();
        meshes.remove(mesh);
        simple_color_materials.remove(material);
    });

    system_state.apply(world);
}

fn start(world: &mut World) {
    let mut system_state: SystemState<(
        Commands,
        ResMut<Assets<Mesh>>,
        ResMut<Assets<SimpleColorMaterial>>,
        Res<Configuration>,
    )> = SystemState::new(world);

    let (mut commands, meshes, simple_color_materials, config) = system_state.get_mut(world);

    spawn_trail_heads(&mut commands, meshes, simple_color_materials, config);

    system_state.apply(world);
}
