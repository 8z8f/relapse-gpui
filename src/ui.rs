use gpui_kit::{
    App, ClickEvent, Context, Div, Entity, FontWeight, Hsla, IntoElement, ParentElement, Render,
    Styled, Window,
    component::{
        ActiveTheme, Icon, TitleBar,
        input::{Enter, IndentInline, Input},
        menu::{ContextMenuExt, PopupMenu, PopupMenuItem},
    },
    div, prelude::*, px, rgb,
};

use crate::app::{AcceptCompletion, Page, RelapseApp};

fn risk_color(score: Option<u8>) -> u32 {
    match score {
        Some(0..=3) => STATUS_READY,
        Some(4..=6) => STATUS_WAIT,
        Some(_) => STATUS_DOWN,
        None => 0x8a8e98,
    }
}
use gpui_kit::assets::IconName;

const STATUS_READY: u32 = 0x2ea043;
const STATUS_WAIT: u32 = 0xd29922;
const STATUS_DOWN: u32 = 0xd1322b;

fn conn_color(connection: &str) -> u32 {
    match connection {
        "Ready" => STATUS_READY,
        "Connecting" | "Not ready" => STATUS_WAIT,
        _ => STATUS_DOWN,
    }
}

#[derive(Clone, Copy)]
struct Skin {
    canvas: Hsla,
    card: Hsla,
    border: Hsla,
    fg: Hsla,
    muted: Hsla,
    head: Hsla,
    accent: Hsla,
    accent_fg: Hsla,
    accent_hover: Hsla,
    row_active: Hsla,
    row_hover: Hsla,
    tab_active_fg: Hsla,
    status: Hsla,
    status_fg: Hsla,
    status_hover: Hsla,
}

impl Skin {
    fn new(cx: &App) -> Self {
        let t = cx.theme();
        let dark = t.mode.is_dark();
        let white: Hsla = rgb(0xffffff).into();
        Self {
            canvas: t.sidebar,
            card: t.background,
            border: t.border,
            fg: t.foreground,
            muted: t.muted_foreground,
            head: t.muted_foreground,
            accent: t.accent,
            accent_fg: t.accent_foreground,
            accent_hover: t.primary_hover,
            row_active: t.list_active,
            row_hover: t.sidebar_accent,
            tab_active_fg: t.tab_active_foreground,
            status: t.status_bar,
            status_fg: if dark { t.foreground } else { white },
            status_hover: white.opacity(0.14),
        }
    }
}

impl Render for RelapseApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let skin = Skin::new(cx);
        let page = self.page;
        let dark = self.dark;

        let body = match page {
            Page::Editor => editor_page(self, skin, cx).into_any_element(),
            Page::Profiles => profiles_page(self, skin, cx).into_any_element(),
            Page::Hub => hub_page(self, skin, cx).into_any_element(),
            Page::Console => console_page(self, skin, cx).into_any_element(),
            Page::Settings => settings_page(self, skin, cx).into_any_element(),
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(skin.canvas)
            .text_color(skin.fg)
            .text_size(px(11.))
            .on_action(cx.listener(|this, _: &AcceptCompletion, window, cx| {
                let menu_open = this.editor.read(cx).completion_menu_state().open;
                if menu_open {
                    window.dispatch_action(
                        Box::new(Enter {
                            secondary: false,
                            shift: false,
                        }),
                        cx,
                    );
                } else {
                    window.dispatch_action(Box::new(IndentInline), cx);
                }
            }))
            .child(title_bar(skin, page, dark, cx))
            .child(div().flex_1().min_h_0().px(px(10.)).pt(px(10.)).child(body))
            .child(action_bar(self, skin, cx))
            .child(status_bar(self, skin, cx))
    }
}

fn title_bar(skin: Skin, page: Page, dark: bool, cx: &mut Context<RelapseApp>) -> impl IntoElement {
    TitleBar::new()
        .text_color(if dark { rgb(0xe6edf3) } else { rgb(0x1f2328) })
        .child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .gap(px(8.))
                .pl(px(7.))
                .pr(px(6.))
                .child(
                    gpui_kit::svg()
                        .flex_shrink_0()
                        .path(if dark { "relapse-dark.svg" } else { "relapse.svg" })
                        .text_color(if dark { rgb(0xe6edf3) } else { rgb(0x1f2328) })
                        .size(px(17.)),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .mr(px(4.))
                        .text_size(px(11.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Relapse"),
                )
                .child(nav_pill(
                    "nav-editor",
                    IconName::Code,
                    "Editor",
                    page == Page::Editor,
                    skin,
                    cx.listener(|this, _, _, cx| {
                        this.page = Page::Editor;
                        cx.notify();
                    }),
                ))
                .child(nav_pill(
                    "nav-profiles",
                    IconName::Users,
                    "Profiles",
                    page == Page::Profiles,
                    skin,
                    cx.listener(|this, _, _, cx| {
                        this.page = Page::Profiles;
                        cx.notify();
                    }),
                ))
                .child(nav_pill(
                    "nav-hub",
                    IconName::Download,
                    "Hub",
                    page == Page::Hub,
                    skin,
                    cx.listener(|this, _, _, cx| {
                        this.page = Page::Hub;
                        cx.notify();
                    }),
                ))
                .child(nav_pill(
                    "nav-console",
                    IconName::Terminal,
                    "Console",
                    page == Page::Console,
                    skin,
                    cx.listener(|this, _, _, cx| {
                        this.page = Page::Console;
                        cx.notify();
                    }),
                ))
                .child(nav_pill(
                    "nav-settings",
                    IconName::Settings,
                    "Settings",
                    page == Page::Settings,
                    skin,
                    cx.listener(|this, _, _, cx| {
                        this.page = Page::Settings;
                        cx.notify();
                    }),
                ))
                .child(div().flex_1())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(1.))
                        .child(window_button("win-min", IconName::Minus, false, skin, |window| {
                            window.minimize_window()
                        }))
                        .child(window_button("win-max", IconName::Square, false, skin, |window| {
                            window.zoom_window()
                        }))
                        .child(window_button("win-close", IconName::X, true, skin, |window| {
                            window.remove_window()
                        })),
                ),
        )
}

fn nav_pill(
    id: &'static str,
    name: IconName,
    label: &'static str,
    active: bool,
    skin: Skin,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let fg = if active { skin.accent } else { skin.muted };
    let mut pill = div()
        .id(id)
        .h(px(23.))
        .px(px(9.))
        .flex()
        .items_center()
        .gap(px(5.))
        .rounded(px(6.))
        .text_size(px(11.))
        .text_color(fg)
        .cursor_pointer()
        .child(Icon::new(name).size(px(12.)).text_color(fg))
        .child(label)
        .on_click(on_click);
    if active {
        pill = pill.bg(skin.row_active);
    } else {
        pill = pill.hover(move |s| s.bg(skin.row_hover).text_color(skin.fg));
    }
    pill
}

fn window_button(
    id: &'static str,
    name: IconName,
    close: bool,
    skin: Skin,
    action: impl Fn(&mut Window) + 'static,
) -> impl IntoElement {
    let danger: Hsla = rgb(STATUS_DOWN).into();
    let white: Hsla = rgb(0xffffff).into();
    div()
        .id(id)
        .w(px(28.))
        .h(px(22.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(5.))
        .text_color(skin.muted)
        .cursor_pointer()
        .hover(move |s| {
            if close {
                s.bg(danger).text_color(white)
            } else {
                s.bg(skin.row_hover).text_color(skin.fg)
            }
        })
        .child(Icon::new(name).size(px(11.)))
        .on_click(move |_, window, _| action(window))
}

fn script_menu(
    app: Entity<RelapseApp>,
) -> impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static {
    move |menu, _window, _cx| {
        let open = app.clone();
        let save = app.clone();
        let run = app.clone();
        menu.item(
            PopupMenuItem::new("Open...")
                .icon(IconName::FolderOpen)
                .on_click(move |_, window, cx| {
                    open.update(cx, |this, cx| this.open_script(window, cx));
                }),
        )
        .item(
            PopupMenuItem::new("Save")
                .icon(IconName::Save)
                .on_click(move |_, window, cx| {
                    save.update(cx, |this, cx| this.save_script(window, cx));
                }),
        )
        .separator()
        .item(
            PopupMenuItem::new("Execute")
                .icon(IconName::Play)
                .on_click(move |_, _window, cx| {
                    run.update(cx, |this, cx| this.execute(cx));
                }),
        )
    }
}

fn profile_menu(
    app: Entity<RelapseApp>,
    name: String,
) -> impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static {
    move |menu, _window, _cx| {
        let select = app.clone();
        let select_name = name.clone();
        let rename = app.clone();
        let rename_name = name.clone();
        let launch = app.clone();
        let launch_name = name.clone();
        let remove = app.clone();
        let remove_name = name.clone();
        menu.item(
            PopupMenuItem::new("Select")
                .icon(IconName::Check)
                .on_click(move |_, _window, cx| {
                    let target = select_name.clone();
                    select.update(cx, |this, cx| this.select_profile(target, cx));
                }),
        )
        .item(
            PopupMenuItem::new("Rename")
                .icon(IconName::Pencil)
                .on_click(move |_, window, cx| {
                    let target = rename_name.clone();
                    rename.update(cx, |this, cx| this.start_rename(target, window, cx));
                }),
        )
        .item(
            PopupMenuItem::new("Launch")
                .icon(IconName::Rocket)
                .on_click(move |_, _window, cx| {
                    let target = launch_name.clone();
                    launch.update(cx, |this, cx| this.launch_profile(target, cx));
                }),
        )
        .separator()
        .item(
            PopupMenuItem::new("Delete")
                .icon(IconName::Trash)
                .on_click(move |_, _window, cx| {
                    let target = remove_name.clone();
                    remove.update(cx, |this, cx| this.delete_profile(target, cx));
                }),
        )
    }
}

fn card(skin: Skin) -> Div {
    div()
        .bg(skin.card)
        .border(px(1.))
        .border_color(skin.border)
        .rounded(px(8.))
        .overflow_hidden()
}

fn panel_head(name: IconName, label: &'static str, skin: Skin) -> Div {
    div()
        .h(px(32.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(7.))
        .px(px(12.))
        .border_b(px(1.))
        .border_color(skin.border)
        .child(Icon::new(name).size(px(13.)).text_color(skin.accent))
        .child(
            div()
                .text_size(px(11.5))
                .text_color(skin.fg)
                .child(label),
        )
}

fn section_label(label: &'static str, skin: Skin) -> Div {
    div()
        .text_size(px(9.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(skin.head)
        .child(label)
}

fn editor_page(app: &RelapseApp, skin: Skin, cx: &mut Context<RelapseApp>) -> Div {
    let mono = cx.theme().mono_font_family.clone();
    let output_empty = app.output.trim().is_empty();
    let output = if output_empty {
        "No output yet.".to_string()
    } else {
        app.output.clone()
    };

    div()
        .size_full()
        .flex()
        .gap(px(10.))
        .child(scripts_sidebar(app, skin, cx))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(
                    card(skin)
                        .flex_1()
                        .min_h_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .id("editor-header")
                                .h(px(32.))
                                .flex_shrink_0()
                                .flex()
                                .items_center()
                                .gap(px(7.))
                                .px(px(12.))
                                .border_b(px(1.))
                                .border_color(skin.border)
                                .child(
                                    Icon::new(IconName::FileCode)
                                        .size(px(13.))
                                        .text_color(skin.accent),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(11.5))
                                        .text_color(skin.tab_active_fg)
                                        .child(app.active_script.clone()),
                                )
                                .child(
                                    div()
                                        .id("editor-new-script")
                                        .w(px(22.))
                                        .h(px(22.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(5.))
                                        .text_color(skin.muted)
                                        .cursor_pointer()
                                        .hover(move |s| s.bg(skin.row_hover).text_color(skin.accent))
                                        .child(Icon::new(IconName::Plus).size(px(13.)))
                                        .on_click(
                                            cx.listener(|this, _, window, cx| {
                                                this.new_script(window, cx)
                                            }),
                                        ),
                                )
                                .context_menu(script_menu(cx.entity())),
                        )
                        .child(
                            div().flex_1().min_h_0().bg(skin.card).child(
                                gpui_kit::component::input::Editor::new(&app.editor)
                                    .appearance(false)
                                    .bordered(false)
                                    .h_full()
                                    .w_full()
                                    .aria_label("Script editor"),
                            ),
                        ),
                )
                .child(
                    card(skin)
                        .h(px(150.))
                        .flex_shrink_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .h(px(28.))
                                .flex_shrink_0()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .px(px(12.))
                                .border_b(px(1.))
                                .border_color(skin.border)
                                .child(
                                    Icon::new(IconName::Terminal)
                                        .size(px(11.))
                                        .text_color(skin.muted),
                                )
                                .child(section_label("OUTPUT", skin))
                                .child(div().flex_1())
                                .child(
                                    div()
                                        .id("clear-output")
                                        .flex()
                                        .items_center()
                                        .gap(px(4.))
                                        .px(px(7.))
                                        .py(px(2.))
                                        .rounded(px(4.))
                                        .text_size(px(9.5))
                                        .text_color(skin.muted)
                                        .cursor_pointer()
                                        .hover(move |s| s.bg(skin.row_hover).text_color(skin.fg))
                                        .child(Icon::new(IconName::Trash).size(px(10.)))
                                        .child("Clear")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.output.clear();
                                            cx.notify();
                                        })),
                                ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_h_0()
                                .overflow_hidden()
                                .px(px(12.))
                                .py(px(8.))
                                .font_family(mono)
                                .text_size(px(11.))
                                .text_color(if output_empty { skin.muted } else { skin.fg })
                                .child(output),
                        ),
                ),
        )
}

fn scripts_sidebar(app: &RelapseApp, skin: Skin, cx: &mut Context<RelapseApp>) -> Div {
    let active = app.active_script.clone();
    let can_delete = app.scripts.len() > 1;
    let mut list = div().p(px(7.)).flex().flex_col().gap(px(1.));
    for name in app.scripts.iter() {
        let is_active = *name == active;
        let display = name.clone();
        let switch_name = name.clone();
        let delete_name = name.clone();
        let delete_button = if can_delete {
            Some(
                div()
                    .id(("script-del", profile_key(&display)))
                    .w(px(20.))
                    .h(px(20.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .text_color(skin.muted)
                    .cursor_pointer()
                    .hover(move |s| s.bg(skin.row_hover).text_color(rgb(STATUS_DOWN)))
                    .child(Icon::new(IconName::Trash).size(px(11.)))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.delete_script(delete_name.clone(), window, cx);
                    })),
            )
        } else {
            None
        };
        list = list.child(
            div()
                .h(px(26.))
                .px(px(3.))
                .flex()
                .items_center()
                .gap(px(2.))
                .rounded(px(6.))
                .when(is_active, move |this| this.bg(skin.row_active))
                .when(!is_active, move |this| this.hover(move |s| s.bg(skin.row_hover)))
                .child(
                    div()
                        .id(("script-open", profile_key(&display)))
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .flex()
                        .items_center()
                        .gap(px(7.))
                        .px(px(5.))
                        .cursor_pointer()
                        .child(Icon::new(IconName::FileCode).size(px(13.)).text_color(
                            if is_active { skin.accent } else { skin.muted },
                        ))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(11.))
                                .text_color(skin.fg)
                                .child(display.clone()),
                        )
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.switch_script(switch_name.clone(), window, cx);
                        }))
                        .context_menu(script_menu(cx.entity())),
                )
                .children(delete_button),
        );
    }

    card(skin)
        .w(px(208.))
        .flex_shrink_0()
        .h_full()
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(32.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(px(7.))
                .px(px(12.))
                .border_b(px(1.))
                .border_color(skin.border)
                .child(Icon::new(IconName::FileCode).size(px(13.)).text_color(skin.accent))
                .child(div().flex_1().text_size(px(11.5)).text_color(skin.fg).child("Scripts"))
                .child(
                    div()
                        .id("scripts-new")
                        .w(px(22.))
                        .h(px(22.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(5.))
                        .text_color(skin.muted)
                        .cursor_pointer()
                        .hover(move |s| s.bg(skin.row_hover).text_color(skin.accent))
                        .child(Icon::new(IconName::Plus).size(px(13.)))
                        .on_click(cx.listener(|this, _, window, cx| this.new_script(window, cx))),
                ),
        )
        .child(div().id("scripts-scroll").flex_1().min_h_0().overflow_y_scroll().child(list))
}

fn profile_key(name: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    name.hash(&mut hasher);
    hasher.finish()
}

fn profiles_page(app: &RelapseApp, skin: Skin, cx: &mut Context<RelapseApp>) -> Div {
    let selected = app.selected_profile.clone();
    let running = app.profiles.iter().filter(|profile| profile.running).count();
    let checked = app.checked.len();

    let mut list = div().p(px(8.)).flex().flex_col().gap(px(2.));
    if app.profiles.is_empty() {
        list = list.child(
            div()
                .py(px(24.))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(6.))
                .text_color(skin.muted)
                .child(Icon::new(IconName::Users).size(px(22.)).text_color(skin.muted))
                .child(div().text_size(px(11.)).child("No profiles yet")),
        );
    }
    for profile in app.profiles.iter() {
        let name = profile.name.clone();
        let active = selected.as_deref() == Some(profile.name.as_str());
        let is_running = profile.running;
        let is_checked = app.checked.contains(&profile.name);
        let select_name = name.clone();
        let check_name = name.clone();
        let launch_name = name.clone();
        let delete_name = name.clone();
        let icon_fg = if is_running {
            rgb(STATUS_READY).into()
        } else if active {
            skin.accent
        } else {
            skin.muted
        };
        list = list.child(
            div()
                .h(px(34.))
                .px(px(7.))
                .flex()
                .items_center()
                .gap(px(6.))
                .rounded(px(7.))
                .border_l(px(2.))
                .border_color(if is_checked { skin.accent } else { skin.card })
                .when(active && !is_checked, move |this| this.bg(skin.row_active))
                .when(is_checked, move |this| this.bg(skin.row_active))
                .when(!active && !is_checked, move |this| {
                    this.hover(move |s| s.bg(skin.row_hover))
                })
                .child(
                    div()
                        .id(("profile-check", profile_key(&name)))
                        .w(px(16.))
                        .h(px(16.))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.))
                        .border(px(1.))
                        .border_color(if is_checked { skin.accent } else { skin.border })
                        .cursor_pointer()
                        .when(is_checked, move |this| this.bg(skin.accent))
                        .when(!is_checked, move |this| this.hover(move |s| s.border_color(skin.accent)))
                        .when(is_checked, move |this| {
                            this.child(Icon::new(IconName::Check).size(px(11.)).text_color(skin.accent_fg))
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_check(check_name.clone(), cx);
                        })),
                )
                .child(
                    div()
                        .id(("profile-row", profile_key(&name)))
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .flex()
                        .items_center()
                        .gap(px(9.))
                        .cursor_pointer()
                        .child(Icon::new(IconName::CircleUser).size(px(17.)).text_color(icon_fg))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(11.5))
                                        .text_color(skin.fg)
                                        .child(name.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(9.))
                                        .text_color(if is_running {
                                            rgb(STATUS_READY).into()
                                        } else {
                                            skin.muted
                                        })
                                        .child(if is_running { "running" } else { "stopped" }),
                                ),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.select_profile(select_name.clone(), cx);
                        }))
                        .context_menu(profile_menu(cx.entity(), name.clone())),
                )
                .child(
                    div()
                        .id(("profile-launch", profile_key(&name)))
                        .w(px(24.))
                        .h(px(24.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(5.))
                        .text_color(skin.muted)
                        .cursor_pointer()
                        .hover(move |s| s.bg(skin.row_hover).text_color(skin.accent))
                        .child(Icon::new(IconName::Rocket).size(px(13.)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.launch_profile(launch_name.clone(), cx);
                        })),
                )
                .child(
                    div()
                        .id(("profile-delete", profile_key(&name)))
                        .w(px(24.))
                        .h(px(24.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(5.))
                        .text_color(skin.muted)
                        .cursor_pointer()
                        .hover(move |s| s.bg(skin.row_hover).text_color(rgb(STATUS_DOWN)))
                        .child(Icon::new(IconName::Trash).size(px(13.)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.delete_profile(delete_name.clone(), cx);
                        })),
                ),
        );
    }

    div().size_full().child(
        card(skin)
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(32.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .px(px(12.))
                    .border_b(px(1.))
                    .border_color(skin.border)
                    .child(Icon::new(IconName::Users).size(px(13.)).text_color(skin.accent))
                    .child(div().text_size(px(11.5)).text_color(skin.fg).child("Profiles"))
                    .child(
                        div()
                            .text_size(px(9.5))
                            .text_color(skin.muted)
                            .child(if checked > 0 {
                                format!("{checked} selected")
                            } else {
                                format!("{running} running")
                            }),
                    )
                    .child(div().flex_1())
                    .when(checked > 0, |this| {
                        this.child(pill_button(
                            "profile-launch-selected",
                            IconName::Rocket,
                            "Launch",
                            true,
                            skin,
                            cx.listener(|this, _, _, cx| this.launch_checked(cx)),
                        ))
                        .child(pill_button(
                            "profile-clear-selected",
                            IconName::X,
                            "Clear",
                            false,
                            skin,
                            cx.listener(|this, _, _, cx| this.clear_checked(cx)),
                        ))
                    })
                    .child(pill_button(
                        "profile-new",
                        IconName::Plus,
                        "New",
                        true,
                        skin,
                        cx.listener(|this, _, _, cx| this.new_profile(cx)),
                    ))
                    .child(pill_button(
                        "profile-refresh",
                        IconName::RotateCw,
                        "Refresh",
                        false,
                        skin,
                        cx.listener(|this, _, _, cx| this.refresh_profiles(cx)),
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(10.))
                    .py(px(7.))
                    .border_b(px(1.))
                    .border_color(skin.border)
                    .child(Icon::new(IconName::Rocket).size(px(12.)).text_color(skin.muted))
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_size(px(10.))
                            .text_color(skin.muted)
                            .child("Join"),
                    )
                    .child(div().w(px(118.)).flex_shrink_0().child(Input::new(&app.place_input)))
                    .child(div().flex_1().min_w_0().child(Input::new(&app.job_input))),
            )
            .when(app.renaming.is_some(), |this| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .px(px(10.))
                        .py(px(7.))
                        .border_b(px(1.))
                        .border_color(skin.border)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(Input::new(&app.rename_input)),
                        )
                        .child(stepper_button(
                            "rename-commit",
                            IconName::Check,
                            skin,
                            cx.listener(|this, _, _, cx| this.commit_rename(cx)),
                        ))
                        .child(stepper_button(
                            "rename-cancel",
                            IconName::X,
                            skin,
                            cx.listener(|this, _, _, cx| this.cancel_rename(cx)),
                        )),
                )
            })
            .child(div().id("profiles-scroll").flex_1().min_h_0().overflow_y_scroll().child(list)),
    )
}

fn pill_button(
    id: &'static str,
    name: IconName,
    label: &'static str,
    filled: bool,
    skin: Skin,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let fg = if filled { skin.accent_fg } else { skin.fg };
    let mut button = div()
        .id(id)
        .h(px(22.))
        .px(px(9.))
        .flex()
        .items_center()
        .gap(px(5.))
        .rounded(px(6.))
        .text_size(px(11.))
        .text_color(fg)
        .cursor_pointer()
        .child(Icon::new(name).size(px(12.)).text_color(fg))
        .child(label)
        .on_click(on_click);
    if filled {
        button = button.bg(skin.accent).hover(move |s| s.bg(skin.accent_hover));
    } else {
        button = button
            .border(px(1.))
            .border_color(skin.border)
            .hover(move |s| s.bg(skin.row_hover));
    }
    button
}

fn stepper_button(
    id: &'static str,
    name: IconName,
    skin: Skin,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .w(px(24.))
        .h(px(24.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .border(px(1.))
        .border_color(skin.border)
        .text_color(skin.fg)
        .cursor_pointer()
        .hover(move |s| s.bg(skin.row_hover))
        .child(Icon::new(name).size(px(12.)))
        .on_click(on_click)
}

fn toggle_switch(
    id: &'static str,
    on: bool,
    skin: Skin,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let white: Hsla = rgb(0xffffff).into();
    div()
        .id(id)
        .w(px(32.))
        .h(px(18.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .p(px(2.))
        .rounded_full()
        .cursor_pointer()
        .bg(if on { skin.accent } else { skin.border })
        .when(on, |this| this.justify_end())
        .child(div().size(px(14.)).rounded_full().bg(white))
        .on_click(on_click)
}

fn setting_toggle(
    id: &'static str,
    label: &'static str,
    description: &'static str,
    on: bool,
    skin: Skin,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(10.))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().text_size(px(11.)).text_color(skin.fg).child(label))
                .child(
                    div()
                        .text_size(px(9.5))
                        .text_color(skin.muted)
                        .child(description),
                ),
        )
        .child(toggle_switch(id, on, skin, on_click))
}

fn indent_choice(
    id: &'static str,
    width: u8,
    active: bool,
    skin: Skin,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .h(px(24.))
        .px(px(10.))
        .flex()
        .items_center()
        .rounded(px(5.))
        .border(px(1.))
        .border_color(if active { skin.accent } else { skin.border })
        .bg(if active { skin.row_active } else { skin.card })
        .text_color(if active { skin.accent } else { skin.fg })
        .text_size(px(11.))
        .cursor_pointer()
        .when(!active, move |this| this.hover(move |s| s.bg(skin.row_hover)))
        .child(format!("{width}"))
        .on_click(on_click)
}

fn action_bar(app: &RelapseApp, skin: Skin, cx: &mut Context<RelapseApp>) -> Div {
    div()
        .h(px(36.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(6.))
        .px(px(10.))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(5.))
                .text_size(px(10.))
                .text_color(skin.muted)
                .child(Icon::new(IconName::Users).size(px(11.)).text_color(skin.muted))
                .child(
                    app.selected_profile
                        .clone()
                        .unwrap_or_else(|| "No profile".into()),
                ),
        )
        .child(div().flex_1())
        .child(pill_button(
            "action-open",
            IconName::FolderOpen,
            "Open",
            false,
            skin,
            cx.listener(|this, _, window, cx| this.open_script(window, cx)),
        ))
        .child(pill_button(
            "action-save",
            IconName::Save,
            "Save",
            false,
            skin,
            cx.listener(|this, _, window, cx| this.save_script(window, cx)),
        ))
        .child(pill_button(
            "action-execute",
            IconName::Play,
            "Execute",
            true,
            skin,
            cx.listener(|this, _, _, cx| this.execute(cx)),
        ))
}

fn status_bar(app: &RelapseApp, skin: Skin, cx: &mut Context<RelapseApp>) -> Div {
    let dot = conn_color(&app.connection);
    div()
        .h(px(24.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(10.))
        .px(px(10.))
        .bg(skin.status)
        .text_color(skin.status_fg)
        .text_size(px(10.))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(div().size(px(7.)).rounded_full().bg(rgb(dot)))
                .child(app.connection.clone())
                .when(!app.detail.is_empty(), |this| {
                    this.child(div().opacity(0.75).child(app.detail.clone()))
                }),
        )
        .child(div().flex_1())
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(5.))
                .opacity(0.85)
                .child(Icon::new(IconName::Braces).size(px(10.)))
                .child("Luau"),
        )
        .child(
            div()
                .id("status-theme")
                .w(px(26.))
                .h(px(18.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .cursor_pointer()
                .hover(move |s| s.bg(skin.status_hover))
                .child(
                    Icon::new(if app.dark {
                        IconName::Moon
                    } else {
                        IconName::Sun
                    })
                    .size(px(11.)),
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.dark = !this.dark;
                    crate::theme::set(this.dark, cx);
                    crate::theme::set_font_size(this.font_size, cx);
                    this.persist_settings();
                    cx.notify();
                })),
        )
}

fn hub_page(app: &RelapseApp, skin: Skin, cx: &mut Context<RelapseApp>) -> Div {
    let mut list = div().p(px(8.)).flex().flex_col().gap(px(6.));
    for (index, hit) in app.hub_results.iter().enumerate() {
        let raw_url = hit.raw_url.clone();
        let title = hit.title.clone();
        let dot = risk_color(hit.risk_score);
        let risk_text = if hit.risk_level.is_empty() {
            "unscanned".to_string()
        } else {
            hit.risk_level.clone()
        };
        let subtitle = match (hit.game.is_empty(), hit.creator.is_empty()) {
            (false, false) => format!("{}  by {}", hit.game, hit.creator),
            (false, true) => hit.game.clone(),
            (true, false) => format!("by {}", hit.creator),
            (true, true) => "Universal".to_string(),
        };
        list = list.child(
            div()
                .id(("hub-hit", index as u64))
                .p(px(9.))
                .flex()
                .flex_col()
                .gap(px(5.))
                .rounded(px(8.))
                .border(px(1.))
                .border_color(skin.border)
                .bg(skin.card)
                .cursor_pointer()
                .hover(move |s| s.bg(skin.row_hover).border_color(skin.accent))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(7.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(12.))
                                .text_color(skin.fg)
                                .child(title.clone()),
                        )
                        .when(hit.key_system, |this| {
                            this.child(
                                div()
                                    .px(px(5.))
                                    .rounded(px(3.))
                                    .text_size(px(8.5))
                                    .text_color(rgb(STATUS_WAIT))
                                    .child("KEY"),
                            )
                        })
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .child(div().size(px(6.)).rounded_full().bg(rgb(dot)))
                                .child(
                                    div()
                                        .text_size(px(9.))
                                        .text_color(skin.muted)
                                        .child(risk_text),
                                ),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(10.))
                        .text_color(skin.muted)
                        .child(div().flex_1().min_w_0().truncate().child(subtitle))
                        .child(Icon::new(IconName::Eye).size(px(11.)).text_color(skin.muted))
                        .child(format!("{}", hit.views))
                        .child(Icon::new(IconName::Star).size(px(11.)).text_color(skin.muted))
                        .child(format!("{}", hit.likes)),
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.hub_load(raw_url.clone(), title.clone(), window, cx);
                })),
        );
    }

    div().size_full().child(
        card(skin)
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(40.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(12.))
                    .border_b(px(1.))
                    .border_color(skin.border)
                    .child(Icon::new(IconName::Download).size(px(13.)).text_color(skin.accent))
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_size(px(11.5))
                            .text_color(skin.fg)
                            .child("Script Hub"),
                    )
                    .child(div().flex_1().min_w_0().child(Input::new(&app.hub_input)))
                    .child(pill_button(
                        "hub-search",
                        IconName::Search,
                        "Search",
                        true,
                        skin,
                        cx.listener(|this, _, _, cx| this.hub_search(cx)),
                    )),
            )
            .child(
                div()
                    .h(px(22.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .px(px(12.))
                    .text_size(px(9.5))
                    .text_color(skin.muted)
                    .child(app.hub_status.clone()),
            )
            .child(div().id("hub-scroll").flex_1().min_h_0().overflow_y_scroll().child(list)),
    )
}

fn console_page(app: &RelapseApp, skin: Skin, cx: &mut Context<RelapseApp>) -> Div {
    let mono = cx.theme().mono_font_family.clone();
    let empty = app.console_output.trim().is_empty();
    let body = if empty {
        "Console is empty. Call rconsoleprint from a script.".to_string()
    } else {
        app.console_output.clone()
    };
    let awaiting = app.pending_console_input.is_some();
    div().size_full().child(
        card(skin)
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(32.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .px(px(12.))
                    .border_b(px(1.))
                    .border_color(skin.border)
                    .child(Icon::new(IconName::Terminal).size(px(13.)).text_color(skin.accent))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(11.5))
                            .text_color(skin.fg)
                            .child(app.console_title.clone()),
                    )
                    .child(
                        div()
                            .id("console-clear")
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .px(px(7.))
                            .py(px(2.))
                            .rounded(px(4.))
                            .text_size(px(9.5))
                            .text_color(skin.muted)
                            .cursor_pointer()
                            .hover(move |s| s.bg(skin.row_hover).text_color(skin.fg))
                            .child(Icon::new(IconName::Trash).size(px(10.)))
                            .child("Clear")
                            .on_click(cx.listener(|this, _, _, cx| this.clear_console(cx))),
                    ),
            )
            .child(
                div()
                    .id("console-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(12.))
                    .py(px(8.))
                    .font_family(mono)
                    .text_size(px(11.))
                    .text_color(if empty { skin.muted } else { skin.fg })
                    .child(body),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .px(px(10.))
                    .py(px(7.))
                    .border_t(px(1.))
                    .border_color(skin.border)
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_size(px(10.))
                            .text_color(if awaiting { skin.accent } else { skin.muted })
                            .child(if awaiting { "input requested" } else { "idle" }),
                    )
                    .child(div().flex_1().min_w_0().child(Input::new(&app.console_input)))
                    .child(pill_button(
                        "console-send",
                        IconName::Check,
                        "Send",
                        awaiting,
                        skin,
                        cx.listener(|this, _, window, cx| this.submit_console_input(window, cx)),
                    )),
            ),
    )
}

fn settings_page(app: &RelapseApp, skin: Skin, cx: &mut Context<RelapseApp>) -> Div {
    div().size_full().child(
        card(skin)
            .size_full()
            .flex()
            .flex_col()
            .child(panel_head(IconName::Settings, "Settings", skin))
            .child(
                div()
                    .id("settings-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(
                        div()
                            .p(px(20.))
                            .flex()
                            .flex_col()
                            .gap(px(18.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(section_label("APPEARANCE", skin))
                            .child(
                                div()
                                    .flex()
                                    .gap(px(8.))
                                    .child(theme_choice("theme-light", "Light", !app.dark, skin, cx))
                                    .child(theme_choice("theme-dark", "Dark", app.dark, skin, cx)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(section_label("EDITOR", skin))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(10.))
                                    .child(
                                        Icon::new(IconName::FileCode)
                                            .size(px(13.))
                                            .text_color(skin.muted),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(skin.fg)
                                            .child("Font size"),
                                    )
                                    .child(div().flex_1())
                                    .child(stepper_button(
                                        "font-dec",
                                        IconName::Minus,
                                        skin,
                                        cx.listener(|this, _, _, cx| this.set_font_size(-1.0, cx)),
                                    ))
                                    .child(
                                        div()
                                            .w(px(26.))
                                            .flex()
                                            .justify_center()
                                            .text_size(px(11.))
                                            .text_color(skin.fg)
                                            .child(format!("{}", app.font_size as i32)),
                                    )
                                    .child(stepper_button(
                                        "font-inc",
                                        IconName::Plus,
                                        skin,
                                        cx.listener(|this, _, _, cx| this.set_font_size(1.0, cx)),
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(10.))
                            .child(setting_toggle(
                                "set-wrap",
                                "Word wrap",
                                "Wrap long lines to fit the editor",
                                app.word_wrap,
                                skin,
                                cx.listener(|this, _, _, cx| this.toggle_word_wrap(cx)),
                            ))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(10.))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                div()
                                                    .text_size(px(11.))
                                                    .text_color(skin.fg)
                                                    .child("Indentation"),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(9.5))
                                                    .text_color(skin.muted)
                                                    .child("Spaces per indent"),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .gap(px(5.))
                                            .child(indent_choice(
                                                "indent-2",
                                                2,
                                                app.indent == 2,
                                                skin,
                                                cx.listener(|this, _, _, cx| this.set_indent(2, cx)),
                                            ))
                                            .child(indent_choice(
                                                "indent-4",
                                                4,
                                                app.indent == 4,
                                                skin,
                                                cx.listener(|this, _, _, cx| this.set_indent(4, cx)),
                                            ))
                                            .child(indent_choice(
                                                "indent-8",
                                                8,
                                                app.indent == 8,
                                                skin,
                                                cx.listener(|this, _, _, cx| this.set_indent(8, cx)),
                                            )),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(section_label("SESSION", skin))
                            .child(setting_toggle(
                                "set-restore",
                                "Restore tabs on startup",
                                "Reopen tabs and unsaved drafts",
                                app.restore_tabs,
                                skin,
                                cx.listener(|this, _, _, cx| this.toggle_restore_tabs(cx)),
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(section_label("OUTPUT", skin))
                            .child(setting_toggle(
                                "set-redirect",
                                "Redirect script output",
                                "Show your scripts output below the editor",
                                app.redirect_output,
                                skin,
                                cx.listener(|this, _, _, cx| this.toggle_redirect_output(cx)),
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(section_label("SCRIPT HUB", skin))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(
                                        Icon::new(IconName::Download)
                                            .size(px(13.))
                                            .text_color(skin.muted),
                                    )
                                    .child(div().flex_1().min_w_0().child(Input::new(&app.key_input)))
                                    .child(pill_button(
                                        "key-save",
                                        IconName::Check,
                                        "Save key",
                                        true,
                                        skin,
                                        cx.listener(|this, _, _, cx| this.save_rscripts_key(cx)),
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(section_label("LAUNCHER", skin))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(10.))
                                    .child(
                                        Icon::new(IconName::Rocket)
                                            .size(px(13.))
                                            .text_color(skin.muted),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .truncate()
                                            .text_size(px(11.))
                                            .text_color(if app.launcher.is_some() {
                                                skin.fg
                                            } else {
                                                skin.muted
                                            })
                                            .child(
                                                app.launcher
                                                    .as_ref()
                                                    .map(|path| path.display().to_string())
                                                    .unwrap_or_else(|| "None selected".to_string()),
                                            ),
                                    )
                                    .child(pill_button(
                                        "launcher-select",
                                        IconName::FolderOpen,
                                        "Select",
                                        false,
                                        skin,
                                        cx.listener(|this, _, window, cx| {
                                            this.set_launcher(window, cx)
                                        }),
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(section_label("CONNECTION", skin))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .text_size(px(11.))
                                    .text_color(skin.fg)
                                    .child(
                                        div()
                                            .size(px(7.))
                                            .rounded_full()
                                            .bg(rgb(conn_color(&app.connection))),
                                    )
                                    .child(app.connection.clone())
                                    .child(
                                        div().text_color(skin.muted).child(app.detail.clone()),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(section_label("ABOUT", skin))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .text_size(px(11.))
                                    .text_color(skin.muted)
                                    .child(
                                        gpui_kit::svg()
                                            .flex_shrink_0()
                                            .path(if app.dark {
                                                "relapse-dark.svg"
                                            } else {
                                                "relapse.svg"
                                            })
                                            .text_color(skin.accent)
                                            .size(px(15.)),
                                    )
                                    .child("discord.gg/getrelapse"),
                            ),
                    ),
                    ),
            ),
    )
}

fn theme_choice(
    id: &'static str,
    label: &'static str,
    active: bool,
    skin: Skin,
    cx: &mut Context<RelapseApp>,
) -> impl IntoElement {
    let dark = label == "Dark";
    div()
        .id(id)
        .h(px(28.))
        .px(px(12.))
        .flex()
        .items_center()
        .gap(px(7.))
        .rounded(px(6.))
        .border(px(1.))
        .border_color(if active { skin.accent } else { skin.border })
        .bg(if active { skin.row_active } else { skin.card })
        .text_color(if active { skin.accent } else { skin.fg })
        .text_size(px(11.))
        .cursor_pointer()
        .when(!active, move |this| this.hover(move |s| s.bg(skin.row_hover)))
        .child(
            Icon::new(if dark { IconName::Moon } else { IconName::Sun })
                .size(px(13.))
                .text_color(if active { skin.accent } else { skin.muted }),
        )
        .child(label)
        .on_click(cx.listener(move |this, _, _, cx| {
            this.dark = dark;
            crate::theme::set(dark, cx);
            crate::theme::set_font_size(this.font_size, cx);
            this.persist_settings();
            cx.notify();
        }))
}
