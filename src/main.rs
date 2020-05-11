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

/*
TODO:

- FPS Camera (vectors)
- FPS camera (quaternions)
- Arcball camera (quaternions)
- Chunk

- Logging
- Debug view
- Console

- Lighting
- Ambient Occlusion

*/

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

        if let Err(e) = window.set_cursor_grab(true){
            println!("Couldn't grab mouse, error: {}", e);
        }

        window.set_cursor_visible(false);

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
                println!("UPS: {} FPS:{} DT: {}", updates, frames, timer.get_delta().as_secs_f32());
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
                        renderer.update(timer.get_delta().as_secs_f32());
                        timer.update();
                        updates += 1;
                    }

                    window.request_redraw();
                },
                Event::DeviceEvent { ref event, .. } => match event{
                    DeviceEvent::MouseMotion { delta } => {
                        let (dx, dy) = delta;
                        renderer.get_camera().mouse_update(*dx as f32, *dy as f32);
                        let size = window.inner_size();
                        if let Err(e) = window.set_cursor_position(winit::dpi::PhysicalPosition::new(size.width / 2, size.height / 2)){
                            println!("Couldn't set cursor position, error: {}", e);
                        }
                    },
                    DeviceEvent::MouseWheel { delta } => {
                        let (_, dy) = match delta{
                            MouseScrollDelta::LineDelta(x, y) => (*x, *y),
                            MouseScrollDelta::PixelDelta(p) => (p.x as f32, p.y as f32),
                        };

                        renderer.get_camera().fovy -= dy.to_radians() * 2.;
                        renderer.get_camera().fovy = renderer.get_camera().fovy.max(45.).min(120.);
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
                                virtual_keycode: Some(keycode),
                                ..
                            } => {
                                let velocity = 20.;
                                match keycode{
                                    VirtualKeyCode::Escape => running = false,
                                    VirtualKeyCode::W => renderer.camera.velocity.set_z(-velocity),
                                    VirtualKeyCode::S => renderer.camera.velocity.set_z(velocity),
                                    VirtualKeyCode::A => renderer.camera.velocity.set_x(-velocity),
                                    VirtualKeyCode::D => renderer.camera.velocity.set_x(velocity),
                                    VirtualKeyCode::Space => renderer.camera.velocity.set_y(velocity),
                                    VirtualKeyCode::LShift => renderer.camera.velocity.set_y(-velocity),
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
