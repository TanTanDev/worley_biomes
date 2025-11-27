use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension},
};
use bracket_fast_noise::prelude::*;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use worley_biomes::{
    biome_picker::{BiomeVariants, SimpleBiomePicker},
    distance_fn::DistanceFn,
    worley::Worley,
};

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

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

pub const GRID_SIZE: i32 = 32 * 4;

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut worley: Worley<BiomeType, SimpleBiomePicker<BiomeType>> = Worley::default();
    worley.zoom = 22.0;
    worley.seed = 12345;
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

    let mut img_data = Vec::new();
    for gx in 0..GRID_SIZE {
        for gz in 0..GRID_SIZE {
            let weights = worley.get(gx as f64, gz as f64);

            // blend colors
            let mut r = 0.0;
            let mut g = 0.0;
            let mut b = 0.0;
            let mut wsum = 0.0;
            for (w, biome) in &weights {
                let c = biome_color(*biome);
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
            align_self: AlignSelf::Center,
            justify_self: JustifySelf::Center,
            height: Val::Percent(100.0),
            ..default()
        },
        ImageNode::new(image_handle.clone()),
        Button,
    ));

    commands.spawn((Camera2d,));
}
