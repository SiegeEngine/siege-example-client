
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use ring::rand::SystemRandom;
use siege_math::{Point3, Vec3, Angle};
use network::PacketSender;
use errors::*;
use siege_plugin_avatar_simple::{Avatar, Placement, Movement, MoveDirection};
use camera::Camera;
use terrain::Terrain;
use config::Config;
use ui::Ui;
use stats::Stats;
use chat::Chat;

pub struct RenderParams {
    pub bloom_strength: f32,
    pub bloom_cliff: f32,
    pub blur_level: f32,
}

// State shared between threads
pub struct State {
    pub start: Instant,
    // Arc is used here because we send a copy to the Renderer
    pub terminating: Arc<AtomicBool>,
    pub rng: Arc<SystemRandom>,
    pub packet_sender: PacketSender,
    // Arc is used here because we send a copy to the Renderer
    pub resized: Arc<AtomicBool>,
    pub avatar: RwLock<Avatar>,
    pub camera: RwLock<Camera>,
    pub terrain: Terrain, // read only
    pub render_params: RwLock<RenderParams>,
    pub ui: Ui,
    pub stats: RwLock<Stats>,
    pub chat: RwLock<Chat>,
}

impl State {
    pub fn new(config: &Config) -> Result<State>
    {
        let mut ui = Ui::new(&*config.graphics.renderer.asset_path)?;
        let chat = Chat::new(&mut ui);

        Ok(State {
            start: Instant::now(),
            terminating: Arc::new(AtomicBool::new(false)),
            rng: Arc::new(SystemRandom::new()),
            packet_sender: PacketSender::new(),
            resized: Arc::new(AtomicBool::new(false)),
            avatar: RwLock::new(Avatar::new(
                Placement::new(
                    Point3(Vec3::new(0.0, 0.0, 0.0)), // at the origin
                    0.0,   // no pitch
                    0.0),  // no yaw, looking down +Z
                Movement::new(),
                Instant::now())),
            camera: RwLock::new(Camera::new(config)),
            terrain: Terrain::new(config, "sample_terrain")?,
            render_params: RwLock::new(RenderParams {
                bloom_strength: 0.60,
                bloom_cliff: 0.35,
                blur_level: 0.0,
            }),
            ui: ui,
            stats: RwLock::new(Stats::new()),
            chat: RwLock::new(chat),
        })
    }

    pub fn periodic_update(&self) {
        // Update camera (movement commands are only change events, but camera needs
        // update continuously while movement is happening)
        {
            let mut placement = {
                self.avatar.read().unwrap().get_current_placement()
            };

            // Place above terrain
            placement.position.0.y = self.terrain.get_y(
                placement.position.0.x,
                placement.position.0.z
            );

            let mut camera = self.camera.write().unwrap();
            camera.update_placement(&placement);
            /*debug!("Position = ({},{},{})",
                   placement.position.x,
                   placement.position.y,
                   placement.position.z);*/
        }
    }

    pub fn movement_cmd(&self, direction: MoveDirection, positive: bool) {
        let mut avatar = self.avatar.write().unwrap();
        avatar.movement_cmd(
            direction, positive, Instant::now());

        // Place above terrain
        avatar.placement.position.0.y = self.terrain.get_y(
            avatar.placement.position.0.x,
            avatar.placement.position.0.z
        );
    }

    pub fn adjust_fovx(&self, angle: Angle<f32>) {
        self.camera.write().unwrap().adjust_fovx(angle);
    }

    pub fn adjust_bloom_strength(&self, positive: bool) {
        let mut bloom_strength = self.render_params.read().unwrap().bloom_strength;
        let delta = if positive { 0.01 } else { -0.01 };
        bloom_strength += delta;
        if bloom_strength < 0.0 { bloom_strength = 0.0; }
        if bloom_strength > 1.0 { bloom_strength = 1.0; }
        info!("Bloom strength = {}", bloom_strength);
        self.render_params.write().unwrap().bloom_strength = bloom_strength;
    }

    pub fn adjust_bloom_cliff(&self, positive: bool) {
        let mut bloom_cliff = self.render_params.read().unwrap().bloom_cliff;
        let delta = if positive { 0.01 } else { -0.01 };
        bloom_cliff += delta;
        if bloom_cliff < 0.0 { bloom_cliff = 0.0; }
        if bloom_cliff > 1.0 { bloom_cliff = 1.0; }
        info!("Bloom cliff = {}", bloom_cliff);
        self.render_params.write().unwrap().bloom_cliff = bloom_cliff;
    }

    pub fn adjust_blur_level(&self, positive: bool) {
        let mut blur_level = self.render_params.read().unwrap().blur_level;
        let delta = if positive { 0.01 } else { -0.01 };
        blur_level += delta;
        if blur_level < 0.0 { blur_level = 0.0; }
        if blur_level > 1.0 { blur_level = 1.0; }
        self.render_params.write().unwrap().blur_level = blur_level;
    }
}
