
use std::sync::Arc;
use dacite::core::{Pipeline, PipelineLayout, DescriptorSet, DescriptorSetLayout,
                   Sampler, DescriptorSetLayoutBinding, CommandBuffer,
                   Extent2D, ImageLayout, PrimitiveTopology, CullModeFlags,
                   FrontFace, ShaderStageFlags, PipelineBindPoint, ImageView,
                   SpecializationInfo, SpecializationMapEntry,
                   PushConstantRange, Viewport, Rect2D, Offset2D};
use ui::{UiElement, AbsRect};
use siege_render::{Renderer, Stats, Plugin, Params, BlendMode,
                   Pass, ImageWrap, PipelineSetup};
use state::State;
use errors::*;

#[allow(dead_code)] // appears dead to rust, but we send this to vulkan
#[derive(Debug)]
struct PushConsts {
    uv_x1: f32,
    uv_y1: f32,
    uv_width: f32,
    uv_height: f32,
    screen_pin_x1: f32,
    screen_pin_y1: f32,
    screen_pin_width: f32,
    screen_pin_height: f32,
    screen_area_x1: f32,
    screen_area_y1: f32,
    screen_area_width: f32,
    screen_area_height: f32,
    alpha: f32,
}

pub struct UiImageGfx {
    pipeline: Pipeline,
    pipeline_layout: PipelineLayout,
    descriptor_set: DescriptorSet,
    #[allow(dead_code)]
    desc_layout: DescriptorSetLayout,
    #[allow(dead_code)]
    image_view: ImageView,
    #[allow(dead_code)]
    image: ImageWrap,
    #[allow(dead_code)]
    sampler: Sampler,
    full_viewport: Viewport,
    state: Arc<State>,
}

impl UiImageGfx {
    pub fn new(renderer: &mut Renderer,
               state: Arc<State>)
               -> Result<UiImageGfx>
    {
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
                anisotropy_enable: false,
                max_anisotropy: 0.0,
                compare_enable: false,
                compare_op: CompareOp::Never,
                min_lod: 0.0,
                max_lod: 0.5,
                border_color: BorderColor::FloatOpaqueWhite,
                unnormalized_coordinates: true,
                chain: None
            })?
        };

        let image = renderer.load_texture("ui_atlas_BC7")?;
        let image_view = renderer.get_image_view(&image)?;

        let desc_bindings = {
            use dacite::core::{DescriptorType, ShaderStageFlags};
            vec![
                DescriptorSetLayoutBinding {
                    binding: 0,
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
                        elements: WriteDescriptorSetElements::ImageInfo(
                            vec![DescriptorImageInfo {
                                sampler: Some(sampler.clone()),
                                image_view: Some(image_view.clone()),
                                image_layout: ImageLayout::ShaderReadOnlyOptimal,
                            }]
                        ),
                        chain: None,
                    },
                ]),
                None
            );
        }

        // Specialization constant: does UI output need sRGB gamma function?
        let fragment_spec = SpecializationInfo {
            map_entries: vec![
                SpecializationMapEntry {
                    constant_id: 0,
                    offset: 0,
                    size: 4,
                },
            ],
            data: if renderer.ui_needs_gamma() {
                vec![ 0x01, 0x00, 0x00, 0x00 ]
            } else {
                vec![ 0x00, 0x00, 0x00, 0x00 ]
            }
        };

        let (pipeline_layout, pipeline) = renderer.create_pipeline(
            PipelineSetup {
                desc_set_layouts: vec![desc_layout.clone()],
                vertex_shader: Some("ui_image.vert"),
                vertex_shader_spec: None,
                fragment_shader: Some("ui_image.frag"),
                fragment_shader_spec: Some(fragment_spec),
                vertex_type: None,
                topology: PrimitiveTopology::TriangleStrip,
                cull_mode: CullModeFlags::BACK,
                front_face: FrontFace::CounterClockwise,
                test_depth: true,
                write_depth: true,
                blend: vec![BlendMode::Alpha],
                pass: Pass::Ui,
                push_constant_ranges: vec![
                    PushConstantRange { // used for depth
                        stage_flags: ShaderStageFlags::VERTEX,
                        offset: 0,
                        size: ::std::mem::size_of::<PushConsts>() as u32,
                    }
                ],
            })?;

        Ok(UiImageGfx {
            pipeline: pipeline,
            pipeline_layout: pipeline_layout,
            descriptor_set: descriptor_set,
            desc_layout: desc_layout,
            image_view: image_view,
            image: image,
            sampler: sampler,
            full_viewport: renderer.get_viewport(),
            state: state
        })
    }
}

impl Plugin for UiImageGfx {
    fn record_geometry(&self, _command_buffer: CommandBuffer) {
    }

    fn record_transparent(&self, _command_buffer: CommandBuffer) {
    }

    fn record_ui(&self, command_buffer: CommandBuffer) {

        command_buffer.bind_pipeline(PipelineBindPoint::Graphics, &self.pipeline);

        command_buffer.bind_descriptor_sets(
            PipelineBindPoint::Graphics,
            &self.pipeline_layout,
            0, // first_set
            &[self.descriptor_set.clone()],
            None,
        );


        // fn to compute max_depth at any node in the UI tree
        //
        let max_depth = self.full_viewport.max_depth;
        // NOTE: we use 0.00000012 because the next f32 representation below
        // 1.0 is 0.99999994.  Using double of that spacing, we can be certain
        // that each float is separately representable.
        // More ideally we would use u32, but our depth buffer is unfortunately
        // for this case, stored as f32s.
        let depth_incr = if self.full_viewport.min_depth < self.full_viewport.max_depth {
            0.00000012
        } else {
            -0.00000012
        };
        let get_max_depth = |nodedepth| -> f32 { max_depth - depth_incr * nodedepth as f32 };


        for (nodeinfo, nodeguard) in self.state.ui.walk(
            self.full_viewport.width, self.full_viewport.height)

        {
            let element = &(*nodeguard).element;

            // This pipeline only renders UiImages
            let image = match element {
                &UiElement::Image(ref i) => i,
                _ => continue
            };

            // Set viewport
            command_buffer.set_viewport(0, &[
                Viewport {
                    x: nodeinfo.rect.x,
                    y: nodeinfo.rect.y,
                    width: nodeinfo.rect.width,
                    height: nodeinfo.rect.height,
                    min_depth: self.full_viewport.min_depth,
                    max_depth: get_max_depth(nodeinfo.depth)
                }]);

            // Set scissor
            command_buffer.set_scissor(0, &[Rect2D {
                offset: Offset2D { x: nodeinfo.rect.x as i32, y: nodeinfo.rect.y as i32 },
                extent: Extent2D { width: nodeinfo.rect.width as u32, height: nodeinfo.rect.height as u32 },
            }]);

            let parent: AbsRect = AbsRect {
                x: 0.0,
                y: 0.0,
                width: nodeinfo.rect.width,
                height: nodeinfo.rect.height,
            };
            let pinrect = image.widget_pin_rect.absolute(&parent);
            let drawrect = image.screen_draw_rect.absolute(&parent);

            let push_consts = PushConsts {
                uv_x1: image.widget.x,// +0.5?
                uv_y1: image.widget.y,
                uv_width: image.widget.width,
                uv_height: image.widget.height,
                screen_pin_x1: pinrect.x / parent.width,
                screen_pin_y1: pinrect.y / parent.height,
                screen_pin_width: pinrect.width / parent.width,
                screen_pin_height: pinrect.height / parent.height,
                screen_area_x1: drawrect.x / parent.width,
                screen_area_y1: drawrect.y / parent.height,
                screen_area_width: drawrect.width / parent.width,
                screen_area_height: drawrect.height / parent.height,
                alpha: nodeinfo.alpha,
            };
            let constants: &[u8] = unsafe {
                ::std::slice::from_raw_parts(
                    (&push_consts as *const PushConsts) as *const u8,
                    ::std::mem::size_of::<PushConsts>()
                )
            };
            command_buffer.push_constants(
                &self.pipeline_layout,
                ShaderStageFlags::VERTEX,
                0,
                constants);

            // Draw
            command_buffer.draw(
                4, // vertex count
                1, // instance count
                0, // first index
                0 // first instance (base instance ID=0)
            );
        }

        self.state.ui.clear_image_dirty();
    }

    fn update(&mut self, _params: &mut Params, _stats: &Stats) -> ::siege_render::Result<bool> {
        let dirty = self.state.ui.is_image_dirty();
        if dirty {
            Ok(true) // re-record
        } else {
            Ok(false)
        }
    }

    fn gpu_update(&mut self) -> ::siege_render::Result<()> {
        // FIXME GINA
        Ok(())
    }

    fn rebuild(&mut self, extent: Extent2D) -> ::siege_render::Result<()> {
        self.full_viewport.width = extent.width as f32;
        self.full_viewport.height = extent.height as f32;
        Ok(())
    }
}
