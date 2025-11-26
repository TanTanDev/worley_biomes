use bevy::prelude::*;

use std::marker::PhantomData;

use crate::{
    biome_picker::{BiomePicker, BiomeVariants},
    distance_fn::DistanceFn,
    worley::Worley,
};
use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    render::render_resource::{Extent3d, TextureDimension},
};
use bevy_inspector_egui::{
    bevy_egui::{self, EguiContext, EguiPrimaryContextPass},
    egui,
};
use bracket_fast_noise::prelude::*;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

///! VISUALIZE + TWEAK UI, worley
#[derive(Default)]
pub struct DebugPlugin<WorleyResT, BiomeT, Picker>
where
    WorleyResT: Resource + GetWorley<BiomeT, Picker>,
    BiomeT: BiomeVariants
        + DebugColor<BiomeT>
        + Serialize
        + for<'de> Deserialize<'de>
        + std::default::Default,
    Picker: BiomePicker<BiomeT> + Serialize + Default + for<'de> Deserialize<'de>,
{
    pub settings: DebugPluginSettings,
    pub _phantom: PhantomData<(WorleyResT, BiomeT, Picker)>,
}

#[derive(Resource, Clone)]
pub struct DebugPluginSettings {
    ///! true: plugin will spawn a ui entity for showcasing worley
    ///! false: don't spawn it. (you can manually do so if you need customization)
    pub spawn_preview_image: bool,

    // preview + ui visibility controll
    pub show_preview_image: bool,
    pub show_inspector_ui: bool,
}

impl Default for DebugPluginSettings {
    fn default() -> Self {
        Self {
            spawn_preview_image: true,
            show_preview_image: true,
            show_inspector_ui: true,
        }
    }
}

impl<WorleyResT, BiomeT, Picker> Plugin for DebugPlugin<WorleyResT, BiomeT, Picker>
where
    WorleyResT: Resource + GetWorley<BiomeT, Picker>,
    BiomeT: BiomeVariants
        + DebugColor<BiomeT>
        + Serialize
        + for<'de> Deserialize<'de>
        + Sync
        + Send
        + std::default::Default
        + 'static,
    Picker: BiomePicker<BiomeT>
        + Serialize
        + Default
        + for<'de> Deserialize<'de>
        + Sync
        + Send
        + 'static,
{
    fn build(&self, app: &mut App) {
        app.insert_resource(self.settings.clone());
        app.add_systems(
            EguiPrimaryContextPass,
            inspector_ui::<WorleyResT, BiomeT, Picker>.run_if(if_show_inspector),
        );
        app.add_systems(Update, texture_tap);
        app.add_systems(Update, update_preview_visibility);
        app.add_systems(
            PostUpdate,
            rebuild_preview_image::<WorleyResT, BiomeT, Picker>,
        );
    }
}

pub fn if_show_inspector(settings: Res<DebugPluginSettings>) -> bool {
    settings.show_inspector_ui
}

///! color of biome to display in debug worley texture
pub trait DebugColor<BiomeT> {
    fn get_color(&self) -> Srgba;
}

///! required for the debug_plugin to find what worley to visualize
pub trait GetWorley<BiomeT, Picker>
where
    BiomeT: BiomeVariants + Serialize,
    Picker: BiomePicker<BiomeT> + Serialize + Default,
{
    fn get_worley<'a>(&'a self) -> &'a Worley<BiomeT, Picker>;
    fn get_worley_mut<'a>(&'a mut self) -> &'a mut Worley<BiomeT, Picker>;
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

///! the size of the preview image
pub const IMG_SIZE: i32 = 32 * 4;

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

///! fetch worley data to UPDATE the preview image
fn rebuild_preview_image<WorleyResT, BiomeT, Picker>(
    map_settings: Res<WorleyResT>,
    debug_plugin_settings: Res<DebugPluginSettings>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut worley_image: Option<ResMut<WorleyImage>>,
) where
    WorleyResT: Resource + GetWorley<BiomeT, Picker>,
    BiomeT: BiomeVariants + Serialize + 'static + DebugColor<BiomeT> + std::default::Default,
    Picker: BiomePicker<BiomeT> + Serialize + Default + 'static,
{
    if !map_settings.is_changed() {
        return;
    }

    let mut img_data = Vec::new();
    let worley = WorleyResT::get_worley(&map_settings);

    let worley_offset = worley_image
        .as_mut()
        .map_or((0.0, 0.0), |w| w.preview_offset);
    for gx in 0..IMG_SIZE {
        for gz in 0..IMG_SIZE {
            let weights = worley.get(gx as f64 + worley_offset.0, gz as f64 + worley_offset.1);

            // blend colors
            let mut r = 0.0;
            let mut g = 0.0;
            let mut b = 0.0;
            let mut wsum = 0.0;
            for (w, biome) in &weights {
                let c = DebugColor::get_color(biome);
                r += c.red as f64 * w;
                g += c.green as f64 * w;
                b += c.blue as f64 * w;
                wsum += w;
            }

            let color = Srgba::new(r as f32, g as f32, b as f32, 1.0);
            img_data.push((color.red * 255.0) as u8);
            img_data.push((color.green * 255.0) as u8);
            img_data.push((color.blue * 255.0) as u8);
            img_data.push(255 as u8);
        }
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
                    width: IMG_SIZE as u32,
                    height: IMG_SIZE as u32,
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
            if debug_plugin_settings.spawn_preview_image {
                commands.spawn((
                    Name::new("worley_ui_preview"),
                    Node {
                        align_self: AlignSelf::Start,
                        ..default()
                    },
                    ImageNode::new(image_handle.clone()),
                    DisplayTextureSize::default(),
                    WorleyUiPreviewTag,
                    Button,
                ));
            }

            commands.insert_resource(WorleyImage {
                handle: image_handle,
                preview_offset: (0.0, 0.0),
            });
        }
    }
}

#[derive(Component)]
pub struct WorleyUiPreviewTag;

fn update_preview_visibility(
    settings: Res<DebugPluginSettings>,
    mut query: Query<&mut Node, With<WorleyUiPreviewTag>>,
) {
    if !settings.is_changed() {
        return;
    }
    for mut node in query.iter_mut() {
        match settings.show_preview_image {
            true => {
                node.display = Display::Flex;
            }
            false => {
                node.display = Display::None;
            }
        }
    }
}

///! reference the preview image of the worley world
#[derive(Resource)]
pub struct WorleyImage {
    handle: Handle<Image>,
    ///! preview image sampling is offset by this
    pub preview_offset: (f64, f64),
}

#[derive(Resource)]
pub struct SaveWorleyFilename(pub String);

impl FromWorld for SaveWorleyFilename {
    fn from_world(_world: &mut World) -> Self {
        Self(String::default())
    }
}

fn inspector_ui<WorleyResT, BiomeT, Picker>(world: &mut World)
where
    WorleyResT: Resource + GetWorley<BiomeT, Picker>,
    BiomeT: BiomeVariants + Serialize + for<'de> Deserialize<'de> + 'static,
    Picker: BiomePicker<BiomeT> + Serialize + Default + for<'de> Deserialize<'de> + 'static,
{
    let mut egui_context = world
        .query_filtered::<&mut EguiContext, With<bevy_egui::PrimaryEguiContext>>()
        .single(world)
        .expect("EguiContext not found")
        .clone();

    egui::Window::new("worley UI").show(egui_context.get_mut(), |ui| {
        egui::ScrollArea::both().show(ui, |ui| {
            let mut worley_file_name = world.get_resource_or_init::<SaveWorleyFilename>();
            ui.add(egui::Label::new("worley file name: (save or load)"));
            ui.add(egui::TextEdit::singleline(&mut worley_file_name.0));
            let file_name = worley_file_name.0.clone();
            if ui.add(egui::Button::new("save worley to file")).clicked() {
                info!("save");
                // let map_settings = world.get_resource::<MapSettings>().expect("map settings");
                let map_settings = world.get_resource::<WorleyResT>().expect("WorleyResT");
                let deserialized =
                    ron::ser::to_string_pretty(map_settings.get_worley(), PrettyConfig::default())
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
                        let result = ron::from_str::<Worley<BiomeT, Picker>>(&f);
                        match result {
                            Ok(new_worley) => {
                                // REPLACE
                                let mut map_settings = world.resource_mut::<WorleyResT>();
                                let worley = map_settings.get_worley_mut();
                                *worley = new_worley;
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

            let mut map_settings = world.resource_mut::<WorleyResT>();
            let ms = map_settings.bypass_change_detection();
            let mut worley = ms.get_worley_mut();

            let mut any_changed = false;
            any_changed |= ui
                .add(egui::Slider::new(&mut worley.seed, 0..=100).text("seed"))
                .changed();
            any_changed |= ui
                .add(egui::Slider::new(&mut worley.sharpness, 0.5..=20.0).text("Sharpness"))
                .changed();

            any_changed |= ui
                .add(egui::Slider::new(&mut worley.k, 1..=8).text("k (nearest)"))
                .changed();
            any_changed |= ui
                .add(egui::Slider::new(&mut worley.zoom, 10.0..=200.0).text("Zoom"))
                .changed();

            let mut kill_per = worley.kill_percent_threshold.unwrap_or(0.0);
            let kill_per_changed = ui
                .add(egui::Slider::new(&mut kill_per, 0.0..=0.99).text("kill threshold"))
                .changed();
            any_changed |= kill_per_changed;
            if kill_per_changed {
                worley.kill_percent_threshold = Some(kill_per);
            }

            egui::CollapsingHeader::new("distance fn").show(ui, |ui| {
                let mut s = |worley: &mut Worley<BiomeT, Picker>,
                             any_changed: &mut bool,
                             target_metric: DistanceFn| {
                    if ui
                        .add(egui::widgets::Button::selectable(
                            worley.distance_fn_config == target_metric,
                            format!("{:?}", target_metric),
                        ))
                        .clicked()
                    {
                        worley.distance_fn_config = target_metric;
                        worley.distance_fn = target_metric.to_func();
                        *any_changed |= true;
                    }
                };
                s(&mut worley, &mut any_changed, DistanceFn::Euclidean);
                s(&mut worley, &mut any_changed, DistanceFn::EuclideanSquared);
                s(&mut worley, &mut any_changed, DistanceFn::Manhattan);
                s(&mut worley, &mut any_changed, DistanceFn::Chebyshev);
                s(&mut worley, &mut any_changed, DistanceFn::Hybrid);
            });

            ui.group(|ui| {
                if ui
                    .add(
                        egui::Slider::new(&mut worley.warp_settings.strength, 0.0..=3.0)
                            .text("Warp strength"),
                    )
                    .changed()
                {
                    any_changed = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut worley.warp_settings.noise.frequency, 0.0..=1.0)
                            .text("Warp frequency"),
                    )
                    .changed()
                {
                    any_changed = true;
                }
                if ui
                    .add(
                        egui::Slider::new(
                            &mut worley.warp_settings.noise.fractal_lacunarity,
                            0.0..=4.0,
                        )
                        .text("fractal lacunarity"),
                    )
                    .changed()
                {
                    any_changed = true;
                }

                let mut fractal_gain = worley.warp_settings.noise.get_fractal_gain();
                if ui
                    .add(egui::Slider::new(&mut fractal_gain, 0.0..=3.0).text("fractal gain"))
                    .changed()
                {
                    worley.warp_settings.noise.set_fractal_gain(fractal_gain);
                    any_changed = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut worley.warp_settings.noise.fractal_octaves, 0..=5)
                            .text("fractal octaves"),
                    )
                    .changed()
                {
                    any_changed = true;
                }

                ui.label("warp noise");
                egui::CollapsingHeader::new("noise type").show(ui, |ui| {
                    let mut noise = |worley: &mut Worley<BiomeT, Picker>, noise_type: NoiseType| {
                        if ui
                            .add(egui::widgets::Button::selectable(
                                worley.warp_settings.noise.noise_type == noise_type,
                                format!("{:?}", noise_type),
                            ))
                            .clicked()
                        {
                            worley.warp_settings.noise.noise_type = noise_type;
                            any_changed = true;
                        }
                    };
                    noise(&mut worley, NoiseType::Value);
                    noise(&mut worley, NoiseType::ValueFractal);
                    noise(&mut worley, NoiseType::Perlin);
                    noise(&mut worley, NoiseType::PerlinFractal);
                    noise(&mut worley, NoiseType::Simplex);
                    noise(&mut worley, NoiseType::SimplexFractal);
                    noise(&mut worley, NoiseType::Cellular);
                    noise(&mut worley, NoiseType::WhiteNoise);
                    noise(&mut worley, NoiseType::Cubic);
                    noise(&mut worley, NoiseType::CubicFractal);
                });
                egui::CollapsingHeader::new("fractal type").show(ui, |ui| {
                    let mut frac = |worley: &mut Worley<BiomeT, Picker>,
                                    fractal_type: FractalType| {
                        if ui
                            .add(egui::widgets::Button::selectable(
                                worley.warp_settings.noise.fractal_type == fractal_type,
                                format!("{:?}", fractal_type),
                            ))
                            .clicked()
                        {
                            worley.warp_settings.noise.fractal_type = fractal_type;
                            any_changed = true;
                        }
                    };
                    frac(&mut worley, FractalType::FBM);
                    frac(&mut worley, FractalType::Billow);
                    frac(&mut worley, FractalType::RigidMulti);
                });
            });

            if any_changed {
                // trigger change to MapSettings, causing an update to voxels
                map_settings.set_changed();
            }
        });
    });
}
