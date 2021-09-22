mod graphics;
mod systems;

use crate::systems::game::{GameSystem, ShouldQuit};

use winit::dpi::PhysicalSize;

use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    let event_loop = EventLoop::new();

    let title = "void";
    let window = winit::window::WindowBuilder::new()
        .with_title(title)
        .with_inner_size(PhysicalSize::new(640, 360))
        .build(&event_loop)
        .unwrap();

    let mut state = GameSystem::new(window);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if matches!(state.handle_events(&event), ShouldQuit::True) {
            *control_flow = ControlFlow::Exit
        }
    });
}

// TODO: remember that WINIT_UNIX_BACKEND=x11 is set!
