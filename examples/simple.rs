use std::marker::PhantomData;

use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension},
};
use serde::{Deserialize, Serialize};
use worley_biomes::{
    biome_picker::{Biome, BiomeGenerator},
    distance_fn::DistanceFn,
    warp::{FractalType, NoiseType, WarpSettings},
    worley::Worley,
};

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

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

pub const GRID_SIZE: i32 = 32 * 4;

pub const WORLD_SEED: u64 = 12345;

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut worley: Worley<BiomeType> = Worley {
        zoom: 17.0,
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
    // the warp noise data must be set up from worley settings
    worley.rebuild_cached_noise();

    let mut img_data = Vec::new();
    for gx in 0..GRID_SIZE {
        for gz in 0..GRID_SIZE {
            let weights = worley.get(WORLD_SEED, gx as f64, gz as f64);

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
