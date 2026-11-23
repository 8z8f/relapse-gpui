mod app;
mod completions;
mod profiles;
mod protocol;
mod scripts;
mod theme;
mod ui;

use std::{sync::mpsc, time::Duration};

use gpui_kit::{
    App, AppContext, AssetSource, Bounds, Result, SharedString, TitlebarOptions, WindowBounds,
    WindowOptions, px, size,
};

use app::RelapseApp;

struct AppAssets;

macro_rules! bundled_icon {
    ($name:literal) => {
        (
            concat!("icons/", $name, ".svg"),
            include_bytes!(concat!("../assets/icons/", $name, ".svg")) as &[u8],
        )
    };
}

const BUNDLED_ICONS: &[(&str, &[u8])] = &[
    bundled_icon!("braces"),
    bundled_icon!("check"),
    bundled_icon!("circle-user"),
    bundled_icon!("code"),
    bundled_icon!("download"),
    bundled_icon!("eye"),
    bundled_icon!("file-code"),
    bundled_icon!("folder-open"),
    bundled_icon!("search"),
    bundled_icon!("shield-alert"),
    bundled_icon!("star"),
    bundled_icon!("minus"),
    bundled_icon!("moon"),
    bundled_icon!("pencil"),
    bundled_icon!("play"),
    bundled_icon!("plus"),
    bundled_icon!("rocket"),
    bundled_icon!("rotate-cw"),
    bundled_icon!("save"),
    bundled_icon!("settings"),
    bundled_icon!("square"),
    bundled_icon!("sun"),
    bundled_icon!("terminal"),
    bundled_icon!("trash"),
    bundled_icon!("users"),
    bundled_icon!("x"),
];

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        if path == "relapse.svg" {
            return Ok(Some(std::borrow::Cow::Borrowed(include_bytes!(
                "../assets/relapse.svg"
            ))));
        }
        if path == "relapse-dark.svg" {
            return Ok(Some(std::borrow::Cow::Borrowed(include_bytes!(
                "../assets/relapse-dark.svg"
            ))));
        }
        if let Some((_, bytes)) = BUNDLED_ICONS.iter().find(|(name, _)| *name == path) {
            return Ok(Some(std::borrow::Cow::Borrowed(bytes)));
        }
        gpui_kit::assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        gpui_kit::assets::Assets.list(path)
    }
}

fn main() {
    let (command_tx, command_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();
    protocol::start(command_rx, event_tx);
    gpui_kit::application()
        .with_assets(AppAssets)
        .run(move |cx: &mut App| {
            gpui_kit::init(cx);
            cx.bind_keys([gpui_kit::KeyBinding::new(
                "tab",
                app::AcceptCompletion,
                Some("Input"),
            )]);
            theme::set(false, cx);
            let tx = command_tx.clone();
            let events = event_rx;
            let (_, view) = gpui_kit::open_window(
                WindowOptions {
                    titlebar: Some(TitlebarOptions {
                        title: Some("Relapse".into()),
                        appears_transparent: true,
                        traffic_light_position: None,
                    }),
                    app_owns_titlebar_drag: true,
                    window_min_size: Some(size(px(432.), px(276.))),
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(796.), px(397.)),
                        cx,
                    ))),
                    ..Default::default()
                },
                cx,
                move |window, cx| cx.new(|cx| RelapseApp::new(window, cx, tx, events)),
            )
            .expect("open Relapse GPUI window");
            let poll_view = view.clone();
            cx.spawn(async move |cx| {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(150))
                        .await;
                    poll_view.update(cx, |app, cx| app.poll_events(cx));
                }
            })
            .detach();
            cx.activate(true);
        });
}
