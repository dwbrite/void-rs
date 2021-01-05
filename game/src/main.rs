use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

use crate::systems::game::GameSystem;
use winit::dpi::PhysicalSize;

mod dialogue;
mod graphics;
mod resources;
mod systems;

fn main() {
    let event_loop = EventLoop::new();

    let title = "void";
    let window = winit::window::WindowBuilder::new()
        .with_title(title)
        .with_inner_size(PhysicalSize::new(512, 288))
        .with_resizable(false)
        .build(&event_loop)
        .unwrap();

    use futures::executor::block_on;

    let mut state = block_on(GameSystem::new(window));

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::MainEventsCleared => {
                // simple fixed gameloop, we're going to let vsync handle the framerate
                state.update();
                state.draw();
                state.render();

                // TODO: sleep if frames are too fast
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.gc.window.id() => {
                if !state.handle_input_events(event) {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    });
}
