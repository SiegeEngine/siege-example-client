
use std::fs::File;
use std::io::Read;
use siege_math::Vec2;
use errors::*;
use config::Config;

pub const WIDTH: usize = 513;
pub const HEIGHT: usize = 513;

pub struct Terrain {
    heightmap: Vec<u8>,
}

impl Terrain {
    pub fn new(config: &Config, name: &str) -> Result<Terrain>
    {
        let mut path = config.graphics.renderer.asset_path.clone();
        path.push("buffers");
        path.push(format!("{}.raw.zst", name));
        let f = File::open(path)?;

        // Decompress
        use zstd::stream::Decoder;
        let mut d = Decoder::new(f)?;

        let mut vec: Vec<u8> = Vec::new();
        d.read_to_end(&mut vec)?;

        Ok(Terrain {
            heightmap: vec
        })
    }

    pub fn get_y(&self, x: f32, z: f32) -> f32
    {
        const QUANTA_PER_METER: f32 = 2048.0; // 65536 quanta span 32 meters

        // Coordinates of the anchor of the texture (the NW corner)
        let anchor = Vec2::new(-(WIDTH as f32) / 2.0, HEIGHT as f32 / 2.0);

        // Get x and z relative to the texture
        let rx = x - anchor[0];
        let rz = anchor[1] - z;

        // If out of bounds, return a safe value
        if rx < 0.0 || rx >= (WIDTH - 1) as f32 || rz < 0.0 || rz >= (HEIGHT - 1) as f32 {
            return 30.0; // bottom of y range (32.0 + 2 meters for eyes)
        }

        // Convert to pixel coordinates
        let px0 = rx.floor() as usize;
        let px1 = rx.ceil() as usize;
        let py0 = rz.floor() as usize;
        let py1 = rz.ceil() as usize;

        // Define a function to sample the height map at pixel (x,y)
        let data = &*self.heightmap;
        let val = |x,y| -> f32 {
            let i = x + y * (WIDTH as usize);
            data[i*2] as f32 + 256_f32 * data[i*2+1] as f32
        };

        // Sample 4 points and average them
        let upper_left_height = val(px0, py0);
        let upper_right_height = val(px1, py0);
        let lower_left_height = val(px0, py1);
        let lower_right_height = val(px1, py1);

        let from_left_sq = (rx - px0 as f32).powi(2);
        let from_right_sq = (rx - px1 as f32).powi(2);
        let from_top_sq = (rz - py0 as f32).powi(2);
        let from_bottom_sq = (rz - py1 as f32).powi(2);

        let upper_left_weight = 1.0 / (from_left_sq + from_top_sq);
        let lower_left_weight = 1.0 / (from_left_sq + from_bottom_sq);
        let upper_right_weight = 1.0 / (from_right_sq + from_top_sq);
        let lower_right_weight = 1.0 / (from_right_sq + from_bottom_sq);
        let sum = upper_left_weight + lower_left_weight + upper_right_weight + lower_right_weight;

        let mut value = (upper_left_height * upper_left_weight
                         + upper_right_height * upper_right_weight
                         + lower_left_height * lower_left_weight
                         + lower_right_height * lower_right_weight) / sum;

        // Add a couple of meters (eye level)
        value += 2.0 * QUANTA_PER_METER;

        // Translate from quanta into y coordinate value
        32.0 - (value / QUANTA_PER_METER)
    }
}
