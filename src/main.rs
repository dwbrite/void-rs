use std::fs::File;
use std::io::BufReader;
use rodio::Source;

use winit::event_loop::{EventLoop, ControlFlow};
use winit::event::{Event, WindowEvent, KeyboardInput, ElementState, VirtualKeyCode};

use winit::dpi::PhysicalSize;
mod graphics;

fn main() {
    std::thread::spawn(|| {
        let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();

        // Load a sound from a file, using a path relative to Cargo.toml
        let file = File::open("res/lowtide.ogg").unwrap();
        let source = rodio::Decoder::new(BufReader::new(file)).unwrap();
        stream_handle.play_raw(source.speed(0.5).convert_samples().repeat_infinite()).unwrap();

        loop { std::thread::sleep(std::time::Duration::from_millis(16)); }
    });


    let event_loop = EventLoop::new();
    let title = "void";
    let window = winit::window::WindowBuilder::new()
        .with_title(title)
        .with_inner_size(PhysicalSize::new(512, 288))
        .with_resizable(false)
        .build(&event_loop)
        .unwrap();

    ///////////////////////////////////////////////////////////////////////////////////////////////

    use futures::executor::block_on;

    let mut state = block_on(graphics::GameState::new(&window));
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::MainEventsCleared => {
                state.update();
                state.render();
            },
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if !state.input(event) {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::KeyboardInput { input, .. } => match input {
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            } => {
                                *control_flow = ControlFlow::Exit;
                            },
                            _ => {}
                        },
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                            state.render();
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            state.resize(**new_inner_size);
                            state.render();
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    });
}