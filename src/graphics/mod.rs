use winit::window::Window;
use wgpu::RequestAdapterOptions;

pub(crate) mod background;
pub(crate) mod text;
pub(crate) mod draw;

pub(crate) struct GraphicsContext {
    pub(crate) window: winit::window::Window,
    pub(crate) surface: wgpu::Surface,

    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) sc_desc: wgpu::SwapChainDescriptor,
    pub(crate) swap_chain: wgpu::SwapChain,

    pub(crate) size: winit::dpi::PhysicalSize<u32>,
}

impl GraphicsContext {
    pub(crate) async fn new(window: Window) -> Self {
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

pub(crate) struct FrameContext<'a> {
    pub(crate) ctx: &'a GraphicsContext,
    pub(crate) encoder: &'a mut wgpu::CommandEncoder,
    pub(crate) frame_tex: &'a wgpu::SwapChainTexture,
}