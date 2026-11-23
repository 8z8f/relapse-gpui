use std::{
    collections::{HashSet, VecDeque},
    io::{self, Read, Write},
    os::unix::fs::FileTypeExt,
    os::unix::net::UnixStream,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender, TryRecvError},
    },
    thread,
    time::{Duration, Instant},
};

use serde_json::{Value, json};

const DEFAULT_SOCKET: &str = "/tmp/relapse.sock";
const CHANNEL: &str = "@main";
const SESSION_CHANNEL: &str = "relapsegtk.session.v2";
const SESSION_SCRIPT: &str = include_str!("session.luau");
const CONSOLE_CHANNEL: &str = "relapsegtk.console.v1";
const CONSOLE_SCRIPT: &str = include_str!("console.luau");

type ConsoleQueue = Arc<Mutex<VecDeque<(i64, String)>>>;
const MAX_FRAME: usize = 8 * 1024 * 1024;
const BRIDGE_RETRY: Duration = Duration::from_millis(250);

pub enum WorkerCommand {
    Execute { id: i64, source: String },
    Target(Option<String>),
    ConsoleReply { id: i64, text: String },
}

pub enum Event {
    State {
        label: String,
        detail: String,
    },
    Result {
        id: i64,
        submitted: bool,
        detail: String,
    },
    Log {
        level: String,
        text: String,
    },
    Output {
        level: String,
        text: String,
    },
    ConsoleOutput {
        level: String,
        text: String,
    },
    ConsoleClear,
    ConsoleTitle(String),
    ConsoleInput(i64),
    Error(String),
}

pub fn start(commands: Receiver<WorkerCommand>, events: Sender<Event>) {
    thread::Builder::new()
        .name("relapse-protocol".into())
        .spawn(move || run(commands, events))
        .expect("start socket worker");
}

fn run(commands: Receiver<WorkerCommand>, events: Sender<Event>) {
    let configured_path = std::env::var("RELAPSE_SOCKET").ok();
    let mut target: Option<String> = None;
    let mut pending: Vec<(i64, String)> = Vec::new();
    let console_queue: ConsoleQueue = Arc::new(Mutex::new(VecDeque::new()));
    loop {
        loop {
            match commands.try_recv() {
                Ok(WorkerCommand::Target(next)) => target = next,
                Ok(WorkerCommand::Execute { id, source }) => pending.push((id, source)),
                Ok(WorkerCommand::ConsoleReply { .. }) => {}
                Err(TryRecvError::Disconnected) => return,
                Err(TryRecvError::Empty) => break,
            }
        }
        let paths = configured_path
            .clone()
            .map(|path| vec![path])
            .unwrap_or_else(|| discover_sockets(&target));
        if paths.is_empty() {
            let detail = match &target {
                Some(name) => format!("Profile \"{name}\" is not running"),
                None => "No active Relapse socket in /tmp".to_string(),
            };
            let _ = events.send(Event::State {
                label: "DISCONNECTED".into(),
                detail,
            });
            thread::sleep(Duration::from_secs(1));
            continue;
        }
        for path in paths {
            let _ = events.send(Event::State {
                label: "CONNECTING".into(),
                detail: format!("Connecting to {path}"),
            });
            match open(&path, CHANNEL) {
                Ok((mut stream, max_message_bytes, buffer)) => {
                    if let Err(error) = connected_loop(
                        &path,
                        &mut stream,
                        &commands,
                        &events,
                        max_message_bytes,
                        buffer,
                        &mut target,
                        &mut pending,
                        &console_queue,
                    ) {
                        let _ = events.send(Event::Error(format!("Socket closed: {error}")));
                    }
                    break;
                }
                Err(error) => {
                    let _ = events.send(Event::State {
                        label: "DISCONNECTED".into(),
                        detail: format!("{path}: {error}"),
                    });
                }
            }
        }
        thread::sleep(Duration::from_secs(1));
    }
}

fn discover_sockets(target: &Option<String>) -> Vec<String> {
    let mut paths = std::fs::read_dir("/tmp")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_str()?;
            let is_relapse_socket =
                name == "relapse.sock" || (name.starts_with("relapse-") && name.ends_with(".sock"));
            if !is_relapse_socket || !entry.file_type().ok()?.is_socket() {
                return None;
            }
            if !socket_matches(name, target) {
                return None;
            }
            Some(entry.path().to_string_lossy().into_owned())
        })
        .collect::<Vec<_>>();
    paths.sort_by_key(|path| (path != DEFAULT_SOCKET, path.clone()));
    paths
}

fn socket_matches(name: &str, target: &Option<String>) -> bool {
    let Some(profile) = target else {
        return true;
    };
    name == format!("relapse-{profile}.sock")
}

fn open(path: &str, channel: &str) -> io::Result<(UnixStream, usize, Vec<u8>)> {
    let mut stream = UnixStream::connect(path)?;
    let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let mut buffer = Vec::new();
    let limit = handshake(&mut stream, &mut buffer, channel)?;
    Ok((stream, limit, buffer))
}

fn handshake(stream: &mut UnixStream, buffer: &mut Vec<u8>, channel: &str) -> io::Result<usize> {
    write_frame(stream, &json!({"version": 1, "channel": channel}))?;
    let start = Instant::now();
    loop {
        if start.elapsed() > Duration::from_secs(5) {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "opening timed out"));
        }
        if let Some(frame) = read_frame(stream, buffer)? {
            let reply: Value = serde_json::from_slice(&frame).map_err(invalid_data)?;
            if reply["ok"].as_bool() != Some(true) || reply["version"].as_i64() != Some(1) {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionRefused,
                    reply["error"].as_str().unwrap_or("channel rejected"),
                ));
            }
            let limit = reply["max_message_bytes"].as_u64().unwrap_or(0);
            if limit == 0 {
                return Err(invalid_data("invalid channel message limit"));
            }
            return Ok((limit as usize).min(MAX_FRAME));
        }
    }
}

struct Bridge {
    session: i64,
    ready: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
}

impl Drop for Bridge {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}

struct ConsoleBridge {
    session: i64,
    stop: Arc<AtomicBool>,
}

impl Drop for ConsoleBridge {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}

fn connected_loop(
    path: &str,
    stream: &mut UnixStream,
    commands: &Receiver<WorkerCommand>,
    events: &Sender<Event>,
    max_message_bytes: usize,
    mut buffer: Vec<u8>,
    target: &mut Option<String>,
    pending: &mut Vec<(i64, String)>,
    console_queue: &ConsoleQueue,
) -> io::Result<()> {
    let mut session = 0i64;
    let mut hello = false;
    let mut ready = false;
    let mut queued: Vec<(i64, String)> = std::mem::take(pending);
    let mut internal: HashSet<i64> = HashSet::new();
    let mut internal_id = 1i64 << 62;
    let mut bridge: Option<Bridge> = None;
    let mut console: Option<ConsoleBridge> = None;
    loop {
        match commands.try_recv() {
            Ok(WorkerCommand::Execute { id, source }) => {
                if ready {
                    execute(stream, id, session, &source, bridge.as_ref(), max_message_bytes)?;
                } else {
                    queued.push((id, source));
                }
            }
            Ok(WorkerCommand::Target(next)) => {
                if *target != next {
                    *target = next;
                    *pending = queued;
                    return Ok(());
                }
            }
            Ok(WorkerCommand::ConsoleReply { id, text }) => {
                if let Ok(mut queue) = console_queue.lock() {
                    queue.push_back((id, text));
                }
            }
            Err(TryRecvError::Disconnected) => return Ok(()),
            Err(TryRecvError::Empty) => {}
        }
        let Some(frame) = read_frame(stream, &mut buffer)? else {
            continue;
        };
        let message: Value = serde_json::from_slice(&frame).map_err(invalid_data)?;
        match message["type"].as_str().unwrap_or("") {
            "hello" | "status" => {
                let ty = message["type"].as_str().unwrap();
                if !hello && ty != "hello" {
                    return Err(invalid_data("expected hello"));
                }
                if ty == "hello" && hello {
                    return Err(invalid_data("duplicate hello"));
                }
                hello = true;
                session = message["session"]
                    .as_i64()
                    .ok_or_else(|| invalid_data("missing session"))?;
                ready = message["ready"]
                    .as_bool()
                    .ok_or_else(|| invalid_data("missing ready flag"))?;
                if session < 0 || (ready && session == 0) {
                    return Err(invalid_data("invalid session"));
                }
                let _ = events.send(Event::State {
                    label: if ready { "READY" } else { "WAITING" }.into(),
                    detail: if ready {
                        "Executor ready".to_string()
                    } else {
                        "Waiting for a game".to_string()
                    },
                });
                if ready {
                    start_bridge(
                        path,
                        stream,
                        session,
                        max_message_bytes,
                        events,
                        &mut internal,
                        &mut internal_id,
                        &mut bridge,
                    )?;
                    start_console(
                        path,
                        stream,
                        session,
                        max_message_bytes,
                        events,
                        &mut internal,
                        &mut internal_id,
                        &mut console,
                        console_queue,
                    )?;
                    for (id, source) in queued.drain(..) {
                        execute(stream, id, session, &source, bridge.as_ref(), max_message_bytes)?;
                    }
                } else {
                    bridge = None;
                    console = None;
                }
            }
            "exec_result" => {
                let id = message["script_id"].as_i64().unwrap_or(0);
                if internal.remove(&id) {
                    continue;
                }
                let phase = message["phase"].as_str().unwrap_or("");
                let _ = events.send(Event::Result {
                    id,
                    submitted: phase == "submitted",
                    detail: message["detail"]
                        .as_str()
                        .unwrap_or("Request failed")
                        .to_owned(),
                });
            }
            "log" => {
                let _ = events.send(Event::Log {
                    level: message["level"].as_str().unwrap_or("info").to_owned(),
                    text: message["text"].as_str().unwrap_or("").to_owned(),
                });
            }
            "error" => {
                let _ = events.send(Event::Error(
                    message["detail"]
                        .as_str()
                        .unwrap_or("Request failed")
                        .to_owned(),
                ));
            }
            _ => {}
        }
    }
}

fn start_bridge(
    path: &str,
    stream: &mut UnixStream,
    session: i64,
    max_message_bytes: usize,
    events: &Sender<Event>,
    internal: &mut HashSet<i64>,
    internal_id: &mut i64,
    bridge: &mut Option<Bridge>,
) -> io::Result<()> {
    if bridge.as_ref().map(|existing| existing.session) == Some(session) {
        return Ok(());
    }
    *bridge = None;
    let id = *internal_id;
    *internal_id += 1;
    internal.insert(id);
    write_execute(
        stream,
        id,
        session,
        "relapsegtk-session.luau",
        SESSION_SCRIPT,
        max_message_bytes,
    )?;
    let ready = Arc::new(AtomicBool::new(false));
    let stop = Arc::new(AtomicBool::new(false));
    let thread_ready = ready.clone();
    let thread_stop = stop.clone();
    let thread_events = events.clone();
    let thread_path = path.to_owned();
    thread::Builder::new()
        .name("relapse-output".into())
        .spawn(move || bridge_loop(thread_path, thread_ready, thread_stop, thread_events))
        .expect("start output bridge");
    *bridge = Some(Bridge {
        session,
        ready,
        stop,
    });
    Ok(())
}

fn start_console(
    path: &str,
    stream: &mut UnixStream,
    session: i64,
    max_message_bytes: usize,
    events: &Sender<Event>,
    internal: &mut HashSet<i64>,
    internal_id: &mut i64,
    console: &mut Option<ConsoleBridge>,
    queue: &ConsoleQueue,
) -> io::Result<()> {
    if console.as_ref().map(|existing| existing.session) == Some(session) {
        return Ok(());
    }
    *console = None;
    if let Ok(mut pending) = queue.lock() {
        pending.clear();
    }
    let id = *internal_id;
    *internal_id += 1;
    internal.insert(id);
    write_execute(
        stream,
        id,
        session,
        "relapsegtk-console.luau",
        CONSOLE_SCRIPT,
        max_message_bytes,
    )?;
    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = stop.clone();
    let thread_events = events.clone();
    let thread_path = path.to_owned();
    let thread_queue = queue.clone();
    thread::Builder::new()
        .name("relapse-console".into())
        .spawn(move || console_loop(thread_path, thread_stop, thread_events, thread_queue))
        .expect("start console bridge");
    *console = Some(ConsoleBridge { session, stop });
    Ok(())
}

fn execute(
    stream: &mut UnixStream,
    id: i64,
    session: i64,
    source: &str,
    bridge: Option<&Bridge>,
    max_message_bytes: usize,
) -> io::Result<()> {
    let bridged = bridge
        .map(|bridge| bridge.ready.load(Ordering::Acquire))
        .unwrap_or(false);
    if bridged {
        let wrapped = format!(
            "getgenv().__relapsegtk_session_v2.execute({},{})",
            lua_string(source),
            lua_string("editor.luau")
        );
        write_execute(
            stream,
            id,
            session,
            "relapsegtk-execute.luau",
            &wrapped,
            max_message_bytes,
        )
    } else {
        write_execute(stream, id, session, "editor.luau", source, max_message_bytes)
    }
}

fn write_execute(
    stream: &mut UnixStream,
    id: i64,
    session: i64,
    name: &str,
    source: &str,
    max_message_bytes: usize,
) -> io::Result<()> {
    let bytes = serde_json::to_vec(&json!({
        "op": "execute",
        "script_id": id,
        "session": session,
        "name": name,
        "source": source,
    }))
    .map_err(invalid_data)?;
    write_bytes(stream, &bytes, max_message_bytes)
}

fn lua_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            '\n' => out.push_str("\\010"),
            '\r' => out.push_str("\\013"),
            '\t' => out.push_str("\\009"),
            ch if (ch as u32) < 32 || ch as u32 == 127 => {
                out.push_str(&format!("\\{:03}", ch as u32));
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn bridge_loop(
    path: String,
    ready: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    events: Sender<Event>,
) {
    while !stop.load(Ordering::Acquire) {
        let (mut stream, _limit, mut buffer) = match open(&path, SESSION_CHANNEL) {
            Ok(connection) => connection,
            Err(_) => {
                thread::sleep(BRIDGE_RETRY);
                continue;
            }
        };
        if write_frame(
            &mut stream,
            &json!({"op": "configure", "version": 2, "redirect": true}),
        )
        .is_err()
        {
            thread::sleep(BRIDGE_RETRY);
            continue;
        }
        loop {
            if stop.load(Ordering::Acquire) {
                break;
            }
            match read_frame(&mut stream, &mut buffer) {
                Ok(Some(frame)) => match serde_json::from_slice::<Value>(&frame) {
                    Ok(message) => {
                        if !handle_bridge_message(&message, &ready, &events) {
                            break;
                        }
                    }
                    Err(_) => break,
                },
                Ok(None) => {}
                Err(_) => break,
            }
        }
        ready.store(false, Ordering::Release);
        if !stop.load(Ordering::Acquire) {
            thread::sleep(BRIDGE_RETRY);
        }
    }
    ready.store(false, Ordering::Release);
}

fn handle_bridge_message(message: &Value, ready: &AtomicBool, events: &Sender<Event>) -> bool {
    match message["type"].as_str().unwrap_or("") {
        "configured" => {
            if message["version"].as_i64() == Some(2)
                && message["redirect"].as_bool() == Some(true)
            {
                ready.store(true, Ordering::Release);
                true
            } else {
                false
            }
        }
        "failed" => {
            let _ = events.send(Event::Log {
                level: "error".into(),
                text: message["detail"]
                    .as_str()
                    .unwrap_or("Output channel failed")
                    .to_owned(),
            });
            false
        }
        "output" => {
            let Some(entries) = message["entries"].as_array() else {
                return false;
            };
            if entries.is_empty() || entries.len() > 32 {
                return false;
            }
            for entry in entries {
                let level = entry["level"].as_str().unwrap_or("print");
                let text = entry["text"].as_str().unwrap_or("");
                if !matches!(level, "print" | "warn" | "error" | "info") || text.len() > 4096 {
                    return false;
                }
                let _ = events.send(Event::Output {
                    level: level.to_owned(),
                    text: text.to_owned(),
                });
            }
            let dropped = message["dropped"].as_i64().unwrap_or(0);
            if dropped > 0 {
                let _ = events.send(Event::Log {
                    level: "warn".into(),
                    text: format!("{dropped} messages dropped: output arrived too quickly."),
                });
            }
            true
        }
        "dropped" => {
            let count = message["count"].as_i64().unwrap_or(0).clamp(1, i32::MAX as i64);
            let _ = events.send(Event::Log {
                level: "warn".into(),
                text: format!("{count} messages dropped: output arrived too quickly."),
            });
            true
        }
        _ => false,
    }
}

fn console_loop(path: String, stop: Arc<AtomicBool>, events: Sender<Event>, queue: ConsoleQueue) {
    while !stop.load(Ordering::Acquire) {
        let (mut stream, _limit, mut buffer) = match open(&path, CONSOLE_CHANNEL) {
            Ok(connection) => connection,
            Err(_) => {
                thread::sleep(BRIDGE_RETRY);
                continue;
            }
        };
        loop {
            if stop.load(Ordering::Acquire) {
                break;
            }
            let replies: Vec<(i64, String)> = match queue.lock() {
                Ok(mut pending) => pending.drain(..).collect(),
                Err(_) => Vec::new(),
            };
            let mut broken = false;
            for (id, text) in replies {
                if write_frame(
                    &mut stream,
                    &json!({"op": "input_response", "id": id, "text": text}),
                )
                .is_err()
                {
                    broken = true;
                    break;
                }
            }
            if broken {
                break;
            }
            match read_frame(&mut stream, &mut buffer) {
                Ok(Some(frame)) => match serde_json::from_slice::<Value>(&frame) {
                    Ok(message) => {
                        if !handle_console_message(&message, &events) {
                            break;
                        }
                    }
                    Err(_) => break,
                },
                Ok(None) => {}
                Err(_) => break,
            }
        }
        if !stop.load(Ordering::Acquire) {
            thread::sleep(BRIDGE_RETRY);
        }
    }
}

fn handle_console_message(message: &Value, events: &Sender<Event>) -> bool {
    match message["type"].as_str().unwrap_or("") {
        "state" => {
            let title = message["title"]
                .as_str()
                .unwrap_or("Relapse Console")
                .to_owned();
            let _ = events.send(Event::ConsoleTitle(title));
            true
        }
        "title" => {
            let Some(title) = message["text"].as_str() else {
                return false;
            };
            if title.len() > 255 {
                return false;
            }
            let _ = events.send(Event::ConsoleTitle(title.to_owned()));
            true
        }
        "clear" => {
            let _ = events.send(Event::ConsoleClear);
            true
        }
        "print" => {
            let level = message["level"].as_str().unwrap_or("print").to_owned();
            let text = message["text"].as_str().unwrap_or("").to_owned();
            if text.len() > 8192 {
                return false;
            }
            let _ = events.send(Event::ConsoleOutput { level, text });
            true
        }
        "input" => {
            let id = message["id"].as_i64().unwrap_or(0);
            if id < 1 {
                return false;
            }
            let _ = events.send(Event::ConsoleInput(id));
            true
        }
        "show" | "hide" => true,
        _ => false,
    }
}

fn write_frame(stream: &mut UnixStream, value: &Value) -> io::Result<()> {
    let bytes = serde_json::to_vec(value).map_err(invalid_data)?;
    write_bytes(stream, &bytes, MAX_FRAME)
}

fn write_bytes(stream: &mut UnixStream, bytes: &[u8], max_message_bytes: usize) -> io::Result<()> {
    if bytes.len() > max_message_bytes {
        return Err(invalid_data("message exceeds channel limit"));
    }
    stream.write_all(&(bytes.len() as u32).to_be_bytes())?;
    stream.write_all(bytes)?;
    stream.flush()
}

fn read_frame(stream: &mut UnixStream, buffer: &mut Vec<u8>) -> io::Result<Option<Vec<u8>>> {
    if let Some(frame) = take_frame(buffer)? {
        return Ok(Some(frame));
    }
    let mut chunk = [0u8; 16 * 1024];
    match stream.read(&mut chunk) {
        Ok(0) => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "peer closed")),
        Ok(count) => buffer.extend_from_slice(&chunk[..count]),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted
            ) =>
        {
            return Ok(None);
        }
        Err(error) => return Err(error),
    }
    take_frame(buffer)
}

fn take_frame(buffer: &mut Vec<u8>) -> io::Result<Option<Vec<u8>>> {
    if buffer.len() < 4 {
        return Ok(None);
    }
    let length = u32::from_be_bytes(buffer[..4].try_into().unwrap()) as usize;
    if length > MAX_FRAME {
        return Err(invalid_data("frame exceeds 8 MiB limit"));
    }
    if buffer.len() < length + 4 {
        return Ok(None);
    }
    let frame = buffer[4..length + 4].to_vec();
    buffer.drain(..length + 4);
    Ok(Some(frame))
}

fn invalid_data(error: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}
