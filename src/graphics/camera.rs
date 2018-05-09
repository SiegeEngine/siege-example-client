
use std::sync::Arc;
use dacite::core::{DescriptorSetLayout, DescriptorSetLayoutBinding,
                   DescriptorSet, WriteDescriptorSetElements,
                   CommandBuffer, Extent2D};
use siege_math::{Mat4, Vec4};
use siege_render::{Renderer, HostVisibleBuffer, Lifetime, Plugin,
                   Params, Stats};
use errors::*;
use State;
use super::{LIGHT_DIR, LIGHT_DIR_2};

/*
 How far can one see on a planet?
 > If the maximum height of an object is like Mt. Everest, you can see about 340 km.
 > In general, you can see  (2Rx + x*x).sqrt().
 >   R for Albedna is 6,371,000m
 >   Everest is about 9000m
 >   = 338.7 km
 > This assumes you are looking at something at sea-level. If the think you are viewing
 > is tall itself, this number can grow unbounded.
 */

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CameraUniforms {
    pub projection_x_view_matrix: Mat4<f32>,
    pub view_matrix: Mat4<f32>,
    pub projection_matrix: Mat4<f32>,
    pub camera_position_wspace: Vec4<f32>,
    pub ambient: f32,
    pub white_level: f32,
    pub extent: Extent2D,
    pub fovx: f32,
}

impl CameraUniforms {
    pub fn new(state: &State)
               -> CameraUniforms
    {
        let mut uniforms = CameraUniforms {
            projection_x_view_matrix: Mat4::identity(),
            view_matrix: Mat4::identity(),
            projection_matrix: Mat4::identity(),
            camera_position_wspace: Vec4::zero(),
            ambient: 1.0,
            white_level: 0.08,
            extent: Extent2D { width: 1280, height: 1024 }, // will be updated
            fovx: 0.0, // will be updated
        };
        uniforms.update(state);
        uniforms
    }

    pub fn update(&mut self,
                  state: &State)
    {
        let (fovx, view_matrix, extent) = {
            let camera = state.camera.read().unwrap();
            (camera.fovx.as_radians(), camera.view_matrix, camera.extent)
        };
        let position = {
            let avatar = state.avatar.read().unwrap();
            let mut placement = avatar.get_current_placement();
            // Place above terrain
            placement.position.0.y = state.terrain.get_y(
                placement.position.0.x,
                placement.position.0.z
            );
            From::from(placement.position)
        };

        self.camera_position_wspace = position;
        self.fovx = fovx;
        self.view_matrix = view_matrix;
        self.extent = extent;

        // Fixme - only redo projection matrix if extent changes OR fovx changes.
        // Right now I have no idea of FOV changed, so we always redo it:
        let ar: f32 = self.extent.width as f32 / self.extent.height as f32;
        self.projection_matrix = perspective_matrix_fov_vulkan(
            self.fovx, ar, ::camera::NEAR_PLANE, ::camera::FAR_PLANE);

        self.projection_x_view_matrix =
            &self.projection_matrix * &self.view_matrix;
    }
}

/// Generates a perspective matrix, mapping "eye" coordinates (a truncated pyramid
/// or frustum) into normalized device coordinates (a cube)
fn perspective_matrix_fov_vulkan(
    fovx_radians: f32,
    aspect_ratio: f32,
    near: f32,
    far: f32) -> Mat4<f32>
{
    let d: f32 = 1.0 / (fovx_radians/2.0).tan();

    let n = near;
    let f = far;

    Mat4::new(
        d,    0.0,             0.0,       0.0,
        0.0,  d*aspect_ratio,  0.0,       0.0,
        0.0,  0.0,             -f/(n-f),  n*f/(n-f),
        0.0,  0.0,             1.0,       0.0
    )
}

/*
fn look_at(eye: Point3<f32>, target: Point3<f32>, up: Direction3<f32>) -> Mat4<f32>
{
    let zaxis: Direction3<f32> = From::from(eye - target); // The "forward" vector.
    let xaxis: Direction3<f32> = up.cross(zaxis); // The "right" vector.
    let yaxis: Direction3<f32> = zaxis.cross(xaxis); // The "up" vector.

    let eye: Direction3<f32> = From::from(eye.0);
    Mat4::new(
        xaxis.x, xaxis.y, xaxis.z, -xaxis.dot(eye)),
        yaxis.x, yaxis.y, yaxis.z, -yaxis.dot(eye)),
        zaxis.x, zaxis.y, zaxis.z, -zaxis.dot(eye)),
        0.0, 0.0, 0.0, 1.0)
}
 */

pub struct CameraGfx {
    pub descriptor_set: DescriptorSet,
    pub desc_layout: DescriptorSetLayout,
    pub uniforms_buffer: HostVisibleBuffer, // FIXME use push constants
    pub camera_uniforms: CameraUniforms,
    pub state: Arc<State>,
}

impl CameraGfx {
    pub fn new(renderer: &mut Renderer,
               state: Arc<State>)
        -> Result<CameraGfx>
    {
        use dacite::core::{DescriptorType, ShaderStageFlags, BufferUsageFlags,
                           DescriptorSetLayoutCreateInfo};

        let camera_uniforms = CameraUniforms::new(&state);

        let mut uniforms_buffer = renderer.create_host_visible_buffer::<CameraUniforms>(
            1, BufferUsageFlags::UNIFORM_BUFFER,
            Lifetime::Permanent, "Camera Uniforms")?;
        uniforms_buffer.write_one::<CameraUniforms>(&camera_uniforms, None)?;

        let desc_bindings = vec![
            DescriptorSetLayoutBinding {
                binding: 0, // set=0, binding=0
                descriptor_type: DescriptorType::UniformBuffer,
                descriptor_count: 1, // just one UBO
                stage_flags: ShaderStageFlags::VERTEX
                    | ShaderStageFlags::FRAGMENT,
                immutable_samplers: vec![],
            }
        ];

        let (desc_layout, descriptor_set) = renderer.create_descriptor_set(
            DescriptorSetLayoutCreateInfo {
                flags: Default::default(),
                bindings: desc_bindings.clone(),
                chain: None,
            })?;

        // write descriptor set
        {
            use dacite::core:: WriteDescriptorSet;
            use dacite::core::{OptionalDeviceSize, DescriptorBufferInfo};

            let mut write_sets = Vec::new();
            for binding in desc_bindings {
                write_sets.push(WriteDescriptorSet {
                    dst_set: descriptor_set.clone(),
                    dst_binding: binding.binding,
                    dst_array_element: 0, // only have 1 element
                    descriptor_type: binding.descriptor_type,
                    elements: WriteDescriptorSetElements::BufferInfo(
                        vec![
                            DescriptorBufferInfo {
                                buffer: uniforms_buffer.inner(),
                                offset: 0,
                                range: OptionalDeviceSize::WholeSize,
                            }
                        ]
                    ),
                    chain: None,
                });
            }
            DescriptorSet::update(Some(&*write_sets), None);
        }

        Ok(CameraGfx {
            descriptor_set: descriptor_set,
            desc_layout: desc_layout,
            uniforms_buffer: uniforms_buffer,
            camera_uniforms: camera_uniforms,
            state: state,
        })
    }

    pub fn inv_projection(&self) -> Mat4<f32> {
        let p: &mut CameraUniforms = self.uniforms_buffer.as_ptr().unwrap();
        p.projection_matrix.inverse().unwrap()
    }
}

impl Plugin for CameraGfx {
    fn record_geometry(&self, _command_buffer: CommandBuffer) {
    }

    fn record_transparent(&self, _command_buffer: CommandBuffer) {
    }

    fn record_ui(&self, _command_buffer: CommandBuffer) {
    }

    fn update(&mut self, params: &mut Params, _stats: &Stats) -> ::siege_render::Result<bool> {

        let (bloom_strength, bloom_cliff, blur_level) = {
            let rp = self.state.render_params.read().unwrap();
            (
                rp.bloom_strength,
                rp.bloom_cliff,
                rp.blur_level
            )
        };

        // Update the uniforms
        self.camera_uniforms.update(&self.state);

        // Update the renderer
        params.dlight_directions[0] = &self.camera_uniforms.view_matrix * &LIGHT_DIR;
        params.dlight_directions[1] = &self.camera_uniforms.view_matrix * &LIGHT_DIR_2;
        params.inv_projection = self.inv_projection();
        params.bloom_strength = bloom_strength;
        params.bloom_cliff = bloom_cliff;
        params.blur_level = blur_level;

        Ok(false)
    }

    fn gpu_update(&mut self) -> ::siege_render::Result<()> {
        self.uniforms_buffer.write_one::<CameraUniforms>(&self.camera_uniforms, None)?;

        Ok(())
    }

    fn rebuild(&mut self, extent: Extent2D) -> ::siege_render::Result<()> {
        // We take responsibility for saving the extent into the state.camera
        {
            let mut camera = self.state.camera.write().unwrap();
            camera.extent = extent;
        }

        // Update the uniforms
        let p: &mut CameraUniforms = self.uniforms_buffer.as_ptr().unwrap();
        p.update(&self.state);
        Ok(())
    }
}
