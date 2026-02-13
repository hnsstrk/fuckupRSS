# CI/CD Setup Guide

## Übersicht

fuckupRSS nutzt **Gitea Actions** mit `act_runner` im Host-Modus für CI/CD.

### Warum Gitea Actions?

| Kriterium | Gitea Actions | Woodpecker CI | Drone CI |
|-----------|:------------:|:-------------:|:--------:|
| Nativ in Gitea | Ja | Nein | Nein |
| macOS Host-Modus | Ja | Nur Docker | Nur Docker |
| GitHub Actions Syntax | Ja | Eigene | Eigene |
| Zusätzlicher Server | Nein (nur Runner) | Server + Agent | Server + Runner |

**Begründung:** Tauri-Builds auf macOS brauchen nativen Zugriff auf Xcode/WebKit. Docker-basierte Lösungen funktionieren nicht.

## Architektur

```
        Gitea (192.168.177.11:3000)
                    |
          ┌─────────┴─────────┐
    Linux Runner          macOS Runner
    (linux-x64:host)      (macos-arm64:host)
    - Lint                - Rust Tests (macOS)
    - Tests               - macOS Build (.dmg)
    - Security (Semgrep)
    - Linux Build (.deb, .AppImage)
    - SBOM
```

## Voraussetzung: Gitea Server

In `/etc/gitea/app.ini` sicherstellen:

```ini
[actions]
ENABLED = true
```

Nach Änderung: `sudo systemctl restart gitea`

## Runner-Setup

### macOS (Apple Silicon)

```bash
# Installation
brew tap go-gitea/gitea && brew install act_runner

# Token aus Gitea holen:
# Gitea > Site Administration > Runners > Create new Runner

# Registrieren
act_runner register \
  --instance http://192.168.177.11:3000 \
  --token <TOKEN> \
  --name fuckuprss-macos \
  --labels "macos-arm64:host"

# Als launchd-Service einrichten
cat > ~/Library/LaunchAgents/com.gitea.act-runner.plist << 'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.gitea.act-runner</string>
  <key>ProgramArguments</key>
  <array>
    <string>/opt/homebrew/bin/act_runner</string>
    <string>daemon</string>
  </array>
  <key>WorkingDirectory</key>
  <string>/Users/hnsstrk/.config/act-runner</string>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>/tmp/act-runner.log</string>
  <key>StandardErrorPath</key>
  <string>/tmp/act-runner.err</string>
</dict>
</plist>
PLIST

launchctl load ~/Library/LaunchAgents/com.gitea.act-runner.plist
```

### Linux (RTX 3080Ti Server)

```bash
# Installation
wget https://gitea.com/gitea/act_runner/releases/latest/download/act_runner-linux-amd64
chmod +x act_runner-linux-amd64
sudo mv act_runner-linux-amd64 /usr/local/bin/act_runner

# Voraussetzungen
pip install semgrep
npx playwright install chromium --with-deps
cargo install cargo-audit cargo-cyclonedx

# Registrieren
act_runner register \
  --instance http://192.168.177.11:3000 \
  --token <TOKEN> \
  --name fuckuprss-linux \
  --labels "linux-x64:host"

# Als systemd-Service einrichten
sudo cat > /etc/systemd/system/act-runner.service << 'SERVICE'
[Unit]
Description=Gitea Act Runner
After=network.target

[Service]
Type=simple
User=runner
WorkingDirectory=/home/runner/.config/act-runner
ExecStart=/usr/local/bin/act_runner daemon
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
SERVICE

sudo systemctl enable --now act-runner
```

## Pipeline-Stages

Die Pipeline ist in `.gitea/workflows/ci.yaml` definiert:

| Stage | Runner | Jobs |
|-------|--------|------|
| **1. Lint** | linux-x64 | ESLint, Prettier, svelte-check, tsc --noEmit, cargo fmt, Clippy |
| **2. Tests** | Beide | Vitest (Linux), cargo test (beide), E2E (Linux) |
| **3. Security** | linux-x64 | Semgrep, npm audit |
| **4. Build** | Beide | Linux (.deb, .AppImage), macOS (.dmg) |
| **5. SBOM** | linux-x64 | Frontend + Backend SBOMs |

## Troubleshooting

### Runner verbindet sich nicht
```bash
# Logs prüfen
act_runner daemon --log-level debug
# Netzwerk prüfen
curl http://192.168.177.11:3000/api/v1/version
```

### Build schlägt fehl
```bash
# Lokal testen was CI tut
npm run lint && npm run format:check && npm run test
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
