use include_tailwind::load_tailwind;
use maud::{html, Markup, DOCTYPE};

#[cfg(not(debug_assertions))]
fn debug_banner() -> Markup { html!() }

#[cfg(debug_assertions)]
fn debug_banner() -> Markup {
    html! {
        div."bg-red-300 absolute text-center"
            ."px-12 bottom-0 right-0"
        { "! RUNNING IN DEBUG MODE !" }
    }
}

pub fn layout(title: &str, body: Markup) -> Markup {
    html! {
        (DOCTYPE);
        head {
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1.0";
            meta name="X-UA-Compatible" content="id=edge";

            title { (title) };
            (load_tailwind!());
            // link rel="icon" type="image/svg+xml" href="logo.svg";
            // script src="htmx.js" {};
            // script src="htmx-sse.js" {};
            // (htmx_dbg())
        };
        body {
            (debug_banner());
            (body);
        };
    }
    
}

