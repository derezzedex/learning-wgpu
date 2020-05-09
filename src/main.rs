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
mod timer;

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

        let mut timer = timer::Timer::new();
        let mut c_timer = std::time::Instant::now();
        let mut updates = 0;
        let mut frames = 0;

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;
            if !running{
                *control_flow = ControlFlow::Exit;
            }

            if c_timer.elapsed() >= std::time::Duration::from_secs(1){
                println!("UPS: {}/{}", updates, timer::Timer::UPS);
                println!("FPS: {}\n", frames);
                updates = 0;
                frames = 0;
                c_timer = std::time::Instant::now();
            }

            match event {
                Event::RedrawRequested(_) => {
                    renderer.render();
                    frames += 1;
                },
                Event::MainEventsCleared => {
                    timer.reset();

                    while timer.should_update(){
                        renderer.update();
                        timer.update();
                        updates += 1;
                    }

                    window.request_redraw();
                },
                Event::DeviceEvent { ref event, .. } => match event{
                    DeviceEvent::MouseMotion { delta } => {
                        let (_dx, _dy) = delta;
                    },
                    _ => (),
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
