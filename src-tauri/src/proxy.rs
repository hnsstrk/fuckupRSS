//! Ollama LAN-Proxy Manager
//!
//! Manages a Python-based HTTP proxy that forwards Ollama requests from localhost
//! to a remote Ollama instance. This is a workaround for macOS Tahoe blocking
//! direct LAN access for unsigned apps.

use log::{error, info, warn};
use std::io::Write;
use std::process::{Child, Command, Stdio};
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

    /// Start the proxy, forwarding from localhost:DEFAULT_LOCAL_PORT to remote_host:remote_port.
    /// If a proxy is already running, it will be stopped first.
    pub fn start(&self, remote_host: &str, remote_port: u16) -> Result<(), String> {
        // Stop existing proxy first
        self.stop()?;

        // Write script to temp file
        let mut tmp = tempfile::Builder::new()
            .prefix("ollama-proxy-")
            .suffix(".py")
            .tempfile()
            .map_err(|e| format!("Temp-Datei erstellen fehlgeschlagen: {e}"))?;
        tmp.write_all(PROXY_SCRIPT.as_bytes())
            .map_err(|e| format!("Script schreiben fehlgeschlagen: {e}"))?;
        let script_path = tmp.into_temp_path();

        info!(
            "Starting Ollama LAN-Proxy: localhost:{} -> {}:{}",
            DEFAULT_LOCAL_PORT, remote_host, remote_port
        );

        let child = Command::new("/usr/bin/python3")
            .arg(script_path.to_str().unwrap())
            .arg("--remote")
            .arg(remote_host)
            .arg("--remote-port")
            .arg(remote_port.to_string())
            .arg("--local-port")
            .arg(DEFAULT_LOCAL_PORT.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Proxy starten fehlgeschlagen: {e}"))?;

        // Keep temp path alive by persisting it (cleaned up on process exit by OS)
        if let Err(e) = script_path.keep() {
            warn!("Could not persist temp script path: {}", e);
        }

        *self.child.lock().unwrap() = Some(child);
        *self.active_remote.lock().unwrap() = Some((remote_host.to_string(), remote_port));

        // Wait briefly for proxy to bind the port
        std::thread::sleep(std::time::Duration::from_millis(500));

        Ok(())
    }

    /// Stop the running proxy process.
    pub fn stop(&self) -> Result<(), String> {
        let mut child_guard = self.child.lock().unwrap();
        if let Some(ref mut child) = *child_guard {
            info!("Stopping Ollama LAN-Proxy (pid: {})", child.id());
            let _ = child.kill();
            let _ = child.wait();
        }
        *child_guard = None;
        *self.active_remote.lock().unwrap() = None;
        Ok(())
    }

    /// Check if the proxy process is still running.
    pub fn is_running(&self) -> bool {
        let mut child_guard = self.child.lock().unwrap();
        match *child_guard {
            Some(ref mut child) => {
                // try_wait returns Ok(Some(status)) if exited, Ok(None) if still running
                child.try_wait().ok().flatten().is_none()
            }
            None => false,
        }
    }

    /// Get the local proxy URL if the proxy is running.
    pub fn get_local_url(&self) -> Option<String> {
        if self.is_running() {
            Some(format!("http://localhost:{DEFAULT_LOCAL_PORT}"))
        } else {
            None
        }
    }

    /// Get the remote host and port of the active proxy connection.
    pub fn get_active_remote(&self) -> Option<(String, u16)> {
        self.active_remote.lock().unwrap().clone()
    }
}

impl Drop for ProxyManager {
    fn drop(&mut self) {
        if let Err(e) = self.stop() {
            error!("Error stopping proxy on drop: {}", e);
        }
    }
}
