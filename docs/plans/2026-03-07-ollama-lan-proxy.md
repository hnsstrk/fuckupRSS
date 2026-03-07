# Ollama LAN-Proxy Integration - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Ollama LAN-Proxy in die App-Settings integrieren, sodass der Python-Proxy automatisch gestartet/gestoppt wird wenn eine nicht-localhost Ollama-URL konfiguriert ist.

**Architecture:** Der Python-Proxy-Script wird via `include_str!` in die Rust-Binary eingebettet und zur Laufzeit in eine temporaere Datei geschrieben. Ein `ProxyManager` in `AppState` verwaltet den Child-Process. Wenn der Proxy aktiv ist, wird `get_ollama_url()` transparent auf `localhost:11435` umgeleitet.

**Tech Stack:** Rust (std::process::Command, tempfile), Svelte 5, Tauri v2 Commands, Python 3 (existing proxy script)

---

## Task 1: Rust - ProxyManager Modul

**Files:**
- Create: `src-tauri/src/proxy.rs`
- Modify: `src-tauri/src/lib.rs` (mod declaration + AppState)

**Step 1: Create proxy.rs with ProxyManager struct**

```rust
// src-tauri/src/proxy.rs
use std::io::Write;
use std::process::{Child, Command};
use std::sync::Mutex;

const PROXY_SCRIPT: &str = include_str!("../../scripts/ollama-proxy.py");
const DEFAULT_LOCAL_PORT: u16 = 11435;

pub struct ProxyManager {
    child: Mutex<Option<Child>>,
    active_remote: Mutex<Option<(String, u16)>>,
}

impl ProxyManager {
    pub fn new() -> Self {
        Self {
            child: Mutex::new(None),
            active_remote: Mutex::new(None),
        }
    }

    pub fn start(&self, remote_host: &str, remote_port: u16) -> Result<(), String> {
        // Stop existing proxy first
        self.stop()?;

        // Write script to temp file
        let mut tmp = tempfile::NamedTempFile::new()
            .map_err(|e| format!("Temp-Datei erstellen fehlgeschlagen: {e}"))?;
        tmp.write_all(PROXY_SCRIPT.as_bytes())
            .map_err(|e| format!("Script schreiben fehlgeschlagen: {e}"))?;
        let script_path = tmp.into_temp_path();

        let child = Command::new("/usr/bin/python3")
            .arg(script_path.to_str().unwrap())
            .arg("--remote")
            .arg(remote_host)
            .arg("--remote-port")
            .arg(remote_port.to_string())
            .arg("--local-port")
            .arg(DEFAULT_LOCAL_PORT.to_string())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("Proxy starten fehlgeschlagen: {e}"))?;

        // Keep temp path alive by leaking it (cleaned up on process exit)
        let _ = script_path.keep();

        *self.child.lock().unwrap() = Some(child);
        *self.active_remote.lock().unwrap() = Some((remote_host.to_string(), remote_port));

        // Wait briefly for proxy to bind
        std::thread::sleep(std::time::Duration::from_millis(500));

        Ok(())
    }

    pub fn stop(&self) -> Result<(), String> {
        let mut child_guard = self.child.lock().unwrap();
        if let Some(ref mut child) = *child_guard {
            let _ = child.kill();
            let _ = child.wait();
        }
        *child_guard = None;
        *self.active_remote.lock().unwrap() = None;
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        let mut child_guard = self.child.lock().unwrap();
        match *child_guard {
            Some(ref mut child) => child.try_wait().ok().flatten().is_none(),
            None => false,
        }
    }

    pub fn get_local_url(&self) -> Option<String> {
        if self.is_running() {
            Some(format!("http://localhost:{DEFAULT_LOCAL_PORT}"))
        } else {
            None
        }
    }

    pub fn get_active_remote(&self) -> Option<(String, u16)> {
        self.active_remote.lock().unwrap().clone()
    }
}

impl Drop for ProxyManager {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
```

**Step 2: Add mod declaration and extend AppState in lib.rs**

In `src-tauri/src/lib.rs`:
- Add `mod proxy;` after `mod sync;` (line ~16)
- Add `pub use proxy::ProxyManager;`
- Add `proxy_manager: Arc<ProxyManager>` to `AppState` struct
- Initialize in setup closure

**Step 3: Verify it compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

**Step 4: Commit**

```bash
git add src-tauri/src/proxy.rs src-tauri/src/lib.rs
git commit -m "feat: ProxyManager Modul fuer Ollama LAN-Proxy"
```

---

## Task 2: Rust - Tauri Commands fuer Proxy

**Files:**
- Create: `src-tauri/src/commands/proxy.rs`
- Modify: `src-tauri/src/commands/mod.rs` (add pub mod proxy)
- Modify: `src-tauri/src/lib.rs` (register commands)

**Step 1: Create commands/proxy.rs**

```rust
// src-tauri/src/commands/proxy.rs
use crate::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct ProxyStatus {
    pub running: bool,
    pub remote_host: Option<String>,
    pub remote_port: Option<u16>,
    pub local_url: Option<String>,
}

#[tauri::command]
pub async fn start_ollama_proxy(
    state: State<'_, AppState>,
    remote_host: String,
    remote_port: u16,
) -> Result<ProxyStatus, String> {
    state.proxy_manager.start(&remote_host, remote_port)?;

    // Save proxy settings to DB
    {
        let db = state.db_conn().map_err(|e| e.to_string())?;
        let conn = db.conn();
        let _ = conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('ollama_proxy_enabled', 'true')",
            [],
        );
        let _ = conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('ollama_proxy_remote_host', ?1)",
            [&remote_host],
        );
        let _ = conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('ollama_proxy_remote_port', ?1)",
            [&remote_port.to_string()],
        );
    }

    Ok(ProxyStatus {
        running: true,
        remote_host: Some(remote_host),
        remote_port: Some(remote_port),
        local_url: state.proxy_manager.get_local_url(),
    })
}

#[tauri::command]
pub async fn stop_ollama_proxy(
    state: State<'_, AppState>,
) -> Result<ProxyStatus, String> {
    state.proxy_manager.stop()?;

    // Update DB
    {
        let db = state.db_conn().map_err(|e| e.to_string())?;
        let conn = db.conn();
        let _ = conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('ollama_proxy_enabled', 'false')",
            [],
        );
    }

    Ok(ProxyStatus {
        running: false,
        remote_host: None,
        remote_port: None,
        local_url: None,
    })
}

#[tauri::command]
pub async fn get_ollama_proxy_status(
    state: State<'_, AppState>,
) -> Result<ProxyStatus, String> {
    let remote = state.proxy_manager.get_active_remote();
    Ok(ProxyStatus {
        running: state.proxy_manager.is_running(),
        remote_host: remote.as_ref().map(|(h, _)| h.clone()),
        remote_port: remote.as_ref().map(|(_, p)| *p),
        local_url: state.proxy_manager.get_local_url(),
    })
}
```

**Step 2: Add `pub mod proxy;` to commands/mod.rs**

**Step 3: Register commands in lib.rs invoke_handler**

Add after the maintenance commands block:
```rust
// Ollama LAN-Proxy
commands::proxy::start_ollama_proxy,
commands::proxy::stop_ollama_proxy,
commands::proxy::get_ollama_proxy_status,
```

**Step 4: Verify it compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

**Step 5: Commit**

```bash
git add src-tauri/src/commands/proxy.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat: Tauri Commands fuer Ollama LAN-Proxy"
```

---

## Task 3: Rust - URL-Routing bei aktivem Proxy

**Files:**
- Modify: `src-tauri/src/commands/ai/helpers.rs:40-48`

**Step 1: Modify get_ollama_url to check proxy state**

Change `get_ollama_url` to accept an optional ProxyManager reference:

```rust
/// Get Ollama URL - returns proxy URL if proxy is active, otherwise DB setting
pub fn get_ollama_url(db: &Database) -> String {
    db.conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'ollama_url'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "http://localhost:11434".to_string())
}

/// Get effective Ollama URL considering proxy state
pub fn get_effective_ollama_url(db: &Database, proxy: &crate::proxy::ProxyManager) -> String {
    if let Some(local_url) = proxy.get_local_url() {
        local_url
    } else {
        get_ollama_url(db)
    }
}
```

**Step 2: Update callers to use get_effective_ollama_url where AppState is available**

Search for all `get_ollama_url` callers and update those that have access to `state`:
- In batch_processor.rs, article_processor.rs, model_management.rs etc.
- Pass `&state.proxy_manager` as additional parameter

**Step 3: Verify it compiles and tests pass**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

**Step 4: Commit**

```bash
git add src-tauri/src/commands/ai/helpers.rs
git commit -m "feat: URL-Routing ueber Proxy bei aktivem LAN-Proxy"
```

---

## Task 4: Rust - Proxy Auto-Start bei App-Start

**Files:**
- Modify: `src-tauri/src/lib.rs` (setup closure)

**Step 1: Add auto-start logic after AppState creation**

In the `setup` closure, after `app.manage(AppState { ... })`, add:

```rust
// Auto-start proxy if previously enabled
{
    let state: State<AppState> = app.state();
    let db = state.db_conn().expect("DB lock for proxy auto-start");
    let proxy_enabled = db.conn()
        .query_row("SELECT value FROM settings WHERE key = 'ollama_proxy_enabled'", [], |row| row.get::<_, String>(0))
        .unwrap_or_default() == "true";

    if proxy_enabled {
        let remote_host = db.conn()
            .query_row("SELECT value FROM settings WHERE key = 'ollama_proxy_remote_host'", [], |row| row.get::<_, String>(0))
            .unwrap_or_default();
        let remote_port: u16 = db.conn()
            .query_row("SELECT value FROM settings WHERE key = 'ollama_proxy_remote_port'", [], |row| row.get::<_, String>(0))
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(11434);

        if !remote_host.is_empty() {
            drop(db); // Release lock before starting proxy
            if let Err(e) = state.proxy_manager.start(&remote_host, remote_port) {
                error!("Failed to auto-start Ollama proxy: {}", e);
            } else {
                info!("Ollama LAN-Proxy auto-started: localhost:11435 -> {}:{}", remote_host, remote_port);
            }
        }
    }
}
```

**Step 2: Verify it compiles**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

**Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: Proxy Auto-Start bei App-Start wenn zuvor aktiviert"
```

---

## Task 5: Frontend - i18n Keys

**Files:**
- Modify: `src/lib/i18n/de.json`
- Modify: `src/lib/i18n/en.json`

**Step 1: Add proxy i18n keys**

In `de.json` under `settings.ollama`:
```json
"proxy": {
  "title": "LAN-Proxy (macOS Tahoe Workaround)",
  "description": "Leitet Ollama-Anfragen ueber einen lokalen Proxy weiter, da macOS Tahoe den direkten LAN-Zugriff fuer unsignierte Apps blockiert.",
  "remoteHost": "Remote Ollama Host",
  "remotePort": "Remote Port",
  "start": "Proxy starten",
  "stop": "Proxy stoppen",
  "running": "Proxy aktiv",
  "stopped": "Proxy inaktiv",
  "startError": "Proxy konnte nicht gestartet werden",
  "routingVia": "Routing via localhost:{port}"
}
```

In `en.json` under `settings.ollama`:
```json
"proxy": {
  "title": "LAN Proxy (macOS Tahoe Workaround)",
  "description": "Routes Ollama requests through a local proxy because macOS Tahoe blocks direct LAN access for unsigned apps.",
  "remoteHost": "Remote Ollama Host",
  "remotePort": "Remote Port",
  "start": "Start Proxy",
  "stop": "Stop Proxy",
  "running": "Proxy active",
  "stopped": "Proxy inactive",
  "startError": "Failed to start proxy",
  "routingVia": "Routing via localhost:{port}"
}
```

**Step 2: Commit**

```bash
git add src/lib/i18n/de.json src/lib/i18n/en.json
git commit -m "feat: i18n Keys fuer Ollama LAN-Proxy"
```

---

## Task 6: Frontend - Proxy UI in Settings

**Files:**
- Modify: `src/lib/components/settings/SettingsOllama.svelte`
- Modify: `src/lib/components/settings/SettingsOllamaProvider.svelte`

**Step 1: Add proxy state and handlers to SettingsOllama.svelte**

Add state variables:
```typescript
// Proxy state
let proxyEnabled = $state(false);
let proxyRemoteHost = $state("");
let proxyRemotePort = $state(11434);
let proxyRunning = $state(false);
let proxyStarting = $state(false);
```

Add init loading (in `init()` function):
```typescript
// Load proxy status
try {
  const proxyStatus = await invoke<{
    running: boolean;
    remote_host: string | null;
    remote_port: number | null;
    local_url: string | null;
  }>("get_ollama_proxy_status");
  proxyRunning = proxyStatus.running;
  if (proxyStatus.remote_host) proxyRemoteHost = proxyStatus.remote_host;
  if (proxyStatus.remote_port) proxyRemotePort = proxyStatus.remote_port;
} catch { /* ignore */ }

const savedProxyEnabled = await invoke<string | null>("get_setting", { key: "ollama_proxy_enabled" });
proxyEnabled = savedProxyEnabled === "true";
const savedProxyHost = await invoke<string | null>("get_setting", { key: "ollama_proxy_remote_host" });
if (savedProxyHost) proxyRemoteHost = savedProxyHost;
const savedProxyPort = await invoke<string | null>("get_setting", { key: "ollama_proxy_remote_port" });
if (savedProxyPort) proxyRemotePort = parseInt(savedProxyPort) || 11434;
```

Add handler functions:
```typescript
async function startProxy() {
  proxyStarting = true;
  try {
    const result = await invoke<{ running: boolean }>("start_ollama_proxy", {
      remoteHost: proxyRemoteHost,
      remotePort: proxyRemotePort,
    });
    proxyRunning = result.running;
    proxyEnabled = true;
    toasts.success($_("settings.ollama.proxy.running"));
  } catch (e) {
    toasts.error($_("settings.ollama.proxy.startError") + ": " + String(e));
  }
  proxyStarting = false;
}

async function stopProxy() {
  try {
    await invoke("stop_ollama_proxy");
    proxyRunning = false;
    proxyEnabled = false;
  } catch { /* ignore */ }
}
```

**Step 2: Pass proxy props to SettingsOllamaProvider**

Add these props to `<SettingsOllamaProvider>`:
```svelte
{proxyEnabled}
{proxyRunning}
{proxyStarting}
bind:proxyRemoteHost
bind:proxyRemotePort
onStartProxy={startProxy}
onStopProxy={stopProxy}
```

**Step 3: Add proxy section to SettingsOllamaProvider.svelte**

Add new props to the component's props interface. Add a proxy section after the server URL section, showing:
- Collapsible section with title "LAN-Proxy (macOS Tahoe Workaround)"
- Description text
- Remote Host input field
- Remote Port input field
- Start/Stop button with status indicator (green/red dot)
- "Routing via localhost:11435" info when active

**Step 4: Test the UI in dev mode**

```bash
npm run tauri dev
```

Navigate to Settings, verify proxy section appears and works.

**Step 5: Commit**

```bash
git add src/lib/components/settings/SettingsOllama.svelte src/lib/components/settings/SettingsOllamaProvider.svelte
git commit -m "feat: LAN-Proxy UI in Ollama Settings"
```

---

## Task 7: Cargo.toml - tempfile Dependency

**Files:**
- Modify: `src-tauri/Cargo.toml`

**Step 1: Add tempfile crate**

```toml
[dependencies]
tempfile = "3"
```

**Step 2: Verify**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

**Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: tempfile Dependency fuer Proxy-Script"
```

**WICHTIG:** Dieser Task muss VOR Task 1 ausgefuehrt werden (oder zusammen mit Task 1).

---

## Abhaengigkeiten

```
Task 7 (Cargo.toml) -> Task 1 (ProxyManager) -> Task 2 (Commands) -> Task 3 (URL-Routing)
                                                                    -> Task 4 (Auto-Start)
Task 5 (i18n) -> Task 6 (Frontend UI)
```

**Parallelisierbar:**
- Tasks 7+1+2+3+4 (Backend) und Tasks 5+6 (Frontend) koennen parallel bearbeitet werden
- Task 5 (i18n) ist unabhaengig vom Backend
