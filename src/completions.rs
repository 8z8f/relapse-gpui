use std::ops::Range;

use gpui_kit::{
    App, Result, Task, Window,
    component::input::{CompletionProvider, Rope},
};
use lsp_types::{CompletionContext, CompletionItem, CompletionItemKind, CompletionResponse};

pub fn no_highlight(_query: &str, _item: &CompletionItem) -> Vec<Range<usize>> {
    Vec::new()
}

pub struct LuauCompletions {
    items: Vec<CompletionItem>,
}

impl LuauCompletions {
    pub fn new() -> Self {
        Self { items: build_items() }
    }
}

impl CompletionProvider for LuauCompletions {
    fn completions(
        &self,
        text: &Rope,
        offset: usize,
        _trigger: CompletionContext,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Task<Result<CompletionResponse>> {
        let prefix = word_prefix(text, offset);
        let items = if prefix.is_empty() {
            self.items.clone()
        } else {
            let needle = prefix.to_lowercase();
            self.items
                .iter()
                .filter(|item| item.label.to_lowercase().starts_with(&needle))
                .cloned()
                .collect()
        };
        Task::ready(Ok(CompletionResponse::Array(items)))
    }

    fn is_completion_trigger(&self, _offset: usize, new_text: &str, _cx: &mut App) -> bool {
        new_text
            .chars()
            .next()
            .map(|ch| ch.is_alphanumeric() || ch == '_' || ch == '.' || ch == ':')
            .unwrap_or(false)
    }
}

fn word_prefix(text: &Rope, offset: usize) -> String {
    let word_len: usize = text
        .chars_at(offset)
        .reversed()
        .take_while(|ch| ch.is_alphanumeric() || *ch == '_')
        .map(char::len_utf8)
        .sum();
    let start = offset.saturating_sub(word_len);
    text.slice(start..offset).to_string()
}

fn item(label: &str, kind: CompletionItemKind, detail: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(kind),
        detail: (!detail.is_empty()).then(|| detail.to_string()),
        ..Default::default()
    }
}

fn build_items() -> Vec<CompletionItem> {
    use CompletionItemKind as K;
    let mut items = Vec::new();

    let keywords = [
        "and", "break", "continue", "do", "else", "elseif", "end", "export", "false", "for",
        "function", "if", "in", "local", "nil", "not", "or", "repeat", "return", "then", "true",
        "type", "until", "while",
    ];
    for name in keywords {
        items.push(item(name, K::KEYWORD, "keyword"));
    }

    let globals = [
        "assert", "collectgarbage", "error", "gcinfo", "getfenv", "getmetatable", "ipairs",
        "loadstring", "newproxy", "next", "pairs", "pcall", "print", "rawequal", "rawget",
        "rawlen", "rawset", "require", "select", "setfenv", "setmetatable", "tonumber", "tostring",
        "type", "typeof", "unpack", "warn", "xpcall", "tick", "time", "wait", "delay", "spawn",
    ];
    for name in globals {
        items.push(item(name, K::FUNCTION, "global"));
    }

    let libraries = [
        "bit32", "buffer", "coroutine", "debug", "math", "os", "string", "table", "task", "utf8",
    ];
    for name in libraries {
        items.push(item(name, K::MODULE, "library"));
    }

    let roblox_globals = [
        "game", "workspace", "script", "shared", "plugin", "Enum", "Instance", "Vector3",
        "Vector2", "Vector3int16", "Vector2int16", "CFrame", "Color3", "ColorSequence",
        "ColorSequenceKeypoint", "NumberSequence", "NumberSequenceKeypoint", "NumberRange",
        "UDim", "UDim2", "Rect", "Ray", "Region3", "TweenInfo", "BrickColor", "PhysicalProperties",
        "Random", "DateTime", "Faces", "Axes", "Font",
    ];
    for name in roblox_globals {
        items.push(item(name, K::CLASS, "roblox"));
    }

    let services = [
        "Players", "Workspace", "ReplicatedStorage", "ReplicatedFirst", "ServerScriptService",
        "ServerStorage", "StarterGui", "StarterPack", "StarterPlayer", "Lighting", "RunService",
        "UserInputService", "TweenService", "HttpService", "Debris", "SoundService",
        "CollectionService", "ContextActionService", "MarketplaceService", "PathfindingService",
        "PhysicsService", "Teams", "Chat", "TextService", "GuiService", "LocalizationService",
        "TeleportService", "ProximityPromptService", "VirtualInputManager", "CoreGui",
    ];
    for name in services {
        items.push(item(name, K::CONSTANT, "service"));
    }

    let members = [
        "GetService", "FindFirstChild", "FindFirstChildOfClass", "FindFirstChildWhichIsA",
        "FindFirstAncestor", "WaitForChild", "GetChildren", "GetDescendants", "IsA", "Clone",
        "Destroy", "Connect", "Once", "Wait", "Fire", "FireServer", "FireClient",
        "FireAllClients", "InvokeServer", "InvokeClient", "GetAttribute", "SetAttribute",
        "GetPropertyChangedSignal", "GetPlayers", "GetMouse", "GetChildren", "GetFullName",
    ];
    for name in members {
        items.push(item(name, K::METHOD, "method"));
    }

    let executor = [
        "getgenv", "getrenv", "getreg", "getgc", "getloadedmodules", "getconnections",
        "getcallbackvalue", "getinstances", "getnilinstances", "getscriptclosure",
        "firesignal", "fireclickdetector", "firetouchinterest", "fireproximityprompt",
        "hookfunction", "hookmetamethod", "getrawmetatable", "setrawmetatable", "setreadonly",
        "isreadonly", "newcclosure", "iscclosure", "islclosure", "checkcaller", "clonefunction",
        "getnamecallmethod", "setnamecallmethod", "getcallingscript", "setclipboard",
        "identifyexecutor", "getexecutorname", "request", "writefile", "readfile", "appendfile",
        "listfiles", "isfile", "isfolder", "makefolder", "delfile", "delfolder", "getcustomasset",
        "loadfile", "dofile", "queue_on_teleport", "mouse1click", "mouse1press", "mouse1release",
        "mouse2click", "keypress", "keyrelease", "HttpGet", "HttpPost", "rconsoleprint",
        "rconsoleinfo", "rconsolewarn", "rconsoleerr", "rconsoleinput", "rconsoleclear",
        "rconsolename",
    ];
    for name in executor {
        items.push(item(name, K::FUNCTION, "executor"));
    }

    items
}
