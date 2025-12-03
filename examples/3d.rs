use std::collections::HashMap;

use bevy::prelude::*;
use bracket_fast_noise::prelude::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use worley_biomes::{
    bevy::debug_plugin::{DebugColor, DebugPluginSettings, GetWorley, WorleyImage},
    biome_picker::{BiomeVariants, SimpleBiomePicker},
    distance_fn::DistanceFn,
    worley::Worley,
};

use bevy_inspector_egui::bevy_egui::EguiPlugin;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Default)]
enum BiomeType {
    #[default]
    Desert,
    Forest,
    Snow,
    Plains,
}

impl BiomeVariants for BiomeType {
    fn variants() -> &'static [Self] {
        &[Self::Forest, Self::Snow, Self::Plains]
    }
}

use bevy::color::palettes::basic::*;
impl DebugColor<BiomeType> for BiomeType {
    fn get_color(&self) -> Srgba {
        match self {
            BiomeType::Desert => YELLOW,
            BiomeType::Forest => GREEN,
            BiomeType::Snow => BLUE,
            BiomeType::Plains => RED,
        }
    }
}

impl BiomeType {
    fn height(&self) -> f32 {
        match self {
            BiomeType::Desert => 0.0,
            BiomeType::Forest => 10.0,
            BiomeType::Snow => 25.0,
            BiomeType::Plains => 40.0,
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        // THE DEBUG PLUGIN for worley preview + tweak ui
        .add_plugins(worley_biomes::bevy::debug_plugin::DebugPlugin::<
            WorleyHolder,
            BiomeType,
            SimpleBiomePicker<BiomeType>,
        > {
            // you can customize some parts of the tweak ui
            settings: DebugPluginSettings {
                spawn_preview_image: true,
                show_preview_image: true,
                show_inspector_ui: true,
            },
            ..default()
        })
        .insert_resource(VoxelMaterials(HashMap::new()))
        .insert_resource(Offset { x: 0.0, z: 0.0 })
        .add_systems(Startup, setup)
        .add_systems(Startup, setup_voxels)
        .add_systems(PostUpdate, update_voxel_from_worley)
        .add_systems(Update, move_input)
        .add_systems(Update, toggle_preview_visibility)
        .add_systems(Update, animate_height)
        .run();
}

///! resource that holds our worley settings
#[derive(Resource, Default)]
struct WorleyHolder {
    worley: Worley<BiomeType, SimpleBiomePicker<BiomeType>>,
}

// the debug plugin requires methods to access the Worley
impl GetWorley<BiomeType, SimpleBiomePicker<BiomeType>> for WorleyHolder {
    fn get_worley(&self) -> &Worley<BiomeType, SimpleBiomePicker<BiomeType>> {
        &self.worley
    }
    fn get_worley_mut(&mut self) -> &mut Worley<BiomeType, SimpleBiomePicker<BiomeType>> {
        &mut self.worley
    }
}

///! avoid duplication of same color voxel material
#[derive(Resource)]
pub struct VoxelMaterials(HashMap<(u8, u8, u8), Handle<StandardMaterial>>);

///! how many voxels to generate
pub const GRID_SIZE: i32 = 32 * 4;

///! store the voxel x,z pos to later find the correct voxel to update
#[derive(Component)]
struct VoxelCoord {
    gx: i32,
    gz: i32,
}

// tap space to show/hide the preview image and tweak ui
fn toggle_preview_visibility(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut debug_plugin_settings: ResMut<DebugPluginSettings>,
) {
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    }
    // toggle visibility, and keep the show, in sync
    debug_plugin_settings.show_preview_image = !debug_plugin_settings.show_preview_image;
    debug_plugin_settings.show_inspector_ui = debug_plugin_settings.show_preview_image;
}

///! initially spawn all "voxels"
fn setup_voxels(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    // one shared cube mesh
    let cube_mesh = meshes.add(Mesh::from(Cuboid::new(1.0, 1.0, 1.0)));

    for gx in 0..GRID_SIZE {
        for gz in 0..GRID_SIZE {
            commands.spawn((
                VoxelCoord { gx, gz },
                Mesh3d(cube_mesh.clone()),
                // assign a default material for now, will be updated later
                MeshMaterial3d(Handle::<StandardMaterial>::default()),
                Transform::from_translation(Vec3::new(
                    gx as f32 - GRID_SIZE as f32 / 2.0,
                    0.0,
                    gz as f32 - GRID_SIZE as f32 / 2.0,
                )),
                TargetHeight(0.0f32),
            ));
        }
    }
}

///! what's the generated height of the "voxel"
#[derive(Component)]
pub struct TargetHeight(f32);

///! fetch worley data to UPDATE the voxel height + material
fn update_voxel_from_worley(
    worley_holder: Res<WorleyHolder>,
    mut voxels: Query<(
        &VoxelCoord,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut TargetHeight,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut voxel_materials: ResMut<VoxelMaterials>,
    offset: Res<Offset>,
) {
    if !worley_holder.is_changed() {
        return;
    }

    let worley = &worley_holder.worley;

    for (coord, mut mat, mut target_height) in voxels.iter_mut() {
        let gx = coord.gx;
        let gz = coord.gz;
        let weights = worley.get(gx as f64 + offset.x, gz as f64 + offset.z);
        // blend colors
        let mut r = 0.0;
        let mut g = 0.0;
        let mut b = 0.0;
        let mut height = 0.0;
        let mut wsum = 0.0;
        for (w, biome) in &weights {
            let c = biome.get_color();
            r += c.red as f64 * w;
            g += c.green as f64 * w;
            b += c.blue as f64 * w;
            height += biome.height() * *w as f32;
            wsum += w;
        }
        let color = Srgba::new(r as f32, g as f32, b as f32, 1.0);
        let (color, key) = quantize_srgba(color, 32);
        let color_material = voxel_materials
            .0
            .entry(key)
            .or_insert_with(|| materials.add(Color::Srgba(color)));

        *mat = MeshMaterial3d(color_material.clone());
        target_height.0 = height;
    }
}

///! move voxels scale to their target height
fn animate_height(mut query: Query<(&mut Transform, &TargetHeight)>, time: Res<Time>) {
    for (mut transform, target_height) in query.iter_mut() {
        transform.scale.y = transform
            .scale
            .y
            .lerp(target_height.0, time.delta_secs() * 7.0);
    }
}

///! offset the worley position, so we can move around
#[derive(Resource)]
pub struct Offset {
    x: f64,
    z: f64,
}

fn move_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut offset: ResMut<Offset>,
    time: Res<Time>,
    mut worley_holder: ResMut<WorleyHolder>,
    mut worley_image: Option<ResMut<WorleyImage>>,
) {
    let speed = 32.0;
    let f = speed * time.delta_secs_f64();
    let mut changed = false;
    if keyboard.pressed(KeyCode::KeyD) {
        offset.x += f;
        changed = true;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        offset.x -= f;
        changed = true;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        offset.z += f;
        changed = true;
    }
    if keyboard.pressed(KeyCode::KeyW) {
        offset.z -= f;
        changed = true;
    }
    if changed {
        worley_holder.set_changed();
        if let Some(worley_image) = &mut worley_image {
            worley_image.preview_offset.0 = offset.x;
            worley_image.preview_offset.1 = offset.z;
        }
    }
}

fn setup(mut commands: Commands) {
    // SETUP OUR WORLEY VALUES
    let mut worley: Worley<BiomeType, SimpleBiomePicker<BiomeType>> = Worley::default();
    worley.zoom = 62.0;
    worley.set_distance_fn(DistanceFn::Chebyshev);
    worley.biome_picker = SimpleBiomePicker::Any;
    worley.sharpness = 20.0;
    worley.k = 3;
    worley.warp_settings.strength = 0.6;
    worley.warp_settings.noise.set_seed(0);
    worley.warp_settings.noise.frequency = 0.7;
    worley.warp_settings.noise.fractal_lacunarity = 2.0;
    worley.warp_settings.noise.set_fractal_gain(0.6);
    worley.warp_settings.noise.fractal_octaves = 3;
    worley.warp_settings.noise.noise_type = NoiseType::PerlinFractal;
    worley.warp_settings.noise.fractal_type = FractalType::FBM;
    commands.insert_resource(WorleyHolder { worley });

    commands.spawn((
        DirectionalLight { ..default() },
        Transform::from_xyz(0.0, 0.0, 0.0).looking_at(Vec3::new(-0.15, -0.05, 0.25), Vec3::Y),
    ));
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            ..default()
        },
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 77.5, -114.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Quantize an Srgba color so each component is divisible by `step`.
/// Example: step = 32 → each channel maps into {0, 32, 64, …, 224, 255}
/// returns the color + rgb values you can use for a hash key
fn quantize_srgba(color: Srgba, step: u8) -> (Srgba, (u8, u8, u8)) {
    // convert float [0.0..1.0] into byte [0..255]
    let r = (color.red * 255.0).round().clamp(0.0, 255.0) as u8;
    let g = (color.green * 255.0).round().clamp(0.0, 255.0) as u8;
    let b = (color.blue * 255.0).round().clamp(0.0, 255.0) as u8;

    // quantize each component down to nearest multiple of `step`
    let rq = (r / step) * step;
    let gq = (g / step) * step;
    let bq = (b / step) * step;
    let aq = 255;

    let key = (rq, gq, bq);
    // back to normalized floats
    (
        Srgba::new(
            rq as f32 / 255.0,
            gq as f32 / 255.0,
            bq as f32 / 255.0,
            aq as f32 / 255.0,
        ),
        key,
    )
}
