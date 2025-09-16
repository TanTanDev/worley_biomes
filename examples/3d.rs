use std::{collections::HashMap, marker::PhantomData};

use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension},
};
use bevy_inspector_egui::{
    bevy_egui::{self, EguiContext, EguiPlugin, EguiPrimaryContextPass},
    egui,
    quick::WorldInspectorPlugin,
};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use worley_biomes::{
    biome_picker::{Biome, BiomeGenerator},
    distance_fn::DistanceFn,
    warp::{FractalType, NoiseType, WarpSettings},
    worley::Worley,
};

// === Biome system ===
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum BiomeType {
    Desert,
    Forest,
    Snow,
    Plains,
}

impl Biome for BiomeType {
    fn variants() -> &'static [Self] {
        &[Self::Desert, Self::Forest, Self::Snow, Self::Plains]
    }
}

fn biome_color(b: BiomeType) -> Srgba {
    use bevy::color::palettes::basic::*;

    match b {
        BiomeType::Desert => YELLOW,
        BiomeType::Forest => GREEN,
        BiomeType::Snow => BLUE,
        BiomeType::Plains => RED,
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

///! size of the visible worley texture on screen
#[derive(Component, Default)]
enum DisplayTextureSize {
    #[default]
    Auto,
    Medium,
    Big,
}

impl DisplayTextureSize {
    pub fn toggle(&mut self) {
        match self {
            DisplayTextureSize::Auto => *self = DisplayTextureSize::Medium,
            DisplayTextureSize::Medium => *self = DisplayTextureSize::Big,
            DisplayTextureSize::Big => *self = DisplayTextureSize::Auto,
        }
    }
    pub fn node_size(&self) -> bevy::ui::Val {
        match self {
            DisplayTextureSize::Auto => bevy::ui::Val::Auto,
            DisplayTextureSize::Medium => bevy::ui::Val::Px(300.0),
            DisplayTextureSize::Big => bevy::ui::Val::Px(600.0),
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_plugins(WorldInspectorPlugin::new())
        .insert_resource(VoxelMaterials(HashMap::new()))
        .insert_resource(Offset { x: 0.0, z: 0.0 })
        .add_systems(Startup, setup)
        .add_systems(Startup, setup_voxels)
        .add_systems(PostUpdate, rebuild_map)
        .add_systems(Update, move_input)
        .add_systems(Update, texture_tap)
        .add_systems(Update, animate_height)
        .add_systems(EguiPrimaryContextPass, inspector_ui)
        .run();
}

///! the worley generator
#[derive(Resource)]
struct MapSettings {
    worley: Worley<BiomeType>,
}

///! avoid duplication of same color voxel material
#[derive(Resource)]
pub struct VoxelMaterials(HashMap<(u8, u8, u8), Handle<StandardMaterial>>);

// pub const GRID_SIZE: i32 = 32;
///! how many voxels to generate
pub const GRID_SIZE: i32 = 32 * 4;

///! store the voxel x,z pos to later find the correct voxel to update
#[derive(Component)]
struct VoxelCoord {
    gx: i32,
    gz: i32,
}

///! initially spawn all "voxels"
fn setup_voxels(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    // one shared cube mesh
    let cube_mesh = meshes.add(Mesh::from(Cuboid::new(1.0, 1.0, 1.0)));

    for gx in 0..GRID_SIZE {
        for gz in 0..GRID_SIZE {
            commands.spawn((
                VoxelTag,
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

///! toggle the preview image size
fn texture_tap(
    mut interaction_query: Query<
        (&Interaction, &mut DisplayTextureSize, &mut Node),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut display_size, mut node) in interaction_query.iter_mut() {
        if let Interaction::Pressed = interaction {
            // toggle
            display_size.toggle();
            node.width = display_size.node_size();
        }
    }
}

///! what's the generated height of the "voxel"
#[derive(Component)]
pub struct TargetHeight(f32);

///! fetch worley data to UPDATE the voxel height + material
fn rebuild_map(
    map_settings: Res<MapSettings>,
    mut voxels: Query<
        (
            &VoxelCoord,
            &mut MeshMaterial3d<StandardMaterial>,
            &mut TargetHeight,
        ),
        With<VoxelTag>,
    >,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut voxel_materials: ResMut<VoxelMaterials>,
    worley_image: Option<ResMut<WorleyImage>>,
    offset: Res<Offset>,
) {
    if !map_settings.is_changed() {
        return;
    }

    let mut img_data = Vec::new();
    let worley = &map_settings.worley;

    for (coord, mut mat, mut target_height) in voxels.iter_mut() {
        let gx = coord.gx;
        let gz = coord.gz;

        let weights = worley.get(WORLD_SEED, gx as f64 + offset.x, gz as f64 + offset.z);

        // blend colors
        let mut r = 0.0;
        let mut g = 0.0;
        let mut b = 0.0;
        let mut height = 0.0;
        let mut wsum = 0.0;
        for (w, biome) in &weights {
            let c = biome_color(*biome);
            r += c.red as f64 * w;
            g += c.green as f64 * w;
            b += c.blue as f64 * w;
            height += biome.height() * *w as f32;
            wsum += w;
        }

        let color = Srgba::new(r as f32, g as f32, b as f32, 1.0);
        let (color, key) = quantize_srgba(color, 32);
        img_data.push((color.red * 255.0) as u8);
        img_data.push((color.green * 255.0) as u8);
        img_data.push((color.blue * 255.0) as u8);
        img_data.push(255 as u8);

        let color_material = voxel_materials
            .0
            .entry(key)
            .or_insert_with(|| materials.add(Color::Srgba(color)));

        *mat = MeshMaterial3d(color_material.clone());
        target_height.0 = height;
    }
    match worley_image {
        Some(worley_image) => {
            let image = images.get_mut(&worley_image.handle).expect("image");
            image.data = Some(img_data);
        }
        None => {
            // make image
            let mut img = Image::new(
                Extent3d {
                    width: GRID_SIZE as u32,
                    height: GRID_SIZE as u32,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                img_data,
                bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
            );
            img.sampler = ImageSampler::nearest();
            let image_handle = images.add(img);

            // spawn visual representation
            commands.spawn((
                Node {
                    align_self: AlignSelf::Start,
                    ..default()
                },
                ImageNode::new(image_handle.clone()),
                DisplayTextureSize::default(),
                Button,
            ));

            commands.insert_resource(WorleyImage {
                handle: image_handle,
            });
        }
    }
}

///! reference the preview image of the worley world
#[derive(Resource)]
pub struct WorleyImage {
    handle: Handle<Image>,
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

#[derive(Resource)]
pub struct SaveWorleyFilename(pub String);

impl FromWorld for SaveWorleyFilename {
    fn from_world(_world: &mut World) -> Self {
        Self(String::default())
    }
}

fn inspector_ui(world: &mut World) {
    let mut egui_context = world
        .query_filtered::<&mut EguiContext, With<bevy_egui::PrimaryEguiContext>>()
        .single(world)
        .expect("EguiContext not found")
        .clone();

    egui::Window::new("UI").show(egui_context.get_mut(), |ui| {
        egui::ScrollArea::both().show(ui, |ui| {
            // equivalent to `WorldInspectorPlugin`
            // bevy_inspector::ui_for_world(world, ui);
            let mut worley_file_name = world.get_resource_or_init::<SaveWorleyFilename>();
            ui.add(egui::Label::new("worley file name: (save or load)"));
            ui.add(egui::TextEdit::singleline(&mut worley_file_name.0));
            let file_name = worley_file_name.0.clone();
            if ui.add(egui::Button::new("save worley to file")).clicked() {
                info!("save");
                let map_settings = world.get_resource::<MapSettings>().expect("map settings");
                // map_settings.worley.deserialize
                let deserialized =
                    ron::ser::to_string_pretty(&map_settings.worley, PrettyConfig::default())
                        .expect("deserialize");
                let path = format!("assets/{}.worley.ron", &file_name);
                let result = std::fs::write(&path, deserialized);
                info!("saving {:?} result: {:?}", path, result);
            }
            if ui.add(egui::Button::new("load worley file")).clicked() {
                info!("load");
                let path = format!("assets/{}.worley.ron", &file_name);
                let file = std::fs::read_to_string(&path);
                match file {
                    Ok(f) => {
                        let result = ron::from_str::<Worley<BiomeType>>(&f);
                        match result {
                            Ok(new_worley) => {
                                // REPLACE
                                let mut map_settings = world.resource_mut::<MapSettings>();
                                map_settings.worley = new_worley;
                                map_settings.worley.rebuild_cached_noise();
                                info!("replaced current worley");
                            }
                            Err(err) => {
                                error!("failed to deserialize worley: {:?}", err);
                            }
                        }
                    }
                    Err(err) => {
                        error!("err loading worley file: {:?}, {:?}", path, err);
                    }
                }
            }

            let mut map_settings = world.resource_mut::<MapSettings>();

            let ms = map_settings.bypass_change_detection();

            let mut any_changed = false;
            any_changed |= ui
                .add(egui::Slider::new(&mut ms.worley.sharpness, 0.5..=20.0).text("Sharpness"))
                .changed();

            any_changed |= ui
                .add(egui::Slider::new(&mut ms.worley.k, 1..=5).text("k (nearest)"))
                .changed();
            any_changed |= ui
                .add(egui::Slider::new(&mut ms.worley.zoom, 10.0..=200.0).text("Zoom"))
                .changed();

            egui::CollapsingHeader::new("distance fn").show(ui, |ui| {
                let mut s = |worley: &mut Worley<BiomeType>,
                             any_changed: &mut bool,
                             target_metric: DistanceFn| {
                    if ui
                        .add(egui::widgets::Button::selectable(
                            worley.distance_fn == target_metric,
                            format!("{:?}", target_metric),
                        ))
                        .clicked()
                    {
                        worley.distance_fn = target_metric;
                        *any_changed |= true;
                    }
                };
                s(&mut ms.worley, &mut any_changed, DistanceFn::Euclidean);
                s(
                    &mut ms.worley,
                    &mut any_changed,
                    DistanceFn::EuclideanSquared,
                );
                s(&mut ms.worley, &mut any_changed, DistanceFn::Manhattan);
                s(&mut ms.worley, &mut any_changed, DistanceFn::Chebyshev);
                s(&mut ms.worley, &mut any_changed, DistanceFn::Hybrid);
            });

            ui.group(|ui| {
                if ui
                    .add(
                        egui::Slider::new(&mut ms.worley.warp_settings.strength, 0.0..=3.0)
                            .text("Warp strength"),
                    )
                    .changed()
                {
                    ms.worley.rebuild_cached_noise();
                    any_changed = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut ms.worley.warp_settings.noise_frequency, 0.0..=1.0)
                            .text("Warp frequency"),
                    )
                    .changed()
                {
                    ms.worley.rebuild_cached_noise();
                    any_changed = true;
                }
                if ui
                    .add(
                        egui::Slider::new(
                            &mut ms.worley.warp_settings.noise_fractal_lacunarity,
                            0.0..=4.0,
                        )
                        .text("fractal lacunarity"),
                    )
                    .changed()
                {
                    ms.worley.rebuild_cached_noise();
                    any_changed = true;
                }

                if ui
                    .add(
                        egui::Slider::new(
                            &mut ms.worley.warp_settings.noise_fractal_gain,
                            0.0..=3.0,
                        )
                        .text("fractal gain"),
                    )
                    .changed()
                {
                    ms.worley.rebuild_cached_noise();
                    any_changed = true;
                }
                if ui
                    .add(
                        egui::Slider::new(
                            &mut ms.worley.warp_settings.noise_fractal_octaves,
                            0..=5,
                        )
                        .text("fractal octaves"),
                    )
                    .changed()
                {
                    ms.worley.rebuild_cached_noise();
                    any_changed = true;
                }

                ui.label("warp noise");
                egui::CollapsingHeader::new("noise type").show(ui, |ui| {
                    let mut noise = |worley: &mut Worley<BiomeType>, noise_type: NoiseType| {
                        if ui
                            .add(egui::widgets::Button::selectable(
                                worley.warp_settings.noise_noise_type == noise_type,
                                format!("{:?}", noise_type),
                            ))
                            .clicked()
                        {
                            worley.warp_settings.noise_noise_type = noise_type;
                            worley.rebuild_cached_noise();
                            any_changed = true;
                        }
                    };
                    noise(&mut ms.worley, NoiseType::Value);
                    noise(&mut ms.worley, NoiseType::ValueFractal);
                    noise(&mut ms.worley, NoiseType::Perlin);
                    noise(&mut ms.worley, NoiseType::PerlinFractal);
                    noise(&mut ms.worley, NoiseType::Simplex);
                    noise(&mut ms.worley, NoiseType::SimplexFractal);
                    noise(&mut ms.worley, NoiseType::Cellular);
                    noise(&mut ms.worley, NoiseType::WhiteNoise);
                    noise(&mut ms.worley, NoiseType::Cubic);
                    noise(&mut ms.worley, NoiseType::CubicFractal);
                });
                egui::CollapsingHeader::new("fractal type").show(ui, |ui| {
                    let mut frac = |worley: &mut Worley<BiomeType>, fractal_type: FractalType| {
                        if ui
                            .add(egui::widgets::Button::selectable(
                                worley.warp_settings.noise_fractal_type == fractal_type,
                                format!("{:?}", fractal_type),
                            ))
                            .clicked()
                        {
                            worley.warp_settings.noise_fractal_type = fractal_type;
                            worley.rebuild_cached_noise();
                            any_changed = true;
                        }
                    };
                    frac(&mut ms.worley, FractalType::FBM);
                    frac(&mut ms.worley, FractalType::Billow);
                    frac(&mut ms.worley, FractalType::RigidMulti);
                });
            });

            if any_changed {
                // trigger change to MapSettings, causing an update to voxels
                map_settings.set_changed();
            }
        });
    });
}

#[derive(Component)]
pub struct VoxelTag;

pub const WORLD_SEED: u64 = 12345;

#[derive(Resource)]
pub struct Offset {
    x: f64,
    z: f64,
}

fn move_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut offset: ResMut<Offset>,
    time: Res<Time>,
    mut map_settings: ResMut<MapSettings>,
) {
    let speed = 32.0;
    let f = speed * time.delta_secs_f64();
    if keyboard.pressed(KeyCode::KeyD) {
        offset.x += f;
        map_settings.set_changed();
    }
    if keyboard.pressed(KeyCode::KeyA) {
        offset.x -= f;
        map_settings.set_changed();
    }
    if keyboard.pressed(KeyCode::KeyS) {
        offset.z += f;
        map_settings.set_changed();
    }
    if keyboard.pressed(KeyCode::KeyW) {
        offset.z -= f;
        map_settings.set_changed();
    }
}

fn setup(mut commands: Commands) {
    let mut worley: Worley<BiomeType> = Worley {
        zoom: 62.0,
        distance_fn: DistanceFn::Chebyshev,
        biome_generator: BiomeGenerator::UniformDistribution,
        _phantom: PhantomData::default(),
        sharpness: 20.0,
        k: 3,
        warp_settings: WarpSettings {
            strength: 0.6,
            noise_seed: 0,
            noise_frequency: 0.7,
            noise_fractal_lacunarity: 2.0,
            noise_fractal_gain: 0.6,
            noise_fractal_octaves: 5,
            noise_noise_type: NoiseType::PerlinFractal,
            noise_fractal_type: FractalType::FBM,
        },
        cached_warp_noise: bracket_noise::prelude::FastNoise::new(),
    };
    worley.rebuild_cached_noise();
    commands.insert_resource(MapSettings { worley });

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
pub fn quantize_srgba(color: Srgba, step: u8) -> (Srgba, (u8, u8, u8)) {
    // convert float [0.0..1.0] into byte [0..255]
    let r = (color.red * 255.0).round().clamp(0.0, 255.0) as u8;
    let g = (color.green * 255.0).round().clamp(0.0, 255.0) as u8;
    let b = (color.blue * 255.0).round().clamp(0.0, 255.0) as u8;
    // let a = (color.alpha * 255.0).round().clamp(0.0, 255.0) as u8;

    // quantize each component down to nearest multiple of `step`
    let rq = (r / step) * step;
    let gq = (g / step) * step;
    let bq = (b / step) * step;
    // let aq = (a / step) * step; // optional, usually just 255
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
