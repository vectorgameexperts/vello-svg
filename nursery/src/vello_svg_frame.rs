use lazy_static::lazy_static;
use log::{error, info};
use std::sync::{Arc, Mutex};
use stylist::yew::styled_component;
use vello_svg::{
    usvg::{self, TreeParsing},
    vello::{
        kurbo::{Affine, Vec2},
        peniko::Color,
        util::RenderContext,
        util::RenderSurface,
        AaConfig, AaSupport, RenderParams, RendererOptions, Scene,
        SceneBuilder,
    },
};
use winit::{
    dpi::LogicalSize, event_loop::EventLoopBuilder,
    platform::web::WindowExtWebSys, window::WindowBuilder,
};
use yew::prelude::*;

lazy_static! {
    static ref RENDER_STATE: Arc<Mutex<Option<RenderState>>> =
        Arc::new(Mutex::new(None));
}

struct RenderState {
    ctx: RenderContext,
    surface: RenderSurface,
    vello_renderer: vello_svg::vello::Renderer,
}
unsafe impl Send for RenderState {}

#[derive(Properties, PartialEq)]
pub struct PlayerProps {
    pub file: AttrValue,
}

#[styled_component]
pub fn VellottiePlayer(props: &PlayerProps) -> Html {
    let ctr_css = css! {
        display: inline-grid;
        margin: 10px;

        canvas {
            border: 1px solid black;
        }
    };

    use_effect({
        let path = props.file.to_string();
        move || {
            wasm_bindgen_futures::spawn_local(async move {
                // Init GPU Canvas, if not initialized.
                init_state().await;
                // Load file
                info!("loading {path}...");
                let contents =
                    reqwest::get(path).await.unwrap().text().await.unwrap();
                info!("retrieved contents, parsing...");
                let svg =
                    usvg::Tree::from_str(&contents, &usvg::Options::default());
                match svg {
                    Ok(mut svg) => {
                        info!("read file successfully");
                        render(&mut svg);
                    }
                    Err(e) => error!("Bad svg: {e}"),
                }
            });
        }
    });
    html! {
        <div id="canvas_holster" class={ctr_css}>
            <h1>{"Vello SVG"}</h1>
        </div>
    }
}

async fn init_state() {
    if (*RENDER_STATE).lock().unwrap().is_some() {
        return;
    }
    // Create the GPU Canvas
    info!("creating GPU canvas...");
    let event_loop = EventLoopBuilder::new().build();
    #[allow(unused_mut)]
    let mut ctx = RenderContext::new().unwrap();
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(400, 400))
        .with_resizable(false)
        .build(&event_loop)
        .unwrap();
    // On wasm, append the canvas to the document body
    let canvas = window.canvas();
    let size = window.inner_size();
    canvas.set_width(size.width);
    canvas.set_height(size.height);
    web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| doc.get_element_by_id("canvas_holster"))
        .and_then(|parent| parent.append_child(canvas.as_ref()).ok())
        .expect("couldn't append canvas to document");
    _ = web_sys::HtmlElement::from(canvas).focus();

    let size = window.inner_size();
    let surface = ctx
        .create_surface(&window, size.width, size.height)
        .await
        .unwrap();
    let device_handle = &ctx.devices[surface.dev_id];
    let vello_renderer = vello_svg::vello::Renderer::new(
        &device_handle.device,
        RendererOptions {
            surface_format: Some(surface.format),
            timestamp_period: 0.0,
            use_cpu: false,
            antialiasing_support: AaSupport::area_only(),
        },
    )
    .unwrap();
    (*RENDER_STATE).lock().unwrap().replace(RenderState {
        vello_renderer,
        ctx,
        surface,
    });
    info!("GPU canvas created");
}

fn render(svg: &mut usvg::Tree) {
    let mut state_lock = (*RENDER_STATE).lock().unwrap();
    let state: &mut RenderState = state_lock.as_mut().unwrap();
    let device_handle = &state.ctx.devices[state.surface.dev_id];

    let mut scene = Scene::new();
    let width = state.surface.config.width;
    let height = state.surface.config.height;
    let scale = (state.surface.config.width as f64 / svg.size.width())
        .min(state.surface.config.height as f64 / svg.size.height());
    // Center the object in the canvas and scale it up like SVG does on web
    let transform = if svg.size.width() < svg.size.height() {
        let dx = (state.surface.config.width as f64 - svg.size.width() * scale)
            .abs()
            / 2.0;
        Affine::scale(scale).then_translate(Vec2::new(dx, 0.0))
    } else {
        let dy = (state.surface.config.height as f64
            - svg.size.height() * scale)
            .abs()
            / 2.0;
        Affine::scale(scale).then_translate(Vec2::new(0.0, dy))
    };

    let mut builder = SceneBuilder::for_scene(&mut scene);
    vello_svg::render_tree(&mut builder, svg, Some(transform));

    let surface_texture = state
        .surface
        .surface
        .get_current_texture()
        .expect("failed to get surface texture");
    state
        .vello_renderer
        .render_to_surface(
            &device_handle.device,
            &device_handle.queue,
            &scene,
            &surface_texture,
            &RenderParams {
                base_color: Color::WHITE,
                width,
                height,
                antialiasing_method: AaConfig::Area,
            },
        )
        .expect("failed to render to surface");
    surface_texture.present();
}
