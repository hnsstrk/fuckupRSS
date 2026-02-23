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
              Linux Runner (Callisto)
              (linux-x64:host)
              - CI: Security Scan + SBOM
              - Release: Linux Build (.deb, .AppImage)
```

**Hinweis:** Lint, Tests und Typechecks laufen lokal via Git Hooks (Pre-commit + Pre-push) und sind nicht mehr in der CI-Pipeline. macOS-Builds erfolgen manuell via `scripts/build-macos.sh`.

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

## Qualitaetssicherung: Lokale Hooks vs. CI

Lint, Tests und Typechecks sind **nicht** in der CI-Pipeline, sondern laufen lokal via Git Hooks:

| Pruefung | Hook | Details |
|----------|------|---------|
| ESLint + Prettier | Pre-commit (lint-staged) | Nur auf staged Dateien |
| cargo fmt + Clippy | Pre-commit | Nur bei .rs-Aenderungen |
| Vitest (Frontend) | Pre-push | Alle Frontend-Tests |
| cargo test | Pre-push | `--lib --bins` |
| svelte-check | Pre-push | TypeScript-Pruefung |

**Begründung:** Da alle Qualitaetspruefungen bereits lokal erzwungen werden, waeren CI-Jobs redundant und wuerden den schwachen Runner (Intel N100) unnoetig belasten.

## CI-Pipeline (Security + SBOM)

Die CI-Pipeline ist in `.gitea/workflows/ci.yaml` definiert und besteht aus **einem einzigen Job**.

```
Push/PR zu main ──→ security-sbom (Security Scan + SBOM Generation)
```

### Security-Checks

| Check | Command | Beschreibung |
|-------|---------|-------------|
| Semgrep auto | `semgrep scan --config auto --error` | Automatische Regeln, Fehler bei Fund |
| Semgrep OWASP | `semgrep scan --config p/owasp-top-ten --error` | OWASP Top 10 Pruefung |
| npm audit | `npm audit --audit-level=high --omit=dev` | Frontend-Dependencies (high+) |
| cargo audit | `cargo audit --file src-tauri/Cargo.lock --deny unsound --deny yanked` | Rust-Dependencies |

### SBOM-Generierung

| SBOM | Tool | Output |
|------|------|--------|
| Frontend | `@cyclonedx/cyclonedx-npm` | `sbom/frontend-bom.json` |
| Backend | `cargo-cyclonedx` | `sbom/backend-bom.json` |

Beide SBOMs werden nach Generierung JSON-validiert und als Artefakt hochgeladen.

## Release-Workflow

Der Release-Workflow ist in `.gitea/workflows/release.yaml` definiert und wird **Tag-basiert** ausgeloest.

### Ablauf

1. Ein Git-Tag mit `v`-Prefix erstellen und pushen
2. Der Workflow baut automatisch fuer Linux (.deb, .AppImage)
3. Ein Gitea Release wird erstellt mit Changelog und Linux-Artefakten
4. macOS-Build (.dmg) wird manuell via `scripts/build-macos.sh` erstellt und nachtraeglich zum Release hinzugefuegt

### Release erstellen

```bash
# Version taggen und pushen
git tag v1.0.0
git push --tags

# Oder mit annotiertem Tag
git tag -a v1.0.0 -m "Release v1.0.0: Beschreibung"
git push --tags
```

### Voraussetzungen

| Voraussetzung | Beschreibung |
|---------------|-------------|
| `GITEATOKEN` Secret | Muss in Gitea unter Repository > Settings > Actions > Secrets konfiguriert werden |
| Tag-Format | Muss mit `v` beginnen (z.B. `v1.0.0`, `v2.1.3`) |
| CI-Pipeline | Sollte vorher auf `main` erfolgreich durchgelaufen sein |

### Release-Artefakte

| Plattform | Format | Quelle |
|-----------|--------|--------|
| Linux | `.deb`, `.AppImage` | Automatisch via CI (Callisto) |
| macOS | `.dmg` | Manuell via `scripts/build-macos.sh` |

Der Changelog wird automatisch aus den Commits seit dem letzten Tag generiert.

## Troubleshooting

### Runner verbindet sich nicht
```bash
# Logs prüfen
act_runner daemon --log-level debug
# Netzwerk prüfen
curl http://192.168.177.11:3000/api/v1/version
```

### Build schlaegt fehl
```bash
# Lokal testen was CI tut (Security)
semgrep scan --config auto src-tauri/src/ src/
npm audit --audit-level=high --omit=dev
cargo audit --file src-tauri/Cargo.lock --deny unsound --deny yanked

# Lokal testen was Hooks pruefen
npm run lint && npm run format:check && npm run test
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
