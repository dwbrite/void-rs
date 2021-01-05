use winit::window::Window;
use wgpu::RequestAdapterOptions;

pub mod background;
pub mod text;
pub mod draw;

pub struct GraphicsContext {
    pub window: winit::window::Window,
    pub surface: wgpu::Surface,

    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub sc_desc: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,

    pub size: winit::dpi::PhysicalSize<u32>,
}

impl GraphicsContext {
    pub async fn new(window: Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(&window) };
        let adapter = instance.request_adapter(
            &RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            },
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: Default::default(),
                limits: Default::default(),
                shader_validation: false,
            },
            None,
        ).await.unwrap();

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);


        GraphicsContext {
            window,
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            size,
        }
    }
}

pub struct FrameContext<'a> {
    pub ctx: &'a GraphicsContext,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub frame_tex: &'a wgpu::SwapChainTexture,
}