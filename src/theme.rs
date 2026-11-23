use std::sync::Arc;

use gpui_kit::{
    App, Hsla, px, rgb,
    component::{
        Theme, ThemeColor, ThemeMode,
        highlighter::{HighlightTheme, HighlightThemeStyle},
    },
};

fn h(hex: u32) -> Hsla {
    rgb(hex).into()
}

fn light_colors() -> ThemeColor {
    let mut c = (*ThemeColor::light()).clone();
    c.accent = h(0x0066b8);
    c.accent_foreground = h(0xffffff);
    c.background = h(0xffffff);
    c.border = h(0xd9dee5);
    c.caret = h(0x1f2328);
    c.danger = h(0xd1322b);
    c.danger_foreground = h(0xffffff);
    c.foreground = h(0x1f2328);
    c.input = h(0xd0d7de);
    c.list = h(0xffffff);
    c.list_active = h(0xdce9fe);
    c.list_active_border = h(0x0066b8);
    c.list_even = h(0xf6f8fa);
    c.list_head = h(0xffffff);
    c.list_hover = h(0xeef2f7);
    c.muted = h(0xeef1f5);
    c.muted_foreground = h(0x656d76);
    c.popover = h(0xffffff);
    c.popover_foreground = h(0x1f2328);
    c.primary = h(0x0066b8);
    c.primary_active = h(0x1b4fc4);
    c.primary_foreground = h(0xffffff);
    c.primary_hover = h(0x1d57d6);
    c.progress_bar = h(0x0066b8);
    c.ring = h(0x0066b8);
    c.scrollbar = h(0xffffff);
    c.scrollbar_thumb = h(0xc8cdd5);
    c.scrollbar_thumb_hover = h(0xadb3bd);
    c.secondary = h(0xeef1f5);
    c.secondary_active = h(0xdfe4ea);
    c.secondary_foreground = h(0x1f2328);
    c.secondary_hover = h(0xe6eaf0);
    c.selection = h(0xcfe2ff);
    c.sidebar = h(0xeceff3);
    c.sidebar_accent = h(0xeef2f7);
    c.sidebar_accent_foreground = h(0x1f2328);
    c.sidebar_border = h(0xd9dee5);
    c.sidebar_foreground = h(0x4b535d);
    c.sidebar_primary = h(0x0066b8);
    c.sidebar_primary_foreground = h(0xffffff);
    c.skeleton = h(0xe6eaf0);
    c.slider_bar = h(0x0066b8);
    c.slider_thumb = h(0xffffff);
    c.success = h(0x2ea043);
    c.success_foreground = h(0xffffff);
    c.switch = h(0xc8cdd5);
    c.switch_thumb = h(0xffffff);
    c.tab = h(0xeceff3);
    c.tab_active = h(0xffffff);
    c.tab_active_foreground = h(0x1f2328);
    c.tab_bar = h(0xeceff3);
    c.tab_bar_segmented = h(0xe6eaf0);
    c.tab_foreground = h(0x656d76);
    c.table = h(0xffffff);
    c.table_active = h(0xdce9fe);
    c.table_active_border = h(0x0066b8);
    c.table_even = h(0xf6f8fa);
    c.table_head = h(0xf6f8fa);
    c.table_head_foreground = h(0x656d76);
    c.table_hover = h(0xeef2f7);
    c.table_row_border = h(0xeaedf1);
    c.title_bar = h(0xffffff);
    c.title_bar_border = h(0xd9dee5);
    c.status_bar = h(0x007acc);
    c.status_bar_border = h(0x007acc);
    c.warning = h(0xbf8700);
    c.warning_foreground = h(0xffffff);
    c.window_border = h(0xc8cdd5);
    c.info = h(0x0066b8);
    c.red = h(0xd1322b);
    c.green = h(0x2ea043);
    c.blue = h(0x0066b8);
    c.yellow = h(0xbf8700);
    c.magenta = h(0x8250df);
    c.cyan = h(0x1b7c83);
    c
}

fn dark_colors() -> ThemeColor {
    let mut c = (*ThemeColor::dark()).clone();
    c.accent = h(0x1f6feb);
    c.accent_foreground = h(0xffffff);
    c.background = h(0x161b22);
    c.border = h(0x2d333b);
    c.caret = h(0xe6edf3);
    c.danger = h(0xf85149);
    c.danger_foreground = h(0xffffff);
    c.foreground = h(0xe6edf3);
    c.input = h(0x30363d);
    c.list = h(0x161b22);
    c.list_active = h(0x1b2b45);
    c.list_active_border = h(0x388bfd);
    c.list_even = h(0x1b2129);
    c.list_head = h(0x161b22);
    c.list_hover = h(0x1c2230);
    c.muted = h(0x21262d);
    c.muted_foreground = h(0x8b949e);
    c.popover = h(0x1b2129);
    c.popover_foreground = h(0xe6edf3);
    c.primary = h(0x1f6feb);
    c.primary_active = h(0x1a62d0);
    c.primary_foreground = h(0xffffff);
    c.primary_hover = h(0x2a7bf5);
    c.progress_bar = h(0x1f6feb);
    c.ring = h(0x388bfd);
    c.scrollbar = h(0x0d1117);
    c.scrollbar_thumb = h(0x30363d);
    c.scrollbar_thumb_hover = h(0x3d444d);
    c.secondary = h(0x21262d);
    c.secondary_active = h(0x2d333b);
    c.secondary_foreground = h(0xe6edf3);
    c.secondary_hover = h(0x262c34);
    c.selection = h(0x1f3b66);
    c.sidebar = h(0x0d1117);
    c.sidebar_accent = h(0x1c2230);
    c.sidebar_accent_foreground = h(0xffffff);
    c.sidebar_border = h(0x2d333b);
    c.sidebar_foreground = h(0xadbac7);
    c.sidebar_primary = h(0x1f6feb);
    c.sidebar_primary_foreground = h(0xffffff);
    c.skeleton = h(0x1c2230);
    c.slider_bar = h(0x1f6feb);
    c.slider_thumb = h(0xe6edf3);
    c.success = h(0x3fb950);
    c.success_foreground = h(0x0d1117);
    c.switch = h(0x30363d);
    c.switch_thumb = h(0xe6edf3);
    c.tab = h(0x0d1117);
    c.tab_active = h(0x161b22);
    c.tab_active_foreground = h(0xe6edf3);
    c.tab_bar = h(0x0d1117);
    c.tab_bar_segmented = h(0x21262d);
    c.tab_foreground = h(0x8b949e);
    c.table = h(0x161b22);
    c.table_active = h(0x1b2b45);
    c.table_active_border = h(0x388bfd);
    c.table_even = h(0x1b2129);
    c.table_head = h(0x161b22);
    c.table_head_foreground = h(0x8b949e);
    c.table_hover = h(0x1c2230);
    c.table_row_border = h(0x2d333b);
    c.title_bar = h(0x0d1117);
    c.title_bar_border = h(0x2d333b);
    c.status_bar = h(0x18324e);
    c.status_bar_border = h(0x2d333b);
    c.warning = h(0xd29922);
    c.warning_foreground = h(0x0d1117);
    c.window_border = h(0x2d333b);
    c.info = h(0x388bfd);
    c.red = h(0xf85149);
    c.green = h(0x3fb950);
    c.blue = h(0x388bfd);
    c.yellow = h(0xd29922);
    c.magenta = h(0xbc8cff);
    c.cyan = h(0x39c5cf);
    c
}

const LIGHT_HIGHLIGHT: &str = r##"{
  "editor.background": "#ffffff",
  "editor.foreground": "#1f2328",
  "editor.active_line.background": "#0066b80a",
  "editor.line_number": "#aab0ba",
  "editor.active_line_number": "#0066b8",
  "editor.invisible": "#1f23281f",
  "editor.gutter.background": "#ffffff",
  "syntax": {
    "comment": { "color": "#008000" },
    "comment.doc": { "color": "#008000" },
    "string": { "color": "#a31515" },
    "string.escape": { "color": "#ee0000" },
    "string.special": { "color": "#a31515" },
    "string.special.symbol": { "color": "#a31515" },
    "text.literal": { "color": "#a31515" },
    "number": { "color": "#098658" },
    "boolean": { "color": "#0000ff" },
    "keyword": { "color": "#0000ff" },
    "constant": { "color": "#0070c1" },
    "function": { "color": "#795e26" },
    "type": { "color": "#267f99" },
    "constructor": { "color": "#267f99" },
    "enum": { "color": "#267f99" },
    "variable": { "color": "#001080" },
    "variable.special": { "color": "#001080" },
    "property": { "color": "#001080" },
    "attribute": { "color": "#e50000" },
    "operator": { "color": "#000000" },
    "punctuation": { "color": "#000000" },
    "punctuation.bracket": { "color": "#000000" },
    "punctuation.delimiter": { "color": "#000000" },
    "tag": { "color": "#800000" },
    "title": { "color": "#795e26" },
    "primary": { "color": "#000000" }
  }
}"##;

const DARK_HIGHLIGHT: &str = r##"{
  "editor.background": "#161b22",
  "editor.foreground": "#e6edf3",
  "editor.active_line.background": "#ffffff0a",
  "editor.line_number": "#6e7681",
  "editor.active_line_number": "#e6edf3",
  "editor.invisible": "#ffffff14",
  "editor.gutter.background": "#161b22",
  "syntax": {
    "comment": { "color": "#6a9955" },
    "comment.doc": { "color": "#6a9955" },
    "string": { "color": "#ce9178" },
    "string.escape": { "color": "#d7ba7d" },
    "string.special": { "color": "#ce9178" },
    "string.special.symbol": { "color": "#ce9178" },
    "text.literal": { "color": "#ce9178" },
    "number": { "color": "#b5cea8" },
    "boolean": { "color": "#569cd6" },
    "keyword": { "color": "#569cd6" },
    "constant": { "color": "#4fc1ff" },
    "function": { "color": "#dcdcaa" },
    "type": { "color": "#4ec9b0" },
    "constructor": { "color": "#4ec9b0" },
    "enum": { "color": "#4ec9b0" },
    "variable": { "color": "#9cdcfe" },
    "variable.special": { "color": "#9cdcfe" },
    "property": { "color": "#9cdcfe" },
    "attribute": { "color": "#9cdcfe" },
    "operator": { "color": "#d4d4d4" },
    "punctuation": { "color": "#d4d4d4" },
    "punctuation.bracket": { "color": "#d4d4d4" },
    "punctuation.delimiter": { "color": "#d4d4d4" },
    "tag": { "color": "#569cd6" },
    "title": { "color": "#dcdcaa" },
    "primary": { "color": "#d4d4d4" }
  }
}"##;

fn highlight(json: &str, dark: bool) -> Arc<HighlightTheme> {
    let style: HighlightThemeStyle =
        serde_json::from_str(json).expect("embedded highlight theme");
    Arc::new(HighlightTheme {
        name: if dark { "Relapse Dark" } else { "Relapse Light" }.into(),
        appearance: if dark { ThemeMode::Dark } else { ThemeMode::Light },
        style,
    })
}

pub fn set_font_size(size: f32, cx: &mut App) {
    Theme::update(cx, |theme| {
        theme.font_size = px(size);
        theme.mono_font_size = px(size);
    });
}

pub fn set(dark: bool, cx: &mut App) {
    let mode = if dark { ThemeMode::Dark } else { ThemeMode::Light };
    Theme::change(mode, None, cx);
    Theme::update(cx, |theme| {
        theme.colors = if dark { dark_colors() } else { light_colors() };
        theme.font_size = px(12.);
        theme.mono_font_size = px(12.);
        theme.radius = px(3.);
        theme.radius_lg = px(5.);
        theme.shadow = false;
        theme.focus_ring = false;
        theme.highlight_theme = highlight(if dark { DARK_HIGHLIGHT } else { LIGHT_HIGHLIGHT }, dark);
    });
}
