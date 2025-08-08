use winit::{
    application::ApplicationHandler, event::*, event_loop::{ActiveEventLoop, EventLoop}, window::{Window, WindowId}
};
use std::sync::Arc;

use crate::{common::game::*};


pub struct Application {
    game: Option<Game>,
    window: Option<Arc<Window>>,
}

impl Application {
    pub fn new(_event_loop: &EventLoop<Game>) -> Self {
        Self {
            game: None, 
            window: None
        }
    }
}

impl ApplicationHandler<Game> for Application { 
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();
        window_attributes.title = "FreeBricks demo".to_string();

        let window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .unwrap()
        );

        self.window = Some(window.clone());
        self.game = Some(pollster::block_on(Game::new(window)).unwrap());
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: Game) {
        self.game = Some(event);
    }

    fn window_event(
        &mut self, 
        event_loop: &ActiveEventLoop, 
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let game = match &mut self.game {
            Some(game) => game, 
            None => return 
        };

        #[allow(unused)]
        let window = match &mut self.window {
            Some(window) => window, 
            None => return 
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                let _ = game.resize(size);
            },
            WindowEvent::RedrawRequested => {
                game.update();
            },
            _ => {}
        }


    }

}