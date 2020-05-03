use winit::window::Window;
use std::path::Path;
use crate::texture::Texture;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Vertex{
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex{
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a>{
        use std::mem;
        wgpu::VertexBufferDescriptor{
            stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float3,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float2,
                },
            ]
        }
    }
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

fn load_spirv(path: &Path)-> (std::vec::Vec<u32>, std::vec::Vec<u32>) {
    println!("Loading shaders at: {:?}", &path);
    let vertex_src = std::fs::read_to_string(path.join("shader.vert")).expect("Couldn't load vertex shader");
    let fragment_src = std::fs::read_to_string(path.join("shader.frag")).expect("Couldn't load fragment shader");

    let vertex_spirv = glsl_to_spirv::compile(&vertex_src, glsl_to_spirv::ShaderType::Vertex).expect("Couldn't convert vertex shader");
    let fragment_spirv = glsl_to_spirv::compile(&fragment_src, glsl_to_spirv::ShaderType::Fragment).expect("Couldn't convert fragment shader");

    let vertex = wgpu::read_spirv(vertex_spirv).expect("Couldn't read vertex SPIRV");
    let fragment = wgpu::read_spirv(fragment_spirv).expect("Couldn't read fragment SPIRV");

    (vertex, fragment)
}

#[allow(dead_code)]
pub struct Renderer {
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    tex: Texture,
    tex_bind_group: wgpu::BindGroup,

    render_pipeline: wgpu::RenderPipeline,
    size: winit::dpi::PhysicalSize<u32>,
    indices_len: u32,
}

#[allow(dead_code)]
impl Renderer {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();
        let surface = wgpu::Surface::create(window);

        let adapter = wgpu::Adapter::request(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            },
            wgpu::BackendBit::PRIMARY,
        ).await.expect("Couldn't request the Adapter");

        let (device, mut queue) = adapter.request_device(&wgpu::DeviceDescriptor{
            extensions: wgpu::Extensions{
                anisotropic_filtering: false,
            },
            limits: Default::default(),
        }).await;

        let sc_desc = wgpu::SwapChainDescriptor{
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let img_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/res/img"));
        let img_bytes = std::fs::read(img_path.join("tree.png")).expect("Couldn't read image");
        let (tex, cmd_buffer) = Texture::from_bytes(&device, &img_bytes).expect("Couldn't load texture");
        queue.submit(&[cmd_buffer]);

        let tex_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("texture bind group layout"),
            bindings: &[
                wgpu::BindGroupLayoutEntry{
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture{
                        multisampled: false,
                        dimension: wgpu::TextureViewDimension::D2,
                        component_type: wgpu::TextureComponentType::Uint,
                    },
                },
                wgpu::BindGroupLayoutEntry{
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler{
                        comparison: false,
                    },
                },
            ],
        });

        let tex_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
            layout: &tex_bind_group_layout,
            label: Some("tex bind group"),
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&tex.view),
                },
                wgpu::Binding{
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&tex.sampler),
                },
            ],
        });

        let shader_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders"));
        let (vs, fs) = load_spirv(shader_path);
        let vs_module = device.create_shader_module(&vs);
        let fs_module = device.create_shader_module(&fs);

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor{
            bind_group_layouts: &[&tex_bind_group_layout],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
            layout: &render_pipeline_layout,
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor{
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor{
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::Back,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            color_states: &[
                wgpu::ColorStateDescriptor {
                    format: sc_desc.format,
                    color_blend: wgpu::BlendDescriptor::REPLACE,
                    alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                },
            ],
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            depth_stencil_state: None,
            vertex_state: wgpu::VertexStateDescriptor{
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[
                    Vertex::desc(),
                ],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        let vertices = &[
            Vertex { position: [-0.0868241, 0.49240386, 0.0], tex_coords: [0.4131759, 0.00759614], }, // A
            Vertex { position: [-0.49513406, 0.06958647, 0.0], tex_coords: [0.0048659444, 0.43041354], }, // B
            Vertex { position: [-0.21918549, -0.44939706, 0.0], tex_coords: [0.28081453, 0.949397057], }, // C
            Vertex { position: [0.35966998, -0.3473291, 0.0], tex_coords: [0.85967, 0.84732911], }, // D
            Vertex { position: [0.44147372, 0.2347359, 0.0], tex_coords: [0.9414737, 0.2652641], }, // E
        ];
        let indices: &[u16] = &[
            0, 1, 4,
            1, 2, 4,
            2, 3, 4,
        ];
        let indices_len = indices.len() as u32;

        let vertex_buffer = device.create_buffer_with_data(
            bytemuck::cast_slice(vertices),
            wgpu::BufferUsage::VERTEX,
        );

        let index_buffer = device.create_buffer_with_data(
            bytemuck::cast_slice(indices),
            wgpu::BufferUsage::INDEX,
        );

        Self{
            surface,
            adapter,
            device,
            queue,
            sc_desc,
            swap_chain,
            vertex_buffer,
            index_buffer,
            tex,
            tex_bind_group,
            render_pipeline,
            size,
            indices_len,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    pub fn render(&mut self) {
        let frame = self.swap_chain.get_next_texture().expect("Couldn't get texture");
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor{
            label: Some("Renderer encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor{
                color_attachments: &[
                    wgpu::RenderPassColorAttachmentDescriptor{
                        attachment: &frame.view,
                        resolve_target: None,
                        load_op: wgpu::LoadOp::Clear,
                        store_op: wgpu::StoreOp::Store,
                        clear_color: wgpu::Color{
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        },
                    }
                ],
                depth_stencil_attachment: None,
            });

            render_pass.set_bind_group(0, &self.tex_bind_group, &[]);
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, &self.vertex_buffer, 0, 0);
            render_pass.set_index_buffer(&self.index_buffer, 0, 0);
            render_pass.draw_indexed(0..self.indices_len, 0, 0..1);
        }

        self.queue.submit(&[
            encoder.finish()
        ]);
    }
}
