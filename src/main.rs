use winit::event_loop::EventLoop;

use freebricks::application::application::Application;

fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = Application::new(&event_loop);
    let _ = event_loop.run_app(&mut app);

    Ok(())
}

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    run().unwrap();
}

/*

use freebricks::render_kit::application::App;

fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::with_user_event().build()?;
    let mut app= App::new(&event_loop);
    event_loop.run_app(&mut app);

    Ok(())
}

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();


    info!("Hello world");
    run().unwrap();
}
*/
