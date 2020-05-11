use crate::camera::{Camera};
// use cgmath::{Vec3, Mat4, SquareMatrix};
use glam::{Vec3, Mat4};
use winit::window::Window;
use std::path::Path;
use crate::texture::Texture;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Vertex{
    position: [f32; 3],
    tex_coord: [f32; 2],
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

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Uniforms{
    model: Mat4,
    view: Mat4,
    projection: Mat4,
}

impl Uniforms{
    pub fn new() -> Self{
        Self{
            model: Mat4::from_translation(Vec3::new(0., 0., 0.1)),
            view: Mat4::identity(),
            projection: Mat4::identity(),
        }
    }

    pub fn update_view(&mut self, camera: &Camera){
        self.view = camera.get_view();
        self.projection = camera.get_projection();
    }

    pub fn update_model(&mut self, model: Vec3){
        self.model = Mat4::from_translation(model);
    }
}

unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

fn glsl_to_spirv(path: &Path)-> (std::vec::Vec<u32>, std::vec::Vec<u32>) {
    println!("Loading shaders at: {:?}", &path);
    let vertex_src = std::fs::read_to_string(path.join("shader.vert")).expect("Couldn't load vertex shader");
    let fragment_src = std::fs::read_to_string(path.join("shader.frag")).expect("Couldn't load fragment shader");

    // let vertex_spirv = glsl_to_spirv::compile(&vertex_src, glsl_to_spirv::ShaderType::Vertex).expect("Couldn't convert vertex shader");
    // let fragment_spirv = glsl_to_spirv::compile(&fragment_src, glsl_to_spirv::ShaderType::Fragment).expect("Couldn't convert fragment shader");
    let mut compiler = shaderc::Compiler::new().expect("Couldn't create shaderc compiler");
    let vertex_spirv = compiler.compile_into_spirv(
        &vertex_src, shaderc::ShaderKind::Vertex,
        "shader.vert", "main", None).expect("Couldn't convert GLSL to SPIR-V");

    let fragment_spirv = compiler.compile_into_spirv(
        &fragment_src, shaderc::ShaderKind::Fragment,
        "shader.frag", "main", None).expect("Couldn't convert GLSL to SPIR-V");

    use std::io::Cursor;
    let vertex = wgpu::read_spirv(Cursor::new(vertex_spirv.as_binary_u8())).expect("Couldn't read vertex SPIR-V");
    let fragment = wgpu::read_spirv(Cursor::new(fragment_spirv.as_binary_u8())).expect("Couldn't read fragment SPIR-V");

    (vertex, fragment)
}

#[allow(dead_code)]
pub struct Renderer {
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pub sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,

    img_tex: Texture,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,

    depth_tex: Texture,
    opaque_bind_group: wgpu::BindGroup,
    opaque_pipeline: wgpu::RenderPipeline,

    accum_tex: Texture,
    revealage_tex: Texture,
    transparency_bind_group: wgpu::BindGroup,
    transparency_pipeline: wgpu::RenderPipeline,

    screen_bind_group: wgpu::BindGroup,
    screen_pipeline: wgpu::RenderPipeline,

    pub camera: Camera,
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

        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor{
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

        let camera = Camera {
            eye: (0., 0., 2.).into(),
            target: (0., 0., 0.1).into(),
            up: Vec3::unit_y(),
            aspect: sc_desc.width as f32 / sc_desc.height as f32,
            fovy: 90f32.to_radians(),
            near: 0.1,
            far: 100.,
            velocity: Vec3::new(0., 0., 0.),
            yaw: 0.,
            pitch: 0.,
        };

        // ***************** MVP UBO LAYOUT *****************
        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("uniform bind group layout"),
            bindings: &[
            wgpu::BindGroupLayoutEntry{
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::UniformBuffer{
                    dynamic: false,
                },
            }
            ],
        });

        let mut uniforms = Uniforms::new();
        uniforms.update_view(&camera);

        let uniform_buffer = device.create_buffer_with_data(
            bytemuck::cast_slice(&[uniforms]),
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        );

        // ***************** OPAQUE PIPELINE *****************
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
            label: Some("uniform bing group"),
            layout: &uniform_bind_group_layout,
            bindings: &[
            wgpu::Binding{
                binding: 0,
                resource: wgpu::BindingResource::Buffer{
                    buffer: &uniform_buffer,
                    range: 0..std::mem::size_of_val(&uniforms) as wgpu::BufferAddress,
                }
            }
            ],
        });

        let depth_tex = Texture::create_depth(&device, &sc_desc, "depth texture");
        let opaque_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
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

        let img_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/res/img"));
        let img_bytes = std::fs::read(img_path.join("glass.png")).expect("Couldn't read image");
        let (img_tex, cmd_buffer) = Texture::from_bytes(&device, &img_bytes).expect("Couldn't load texture");
        queue.submit(&[cmd_buffer]);

        // let opaque_out = Texture::create_empty(&device, &sc_desc, wgpu::TextureFormat::Rgba16Float, "opaque out tex");

        let opaque_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
            layout: &opaque_bind_group_layout,
            label: Some("img tex bind group"),
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&img_tex.view),
                },
                wgpu::Binding{
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&img_tex.sampler),
                },
            ],
        });

        let shader_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/opaque"));
        let (vs, fs) = glsl_to_spirv(shader_path);
        let vs_module = device.create_shader_module(&vs);
        let fs_module = device.create_shader_module(&fs);

        let opaque_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor{
            bind_group_layouts: &[
                &opaque_bind_group_layout,
                &uniform_bind_group_layout,
            ],
        });

        let opaque_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
            layout: &opaque_pipeline_layout,
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
                    color_blend: wgpu::BlendDescriptor {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha_blend: wgpu::BlendDescriptor {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    write_mask: wgpu::ColorWrite::ALL,
                },
            ],
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor{
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
                stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
                stencil_read_mask: 0,
                stencil_write_mask: 0,
            }),
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

        // ***************** TRANSPARENCY PIPELINE *****************
        let transparency_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("transparency bind group layout"),
            bindings: &[
                wgpu::BindGroupLayoutEntry{
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture{
                        multisampled: false,
                        dimension: wgpu::TextureViewDimension::D2,
                        component_type: wgpu::TextureComponentType::Float,
                    },
                },
                wgpu::BindGroupLayoutEntry{
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture{
                        multisampled: false,
                        dimension: wgpu::TextureViewDimension::D2,
                        component_type: wgpu::TextureComponentType::Float,
                    },
                },
            ],
        });

        let accum_tex = Texture::create_empty(&device, &sc_desc, wgpu::TextureFormat::Rgba16Float, "accum tex");
        let revealage_tex = Texture::create_empty(&device, &sc_desc, wgpu::TextureFormat::R8Unorm, "revealage tex");

        //for now, ignore this
        let transparency_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
            layout: &transparency_bind_group_layout,
            label: Some("tex bind group"),
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&accum_tex.view),
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&revealage_tex.view),
                },
            ],
        });

        let shader_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/transparency"));
        let (vs, fs) = glsl_to_spirv(shader_path);
        let vs_module = device.create_shader_module(&vs);
        let fs_module = device.create_shader_module(&fs);

        let transparency_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor{
            bind_group_layouts: &[
                &opaque_bind_group_layout, //using opaque
                &uniform_bind_group_layout,
            ],
        });

        let transparency_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
            layout: &transparency_pipeline_layout,
            vertex_stage: wgpu::ProgrammableStageDescriptor{
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
                wgpu::ColorStateDescriptor{
                    format: wgpu::TextureFormat::Rgba16Float,
                    color_blend: wgpu::BlendDescriptor{
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha_blend: wgpu::BlendDescriptor{
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    write_mask: wgpu::ColorWrite::ALL,
                },
                wgpu::ColorStateDescriptor{
                    format: wgpu::TextureFormat::R8Unorm,
                    color_blend: wgpu::BlendDescriptor{
                        src_factor: wgpu::BlendFactor::Zero,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcColor,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha_blend: wgpu::BlendDescriptor{
                        src_factor: wgpu::BlendFactor::Zero,
                        dst_factor: wgpu::BlendFactor::Zero,
                        operation: wgpu::BlendOperation::Add,
                    },
                    write_mask: wgpu::ColorWrite::ALL,
                },
            ],
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor{
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Less,
                stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
                stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
                stencil_read_mask: 0,
                stencil_write_mask: 0,
            }),
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

        // ***************** SCREEN PIPELINE *****************

        let screen_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("screen bind group layout"),
            bindings: &[
                wgpu::BindGroupLayoutEntry{
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture{
                        multisampled: false,
                        dimension: wgpu::TextureViewDimension::D2,
                        component_type: wgpu::TextureComponentType::Float,
                    },
                },
                wgpu::BindGroupLayoutEntry{
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture{
                        multisampled: false,
                        dimension: wgpu::TextureViewDimension::D2,
                        component_type: wgpu::TextureComponentType::Float,
                    },
                },
            ],
        });

        let screen_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
            layout: &screen_bind_group_layout,
            label: Some("tex bind group"),
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&accum_tex.view),
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&revealage_tex.view),
                },
            ],
        });

        let shader_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/screen"));
        let (vs, fs) = glsl_to_spirv(shader_path);
        let vs_module = device.create_shader_module(&vs);
        let fs_module = device.create_shader_module(&fs);

        let screen_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor{
            bind_group_layouts: &[
                &screen_bind_group_layout,
            ],
        });

        let screen_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
            layout: &screen_pipeline_layout,
            vertex_stage: wgpu::ProgrammableStageDescriptor{
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
                wgpu::ColorStateDescriptor{
                    format: sc_desc.format,
                    color_blend: wgpu::BlendDescriptor{
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha_blend: wgpu::BlendDescriptor{
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    write_mask: wgpu::ColorWrite::ALL,
                },
            ],
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            depth_stencil_state: None,
            vertex_state: wgpu::VertexStateDescriptor{
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        // ***************** BUFFERS *****************
        let vertices = &[

            //Quad2
            Vertex { position: [ 0.5, -0.5, 1.], tex_coord: [0., 1.], },
            Vertex { position: [ 1.5, -0.5, 1.], tex_coord: [1., 1.], },
            Vertex { position: [ 0.5,  0.5, 1.], tex_coord: [0., 0.], },
            Vertex { position: [ 1.5,  0.5, 1.], tex_coord: [1., 0.], },

            //Quad1
            Vertex { position: [-0.5, -0.5, 0.], tex_coord: [0., 1.], },
            Vertex { position: [ 0.5, -0.5, 0.], tex_coord: [1., 1.], },
            Vertex { position: [-0.5,  0.5, 0.], tex_coord: [0., 0.], },
            Vertex { position: [ 0.5,  0.5, 0.], tex_coord: [1., 0.], },
        ];
        let indices: &[u16] = &[
            0, 1, 2,
            2, 1, 3,

            4, 5, 6,
            6, 5, 7
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

            img_tex,
            vertex_buffer,
            index_buffer,

            uniforms,
            uniform_buffer,
            uniform_bind_group,

            depth_tex,
            opaque_bind_group,
            opaque_pipeline,

            accum_tex,
            revealage_tex,
            transparency_bind_group,
            transparency_pipeline,

            screen_bind_group,
            screen_pipeline,

            camera,
            size,
            indices_len,
        }
    }

    pub fn get_camera(&mut self) -> &mut Camera{
        &mut self.camera
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.depth_tex = Texture::create_depth(&self.device, &self.sc_desc, "depth texture");
        self.accum_tex = Texture::create_empty(&self.device, &self.sc_desc, wgpu::TextureFormat::Rgba16Float, "accum tex");
        self.revealage_tex = Texture::create_empty(&self.device, &self.sc_desc, wgpu::TextureFormat::R8Unorm, "revealage tex");
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    pub fn update(&mut self, dt: f32){
        self.camera.update(dt);
        self.uniforms.update_view(&self.camera);

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("update encoder"),
        });

        let staging_buffer = self.device.create_buffer_with_data(
            bytemuck::cast_slice(&[self.uniforms]),
            wgpu::BufferUsage::COPY_SRC,
        );

        encoder.copy_buffer_to_buffer(&staging_buffer, 0, &self.uniform_buffer, 0, std::mem::size_of::<Uniforms>() as wgpu::BufferAddress);

        self.queue.submit(&[encoder.finish()]);
    }

    fn clear(&mut self, encoder: &mut wgpu::CommandEncoder, output_view: &wgpu::TextureView){
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor{
            color_attachments: &[
                wgpu::RenderPassColorAttachmentDescriptor{
                    attachment: output_view,
                    resolve_target: None,
                    load_op: wgpu::LoadOp::Clear,
                    store_op: wgpu::StoreOp::Store,
                    clear_color: wgpu::Color{
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    },
                },
                wgpu::RenderPassColorAttachmentDescriptor{
                    attachment: &self.accum_tex.view,
                    resolve_target: None,
                    load_op: wgpu::LoadOp::Clear,
                    store_op: wgpu::StoreOp::Store,
                    clear_color: wgpu::Color{
                        r: 0.,
                        g: 0.,
                        b: 0.,
                        a: 0.,
                    },
                },
                wgpu::RenderPassColorAttachmentDescriptor{
                    attachment: &self.revealage_tex.view,
                    resolve_target: None,
                    load_op: wgpu::LoadOp::Clear,
                    store_op: wgpu::StoreOp::Store,
                    clear_color: wgpu::Color{
                        r: 1.,
                        g: 0.,
                        b: 0.,
                        a: 0.,
                    },
                }
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor{
                attachment: &self.depth_tex.view,
                depth_load_op: wgpu::LoadOp::Clear,
                depth_store_op: wgpu::StoreOp::Store,
                clear_depth: 1.0,
                stencil_load_op: wgpu::LoadOp::Clear,
                stencil_store_op: wgpu::StoreOp::Store,
                clear_stencil: 0,
            }),
        });
    }

    pub fn render(&mut self) {
        let frame = self.swap_chain.get_next_texture().expect("Couldn't get texture");
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor{
            label: Some("Renderer encoder"),
        });

        let models = [
            Vec3::new(-2., 0., -2.),
            Vec3::new(0., 0., 0.),
            Vec3::new(2., 0., 2.),
        ];

        self.clear(&mut encoder, &frame.view);

        for model in models.iter().rev(){
            self.uniforms.update_model(*model);
            let staging_buffer = self.device.create_buffer_with_data(
                bytemuck::cast_slice(&[self.uniforms]),
                wgpu::BufferUsage::COPY_SRC,
            );
            encoder.copy_buffer_to_buffer(&staging_buffer, 0, &self.uniform_buffer, 0, std::mem::size_of::<Uniforms>() as wgpu::BufferAddress);

            {
                let mut transparency_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor{
                    color_attachments: &[
                        wgpu::RenderPassColorAttachmentDescriptor{
                            attachment: &self.accum_tex.view,
                            resolve_target: None,
                            load_op: wgpu::LoadOp::Load,
                            store_op: wgpu::StoreOp::Store,
                            clear_color: wgpu::Color{
                                r: 0.,
                                g: 0.,
                                b: 0.,
                                a: 0.,
                            },
                        },
                        wgpu::RenderPassColorAttachmentDescriptor{
                            attachment: &self.revealage_tex.view,
                            resolve_target: None,
                            load_op: wgpu::LoadOp::Load,
                            store_op: wgpu::StoreOp::Store,
                            clear_color: wgpu::Color{
                                r: 1.,
                                g: 0.,
                                b: 0.,
                                a: 0.,
                            },
                        }
                    ],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor{
                        attachment: &self.depth_tex.view,
                        depth_load_op: wgpu::LoadOp::Load,
                        depth_store_op: wgpu::StoreOp::Store,
                        clear_depth: 0.,
                        stencil_load_op: wgpu::LoadOp::Load,
                        stencil_store_op: wgpu::StoreOp::Store,
                        clear_stencil: 0,
                    }),
                });

                transparency_pass.set_pipeline(&self.transparency_pipeline);

                transparency_pass.set_bind_group(0, &self.opaque_bind_group, &[]);
                transparency_pass.set_bind_group(1, &self.uniform_bind_group, &[]);

                transparency_pass.set_vertex_buffer(0, &self.vertex_buffer, 0, 0);
                transparency_pass.set_index_buffer(&self.index_buffer, 0, 0);

                transparency_pass.draw_indexed(0..self.indices_len, 0, 0..1);
            }


        }
        {
            let mut screen_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor{
                color_attachments: &[
                wgpu::RenderPassColorAttachmentDescriptor{
                    attachment: &frame.view,
                    resolve_target: None,
                    load_op: wgpu::LoadOp::Load,
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

            screen_pass.set_pipeline(&self.screen_pipeline);
            screen_pass.set_bind_group(0, &self.screen_bind_group, &[]);

            screen_pass.draw(0..6, 0..1);
        }

        self.queue.submit(&[
            encoder.finish()
        ]);
    }
}
