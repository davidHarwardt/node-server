use axum::{routing::get, Router};
use layout::layout;
use maud::{html, Markup};

mod layout;

pub async fn app() -> Router {
    Router::new()
        .route("/", get(index))
}

#[axum::debug_handler]
async fn index() -> Markup {
    
    layout("hi", html! {
        "test"
    })
}

async fn connect_node(
    
) {
    
}

