
use std::sync::Arc;
use dacite::core::{Pipeline, PipelineBindPoint, PipelineLayout,
                   PrimitiveTopology, CullModeFlags, FrontFace,
                   CommandBuffer, Extent2D, Offset2D, Rect2D,
                   SpecializationInfo, SpecializationMapEntry,
                   PushConstantRange, ShaderStageFlags,
                   Viewport};
use siege_render::{Renderer, Pass, BlendMode,
                   Plugin, Params, Stats, PipelineSetup};
use ui::UiElement;
use state::State;
use errors::*;

pub struct WindowGfx {
    #[allow(dead_code)]
    pipeline_layout: PipelineLayout,
    pipeline: Pipeline,
    full_viewport: Viewport,
    state: Arc<State>
}

impl WindowGfx {
    pub fn new(renderer: &mut Renderer,
               state: Arc<State>)
               -> Result<WindowGfx>
    {
        // Specialization constant: does UI output need sRGB gamma function?
        let fragment_spec = SpecializationInfo {
            map_entries: vec![
                SpecializationMapEntry { // near depth
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
                desc_set_layouts: vec![],
                vertex_shader: Some("window.vert"),
                vertex_shader_spec: None,
                fragment_shader: Some("window.frag"),
                fragment_shader_spec: Some(fragment_spec),
                vertex_type: None,
                topology: PrimitiveTopology::TriangleList,
                cull_mode: CullModeFlags::BACK,
                front_face: FrontFace::CounterClockwise,
                test_depth: true,
                write_depth: true,
                blend: vec![BlendMode::Alpha],
                pass: Pass::Ui,
                push_constant_ranges: vec![
                    PushConstantRange { // color (4x f32)
                        stage_flags: ShaderStageFlags::VERTEX,
                        offset: 0,
                        size: ::std::mem::size_of::<[f32;4]>() as u32,
                    }
                ],
            })?;

        Ok(WindowGfx {
            pipeline_layout: pipeline_layout,
            pipeline: pipeline,
            full_viewport: renderer.get_viewport(),
            state: state,
        })
    }
}

impl Plugin for WindowGfx {
    fn record_geometry(&self, _command_buffer: CommandBuffer) {
    }

    fn record_transparent(&self, _command_buffer: CommandBuffer) {
    }

    fn record_ui(&self, command_buffer: CommandBuffer) {
        // Bind our pipeline
        command_buffer.bind_pipeline(PipelineBindPoint::Graphics, &self.pipeline);

        // NOTE: we have no vertex buffers (6 vertices are hardcoded in the shader
        // at the extreme corners for full viewport coverage)

        // NOTE we have no descriptor sets to bind either.

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

            // This pipeline does not render text, only ui windows
            let window = match element {
                &UiElement::Window(ref w) => w,
                _ => continue // we only render windows here
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

            // Push constant: color
            {
                let color = window.get_color();
                // View color as a &[u8] without copying
                let constants: &[u8] = unsafe {
                    ::std::slice::from_raw_parts(
                        (&color as *const [f32;4]) as *const u8,
                        ::std::mem::size_of::<[f32;4]>()
                    )
                };
                command_buffer.push_constants(
                    &self.pipeline_layout,
                    ShaderStageFlags::VERTEX,
                    0,
                    constants);
            }

            // draw
            command_buffer.draw(
                6, // 2 triangles
                1, // instance count
                0, // first index
                0, // first instance (base instance ID=0)
            );
        }

        // Restore the top level viewport
        command_buffer.set_viewport(0, &[self.full_viewport]);
        command_buffer.set_scissor(0, &[Rect2D {
            offset: Offset2D { x: self.full_viewport.x as i32, y: self.full_viewport.y as i32 },
            extent: Extent2D { width: self.full_viewport.width as u32, height: self.full_viewport.height as u32 },
        }]);

        self.state.ui.clear_win_dirty();
    }

    fn update(&mut self, _params: &mut Params, _stats: &Stats) -> ::siege_render::Result<bool> {

        let dirty = self.state.ui.is_win_dirty();
        if dirty {
            Ok(true) // re-record
        } else {
            Ok(false)
        }
    }

    fn gpu_update(&mut self) -> ::siege_render::Result<()> {
        Ok(())
    }

    fn rebuild(&mut self, extent: Extent2D) -> ::siege_render::Result<()> {
        self.full_viewport.width = extent.width as f32;
        self.full_viewport.height = extent.height as f32;
        Ok(())
    }
}
