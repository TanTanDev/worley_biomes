use bracket_noise::prelude::*;
use egui_macroquad::egui;
use macroquad::prelude::*;

// ----- Biome Types -----
#[derive(Clone, Copy, Debug)]
enum BiomeType {
    Desert,
    Forest,
    Snow,
    Plains,
    Other1,
    Other2,
    Other3,
    Other4,
}

fn biome_color(b: BiomeType) -> Color {
    match b {
        BiomeType::Desert => YELLOW,
        BiomeType::Forest => GREEN,
        BiomeType::Snow => WHITE,
        BiomeType::Plains => LIME,
        BiomeType::Other1 => Color::new(1.0, 0.0, 0.0, 1.0),
        BiomeType::Other2 => Color::new(0.0, 0.0, 1.0, 1.0),
        BiomeType::Other3 => Color::new(1.0, 1.0, 0.0, 1.0),
        BiomeType::Other4 => Color::new(0.0, 0.0, 0.0, 1.0),
    }
}

// ----- Worley noise biome functions -----
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn hash_u64(seed: u64, x: i32, z: i32) -> u64 {
    let mut hasher = DefaultHasher::new();
    (seed, x, z).hash(&mut hasher);
    hasher.finish()
}

fn pick_biome(seed: u64, cell_x: i32, cell_z: i32) -> BiomeType {
    match hash_u64(seed, cell_x, cell_z) % 8 {
        0 => BiomeType::Desert,
        1 => BiomeType::Forest,
        2 => BiomeType::Snow,
        3 => BiomeType::Plains,
        4 => BiomeType::Other1,
        5 => BiomeType::Other2,
        6 => BiomeType::Other3,
        _ => BiomeType::Other4,
    }
}

fn feature_point(seed: u64, cell_x: i32, cell_z: i32) -> (f64, f64) {
    let h1 = hash_u64(seed.wrapping_add(1337), cell_x, cell_z);
    let h2 = hash_u64(seed.wrapping_add(7331), cell_x, cell_z);

    let fx = cell_x as f64 + ((h1 & 0xFFFF) as f64 / 65535.0);
    let fz = cell_z as f64 + ((h2 & 0xFFFF) as f64 / 65535.0);
    (fx, fz)
}

fn warp_coords(x: f32, z: f32, strength: f32, noise: &FastNoise) -> (f32, f32) {
    let nx = noise.get_noise(x, z);
    let nz = noise.get_noise(x + 103f32, z);
    (x + nx * strength, z + nz * strength)
}

// ----- Visualization -----
#[macroquad::main("Worley Biomes")]
async fn main() {
    // perlin
    let mut noise = FastNoise::seeded(0);
    noise.set_noise_type(NoiseType::PerlinFractal);
    noise.set_fractal_type(FractalType::FBM);
    noise.set_fractal_octaves(5);
    noise.set_fractal_gain(0.6);
    noise.set_fractal_lacunarity(2.0);
    noise.set_frequency(2.0);

    let mut world_seed: u64 = 12345;
    let mut k: usize = 3;
    let mut sharpness: f32 = 20.0;
    let mut zoom: f32 = 90.0;
    let mut offset_x: f32 = 0.0;
    let mut offset_y: f32 = 0.0;

    let mut warp_strength: f32 = 0.6;
    let mut warp_frequency: f32 = 0.7;
    let mut metric = DistanceFn::Euclidean;

    loop {
        clear_background(BLACK);
        noise.set_frequency(warp_frequency);

        let screen_w = screen_width() as i32;
        let screen_h = screen_height() as i32;

        // draw biome pixels
        for sx in (0..screen_w).step_by(4) {
            for sy in (0..screen_h).step_by(4) {
                let wx = offset_x + sx as f32 / zoom;
                let wz = offset_y + sy as f32 / zoom;

                let (wx, wz) = warp_coords(wx, wz, warp_strength, &noise);
                let weights = worley_biome_weights(
                    world_seed,
                    wx as f64,
                    wz as f64,
                    k,
                    sharpness as f64,
                    metric,
                );

                // blend colors
                let mut r = 0.0;
                let mut g = 0.0;
                let mut b = 0.0;
                for (w, biome) in weights {
                    let c = biome_color(biome);
                    r += c.r as f64 * w;
                    g += c.g as f64 * w;
                    b += c.b as f64 * w;
                }
                draw_rectangle(
                    sx as f32,
                    sy as f32,
                    4.0,
                    4.0,
                    Color::new(r as f32, g as f32, b as f32, 1.0),
                );
            }
        }

        // UI
        egui_macroquad::ui(|egui_ctx| {
            egui::Window::new("Biome Controls").show(egui_ctx, |ui| {
                ui.label("World Seed");
                let mut seed_str = world_seed.to_string();
                if ui.text_edit_singleline(&mut seed_str).changed() {
                    if let Ok(v) = seed_str.parse::<u64>() {
                        world_seed = v;
                    }
                }

                ui.add(egui::Slider::new(&mut k, 1..=5).text("k (nearest)"));
                ui.add(egui::Slider::new(&mut sharpness, 0.5..=20.0).text("Sharpness"));
                ui.add(egui::Slider::new(&mut zoom, 10.0..=200.0).text("Zoom"));
                ui.add(egui::Slider::new(&mut warp_strength, 0.0..=3.0).text("Warp strength"));
                ui.add(egui::Slider::new(&mut warp_frequency, 0.0..=1.0).text("Warp frequency"));

                let mut s = |metric: &mut DistanceFn, target_metric: DistanceFn| {
                    if ui
                        .add(egui::widgets::SelectableLabel::new(
                            *metric == target_metric,
                            format!("{:?}", target_metric),
                        ))
                        .clicked()
                    {
                        *metric = target_metric;
                    }
                };
                s(&mut metric, DistanceFn::Euclidean);
                s(&mut metric, DistanceFn::EuclideanSquared);
                s(&mut metric, DistanceFn::Manhattan);
                s(&mut metric, DistanceFn::Chebyshev);
                s(&mut metric, DistanceFn::Hybrid);

                ui.label("Use arrow keys to move");
            });
        });

        let dt = get_frame_time();
        let move_speed = 2.0;
        // Controls for panning
        if is_key_down(KeyCode::Left) {
            offset_x -= move_speed * dt;
        }
        if is_key_down(KeyCode::Right) {
            offset_x += move_speed * dt;
        }
        if is_key_down(KeyCode::Up) {
            offset_y -= move_speed * dt;
        }
        if is_key_down(KeyCode::Down) {
            offset_y += move_speed * dt;
        }

        egui_macroquad::draw();
        next_frame().await
    }
}
