//! Direct Discord control over the local IPC (named pipe / unix socket): sets Discord's own
//! microphone mute through `SET_VOICE_SETTINGS`, so no keybind is needed and Discord's state can
//! never drift from the headset's.
//!
//! Needs a (free) Discord application: Client ID + Client Secret from the developer portal, with
//! redirect `http://localhost`. The first connection asks for approval inside the Discord client;
//! the tokens are stored next to the config (`discord_auth.json`) and refreshed automatically.

use crate::config::{Config, DiscordMode};
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

const REDIRECT_URI: &str = "http://localhost";
const SCOPES: [&str; 2] = ["rpc", "rpc.voice.write"];

static STARTED: AtomicBool = AtomicBool::new(false);
static GEN: AtomicU64 = AtomicU64::new(0);
lazy_static::lazy_static! {
    static ref DESIRED: Mutex<Option<bool>> = Mutex::new(None);
    static ref STATUS: Mutex<String> = Mutex::new(String::from("выключено"));
}

/// Human-readable connection status for the UI.
pub fn status() -> String {
    STATUS.lock().clone()
}

fn set_status(s: &str) {
    let mut cur = STATUS.lock();
    if *cur != s {
        log::info!("[DiscordRPC] {}", s);
        *cur = s.to_string();
    }
}

/// Ask Discord to mute/unmute the microphone (applied by the background worker).
pub fn set_mute(muted: bool) {
    *DESIRED.lock() = Some(muted);
    GEN.fetch_add(1, Ordering::SeqCst);
}

/// Start the background worker once (idempotent). It stays idle unless Discord mode is `Direct`.
pub fn ensure_started() {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::Builder::new()
        .name("discord-rpc".into())
        .spawn(worker)
        .ok();
}

// ---------------------------------------------------------------- transport

struct Conn {
    #[cfg(windows)]
    f: std::fs::File,
    #[cfg(unix)]
    s: std::os::unix::net::UnixStream,
}

#[cfg(windows)]
mod win {
    #[link(name = "kernel32")]
    extern "system" {
        pub fn PeekNamedPipe(h: isize, buf: *mut u8, size: u32, read: *mut u32, avail: *mut u32, left: *mut u32) -> i32;
    }
}

impl Conn {
    fn open() -> std::io::Result<Conn> {
        #[cfg(windows)]
        {
            for i in 0..10 {
                let path = format!(r"\\.\pipe\discord-ipc-{}", i);
                if let Ok(f) = std::fs::OpenOptions::new().read(true).write(true).open(&path) {
                    return Ok(Conn { f });
                }
            }
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Discord не запущен (нет discord-ipc)"))
        }
        #[cfg(unix)]
        {
            let mut dirs: Vec<String> = Vec::new();
            for var in ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"] {
                if let Ok(d) = std::env::var(var) {
                    dirs.push(d);
                }
            }
            dirs.push("/tmp".into());
            let base = dirs.clone();
            for d in base {
                dirs.push(format!("{}/app/com.discordapp.Discord", d));
                dirs.push(format!("{}/snap.discord", d));
            }
            for d in dirs {
                for i in 0..10 {
                    if let Ok(s) = std::os::unix::net::UnixStream::connect(format!("{}/discord-ipc-{}", d, i)) {
                        return Ok(Conn { s });
                    }
                }
            }
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Discord не запущен (нет discord-ipc)"))
        }
    }

    /// True when at least one byte can be read without blocking.
    fn readable(&mut self) -> bool {
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            let mut avail = 0u32;
            let ok = unsafe {
                win::PeekNamedPipe(self.f.as_raw_handle() as isize, std::ptr::null_mut(), 0, std::ptr::null_mut(), &mut avail, std::ptr::null_mut())
            };
            ok != 0 && avail > 0
        }
        #[cfg(unix)]
        {
            let _ = self.s.set_read_timeout(Some(Duration::from_millis(30)));
            let mut b = [0u8; 1];
            // Reading would consume the byte, so peek via a non-consuming recv.
            use std::os::unix::io::AsRawFd;
            let n = unsafe { recv_peek(self.s.as_raw_fd(), &mut b) };
            n > 0
        }
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        #[cfg(windows)]
        {
            self.f.read_exact(buf)
        }
        #[cfg(unix)]
        {
            let _ = self.s.set_read_timeout(Some(Duration::from_secs(10)));
            self.s.read_exact(buf)
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        #[cfg(windows)]
        {
            self.f.write_all(buf)
        }
        #[cfg(unix)]
        {
            self.s.write_all(buf)
        }
    }

    fn send(&mut self, op: u32, v: &Value) -> std::io::Result<()> {
        let body = v.to_string();
        let mut frame = Vec::with_capacity(8 + body.len());
        frame.extend_from_slice(&op.to_le_bytes());
        frame.extend_from_slice(&(body.len() as u32).to_le_bytes());
        frame.extend_from_slice(body.as_bytes());
        self.write_all(&frame)
    }

    fn recv(&mut self) -> std::io::Result<(u32, Value)> {
        let mut hdr = [0u8; 8];
        self.read_exact(&mut hdr)?;
        let op = u32::from_le_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]);
        let len = u32::from_le_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as usize;
        if len > 1_000_000 {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "bad frame"));
        }
        let mut body = vec![0u8; len];
        self.read_exact(&mut body)?;
        Ok((op, serde_json::from_slice(&body).unwrap_or(Value::Null)))
    }

    /// Read frames until `want` returns Some, answering PINGs; gives up after `timeout`.
    fn wait_for<T>(&mut self, timeout: Duration, mut want: impl FnMut(u32, &Value) -> Option<T>) -> anyhow::Result<T> {
        let start = Instant::now();
        loop {
            if start.elapsed() > timeout {
                anyhow::bail!("таймаут ответа Discord");
            }
            if !self.readable() {
                std::thread::sleep(Duration::from_millis(40));
                continue;
            }
            let (op, v) = self.recv()?;
            match op {
                2 => anyhow::bail!("Discord закрыл соединение"),
                3 => self.send(4, &v)?,
                _ => {
                    if let Some(r) = want(op, &v) {
                        return Ok(r);
                    }
                }
            }
        }
    }
}

#[cfg(unix)]
unsafe fn recv_peek(fd: i32, buf: &mut [u8]) -> isize {
    extern "C" {
        fn recv(fd: i32, buf: *mut u8, len: usize, flags: i32) -> isize;
    }
    const MSG_PEEK: i32 = 0x2;
    const MSG_DONTWAIT: i32 = if cfg!(target_os = "macos") { 0x80 } else { 0x40 };
    recv(fd, buf.as_mut_ptr(), buf.len(), MSG_PEEK | MSG_DONTWAIT)
}

// ---------------------------------------------------------------- auth

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
struct Tokens {
    client_id: String,
    access_token: String,
    refresh_token: String,
    expires_at: u64,
}

fn tokens_path() -> std::path::PathBuf {
    Config::path().with_file_name("discord_auth.json")
}
fn load_tokens() -> Tokens {
    std::fs::read_to_string(tokens_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}
fn save_tokens(t: &Tokens) {
    if let Ok(s) = serde_json::to_string_pretty(t) {
        let _ = std::fs::write(tokens_path(), s);
    }
}
fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn token_request(form: &[(&str, &str)]) -> anyhow::Result<Value> {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    let form: Vec<(String, String)> = form.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    rt.block_on(async move {
        let resp = reqwest::Client::new().post("https://discord.com/api/oauth2/token").form(&form).send().await?;
        let status = resp.status();
        let v: Value = resp.json().await?;
        if !status.is_success() {
            anyhow::bail!("Discord отклонил токен: {}", v);
        }
        Ok(v)
    })
}

fn store_token_response(client_id: &str, v: &Value, fallback_refresh: &str) -> anyhow::Result<Tokens> {
    let access = v["access_token"].as_str().ok_or_else(|| anyhow::anyhow!("нет access_token"))?.to_string();
    let t = Tokens {
        client_id: client_id.to_string(),
        access_token: access,
        refresh_token: v["refresh_token"].as_str().unwrap_or(fallback_refresh).to_string(),
        expires_at: now_secs() + v["expires_in"].as_u64().unwrap_or(604_800),
    };
    save_tokens(&t);
    Ok(t)
}

fn authorize(conn: &mut Conn, client_id: &str, secret: &str) -> anyhow::Result<Tokens> {
    set_status("Подтвердите доступ в окне Discord");
    conn.send(1, &json!({"cmd": "AUTHORIZE", "nonce": "auth", "args": {"client_id": client_id, "scopes": SCOPES}}))?;
    let code = conn.wait_for(Duration::from_secs(180), |_, v| {
        if v["nonce"] == "auth" {
            Some(if v["evt"] == "ERROR" {
                Err(v["data"]["message"].as_str().unwrap_or("отказано").to_string())
            } else {
                Ok(v["data"]["code"].as_str().unwrap_or("").to_string())
            })
        } else {
            None
        }
    })?;
    let code = code.map_err(|e| anyhow::anyhow!("авторизация: {}", e))?;
    set_status("Получение токена…");
    let v = token_request(&[
        ("grant_type", "authorization_code"),
        ("code", &code),
        ("client_id", client_id),
        ("client_secret", secret),
        ("redirect_uri", REDIRECT_URI),
    ])?;
    store_token_response(client_id, &v, "")
}

fn refresh(client_id: &str, secret: &str, t: &Tokens) -> anyhow::Result<Tokens> {
    let v = token_request(&[
        ("grant_type", "refresh_token"),
        ("refresh_token", &t.refresh_token),
        ("client_id", client_id),
        ("client_secret", secret),
    ])?;
    store_token_response(client_id, &v, &t.refresh_token)
}

fn authenticate(conn: &mut Conn, token: &str) -> anyhow::Result<bool> {
    conn.send(1, &json!({"cmd": "AUTHENTICATE", "nonce": "authn", "args": {"access_token": token}}))?;
    conn.wait_for(Duration::from_secs(10), |_, v| {
        if v["nonce"] == "authn" {
            Some(v["evt"] != "ERROR")
        } else {
            None
        }
    })
}

// ---------------------------------------------------------------- worker

fn wanted() -> Option<(String, String)> {
    let cfg = Config::load().ok()?;
    if cfg.discord.mode != DiscordMode::Direct {
        set_status("выключено (режим Discord не «Прямой»)");
        return None;
    }
    let id = cfg.discord.direct.app_id.trim().to_string();
    let secret = cfg.discord.direct.client_secret.trim().to_string();
    if id.is_empty() || secret.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        set_status("укажите Client ID и Client Secret");
        return None;
    }
    Some((id, secret))
}

fn session(client_id: &str, secret: &str) -> anyhow::Result<()> {
    set_status("подключение к Discord…");
    let mut conn = Conn::open()?;
    conn.send(0, &json!({"v": 1, "client_id": client_id}))?;
    conn.wait_for(Duration::from_secs(10), |_, v| if v["evt"] == "READY" { Some(()) } else { None })?;

    let mut tokens = load_tokens();
    if tokens.client_id != client_id {
        tokens = Tokens::default();
    }
    if !tokens.access_token.is_empty() && tokens.expires_at < now_secs() + 120 && !tokens.refresh_token.is_empty() {
        match refresh(client_id, secret, &tokens) {
            Ok(t) => tokens = t,
            Err(e) => {
                log::warn!("[DiscordRPC] refresh failed: {}", e);
                tokens = Tokens::default();
            }
        }
    }
    let mut ok = !tokens.access_token.is_empty() && authenticate(&mut conn, &tokens.access_token)?;
    if !ok {
        tokens = authorize(&mut conn, client_id, secret)?;
        ok = authenticate(&mut conn, &tokens.access_token)?;
    }
    if !ok {
        let _ = std::fs::remove_file(tokens_path());
        anyhow::bail!("Discord не принял токен");
    }
    set_status("подключено");

    let mut applied = u64::MAX; // forces one sync of the headset's current mute state on connect
    let mut last_ping = Instant::now();
    loop {
        // Mode switched away from Direct (or credentials removed): drop the connection.
        if last_ping.elapsed() > Duration::from_secs(2) {
            last_ping = Instant::now();
            if wanted().is_none() {
                return Ok(());
            }
            set_status("подключено");
        }
        let gen = GEN.load(Ordering::SeqCst);
        if gen != applied {
            applied = gen;
            if let Some(m) = *DESIRED.lock() {
                conn.send(1, &json!({"cmd": "SET_VOICE_SETTINGS", "nonce": format!("m{}", gen), "args": {"mute": m}}))?;
                log::info!("[DiscordRPC] SET_VOICE_SETTINGS mute={}", m);
            }
        }
        if conn.readable() {
            let (op, v) = conn.recv()?;
            match op {
                2 => anyhow::bail!("Discord закрыл соединение"),
                3 => conn.send(4, &v)?,
                _ => {
                    if v["evt"] == "ERROR" {
                        log::warn!("[DiscordRPC] error: {}", v["data"]);
                    }
                }
            }
        } else {
            std::thread::sleep(Duration::from_millis(40));
        }
    }
}

fn worker() {
    loop {
        match wanted() {
            None => std::thread::sleep(Duration::from_secs(2)),
            Some((id, secret)) => {
                if let Err(e) = session(&id, &secret) {
                    set_status(&format!("ошибка: {}", e));
                    std::thread::sleep(Duration::from_secs(5));
                }
            }
        }
    }
}
