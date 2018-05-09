
use std::sync::Arc;
use bit_vec::BitVec;
use dacite::core::{DescriptorSetLayout, DescriptorSet,
                   DescriptorSetLayoutBinding, WriteDescriptorSetElements,
                   Pipeline, PipelineBindPoint,
                  CommandBuffer, PipelineLayout, Extent2D,
                   PrimitiveTopology, CullModeFlags, FrontFace};
use siege_mesh::GrayboxVertex;
use siege_math::{Vec4, Mat4, Point3};
use siege_render::{Renderer, HostVisibleBuffer, Lifetime, VulkanMesh,
                   Pass, VulkanVertex, BlendMode, Plugin,
                   Params, Stats, PipelineSetup};
use errors::*;
use State;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GrayboxUniforms {
    pub model_matrix: Mat4<f32>,
    pub diffuse: Vec4<f32>,
    pub material: Vec4<f32>,
}

impl GrayboxUniforms {
    /*
    pub fn new(g: &Graybox) -> GrayboxUniforms
    {
        // This is part of physically-improved Blinn-Phong shading.
        // We pre-multiply some data to offload some work from the per-fragment
        // shaders.
        let k_diff = g.diffuse / ::std::f32::consts::PI;
        let k_spec = g.specular * (g.shininess + 8.0) / (8.0 * ::std::f32::consts::PI);

        GrayboxUniforms {
            model_matrix: g.model_matrix,
            k_diff: Vec4::<f32>::new(k_diff.x, k_diff.y, k_diff.z, 0.0),
            k_spec: Vec4::<f32>::new(k_spec.x, k_spec.y, k_spec.z, 0.0),
            shininess: g.shininess,
        }
    }
     */

    pub fn update(&mut self, g: &Graybox) {
        self.model_matrix = g.model_matrix;
        self.diffuse = g.diffuse;
        self.material = g.material;
    }
}

#[derive(Debug, Clone)]
pub struct Graybox {
    pub name: String,
    pub mesh: VulkanMesh,
    pub model_matrix: Mat4<f32>,
    pub diffuse: Vec4<f32>,
    pub material: Vec4<f32>,
    pub visible: bool,
}

impl Graybox {
    pub fn new(renderer: &mut Renderer,
               diffuse: Vec4<f32>,
               material: Vec4<f32>,
               model_matrix: Mat4<f32>,
               name: &str)
               -> Result<Graybox>
    {
        let mesh = renderer.load_mesh("graybox",name)?;

        Ok(Graybox {
            name: name.to_owned(),
            mesh: mesh,
            model_matrix: model_matrix,
            diffuse: diffuse,
            material: material,
            visible: true, // start out conservative
        })
    }
}

pub struct GrayboxGfx {
    pub pipeline: Pipeline,
    pub pipeline_layout: PipelineLayout,
    pub camera_desc_set: DescriptorSet,
    pub grayboxes: Vec<Graybox>,
    pub visibility: BitVec,
    pub descriptor_set: DescriptorSet,
    pub desc_layout: DescriptorSetLayout,
    pub uniforms_buffer: HostVisibleBuffer,
    pub stride: usize,
    pub state: Arc<State>,
}

impl GrayboxGfx {
    pub fn new(renderer: &mut Renderer,
               state: Arc<State>,
               camera_desc_set_layout: DescriptorSetLayout,
               camera_desc_set: DescriptorSet,
               max_grayboxes: usize)
               -> Result<GrayboxGfx>
    {
        // We use one uniform buffer for all the grayboxes.
        use dacite::core::BufferUsageFlags;
        let uniforms_buffer = renderer.create_host_visible_buffer::<GrayboxUniforms>(
            max_grayboxes, BufferUsageFlags::UNIFORM_BUFFER,
            Lifetime::Temporary, "Graybox Uniforms")?;

        let stride = renderer.get_stride::<GrayboxUniforms>(
            BufferUsageFlags::UNIFORM_BUFFER);

        let desc_bindings = {
            use dacite::core::{DescriptorType, ShaderStageFlags};
            vec![
                DescriptorSetLayoutBinding {
                    binding: 0,
                    descriptor_type: DescriptorType::UniformBufferDynamic,
                    descriptor_count: 1, // shader sees only 1, not an array.
                    stage_flags: ShaderStageFlags::FRAGMENT
                        | ShaderStageFlags::VERTEX,
                    immutable_samplers: vec![],
                }
            ]
        };

        use dacite::core::DescriptorSetLayoutCreateInfo;
        let (desc_layout, descriptor_set) = renderer.create_descriptor_set(
            DescriptorSetLayoutCreateInfo {
                flags: Default::default(),
                bindings: desc_bindings.clone(),
                chain: None,
            })?;

        // write descriptor set
        {
            use dacite::core::{OptionalDeviceSize, DescriptorBufferInfo,
                               WriteDescriptorSet};

            DescriptorSet::update(
                Some(&[
                    WriteDescriptorSet {
                        dst_set: descriptor_set.clone(),
                        dst_binding: desc_bindings[0].binding,
                        dst_array_element: 0, // start at element 0
                        descriptor_type: desc_bindings[0].descriptor_type,
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
                    }
                ]),
                None
            );
        }

        let (pipeline_layout, pipeline) = renderer.create_pipeline(
            PipelineSetup {
                desc_set_layouts: vec![camera_desc_set_layout,
                                       desc_layout.clone()],
                vertex_shader: Some("graybox.vert"),
                vertex_shader_spec: None,
                fragment_shader: Some("graybox.frag"),
                fragment_shader_spec: None,
                vertex_type: Some(GrayboxVertex::get_input_state_create_info()),
                topology: PrimitiveTopology::TriangleList,
                cull_mode: CullModeFlags::BACK,
                front_face: FrontFace::CounterClockwise,
                test_depth: true,
                write_depth: true,
                blend: vec![BlendMode::Off, BlendMode::Off, BlendMode::Off],
                pass: Pass::Geometry,
                push_constant_ranges: vec![],
            })?;

        Ok(GrayboxGfx {
            pipeline: pipeline,
            pipeline_layout: pipeline_layout,
            camera_desc_set: camera_desc_set,
            grayboxes: vec![],
            visibility: BitVec::new(),
            descriptor_set: descriptor_set,
            desc_layout: desc_layout,
            uniforms_buffer: uniforms_buffer,
            stride: stride,
            state: state,
        })
    }

    pub fn add_graybox(&mut self, graybox: Graybox) -> Result<()> {
        self.grayboxes.push(graybox);
        self.visibility.push(true); // presume it is visible as a starting point
        Ok(())
    }
}

impl Plugin for GrayboxGfx {
    fn record_geometry(&self, command_buffer: CommandBuffer) {
        use dacite::core::IndexType;

        // Bind our pipeline
        command_buffer.bind_pipeline(PipelineBindPoint::Graphics, &self.pipeline);

        // Bind shared descriptor sets (camera)
        command_buffer.bind_descriptor_sets(
            PipelineBindPoint::Graphics,
            &self.pipeline_layout,
            0, // starting with first set
            &[self.camera_desc_set.clone()],
            None,
        );

        // ..for-each-mesh (we are not storing by mesh yet, even though objects
        // might reuse them)
        {
            // For each object
            for i in 0..self.grayboxes.len()
            {
                // FIXME - this should be per-mesh, but we are storing the mesh
                // per object currently
                command_buffer.bind_vertex_buffers(
                    0, // first binding
                    &[self.grayboxes[i].mesh.vertex_buffer.inner()], // buffers
                    &[0], // offsets
                );

                // FIXME - this should be per-mesh, but we are storing the mesh
                // per object currently
                command_buffer.bind_index_buffer(
                    &self.grayboxes[i].mesh.index_buffer.inner(), // buffer
                    0, // offset
                    IndexType::UInt16
                );

                // BIND dynamic uniform buffer for this particular graybox
                let offset = (i * self.stride) as u32;
                command_buffer.bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    &self.pipeline_layout,
                    1, // 0 was camera
                    &[self.descriptor_set.clone()],
                    Some(&[offset])
                );

                // Draw object
                command_buffer.draw_indexed(
                    self.grayboxes[i].mesh.num_indices, // index count
                    1, // instance count (not instancing, so just 1)
                    0, // first index
                    0, // vertex offset
                    0, // first instance (base instance ID=0)
                );
            }
        }
    }

    fn record_transparent(&self, _command_buffer: CommandBuffer) {
    }

    fn record_ui(&self, _command_buffer: CommandBuffer) {
    }

    fn update(&mut self, _params: &mut Params, _stats: &Stats) -> ::siege_render::Result<bool> {

        let mut current_vis = BitVec::from_elem(self.grayboxes.len(), true);

        for (i, ref mut graybox) in self.grayboxes.iter_mut().enumerate() {
            // Determine visibility
            {
                if let Some(ref bs) = graybox.mesh.bounding_sphere {
                    let camera = self.state.camera.read().unwrap();
                    let v4: Vec4<f32> = From::from(bs.0);
                    let v4: Vec4<f32> = &graybox.model_matrix * &v4;
                    let p3: Point3<f32> = From::from(v4);
                    graybox.visible = camera.might_be_visible(&p3, bs.1);
                }
                else if let Some(ref bc) = graybox.mesh.bounding_cuboid {
                    let camera = self.state.camera.read().unwrap();
                    graybox.visible = false;
                    for corner in 0..8 {
                        let v4: Vec4<f32> = From::from(bc[corner]);
                        let v4: Vec4<f32> = &graybox.model_matrix * &v4;
                        let p3: Point3<f32> = From::from(v4);
                        if camera.might_be_visible(&p3, 0.0) {
                            graybox.visible = true;
                            break;
                        }
                    }
                }
                else {
                    graybox.visible = true;
                }
                current_vis.set(i, graybox.visible);
            }
        }

        if current_vis != self.visibility {
            self.visibility = current_vis;
            //debug!("Re-recording grayboxes: {:?}", self.visibility);
            Ok(true) // re-record cmd buffer
        } else {
            Ok(false) // dont re-record
        }
    }

    fn gpu_update(&mut self) -> ::siege_render::Result<()> {
        for (i, ref mut graybox) in self.grayboxes.iter_mut().enumerate() {
            let p: &mut GrayboxUniforms =
                self.uniforms_buffer.as_ptr_at_offset(i).unwrap();
            p.update(graybox)
        }

        Ok(())
    }

    fn rebuild(&mut self, _extent: Extent2D) -> ::siege_render::Result<()> {
        Ok(())
    }
}
