use std::{
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{Receiver, Sender},
    time::{Duration, Instant},
};

use gpui_kit::{
    AppContext, Context, PathPromptOptions, Window,
    component::input::{EditorState, InputState},
};

use crate::{
    profiles::{self, Profile},
    protocol::{Event, WorkerCommand},
    scripts,
};

gpui_kit::actions!(relapse, [AcceptCompletion]);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Page {
    Editor,
    Profiles,
    Hub,
    Console,
    Settings,
}

#[derive(Clone)]
pub(crate) struct ScriptHit {
    pub(crate) title: String,
    pub(crate) raw_url: String,
    pub(crate) game: String,
    pub(crate) creator: String,
    pub(crate) views: u64,
    pub(crate) likes: u64,
    pub(crate) risk_level: String,
    pub(crate) risk_score: Option<u8>,
    pub(crate) key_system: bool,
}

pub(crate) struct RelapseApp {
    pub(crate) editor: gpui_kit::Entity<EditorState>,
    pub(crate) tx: Sender<WorkerCommand>,
    pub(crate) events: Receiver<Event>,
    pub(crate) connection: String,
    pub(crate) detail: String,
    pub(crate) output: String,
    pub(crate) next_id: i64,
    pub(crate) page: Page,
    pub(crate) dark: bool,
    pub(crate) profiles: Vec<Profile>,
    pub(crate) selected_profile: Option<String>,
    pub(crate) current_path: Option<PathBuf>,
    pub(crate) scripts: Vec<String>,
    pub(crate) active_script: String,
    last_saved: String,
    last_autosave: Instant,
    pub(crate) launcher: Option<PathBuf>,
    pub(crate) font_size: f32,
    pub(crate) rename_input: gpui_kit::Entity<InputState>,
    pub(crate) renaming: Option<String>,
    pub(crate) checked: std::collections::HashSet<String>,
    pub(crate) place_input: gpui_kit::Entity<InputState>,
    pub(crate) job_input: gpui_kit::Entity<InputState>,
    pub(crate) word_wrap: bool,
    pub(crate) indent: u8,
    pub(crate) restore_tabs: bool,
    pub(crate) redirect_output: bool,
    pub(crate) rscripts_key: String,
    pub(crate) console_output: String,
    pub(crate) console_title: String,
    pub(crate) console_input: gpui_kit::Entity<InputState>,
    pub(crate) pending_console_input: Option<i64>,
    pub(crate) hub_input: gpui_kit::Entity<InputState>,
    pub(crate) hub_results: Vec<ScriptHit>,
    pub(crate) hub_status: String,
    pub(crate) hub_loading: bool,
    pub(crate) key_input: gpui_kit::Entity<InputState>,
    refresh: Instant,
}

const OUTPUT_LIMIT: usize = 200_000;
const REFRESH_INTERVAL: Duration = Duration::from_secs(2);
const EDITOR_HIGHLIGHT_LIMIT: usize = 300_000;

fn build_editor(source: String, window: &mut Window, cx: &mut Context<RelapseApp>) -> gpui_kit::Entity<EditorState> {
    let highlight = source.len() <= EDITOR_HIGHLIGHT_LIMIT;
    cx.new(|cx| {
        let state = EditorState::new(window, cx);
        let state = if highlight { state.language("lua") } else { state };
        let mut state = state.default_value(source);
        state.lsp_mut().completion_provider =
            Some(std::rc::Rc::new(crate::completions::LuauCompletions::new()));
        state
            .lsp_mut()
            .set_completion_highlighter(std::rc::Rc::new(crate::completions::no_highlight));
        state
    })
}

fn home_directory() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn is_job_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    bytes.iter().enumerate().all(|(index, byte)| match index {
        8 | 13 | 18 | 23 => *byte == b'-',
        _ => byte.is_ascii_hexdigit(),
    })
}

fn launcher_config_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join(".config")
            .join("relapse-gpui")
            .join("launcher")
    })
}

fn load_launcher() -> Option<PathBuf> {
    let path = launcher_config_path()?;
    let contents = std::fs::read_to_string(path).ok()?;
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

fn save_launcher(path: &Path) {
    let Some(config) = launcher_config_path() else {
        return;
    };
    if let Some(parent) = config.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(config, path.to_string_lossy().as_bytes());
}

struct Settings {
    dark: bool,
    font_size: f32,
    word_wrap: bool,
    indent: u8,
    restore_tabs: bool,
    redirect_output: bool,
    rscripts_key: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            dark: false,
            font_size: 12.0,
            word_wrap: true,
            indent: 4,
            restore_tabs: true,
            redirect_output: true,
            rscripts_key: String::new(),
        }
    }
}

fn settings_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join(".config")
            .join("relapse-gpui")
            .join("settings.json")
    })
}

fn load_settings() -> Settings {
    let defaults = Settings::default();
    let Some(path) = settings_path() else {
        return defaults;
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return defaults;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return defaults;
    };
    Settings {
        dark: value["dark"].as_bool().unwrap_or(defaults.dark),
        font_size: value["font_size"]
            .as_f64()
            .map(|v| (v as f32).clamp(10.0, 20.0))
            .unwrap_or(defaults.font_size),
        word_wrap: value["word_wrap"].as_bool().unwrap_or(defaults.word_wrap),
        indent: value["indent"]
            .as_u64()
            .map(|v| v as u8)
            .filter(|v| matches!(v, 2 | 4 | 8))
            .unwrap_or(defaults.indent),
        restore_tabs: value["restore_tabs"]
            .as_bool()
            .unwrap_or(defaults.restore_tabs),
        redirect_output: value["redirect_output"]
            .as_bool()
            .unwrap_or(defaults.redirect_output),
        rscripts_key: value["rscripts_key"]
            .as_str()
            .unwrap_or(&defaults.rscripts_key)
            .to_string(),
    }
}

fn curl_get(url: &str, key: Option<&str>) -> Result<String, String> {
    let mut command = Command::new("curl");
    command
        .arg("-s")
        .arg("--fail")
        .arg("--max-time")
        .arg("20");
    if let Some(key) = key {
        command.arg("-H").arg(format!("Authorization: Bearer {key}"));
    }
    command.arg(url);
    let output = command
        .output()
        .map_err(|error| format!("curl failed: {error}"))?;
    if !output.status.success() {
        return Err("Request failed (check your API key and connection)".to_string());
    }
    String::from_utf8(output.stdout).map_err(|_| "Malformed response".to_string())
}

fn parse_hits(body: &str) -> Result<Vec<ScriptHit>, String> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|_| "Malformed response".to_string())?;
    if value["success"].as_bool() != Some(true) {
        let message = value["error"]["message"]
            .as_str()
            .or_else(|| value["message"].as_str())
            .unwrap_or("API error");
        return Err(message.to_string());
    }
    let hits = value["data"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|script| {
                    let title = script["title"].as_str()?.to_string();
                    Some(ScriptHit {
                        title,
                        raw_url: script["rawScript"].as_str().unwrap_or("").to_string(),
                        game: script["game"]["title"].as_str().unwrap_or("").to_string(),
                        creator: script["creator"]["username"].as_str().unwrap_or("").to_string(),
                        views: script["views"].as_u64().unwrap_or(0),
                        likes: script["likes"].as_u64().unwrap_or(0),
                        risk_level: script["risk"]["level"].as_str().unwrap_or("").to_string(),
                        risk_score: script["risk"]["score"].as_u64().map(|score| score as u8),
                        key_system: script["isKeySystem"].as_bool().unwrap_or(false),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(hits)
}

fn encode_query(query: &str) -> String {
    let mut out = String::with_capacity(query.len());
    for byte in query.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn sanitize_name(title: &str) -> String {
    let mut name: String = title
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect();
    while name.contains("--") {
        name = name.replace("--", "-");
    }
    let name = name.trim_matches('-');
    let name: String = name.chars().take(48).collect();
    if name.is_empty() {
        "script".to_string()
    } else {
        name
    }
}

fn save_settings(settings: &Settings) {
    let Some(path) = settings_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let value = serde_json::json!({
        "dark": settings.dark,
        "font_size": settings.font_size,
        "word_wrap": settings.word_wrap,
        "indent": settings.indent,
        "restore_tabs": settings.restore_tabs,
        "redirect_output": settings.redirect_output,
        "rscripts_key": settings.rscripts_key,
    });
    let _ = std::fs::write(path, serde_json::to_string_pretty(&value).unwrap_or_default());
}

impl RelapseApp {
    pub(crate) fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        tx: Sender<WorkerCommand>,
        events: Receiver<Event>,
    ) -> Self {
        let settings = load_settings();
        crate::theme::set(settings.dark, cx);
        crate::theme::set_font_size(settings.font_size, cx);
        let scripts = scripts::load();
        let active_script = scripts.first().cloned().unwrap_or_else(|| "script.lua".to_string());
        let active_source = scripts::read(&active_script).unwrap_or_default();
        let last_saved = active_source.clone();
        let editor = build_editor(active_source, window, cx);
        let rename_input = cx.new(|cx| InputState::new(window, cx).placeholder("Profile name"));
        let place_input = cx.new(|cx| InputState::new(window, cx).placeholder("Place ID"));
        let job_input = cx.new(|cx| InputState::new(window, cx).placeholder("Job ID (optional)"));
        let console_input = cx.new(|cx| InputState::new(window, cx).placeholder("rconsole input"));
        let hub_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search rscripts.net"));
        let key_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("rsc_live_...")
                .default_value(settings.rscripts_key.clone())
        });
        let loaded = profiles::load();
        let selected_profile = loaded.iter().find(|profile| profile.running).map(|profile| profile.name.clone());
        let _ = tx.send(WorkerCommand::Target(selected_profile.clone()));
        Self {
            editor,
            tx,
            events,
            connection: "Disconnected".into(),
            detail: "Looking for local executor".into(),
            output: String::new(),
            next_id: 1,
            page: Page::Editor,
            dark: settings.dark,
            profiles: loaded,
            selected_profile,
            current_path: None,
            scripts,
            active_script,
            last_saved,
            last_autosave: Instant::now(),
            launcher: load_launcher(),
            font_size: settings.font_size,
            rename_input,
            renaming: None,
            checked: std::collections::HashSet::new(),
            place_input,
            job_input,
            word_wrap: settings.word_wrap,
            indent: settings.indent,
            restore_tabs: settings.restore_tabs,
            redirect_output: settings.redirect_output,
            rscripts_key: settings.rscripts_key,
            console_output: String::new(),
            console_title: "Relapse Console".to_string(),
            console_input,
            pending_console_input: None,
            hub_input,
            hub_results: Vec::new(),
            hub_status: "Search rscripts.net for a script".to_string(),
            hub_loading: false,
            key_input,
            refresh: Instant::now(),
        }
    }

    pub(crate) fn execute(&mut self, cx: &mut Context<Self>) {
        self.flush_save(cx);
        let source = self.editor.read(cx).value().to_string();
        let id = self.next_id;
        if self.tx.send(WorkerCommand::Execute { id, source }).is_ok() {
            self.push_output(format!("Queued request {id}"));
            self.next_id += 1;
        } else {
            self.push_output("Connection worker stopped".to_string());
        }
        cx.notify();
    }

    fn flush_save(&mut self, cx: &Context<Self>) {
        let current = self.editor.read(cx).value().to_string();
        if current != self.last_saved {
            let _ = scripts::write(&self.active_script, &current);
            self.last_saved = current;
        }
    }

    fn autosave(&mut self, cx: &Context<Self>) {
        if self.last_autosave.elapsed() < Duration::from_millis(1200) {
            return;
        }
        self.last_autosave = Instant::now();
        self.flush_save(cx);
    }

    pub(crate) fn switch_script(&mut self, name: String, window: &mut Window, cx: &mut Context<Self>) {
        if name == self.active_script {
            return;
        }
        self.flush_save(cx);
        let source = scripts::read(&name).unwrap_or_default();
        self.last_saved = source.clone();
        self.editor = build_editor(source, window, cx);
        self.active_script = name;
        cx.notify();
    }

    pub(crate) fn new_script(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.flush_save(cx);
        match scripts::create() {
            Ok(name) => {
                self.scripts = scripts::load();
                self.last_saved = String::new();
                self.editor = build_editor(String::new(), window, cx);
                self.active_script = name.clone();
                self.push_output(format!("New script \"{name}\""));
            }
            Err(error) => self.push_output(format!("New script failed: {error}")),
        }
        cx.notify();
    }

    pub(crate) fn delete_script(&mut self, name: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.scripts.len() <= 1 {
            self.push_output("Keep at least one script".to_string());
            cx.notify();
            return;
        }
        match scripts::delete(&name) {
            Ok(()) => {
                self.push_output(format!("Deleted script \"{name}\""));
                self.scripts = scripts::load();
                if self.active_script == name {
                    let next = self
                        .scripts
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "script.lua".to_string());
                    let source = scripts::read(&next).unwrap_or_default();
                    self.last_saved = source.clone();
                    self.editor = build_editor(source, window, cx);
                    self.active_script = next;
                }
            }
            Err(error) => self.push_output(format!("Delete failed: {error}")),
        }
        cx.notify();
    }

    pub(crate) fn open_script(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let request = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: None,
        });
        cx.spawn_in(window, async move |view, cx| {
            let Ok(Ok(Some(paths))) = request.await else {
                return;
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            match std::fs::read_to_string(&path) {
                Ok(contents) => {
                    let _ = view.update_in(cx, |this, window, cx| {
                        this.flush_save(cx);
                        match scripts::import(&path.to_string_lossy(), &contents) {
                            Ok(name) => {
                                this.scripts = scripts::load();
                                this.last_saved = contents.clone();
                                this.editor = build_editor(contents, window, cx);
                                this.push_output(format!("Imported {name}"));
                                this.active_script = name;
                            }
                            Err(error) => this.push_output(format!("Import failed: {error}")),
                        }
                        cx.notify();
                    });
                }
                Err(error) => {
                    let _ = view.update(cx, |this, cx| {
                        this.push_output(format!("Open failed: {error}"));
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    pub(crate) fn save_script(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let contents = self.editor.read(cx).value().to_string();
        if let Some(path) = self.current_path.clone() {
            match std::fs::write(&path, contents) {
                Ok(()) => self.push_output(format!("Saved {}", path.display())),
                Err(error) => self.push_output(format!("Save failed: {error}")),
            }
            cx.notify();
            return;
        }
        let directory = home_directory();
        let request = cx.prompt_for_new_path(&directory, Some("script.lua"));
        cx.spawn_in(window, async move |view, cx| {
            let Ok(Ok(Some(path))) = request.await else {
                return;
            };
            let result = std::fs::write(&path, contents);
            let _ = view.update(cx, |this, cx| {
                match result {
                    Ok(()) => {
                        this.push_output(format!("Saved {}", path.display()));
                        this.current_path = Some(path);
                    }
                    Err(error) => this.push_output(format!("Save failed: {error}")),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn select_profile(&mut self, name: String, cx: &mut Context<Self>) {
        if self.selected_profile.as_ref() == Some(&name) {
            return;
        }
        self.selected_profile = Some(name.clone());
        let _ = self.tx.send(WorkerCommand::Target(Some(name)));
        cx.notify();
    }

    pub(crate) fn set_launcher(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let request = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: None,
        });
        cx.spawn_in(window, async move |view, cx| {
            let Ok(Ok(Some(paths))) = request.await else {
                return;
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            let _ = view.update(cx, |this, cx| {
                save_launcher(&path);
                this.push_output(format!("Launcher set to {}", path.display()));
                this.launcher = Some(path);
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn launch_profile(&mut self, name: String, cx: &mut Context<Self>) {
        let Some(launcher) = self.launcher.clone() else {
            self.push_output("Set an executable launcher in Settings first".to_string());
            cx.notify();
            return;
        };
        if self
            .profiles
            .iter()
            .any(|profile| profile.name == name && profile.running)
        {
            self.push_output(format!("Profile \"{name}\" is already running"));
            cx.notify();
            return;
        }
        let uri = match self.join_target(cx) {
            Ok(uri) => uri,
            Err(error) => {
                self.push_output(format!("Launch failed: {error}"));
                cx.notify();
                return;
            }
        };
        let mut command = Command::new(&launcher);
        command
            .arg("--profile")
            .arg(&name)
            .arg("--no-launch-ui")
            .env_remove("LD_PRELOAD")
            .env_remove("LD_AUDIT")
            .env_remove("LD_LIBRARY_PATH")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Some(uri) = &uri {
            command.arg("--").arg(uri);
        }
        unsafe {
            command.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
        match command.spawn() {
            Ok(_) => match &uri {
                Some(uri) => self.push_output(format!("Launching \"{name}\" into {uri}")),
                None => self.push_output(format!("Launching profile \"{name}\"")),
            },
            Err(error) => self.push_output(format!("Launch failed: {error}")),
        }
        cx.notify();
    }

    fn join_target(&self, cx: &Context<Self>) -> Result<Option<String>, String> {
        let place = self.place_input.read(cx).value().trim().to_string();
        let job = self.job_input.read(cx).value().trim().to_string();
        if place.is_empty() {
            if !job.is_empty() {
                return Err("A Job ID needs a Place ID".into());
            }
            return Ok(None);
        }
        let valid_place = !place.is_empty()
            && place.chars().all(|c| c.is_ascii_digit())
            && place.parse::<u64>().map(|id| id > 0).unwrap_or(false);
        if !valid_place {
            return Err("Place ID must be a positive number".into());
        }
        if !job.is_empty() && !is_job_id(&job) {
            return Err("Enter a valid Job ID".into());
        }
        let uri = if job.is_empty() {
            format!("roblox://placeId={place}")
        } else {
            format!("roblox://placeId={place}&gameInstanceId={job}")
        };
        Ok(Some(uri))
    }

    pub(crate) fn toggle_word_wrap(&mut self, cx: &mut Context<Self>) {
        self.word_wrap = !self.word_wrap;
        self.persist_settings();
        cx.notify();
    }

    pub(crate) fn set_indent(&mut self, width: u8, cx: &mut Context<Self>) {
        self.indent = width;
        self.persist_settings();
        cx.notify();
    }

    pub(crate) fn toggle_restore_tabs(&mut self, cx: &mut Context<Self>) {
        self.restore_tabs = !self.restore_tabs;
        self.persist_settings();
        cx.notify();
    }

    pub(crate) fn toggle_redirect_output(&mut self, cx: &mut Context<Self>) {
        self.redirect_output = !self.redirect_output;
        self.persist_settings();
        cx.notify();
    }

    pub(crate) fn persist_settings(&self) {
        save_settings(&Settings {
            dark: self.dark,
            font_size: self.font_size,
            word_wrap: self.word_wrap,
            indent: self.indent,
            restore_tabs: self.restore_tabs,
            redirect_output: self.redirect_output,
            rscripts_key: self.rscripts_key.clone(),
        });
    }

    pub(crate) fn set_font_size(&mut self, delta: f32, cx: &mut Context<Self>) {
        self.font_size = (self.font_size + delta).clamp(10.0, 20.0);
        crate::theme::set_font_size(self.font_size, cx);
        self.persist_settings();
        cx.notify();
    }

    pub(crate) fn refresh_profiles(&mut self, cx: &mut Context<Self>) {
        self.profiles = profiles::load();
        self.refresh = Instant::now();
        cx.notify();
    }

    pub(crate) fn new_profile(&mut self, cx: &mut Context<Self>) {
        let existing: std::collections::HashSet<String> =
            self.profiles.iter().map(|profile| profile.name.clone()).collect();
        let mut name = String::from("profile");
        let mut index = 1;
        while existing.contains(&name) {
            index += 1;
            name = format!("profile-{index}");
        }
        match profiles::create(&name) {
            Ok(()) => {
                self.push_output(format!("Created profile \"{name}\""));
                self.profiles = profiles::load();
                self.select_profile(name, cx);
            }
            Err(error) => {
                self.push_output(format!("Create failed: {error}"));
                cx.notify();
            }
        }
    }

    pub(crate) fn toggle_check(&mut self, name: String, cx: &mut Context<Self>) {
        if !self.checked.remove(&name) {
            self.checked.insert(name);
        }
        cx.notify();
    }

    pub(crate) fn clear_checked(&mut self, cx: &mut Context<Self>) {
        self.checked.clear();
        cx.notify();
    }

    pub(crate) fn launch_checked(&mut self, cx: &mut Context<Self>) {
        let mut names: Vec<String> = self.checked.iter().cloned().collect();
        names.sort();
        if names.is_empty() {
            self.push_output("No profiles selected".to_string());
            cx.notify();
            return;
        }
        for name in names {
            self.launch_profile(name, cx);
        }
    }

    pub(crate) fn start_rename(&mut self, name: String, window: &mut Window, cx: &mut Context<Self>) {
        let value = name.clone();
        self.rename_input.update(cx, |state, cx| {
            state.set_value(value, window, cx);
        });
        let _ = window;
        self.renaming = Some(name);
        cx.notify();
    }

    pub(crate) fn cancel_rename(&mut self, cx: &mut Context<Self>) {
        self.renaming = None;
        cx.notify();
    }

    pub(crate) fn commit_rename(&mut self, cx: &mut Context<Self>) {
        let Some(old) = self.renaming.take() else {
            cx.notify();
            return;
        };
        let replacement = self.rename_input.read(cx).value().trim().to_string();
        if replacement.is_empty() || replacement == old {
            cx.notify();
            return;
        }
        match profiles::rename(&old, &replacement) {
            Ok(()) => {
                self.push_output(format!("Renamed \"{old}\" to \"{replacement}\""));
                if self.checked.remove(&old) {
                    self.checked.insert(replacement.clone());
                }
                if self.selected_profile.as_deref() == Some(old.as_str()) {
                    self.select_profile(replacement, cx);
                }
                self.profiles = profiles::load();
            }
            Err(error) => self.push_output(format!("Rename failed: {error}")),
        }
        cx.notify();
    }

    pub(crate) fn delete_profile(&mut self, name: String, cx: &mut Context<Self>) {
        match profiles::delete(&name) {
            Ok(()) => {
                self.push_output(format!("Deleted profile \"{name}\""));
                self.checked.remove(&name);
                if self.selected_profile.as_deref() == Some(name.as_str()) {
                    self.selected_profile = None;
                    let _ = self.tx.send(WorkerCommand::Target(None));
                }
                self.profiles = profiles::load();
            }
            Err(error) => self.push_output(format!("Delete failed: {error}")),
        }
        cx.notify();
    }

    pub(crate) fn poll_events(&mut self, cx: &mut Context<Self>) {
        let mut changed = false;
        while let Ok(event) = self.events.try_recv() {
            changed = true;
            match event {
                Event::State { label, detail } => {
                    self.connection = match label.as_str() {
                        "READY" => "Ready".into(),
                        "WAITING" => "Not ready".into(),
                        "CONNECTING" => "Connecting".into(),
                        _ => "Disconnected".into(),
                    };
                    self.detail = detail;
                }
                Event::Result {
                    id,
                    submitted,
                    detail,
                } => {
                    if submitted {
                        self.push_output(format!("Request {id} submitted"));
                    } else {
                        self.push_output(format!("Request {id} failed: {detail}"));
                    }
                }
                Event::Log { level, text } => {
                    if matches!(level.as_str(), "info" | "print") {
                        self.push_output(text);
                    } else {
                        self.push_output(format!("{level}: {text}"));
                    }
                }
                Event::Output { level, text } => {
                    if matches!(level.as_str(), "info" | "print") {
                        self.push_output(text);
                    } else {
                        self.push_output(format!("{level}: {text}"));
                    }
                }
                Event::ConsoleOutput { level, text } => {
                    let line = if matches!(level.as_str(), "print" | "info") {
                        text
                    } else {
                        format!("[{level}] {text}")
                    };
                    self.push_console(line);
                }
                Event::ConsoleClear => self.console_output.clear(),
                Event::ConsoleTitle(title) => self.console_title = title,
                Event::ConsoleInput(id) => {
                    self.pending_console_input = Some(id);
                    self.push_console("[waiting for input]".to_string());
                }
                Event::Error(text) => self.push_output(text),
            }
        }
        if self.refresh.elapsed() >= REFRESH_INTERVAL {
            let before: Vec<(String, bool)> = self
                .profiles
                .iter()
                .map(|profile| (profile.name.clone(), profile.running))
                .collect();
            self.profiles = profiles::load();
            self.refresh = Instant::now();
            let after: Vec<(String, bool)> = self
                .profiles
                .iter()
                .map(|profile| (profile.name.clone(), profile.running))
                .collect();
            if before != after {
                changed = true;
            }
        }
        self.autosave(cx);
        if changed {
            cx.notify();
        }
    }

    pub(crate) fn hub_search(&mut self, cx: &mut Context<Self>) {
        let query = self.hub_input.read(cx).value().trim().to_string();
        if query.is_empty() {
            self.hub_status = "Type a search query".to_string();
            cx.notify();
            return;
        }
        if self.rscripts_key.trim().is_empty() {
            self.hub_status = "Set your rscripts API key in Settings".to_string();
            cx.notify();
            return;
        }
        let key = self.rscripts_key.clone();
        let url = format!(
            "https://api.rscripts.net/v1/scripts?sort=recommended&limit=24&q={}",
            encode_query(&query)
        );
        self.hub_loading = true;
        self.hub_status = "Searching...".to_string();
        cx.notify();
        cx.spawn(async move |view, cx| {
            let body = cx
                .background_spawn(async move { curl_get(&url, Some(&key)) })
                .await;
            let parsed = body.and_then(|text| parse_hits(&text));
            let _ = view.update(cx, |this, cx| {
                this.hub_loading = false;
                match parsed {
                    Ok(hits) => {
                        this.hub_status = if hits.is_empty() {
                            "No results".to_string()
                        } else {
                            format!("{} results", hits.len())
                        };
                        this.hub_results = hits;
                    }
                    Err(error) => {
                        this.hub_results.clear();
                        this.hub_status = error;
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn hub_load(
        &mut self,
        raw_url: String,
        title: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if raw_url.is_empty() {
            self.hub_status = "This script has no raw source".to_string();
            cx.notify();
            return;
        }
        self.hub_status = format!("Loading {title}...");
        cx.notify();
        cx.spawn_in(window, async move |view, cx| {
            let body = cx
                .background_spawn(async move { curl_get(&raw_url, None) })
                .await;
            let _ = view.update_in(cx, |this, window, cx| {
                match body {
                    Ok(source) => {
                        this.flush_save(cx);
                        let name = format!("{}.lua", sanitize_name(&title));
                        match scripts::import(&name, &source) {
                            Ok(saved) => {
                                this.scripts = scripts::load();
                                this.last_saved = source.clone();
                                this.editor = build_editor(source, window, cx);
                                this.active_script = saved.clone();
                                this.page = Page::Editor;
                                this.push_output(format!("Loaded \"{saved}\" from rscripts"));
                                this.hub_status = format!("Loaded {saved}");
                            }
                            Err(error) => this.hub_status = format!("Import failed: {error}"),
                        }
                    }
                    Err(error) => this.hub_status = error,
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn save_rscripts_key(&mut self, cx: &mut Context<Self>) {
        self.rscripts_key = self.key_input.read(cx).value().trim().to_string();
        self.persist_settings();
        self.hub_status = "API key saved".to_string();
        self.push_output("Saved rscripts API key".to_string());
        cx.notify();
    }

    fn push_output(&mut self, line: String) {
        if !self.output.is_empty() {
            self.output.push('\n');
        }
        self.output.push_str(&line);
        if self.output.len() > OUTPUT_LIMIT {
            let cut = self.output.len() - OUTPUT_LIMIT;
            let cut = self.output[cut..]
                .find('\n')
                .map(|offset| cut + offset + 1)
                .unwrap_or(cut);
            self.output.drain(..cut);
        }
    }

    fn push_console(&mut self, line: String) {
        if !self.console_output.is_empty() {
            self.console_output.push('\n');
        }
        self.console_output.push_str(&line);
        if self.console_output.len() > OUTPUT_LIMIT {
            let cut = self.console_output.len() - OUTPUT_LIMIT;
            let cut = self.console_output[cut..]
                .find('\n')
                .map(|offset| cut + offset + 1)
                .unwrap_or(cut);
            self.console_output.drain(..cut);
        }
    }

    pub(crate) fn clear_console(&mut self, cx: &mut Context<Self>) {
        self.console_output.clear();
        cx.notify();
    }

    pub(crate) fn submit_console_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.console_input.read(cx).value().to_string();
        match self.pending_console_input.take() {
            Some(id) => {
                let _ = self
                    .tx
                    .send(WorkerCommand::ConsoleReply { id, text: text.clone() });
                self.push_console(format!("> {text}"));
            }
            None => self.push_console("[no input requested]".to_string()),
        }
        self.console_input
            .update(cx, |state, cx| state.set_value("", window, cx));
        cx.notify();
    }
}
