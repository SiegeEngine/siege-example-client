
mod window;
use self::window::WindowGfx;

mod ui_image;
use self::ui_image::UiImageGfx;

mod text;
use self::text::TextGfx;

mod graybox;
use self::graybox::{Graybox, GrayboxGfx};

mod terrain;
use self::terrain::TerrainGfx;

mod horizon;
use self::horizon::HorizonGfx;

mod camera;
use self::camera::CameraGfx;

mod stats;
use self::stats::StatsGfx;

use std::sync::Arc;
use winit::Window;
use siege_math::{Vec4, Mat4};
use siege_render::{Renderer, Params};
use errors::*;
use config::Config;
use state::State;

pub struct GraphicsSystem {
    renderer: Renderer,
    #[allow(dead_code)] // we may use it later
    state: Arc<State>,
}

const LIGHT_DIR: Vec4<f32> = Vec4 {
    x:  0.5773502691896258,
    y: -0.5773502691896258,
    z:  0.5773502691896258,
    w: 0.0,
};
const LIGHT_DIR_2: Vec4<f32> = Vec4 {
    x: -0.8017837257372732,
    y: -0.2672612419124244,
    z: -0.5345224838248488,
    w: 0.0,
};

impl GraphicsSystem {
    pub fn new(config: Arc<Config>, state: Arc<State>, window: Arc<Window>)
               -> Result<GraphicsSystem>
    {
        let mut renderer = Renderer::new(config.graphics.renderer.clone(),
                                         window.clone(),
                                         state.resized.clone(),
                                         state.terminating.clone())?;

        {
            let mut camera = state.camera.write().unwrap();
            camera.extent = renderer.get_extent();
        }

        let stats = StatsGfx::new(state.clone())?;

        let camera = CameraGfx::new(&mut renderer, state.clone())?;

        let horizon_gfx = HorizonGfx::new(
            &mut renderer,
            camera.desc_layout.clone(),
            camera.descriptor_set.clone()
        )?;

        let terrain_gfx = TerrainGfx::new(
            &mut renderer,
            camera.desc_layout.clone(),
            camera.descriptor_set.clone(),
        )?;

        let mut graybox_gfx = GrayboxGfx::new(
            &mut renderer,
            state.clone(),
            camera.desc_layout.clone(), camera.descriptor_set.clone(),
            3)?; //  FIXME: max 2 grayboxes

        let graybox = Graybox::new(&mut renderer,
                                   // diffuse:
                                   Vec4::<f32>::new(0.085, 0.080, 0.09, 0.0),
                                   // roughness, metallicity, ao, cavity
                                   Vec4::<f32>::new(0.5, 0.95, 1.0, 1.0),
                                   Mat4::<f32>::new( 1.0, 0.0, 0.0, 10.0,
                                                     0.0, 1.0, 0.0, 10.0,
                                                     0.0, 0.0, 1.0, 0.0,
                                                     0.0, 0.0, 0.0, 1.0),
                                   "cube_graybox")?;
        graybox_gfx.add_graybox(graybox)?;

        let graybox = Graybox::new(&mut renderer,
                                   // diffuse:
                                   Vec4::<f32>::new(0.085, 0.080, 0.09, 0.0) * 3.0,
                                   // roughness, metallicity, ao, cavity
                                   Vec4::<f32>::new(0.5, 0.95, 1.0, 1.0),
                                   Mat4::<f32>::new( 1.0, 0.0, 0.0, 0.0,
                                                     0.0, 1.0, 0.0, 14.1,
                                                     0.0, 0.0, 1.0, 0.0,
                                                     0.0, 0.0, 0.0, 1.0),
                                   "cube_graybox")?;
        graybox_gfx.add_graybox(graybox)?;

        let graybox = Graybox::new(&mut renderer,
                                   // diffuse:
                                   Vec4::<f32>::new(1.0, 0.0, 0.0, 0.0),
                                   // roughness, metallicity, ao, cavity
                                   Vec4::<f32>::new(0.5, 0.95, 1.0, 1.0),
                                   Mat4::<f32>::new( 1.0, 0.0, 0.0, 0.0,
                                                     0.0, 1.0, 0.0, 14.2,
                                                     0.0, 0.0, 1.0, 10.0,
                                                     0.0, 0.0, 0.0, 1.0),
                                   "cube_graybox")?;
        graybox_gfx.add_graybox(graybox)?;

        let window_gfx = WindowGfx::new(&mut renderer, state.clone())?;

        let ui_image_gfx = UiImageGfx::new(&mut renderer, state.clone())?;

        let text_gfx = TextGfx::new(&mut renderer, state.clone())?;

        let params = {
            let (bloom_strength, bloom_cliff, blur_level) = {
                let rp = state.render_params.read().unwrap();
                (
                    rp.bloom_strength,
                    rp.bloom_cliff,
                    rp.blur_level
                )
            };

            Params {
                inv_projection: camera.inv_projection(),
                dlight_directions: [LIGHT_DIR, LIGHT_DIR_2],
                dlight_irradiances: [
                    Vec4::new(1.0, 1.0, 1.0, 0.0),
                    Vec4::new(2.0, 1.8, 1.7, 0.0),
                ],
                bloom_strength: bloom_strength,
                bloom_cliff: bloom_cliff,
                blur_level: blur_level,
                ambient: 0.08,
                white_level: 1.0,
                tonemapper: config.graphics.renderer.tonemapper,
            }
        };

        renderer.set_params(&params)?;

        // Plugin in approximate front-to-back order
        // (although this will be backwards for future transparency ... maybe renderer
        //  can iterate reverse through those)
        renderer.plugin(Box::new(stats))?;
        renderer.plugin(Box::new(camera))?;
        renderer.plugin(Box::new(graybox_gfx))?;
        renderer.plugin(Box::new(horizon_gfx))?;
        renderer.plugin(Box::new(terrain_gfx))?;
        renderer.plugin(Box::new(window_gfx))?;
        renderer.plugin(Box::new(ui_image_gfx))?;
        renderer.plugin(Box::new(text_gfx))?;

        Ok(GraphicsSystem {
            renderer: renderer,
            state: state,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        self.renderer.run()?;
        Ok(())
    }
}
