
use dacite::core::{Pipeline, PipelineBindPoint,
                   CommandBuffer, PipelineLayout,
                   PrimitiveTopology, CullModeFlags, FrontFace,
                   DescriptorSetLayout, DescriptorSet, Extent2D,
                   DescriptorSetLayoutBinding, Format, BufferView,
                   BufferUsageFlags, ImageLayout, ImageView,
                   Sampler};
use siege_render::{Renderer, Pass, BlendMode, Plugin,
                   Params, DeviceLocalBuffer, ImageWrap, Stats,
                   PipelineSetup};
use errors::*;

pub struct TerrainGfx {
    pipeline: Pipeline,
    pipeline_layout: PipelineLayout,
    descriptor_set: DescriptorSet,
    #[allow(dead_code)]
    desc_layout: DescriptorSetLayout,
    num_vertices: u32,
    #[allow(dead_code)]
    cavity_image_view: ImageView,
    #[allow(dead_code)]
    cavity_image: ImageWrap,
    #[allow(dead_code)]
    roughness_image_view: ImageView,
    #[allow(dead_code)]
    roughness_image: ImageWrap,
    #[allow(dead_code)]
    ao_image_view: ImageView,
    #[allow(dead_code)]
    ao_image: ImageWrap,
    #[allow(dead_code)]
    normal_image_view: ImageView,
    #[allow(dead_code)]
    normal_image: ImageWrap,
    #[allow(dead_code)]
    albedo_image_view: ImageView,
    #[allow(dead_code)]
    albedo_image: ImageWrap,
    #[allow(dead_code)]
    sampler: Sampler,
    #[allow(dead_code)]
    heightmap_buffer: DeviceLocalBuffer,
    #[allow(dead_code)]
    heightmap_buffer_view: BufferView,
    camera_desc_set: DescriptorSet,
}

impl TerrainGfx {
    pub fn new(renderer: &mut Renderer,
               camera_desc_set_layout: DescriptorSetLayout,
               camera_desc_set: DescriptorSet)
               -> Result<TerrainGfx>
    {
        let heightmap_buffer = renderer.load_buffer(
            BufferUsageFlags::UNIFORM_TEXEL_BUFFER, "sample_terrain")?;
        let heightmap_buffer_view = renderer.get_buffer_view(
            &heightmap_buffer, Format::R16_UInt)?;

        let sampler = {
            use dacite::core::{SamplerCreateInfo, SamplerMipmapMode, SamplerAddressMode,
                               BorderColor, Filter, CompareOp};

            renderer.create_sampler(SamplerCreateInfo {
                flags: Default::default(),
                mag_filter: Filter::Linear,
                min_filter: Filter::Linear,
                mipmap_mode: SamplerMipmapMode::Linear,
                address_mode_u: SamplerAddressMode::ClampToEdge,
                address_mode_v: SamplerAddressMode::ClampToEdge,
                address_mode_w: SamplerAddressMode::ClampToEdge,
                mip_lod_bias: 0.0,
                anisotropy_enable: true,
                max_anisotropy: 16.0, // FIXME get from renderer
                compare_enable: false,
                compare_op: CompareOp::Never,
                min_lod: 0.0,
                max_lod: 10.0,
                border_color: BorderColor::FloatOpaqueWhite,
                unnormalized_coordinates: false,
                chain: None
            })?
        };

        let albedo_image = renderer.load_texture("lawn_albedo_BC7")?;
        let albedo_image_view = renderer.get_image_view(&albedo_image)?;
        let normal_image = renderer.load_texture("lawn_normal_BC7")?;
        let normal_image_view = renderer.get_image_view(&normal_image)?;
        let ao_image = renderer.load_texture("lawn_ao_BC4")?;
        let ao_image_view = renderer.get_image_view(&ao_image)?;
        let roughness_image = renderer.load_texture("lawn_roughness_BC4")?;
        let roughness_image_view = renderer.get_image_view(&roughness_image)?;
        let cavity_image = renderer.load_texture("lawn_cavity_BC4")?;
        let cavity_image_view = renderer.get_image_view(&cavity_image)?;

        let height = 513;
        let width = 513;
        let num_vertices = (height - 1) * (2 * width - 1) + 1;

        // TBD: We will use one uniform buffer for all the terrain, and we will
        // draw instanced.  For now, we only have 1 terrain so it doesn't matter.

        let desc_bindings = {
            use dacite::core::{DescriptorType, ShaderStageFlags};
            vec![
                DescriptorSetLayoutBinding {
                    binding: 0,
                    descriptor_type: DescriptorType::UniformTexelBuffer,
                    descriptor_count: 1,
                    stage_flags: ShaderStageFlags::VERTEX,
                    immutable_samplers: vec![],
                },
                DescriptorSetLayoutBinding {
                    binding: 1,
                    descriptor_type: DescriptorType::CombinedImageSampler,
                    descriptor_count: 1,
                    stage_flags: ShaderStageFlags::FRAGMENT,
                    immutable_samplers: vec![],
                },
                DescriptorSetLayoutBinding {
                    binding: 2,
                    descriptor_type: DescriptorType::CombinedImageSampler,
                    descriptor_count: 1,
                    stage_flags: ShaderStageFlags::FRAGMENT,
                    immutable_samplers: vec![],
                },
                DescriptorSetLayoutBinding {
                    binding: 3,
                    descriptor_type: DescriptorType::CombinedImageSampler,
                    descriptor_count: 1,
                    stage_flags: ShaderStageFlags::FRAGMENT,
                    immutable_samplers: vec![],
                },
                DescriptorSetLayoutBinding {
                    binding: 4,
                    descriptor_type: DescriptorType::CombinedImageSampler,
                    descriptor_count: 1,
                    stage_flags: ShaderStageFlags::FRAGMENT,
                    immutable_samplers: vec![],
                },
                DescriptorSetLayoutBinding {
                    binding: 5,
                    descriptor_type: DescriptorType::CombinedImageSampler,
                    descriptor_count: 1,
                    stage_flags: ShaderStageFlags::FRAGMENT,
                    immutable_samplers: vec![],
                },
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
            use dacite::core::{WriteDescriptorSet, WriteDescriptorSetElements,
                               DescriptorImageInfo};

            DescriptorSet::update(
                Some(&[
                    WriteDescriptorSet {
                        dst_set: descriptor_set.clone(),
                        dst_binding: desc_bindings[0].binding,
                        dst_array_element: 0, // only have 1 element
                        descriptor_type: desc_bindings[0].descriptor_type,
                        elements: WriteDescriptorSetElements::TexelBufferView(
                            vec![heightmap_buffer_view.clone()]
                        ),
                        chain: None,
                    },
                    WriteDescriptorSet {
                        dst_set: descriptor_set.clone(),
                        dst_binding: desc_bindings[1].binding,
                        dst_array_element: 0, // only have 1 element
                        descriptor_type: desc_bindings[1].descriptor_type,
                        elements: WriteDescriptorSetElements::ImageInfo(
                            vec![DescriptorImageInfo {
                                sampler: Some(sampler.clone()),
                                image_view: Some(albedo_image_view.clone()),
                                image_layout: ImageLayout::ShaderReadOnlyOptimal,
                            }]
                        ),
                        chain: None,
                    },
                    WriteDescriptorSet {
                        dst_set: descriptor_set.clone(),
                        dst_binding: desc_bindings[2].binding,
                        dst_array_element: 0, // only have 1 element
                        descriptor_type: desc_bindings[2].descriptor_type,
                        elements: WriteDescriptorSetElements::ImageInfo(
                            vec![DescriptorImageInfo {
                                sampler: Some(sampler.clone()),
                                image_view: Some(normal_image_view.clone()),
                                image_layout: ImageLayout::ShaderReadOnlyOptimal,
                            }]
                        ),
                        chain: None,
                    },
                    WriteDescriptorSet {
                        dst_set: descriptor_set.clone(),
                        dst_binding: desc_bindings[3].binding,
                        dst_array_element: 0, // only have 1 element
                        descriptor_type: desc_bindings[3].descriptor_type,
                        elements: WriteDescriptorSetElements::ImageInfo(
                            vec![DescriptorImageInfo {
                                sampler: Some(sampler.clone()),
                                image_view: Some(ao_image_view.clone()),
                                image_layout: ImageLayout::ShaderReadOnlyOptimal,
                            }]
                        ),
                        chain: None,
                    },
                    WriteDescriptorSet {
                        dst_set: descriptor_set.clone(),
                        dst_binding: desc_bindings[4].binding,
                        dst_array_element: 0, // only have 1 element
                        descriptor_type: desc_bindings[4].descriptor_type,
                        elements: WriteDescriptorSetElements::ImageInfo(
                            vec![DescriptorImageInfo {
                                sampler: Some(sampler.clone()),
                                image_view: Some(roughness_image_view.clone()),
                                image_layout: ImageLayout::ShaderReadOnlyOptimal,
                            }]
                        ),
                        chain: None,
                    },
                    WriteDescriptorSet {
                        dst_set: descriptor_set.clone(),
                        dst_binding: desc_bindings[5].binding,
                        dst_array_element: 0, // only have 1 element
                        descriptor_type: desc_bindings[5].descriptor_type,
                        elements: WriteDescriptorSetElements::ImageInfo(
                            vec![DescriptorImageInfo {
                                sampler: Some(sampler.clone()),
                                image_view: Some(cavity_image_view.clone()),
                                image_layout: ImageLayout::ShaderReadOnlyOptimal,
                            }]
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
                vertex_shader: Some("terrain.vert"),
                vertex_shader_spec: None,
                fragment_shader: Some("terrain.frag"),
                fragment_shader_spec: None,
                vertex_type: None,
                topology: PrimitiveTopology::TriangleStrip,
                cull_mode: CullModeFlags::BACK,
                front_face: FrontFace::CounterClockwise,
                test_depth: true,
                write_depth: true,
                blend: vec![BlendMode::Off, BlendMode::Off, BlendMode::Off],
                pass: Pass::Geometry,
                push_constant_ranges: vec![],
            })?;

        Ok(TerrainGfx {
            pipeline: pipeline,
            pipeline_layout: pipeline_layout,
            descriptor_set: descriptor_set,
            desc_layout: desc_layout,
            num_vertices: num_vertices,
            cavity_image_view: cavity_image_view,
            cavity_image: cavity_image,
            roughness_image_view: roughness_image_view,
            roughness_image: roughness_image,
            ao_image_view: ao_image_view,
            ao_image: ao_image,
            normal_image_view: normal_image_view,
            normal_image: normal_image,
            albedo_image_view: albedo_image_view,
            albedo_image: albedo_image,
            sampler: sampler,
            heightmap_buffer_view: heightmap_buffer_view,
            heightmap_buffer: heightmap_buffer,
            camera_desc_set: camera_desc_set
        })
    }
}

impl Plugin for TerrainGfx {
    fn record_geometry(&self, command_buffer: CommandBuffer) {
        // Bind our pipeline
        command_buffer.bind_pipeline(PipelineBindPoint::Graphics, &self.pipeline);

        // SET 0 = camera descriptor set (which is shared),
        command_buffer.bind_descriptor_sets(
            PipelineBindPoint::Graphics,
            &self.pipeline_layout,
            0, // first_set
            &[self.camera_desc_set.clone(),
              self.descriptor_set.clone()],
            None,
        );

        // Draw terrain
        command_buffer.draw(
            self.num_vertices,
            1, // instance count (not instancing, so just 1)
            0, // first vertex
            0, // first instance (base instance ID=0)
        );
    }

    fn record_transparent(&self, _command_buffer: CommandBuffer) {
    }

    fn record_ui(&self, _command_buffer: CommandBuffer) {
    }

    fn update(&mut self, _params: &mut Params, _stats: &Stats) -> ::siege_render::Result<bool>
    {
        Ok(false)
    }

    fn gpu_update(&mut self) -> ::siege_render::Result<()> {
        Ok(())
    }

    fn rebuild(&mut self, _extent: Extent2D) -> ::siege_render::Result<()> {
        Ok(())
    }
}
