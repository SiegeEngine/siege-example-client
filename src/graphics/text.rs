
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;
use dacite::core::{Pipeline, PipelineBindPoint, PipelineLayout,
                   PrimitiveTopology, CullModeFlags, FrontFace,
                   CommandBuffer, Extent2D, ImageView,
                   Sampler, DescriptorSet, DescriptorSetLayout,
                   DescriptorSetLayoutBinding, ImageLayout,
                   BufferUsageFlags, PipelineVertexInputStateCreateInfo,
                   VertexInputBindingDescription, VertexInputRate,
                   VertexInputAttributeDescription, Format,
                   SpecializationInfo, SpecializationMapEntry,
                   PushConstantRange, ShaderStageFlags,
                   Viewport, Rect2D, Offset2D};
use siege_font::{FontAtlas, CInfo, Box};
use siege_render::{Renderer, Pass, BlendMode, Stats,
                   Plugin, Params, ImageWrap, Lifetime, HostVisibleBuffer,
                   PipelineSetup};
use state::State;
use ui::{Ui, AbsRect, UiElement, TextLine};
use errors::*;

// We have 6 vertices per glyph (triangle strip wouldn't let us separate glyphs)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GlyphVertex {
    pub encoded_screen: u32,
    pub encoded_uv: u32,

    // props encodes multiple things:
    //   bits 0..7:    alpha (so fonts can fade in and out)
    //   bits 8,9:     font (from a fixed list of 4 fonts)
    //   bits 10..12:  color (from a fixed list of 8 colors)
    //   bits 13..15:  outline color (if outline flag is on)
    //   bit  16:      outline flag
    //   bits 17..23:  reserved
    //   bits 24..31:  10*margin, used for subpixel rendering
    pub props: u32
}

pub struct FontData {
    #[allow(dead_code)]
    image_view: ImageView,
    #[allow(dead_code)]
    image: ImageWrap,
    atlas: FontAtlas,
}

// Each textline produces a vertex group; we generate these on update/rebuild,
// and render them from the generated data during record_ui.
pub struct GroupData {
    pub offset: u32, // into the vertex buffer
    pub len: u32, // count of vertices (not bytes)
    pub arect: AbsRect, // viewport data
    pub depth: usize, // viewport data: increments from the back
}

pub struct TextGfx {
    pipeline: Pipeline,
    pipeline_layout: PipelineLayout,
    descriptor_set: DescriptorSet,
    #[allow(dead_code)]
    desc_layout: DescriptorSetLayout,
    vertex_buffer: Vec<GlyphVertex>,
    gpu_vertex_buffer: HostVisibleBuffer, // 6 per glyph
    vertex_groups: Vec<GroupData>,
    font_data: FontData,
    #[allow(dead_code)]
    sampler: Sampler,
    full_viewport: Viewport,
    state: Arc<State>,
}

impl TextGfx {
    pub fn new(renderer: &mut Renderer,
               state: Arc<State>)
               -> Result<TextGfx>
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

        let mut fonts_path: PathBuf = renderer.get_asset_path();
        fonts_path.push("fonts");

        let font_data = {
            let file = File::open(fonts_path.clone().join("Gudea-Regular.bin"))?;
            let atlas = ::bincode::deserialize_from(&file)?;
            let image = renderer.load_texture("Gudea-Regular")?;
            let image_view = renderer.get_image_view(&image)?;
            FontData {
                image_view: image_view,
                image: image,
                atlas: atlas,
            }
        };

        let gpu_vertex_buffer: HostVisibleBuffer =
            renderer.create_host_visible_buffer::<GlyphVertex>(
                5000 * 6, // Room for 5000 characters
                BufferUsageFlags::VERTEX_BUFFER,
                Lifetime::Permanent,
                "Text Glyph Vertices"
            )?;

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
                                image_view: Some(font_data.image_view.clone()),
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

        let vertex_type = PipelineVertexInputStateCreateInfo {
            flags: Default::default(),
            vertex_binding_descriptions: vec![
                VertexInputBindingDescription {
                    binding: 0_u32,
                    stride: ::std::mem::size_of::<GlyphVertex>() as u32,
                    input_rate: VertexInputRate::Vertex,
                },
            ],
            vertex_attribute_descriptions: vec![
                VertexInputAttributeDescription {
                    location: 0,
                    binding: 0,
                    format: Format::R32_UInt,
                    offset: offset_of!(GlyphVertex, encoded_screen) as u32,
                },
                VertexInputAttributeDescription {
                    location: 1,
                    binding: 0,
                    format: Format::R32_UInt,
                    offset: offset_of!(GlyphVertex, encoded_uv) as u32,
                },
                VertexInputAttributeDescription {
                    location: 2,
                    binding: 0,
                    format: Format::R32_UInt,
                    offset: offset_of!(GlyphVertex, props) as u32,
                }
            ],
            chain: None,
        };

        let (pipeline_layout, pipeline) = renderer.create_pipeline(
            PipelineSetup {
                desc_set_layouts: vec![desc_layout.clone()],
                vertex_shader: Some("text.vert"),
                vertex_shader_spec: None,
                fragment_shader: Some("text.frag"),
                fragment_shader_spec: Some(fragment_spec),
                vertex_type: Some(vertex_type),
                topology: PrimitiveTopology::TriangleList,
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
                        size: 4,
                    }
                ],
            })?;

        Ok(TextGfx {
            pipeline: pipeline,
            pipeline_layout: pipeline_layout,
            descriptor_set: descriptor_set,
            desc_layout: desc_layout,
            vertex_buffer: vec![],
            gpu_vertex_buffer: gpu_vertex_buffer,
            vertex_groups: vec![],
            font_data: font_data,
            sampler: sampler,
            full_viewport: renderer.get_viewport(),
            state: state,
        })
    }

    fn generate_vertices(&self, line: &TextLine, vport: &AbsRect, alpha: f32)
                         -> Vec<GlyphVertex>
    {
        let mut vertices: Vec<GlyphVertex> = Vec::new();

        // Vertices will be rendered while bound to a smaller viewport.
        // But we must treat that vport as having upper-left coordinates
        // of (0,0)
        let mut vport = vport.clone();
        vport.x = 0.0;
        vport.y = 0.0;

        // Interpret coordinates relative to the viewport
        let (mut cursor_x, cursor_y) = line.ui_coordinates.absolute(&vport);

        let atlas: &FontAtlas = &self.font_data.atlas;

        let scale = line.lineheight as f32 / atlas.line_height;
        let margin = atlas.margin * scale;

        for ch in line.text.chars() {
            let atlas_cinfo = match atlas.map.get(&ch) {
                Some(cinfo) => cinfo,
                None => {
                    debug!("Skipping character {} = {}", ch, ch.escape_unicode());
                    continue; // FIXME: use a placeholder character
                },
            };

            let uv = {
                // We need the pre-scaled values ("atlas.margin", not "margin")
                // UV coordinates include the margin area.
                // UV coordinates are in pixels, "center of pixel".
                // Vulkan unnormalized coords are in pixels, "edge of pixel"
                let mut uv = atlas_cinfo.inner_bounding_box.clone();
                uv.x -= atlas.margin + 0.5;
                uv.y -= atlas.margin + 0.5;
                uv.w += 2.0 * atlas.margin + 0.5;
                uv.h += 2.0 * atlas.margin + 0.5;
                uv
            };

            // Scaled cinfo (still using 'inner' bounding box)
            let cinfo = CInfo {
                inner_bounding_box: Box {
                    x: 0.0, // unused once we are in screen coords
                    y: 0.0, // unused once we are in screen coords
                    w: atlas_cinfo.inner_bounding_box.w * scale,
                    h: atlas_cinfo.inner_bounding_box.h * scale,
                },
                pre_draw_advance: atlas_cinfo.pre_draw_advance * scale,
                post_draw_advance: atlas_cinfo.post_draw_advance * scale,
                height_offset: atlas_cinfo.height_offset * scale
            };

            // Adjustment for just this glyph:
            let mut glyph_cursor_x: f32 = cursor_x + cinfo.pre_draw_advance; // left side bearing
            let mut glyph_cursor_y: f32 = cursor_y + cinfo.height_offset; // height offset

            // Compute the screen box, in pixels
            let screen_px: Box = Box {
                x: glyph_cursor_x - margin,
                y: glyph_cursor_y - cinfo.inner_bounding_box.h - margin,
                w: cinfo.inner_bounding_box.w + 2.0 * margin,
                h: cinfo.inner_bounding_box.h + 2.0 * margin,
            };

            // Compute the screen box, in gl coordinates
            let screen_gl: Box = Box {
                x: (2.0 * screen_px.x - vport.width) / vport.width,
                y: (2.0 * screen_px.y - vport.height) / vport.height,
                w: 2.0 * screen_px.w / vport.width,
                h: 2.0 * screen_px.h / vport.height,
            };

            let props = encode_props_plus(&line, margin, alpha);

            let upper_left = GlyphVertex {
                encoded_screen: encode_screen(screen_gl.x, screen_gl.y),
                encoded_uv: encode_uv(uv.x, uv.y),
                props: props,
            };

            let lower_left = GlyphVertex {
                encoded_screen: encode_screen(screen_gl.x, screen_gl.y + screen_gl.h),
                encoded_uv: encode_uv(uv.x, uv.y + uv.h),
                props: props,
            };

            let upper_right = GlyphVertex {
                encoded_screen: encode_screen(screen_gl.x + screen_gl.w, screen_gl.y),
                encoded_uv: encode_uv(uv.x + uv.w, uv.y),
                props: props,
            };

            let lower_right = GlyphVertex {
                encoded_screen: encode_screen(screen_gl.x + screen_gl.w, screen_gl.y + screen_gl.h),
                encoded_uv: encode_uv(uv.x + uv.w, uv.y + uv.h),
                props: props,
            };

            vertices.extend_from_slice(
                &[upper_left, lower_left.clone(), upper_right.clone(),
                  upper_right, lower_left, lower_right]
            );

            // post-draw advance
            cursor_x += cinfo.post_draw_advance;
        }

        vertices
    }

    fn regenerate(&mut self, ui: &Ui) -> ::siege_render::Result<()>
    {
        self.vertex_buffer.clear();
        self.vertex_groups.clear();

        let mut offset = 0;
        for (nodeinfo, nodeguard) in ui.walk(
            self.full_viewport.width, self.full_viewport.height)
        {
            let element = &(*nodeguard).element;

            // This pipeline only renders text
            let textline = match element {
                &UiElement::Text(ref t) => t,
                _ => continue // we only render text here
            };

            let groupvertices = self.generate_vertices(textline, &nodeinfo.rect,
                                                       nodeinfo.alpha);
            let grouplen = groupvertices.len();
            self.vertex_buffer.extend(groupvertices);
            self.vertex_groups.push(GroupData {
                offset: offset as u32,
                len: grouplen as u32,
                arect: nodeinfo.rect,
                depth: nodeinfo.depth,
            });
            offset += grouplen;
        }
        Ok(())
    }
}

impl Plugin for TextGfx {
    fn record_geometry(&self, _command_buffer: CommandBuffer) {
    }

    fn record_transparent(&self, _command_buffer: CommandBuffer) {
    }

    fn record_ui(&self, command_buffer: CommandBuffer) {

        if self.vertex_groups.len() == 0 {
            return;
        }

        command_buffer.bind_pipeline(PipelineBindPoint::Graphics, &self.pipeline);

        command_buffer.bind_descriptor_sets(
            PipelineBindPoint::Graphics,
            &self.pipeline_layout,
            0, // first_set
            &[self.descriptor_set.clone()],
            None,
        );

        command_buffer.bind_vertex_buffers(
            0, // first binding
            &[self.gpu_vertex_buffer.inner()], // buffers
            &[0], // offsets
        );

        // fn to compute max_depth at any node in the UI tree
        //
        // NOTE: we use 0.00000012 because the next f32 representation below
        // 1.0 is 0.99999994.  Using double of that spacing, we can be certain
        // that each float is separately representable.
        // More ideally we would use u32, but our depth buffer is unfortunately
        // for this case, stored as f32s.
        //
        // NOTE: this fn is different from the viewport-related one in window.rs
        // because shaders always see mindepth=0.0 and maxdepth=1.0 regardless
        // of reversed depth buffers.
        //
        let depth_incr = 0.00000012;
        let get_max_depth = |nodedepth| -> f32 { 1.0 - depth_incr * nodedepth as f32 };

        for group in &self.vertex_groups {

            if group.len == 0 { continue; }

            // Set viewport
            command_buffer.set_viewport(0, &[
                Viewport {
                    x: group.arect.x,
                    y: group.arect.y,
                    width: group.arect.width,
                    height: group.arect.height,
                    min_depth: self.full_viewport.min_depth,
                    max_depth: self.full_viewport.max_depth,//(shader sets depth, not viewport)
                }]);

            // Set scissor
            command_buffer.set_scissor(0, &[Rect2D {
                offset: Offset2D { x: group.arect.x as i32, y: group.arect.y as i32 },
                extent: Extent2D { width: group.arect.width as u32,
                                   height: group.arect.height as u32 },
            }]);

            // Push depth
            let depth: f32 = get_max_depth(group.depth);
            let constants: &[u8] = unsafe {
                ::std::slice::from_raw_parts(
                    (&depth as *const f32) as *const u8,
                    ::std::mem::size_of::<f32>()
                )
            };
            command_buffer.push_constants(
                &self.pipeline_layout,
                ShaderStageFlags::VERTEX,
                0,
                constants);

            // Draw
            command_buffer.draw(
                group.len, // vertex count
                1, // instance count
                group.offset, // first index
                0, // first instance (base instance ID=0)
            );
        }

        // Restore the top level viewport
        command_buffer.set_viewport(0, &[self.full_viewport]);
        command_buffer.set_scissor(0, &[Rect2D {
            offset: Offset2D { x: self.full_viewport.x as i32, y: self.full_viewport.y as i32 },
            extent: Extent2D { width: self.full_viewport.width as u32, height: self.full_viewport.height as u32 },
        }]);
    }

    fn update(&mut self, _params: &mut Params, _stats: &Stats) -> ::siege_render::Result<bool> {

        let state = self.state.clone();
        if state.ui.is_text_dirty() {
            self.regenerate(&state.ui)?;
            state.ui.clear_text_dirty();
            Ok(true) // need re-record
        } else {
            Ok(false)
        }
    }

    fn gpu_update(&mut self) -> ::siege_render::Result<()> {
        self.gpu_vertex_buffer.write_array(&*self.vertex_buffer, None)?;
        Ok(())
    }

    fn rebuild(&mut self, extent: Extent2D) -> ::siege_render::Result<()> {
        self.full_viewport.width = extent.width as f32;
        self.full_viewport.height = extent.height as f32;
        let state = self.state.clone();
        self.regenerate(&state.ui)?;
        // Clear dirty bit
        self.state.ui.clear_text_dirty();
        Ok(())
    }
}

#[inline]
fn encode_props_plus(line: &TextLine, margin: f32, alpha: f32) -> u32 {
    let alpha_u: u32 = ((((line.alpha as f32) / 255.0) * alpha) * 255.0) as u32;
    let mut output: u32 = alpha_u;
    output |= (line.font as u32) << 8;
    output |= (line.color as u32) << 10;
    if let Some(ocolor) = line.outline {
        output |= (ocolor as u32) << 13;
        output |= 0x00010000;
    }
    output |= ((margin * 10.0) as u32) << 24;
    output
}

// Send in gl coordinates from -1.0 to 1.0
fn encode_screen(mut x: f32, mut y: f32) -> u32 {
    // put in range [0,1] (offscreen goes out of this range)
    x = (x + 1.0) / 2.0;
    y = (y + 1.0) / 2.0;

    // Map like this, to cover some offscreen coords:
    //   -(10000/45535) --> 0
    //   0.0            --> 10000
    //   1.0            --> 55535
    //   (55535/45535)  --> 65535

    // First clamp in range, because otherwise we break the u16 format invariant.
    const MIN: f32 = -10000.0/45535.0;
    const MAX: f32 = 55535.0/45535.0;
    if x < MIN { x = MIN; }
    if x > MAX { x = MAX; }
    if y < MIN { y = MIN; }
    if y > MAX { y = MAX; }

    // Then map
    x = 45535.0 * x + 10000.0;
    y = 45535.0 * y + 10000.0;

    // Pack into u32
    (x as u32) << 16 | y as u32
}

// Send in pixel coordinates
#[inline]
fn encode_uv(mut x: f32, mut y: f32) -> u32 {
    x = x * 10.0 + 10000.0;
    y = y * 10.0 + 10000.0;

    // Pack into u32
    (x as u32) << 16 | y as u32
}
