//! Tauri Commands for Ollama LAN-Proxy management

use crate::error::CmdResult;
use crate::AppState;
use log::info;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct ProxyStatus {
    pub running: bool,
    pub remote_host: Option<String>,
    pub remote_port: Option<u16>,
    pub local_url: Option<String>,
}

/// Start the Ollama LAN-Proxy, forwarding from localhost:11435 to remote_host:remote_port.
/// Saves proxy settings to the database for auto-start on next launch.
#[tauri::command]
pub async fn start_ollama_proxy(
    state: State<'_, AppState>,
    remote_host: String,
    remote_port: u16,
) -> CmdResult<ProxyStatus> {
    state
        .proxy_manager
        .start(&remote_host, remote_port)
        .map_err(|e| crate::error::FuckupError::Generic(e))?;

    // Save proxy settings to DB
    {
        let db = state.db_conn()?;
        let conn = db.conn();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('ollama_proxy_enabled', 'true')",
            [],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('ollama_proxy_remote_host', ?1)",
            [&remote_host],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('ollama_proxy_remote_port', ?1)",
            [&remote_port.to_string()],
        )?;
    }

    info!(
        "Ollama LAN-Proxy started: localhost:11435 -> {}:{}",
        remote_host, remote_port
    );

    Ok(ProxyStatus {
        running: true,
        remote_host: Some(remote_host),
        remote_port: Some(remote_port),
        local_url: state.proxy_manager.get_local_url(),
    })
}

/// Stop the Ollama LAN-Proxy and update the database.
#[tauri::command]
pub async fn stop_ollama_proxy(state: State<'_, AppState>) -> CmdResult<ProxyStatus> {
    state
        .proxy_manager
        .stop()
        .map_err(|e| crate::error::FuckupError::Generic(e))?;

    // Update DB
    {
        let db = state.db_conn()?;
        let conn = db.conn();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('ollama_proxy_enabled', 'false')",
            [],
        )?;
    }

    info!("Ollama LAN-Proxy stopped");

    Ok(ProxyStatus {
        running: false,
        remote_host: None,
        remote_port: None,
        local_url: None,
    })
}

/// Get the current proxy status.
#[tauri::command]
pub async fn get_ollama_proxy_status(state: State<'_, AppState>) -> CmdResult<ProxyStatus> {
    let remote = state.proxy_manager.get_active_remote();
    Ok(ProxyStatus {
        running: state.proxy_manager.is_running(),
        remote_host: remote.as_ref().map(|(h, _)| h.clone()),
        remote_port: remote.as_ref().map(|(_, p)| *p),
        local_url: state.proxy_manager.get_local_url(),
    })
}
