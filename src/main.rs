use tokio::net::TcpListener;
mod nodes;
mod app;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    let app = app::app().await;
    
    #[cfg(debug_assertions)]
    let app = app.layer(tower_livereload::LiveReloadLayer::new());
    
    let addr = ("0.0.0.0", 8000);
    let listener = TcpListener::bind(addr).await?;
    let addr = listener.local_addr()?;
    
    tracing::info!("starting server on {addr}");
    axum::serve(listener, app).with_graceful_shutdown(async {
        _ = tokio::signal::ctrl_c().await
    }).await?;
    Ok(())
}
