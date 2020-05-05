use futures::executor::block_on;
use winit::{
    event::*,
    event_loop::{EventLoop, ControlFlow},
    window::{WindowBuilder, Window},
};

mod renderer;
use renderer::Renderer;
mod texture;

mod camera;

pub struct Game{
    event_loop: EventLoop<()>,
    window: Window,
    renderer: Renderer,
    running: bool,
}

impl Game{
    pub fn new() -> Self{
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .build(&event_loop)
            .unwrap();

        let renderer = block_on(Renderer::new(&window));

        let running = true;
        Self{
            event_loop,
            window,
            renderer,
            running,
        }
    }

    pub fn run(self){
        let Game {
            event_loop,
            window,
            mut renderer,
            mut running,
        } = self;

        event_loop.run(move |event, _, control_flow| {
            if running{
                *control_flow = ControlFlow::Poll;
            }else{
                *control_flow = ControlFlow::Exit;
            }

            match event {
                Event::RedrawRequested(_) => {
                    renderer.update();
                    renderer.render();
                },
                Event::MainEventsCleared => {
                    window.request_redraw();
                },
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == window.id() => match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => {
                        renderer.resize(*physical_size);
                    },
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        renderer.resize(**new_inner_size);
                    },
                    WindowEvent::KeyboardInput {
                        input,
                        ..
                    } => {
                        match input {
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(keycode),
                                ..
                            } => {
                                match keycode{
                                    VirtualKeyCode::Escape => running = false,
                                    VirtualKeyCode::W => renderer.camera.eye.z += 0.5,
                                    VirtualKeyCode::S => renderer.camera.eye.z -= 0.5,
                                    VirtualKeyCode::A => renderer.camera.eye.x += 0.5,
                                    VirtualKeyCode::D => renderer.camera.eye.x -= 0.5,
                                    VirtualKeyCode::Space => renderer.camera.eye.y += 0.5,
                                    VirtualKeyCode::LShift => renderer.camera.eye.y -= 0.5,
                                    _ => (),
                                }
                            },
                            _ => {}
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        });

    }
}

fn main() {
    let game = Game::new();
    game.run();
}
