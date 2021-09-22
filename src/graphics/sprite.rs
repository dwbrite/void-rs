use crate::graphics::GraphicsContext;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    Buffer, BufferUsages, Color, CommandEncoder, Face, FragmentState, LoadOp, MultisampleState,
    PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    ShaderModule, ShaderModuleDescriptor, ShaderSource, SurfaceTexture, VertexBufferLayout,
    VertexState, VertexStepMode,
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexData {
    pub(crate) position: [f32; 3],
    pub(crate) tex_coords: [f32; 2],
}

impl VertexData {
    pub fn layout<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<VertexData>() as wgpu::BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

pub struct Sprite {
    quad: Vec<VertexData>,
    shader: ShaderModule,
    pipeline: RenderPipeline,
    vertex_buf: Buffer,
}

// todo: include trait for drawable?

impl Sprite {
    pub(crate) fn new(gc: &mut GraphicsContext) -> Self {
        let tris = vec![
            VertexData {
                position: [0.0, 1.0, 0.0],
                tex_coords: [0.0, 0.0],
            },
            VertexData {
                position: [-1.0, 1.0, 0.0],
                tex_coords: [0.0, 0.0],
            },
            VertexData {
                position: [0.0, 0.0, 0.0],
                tex_coords: [0.0, 0.0],
            },
            VertexData {
                position: [0.0, 0.0, 0.0],
                tex_coords: [0.0, 0.0],
            },
            VertexData {
                position: [-1.0, 1.0, 0.0],
                tex_coords: [0.0, 0.0],
            },
            VertexData {
                position: [-1.0, 0.0, 0.0],
                tex_coords: [0.0, 0.0],
            },
        ];

        let vertex_buf = gc.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&tris), // <- insert vertex data
            usage: BufferUsages::VERTEX,
        });

        let shader = gc.device.create_shader_module(&ShaderModuleDescriptor {
            label: None,
            // TODO: put shader somewhere else
            source: ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
        });

        let layout = gc.device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = gc.device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vert_main",
                buffers: &[VertexData::layout()],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                clamp_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "main",
                targets: &[gc.config.format.into()],
            }),
        });

        Self {
            quad: tris,
            shader,
            pipeline,
            vertex_buf,
        }
    }

    pub(crate) fn draw(&mut self, encoder: &mut CommandEncoder, frame_tex: &SurfaceTexture) {
        let view = &frame_tex
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render pass descriptor"),
            color_attachments: &[RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        render_pass.draw(0..6, 0..1);
    }
}
