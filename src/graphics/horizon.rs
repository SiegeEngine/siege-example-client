
use dacite::core::{Pipeline, PipelineBindPoint,
                   CommandBuffer, PipelineLayout,
                   PrimitiveTopology, CullModeFlags, FrontFace,
                   DescriptorSetLayout, DescriptorSet, Extent2D};
use siege_render::{Renderer, Pass, BlendMode, Plugin,
                   Params, Stats, PipelineSetup};
use errors::*;

pub struct HorizonGfx {
    pipeline: Pipeline,
    pipeline_layout: PipelineLayout,
    camera_desc_set: DescriptorSet,
}

impl HorizonGfx {
    pub fn new(renderer: &mut Renderer,
               camera_desc_set_layout: DescriptorSetLayout,
               camera_desc_set: DescriptorSet)
               -> Result<HorizonGfx>
    {
        let (pipeline_layout, pipeline) = renderer.create_pipeline(
            PipelineSetup {
                desc_set_layouts: vec![camera_desc_set_layout],
                vertex_shader: Some("horizon.vert"),
                vertex_shader_spec: None,
                fragment_shader: Some("horizon.frag"),
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

        Ok(HorizonGfx {
            pipeline: pipeline,
            pipeline_layout: pipeline_layout,
            camera_desc_set: camera_desc_set
        })
    }
}

impl Plugin for HorizonGfx {
    fn record_geometry(&self, command_buffer: CommandBuffer) {
        // Bind our pipeline
        command_buffer.bind_pipeline(PipelineBindPoint::Graphics, &self.pipeline);

        // SET 0 = camera descriptor set (which is shared),
        command_buffer.bind_descriptor_sets(
            PipelineBindPoint::Graphics,
            &self.pipeline_layout,
            0, // first_set
            &[self.camera_desc_set.clone()],
            None,
        );

        // Draw horizon
        command_buffer.draw(
            4, // 4 vertices, 2 triangles
            1, // instance count (not instancing, so just 1)
            0, // first vertex
            0, // first instance (base instance ID=0)
        );
    }

    fn record_transparent(&self, _command_buffer: CommandBuffer) {
    }

    fn record_ui(&self, _command_buffer: CommandBuffer) {
    }

    fn update(&mut self, _params: &mut Params, _stats: &Stats) -> ::siege_render::Result<bool> {
        Ok(false)
    }

    fn gpu_update(&mut self) -> ::siege_render::Result<()> {
        Ok(())
    }

    fn rebuild(&mut self, _extent: Extent2D) -> ::siege_render::Result<()> {
        Ok(())
    }
}
