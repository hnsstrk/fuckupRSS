# Release-Checkliste

Schritt-fuer-Schritt-Anleitung fuer einen fuckupRSS-Release. Gilt fuer alle Releases (Major, Minor, Patch).

## 1. Versionierung

Die Version muss in **drei Dateien** synchron aktualisiert werden:

| Datei | Feld | Beispiel |
|-------|------|----------|
| `package.json` | `"version"` | `"version": "1.2.0"` |
| `src-tauri/Cargo.toml` | `version` | `version = "1.2.0"` |
| `src-tauri/tauri.conf.json` | `"version"` | `"version": "1.2.0"` |

**Semantic Versioning:**
- **Major** (x.0.0): Breaking Changes, inkompatible API/Schema-Aenderungen
- **Minor** (0.x.0): Neue Features, rueckwaertskompatibel
- **Patch** (0.0.x): Bugfixes, Security-Patches

## 2. Pre-Release Checks

### 2.1 Code-Qualitaet

```bash
# Frontend Lint + Format
npm run lint
npm run format:check

# Rust Lint + Format
npm run rust:fmt:check
npm run rust:clippy

# TypeScript Typ-Pruefung
npx svelte-check --tsconfig ./tsconfig.json
```

### 2.2 Tests

```bash
# Frontend-Tests
npm run test

# Rust-Tests (nur lib + bins, ohne Doctests)
cargo test --manifest-path src-tauri/Cargo.toml --lib --bins

# E2E-Tests (optional, erfordert laufende App)
npm run test:e2e
```

### 2.3 Security

```bash
# Semgrep Scans
npm run security:scan
npm run security:owasp

# Dependency Audits
npm run security:audit

# SBOM generieren und validieren
npm run sbom:generate
npm run sbom:validate
```

### 2.4 Build-Test (lokal)

```bash
# Sicherstellen, dass der Build durchlaeuft
npm run tauri build
```

### 2.5 Dokumentation pruefen

- [ ] `README.md` - Features, Installation aktuell?
- [ ] `CLAUDE.md` - Entwickler-Kontext aktuell?
- [ ] `docs/ANFORDERUNGEN.md` - Roadmap/Phase-Status aktuell?
- [ ] `docs/api/TAURI_COMMANDS_REFERENCE.md` - Neue Commands dokumentiert?
- [ ] `docs/architecture/DATABASE_SCHEMA.md` - Schema-Aenderungen dokumentiert?
- [ ] Changelog/Release-Notes vorbereitet?

## 3. Release durchfuehren

### 3.1 Version aktualisieren

```bash
# Version in allen drei Dateien aendern
# package.json:        "version": "X.Y.Z"
# src-tauri/Cargo.toml: version = "X.Y.Z"
# src-tauri/tauri.conf.json: "version": "X.Y.Z"
```

### 3.2 Commit und Tag

```bash
# Aenderungen committen
git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json
git commit -m "chore: bump version to vX.Y.Z"

# Tag erstellen
git tag vX.Y.Z

# Push (loest CI/CD und Release-Workflow aus)
git push origin main
git push origin vX.Y.Z
```

### 3.3 Linux-Build (automatisch via CI)

Der Tag-Push loest den Release-Workflow in `.gitea/workflows/release.yaml` aus:

1. **build-linux Job:** Baut `.deb` und `.AppImage` auf Callisto (Linux-Runner)
2. **release Job:** Erstellt Gitea-Release mit Changelog und Linux-Artefakten

**Voraussetzung:** `GITEATOKEN` Secret muss in Gitea Repository-Settings konfiguriert sein.

**Pruefen:**
- [ ] CI-Pipeline (security-sbom) erfolgreich
- [ ] Release-Workflow erfolgreich
- [ ] Linux-Artefakte (.deb, .AppImage) im Release vorhanden
- [ ] Changelog korrekt generiert

### 3.4 macOS-Build (manuell)

Es gibt keinen macOS-CI-Runner. Der Build muss lokal auf dem MacBook durchgefuehrt werden.

```bash
# Production Build
./scripts/build-macos.sh

# Oder Clean Build (empfohlen fuer Releases)
./scripts/build-macos.sh --clean
```

**Ergebnis:**
- `.dmg` in `src-tauri/target/release/bundle/dmg/`
- `.app` in `src-tauri/target/release/bundle/macos/`

### 3.5 macOS-Artefakt zum Release hinzufuegen

Die `.dmg`-Datei muss manuell zum Gitea-Release hinzugefuegt werden:

**Option A: Gitea Web-UI**
1. Release-Seite oeffnen
2. "Edit Release" klicken
3. `.dmg`-Datei als Attachment hinzufuegen
4. Speichern

**Option B: API**
```bash
GITEA_TOKEN="<token>"
RELEASE_ID="<release-id>"
DMG_FILE="src-tauri/target/release/bundle/dmg/fuckupRSS_X.Y.Z_aarch64.dmg"

curl -X POST \
  "http://192.168.177.11:3000/api/v1/repos/hnsstrk/fuckupRSS/releases/${RELEASE_ID}/assets?name=$(basename $DMG_FILE)" \
  -H "Authorization: token ${GITEA_TOKEN}" \
  -H "Content-Type: application/octet-stream" \
  --data-binary "@${DMG_FILE}"
```

## 4. Post-Release Checks

- [ ] Release-Seite auf Gitea pruefen (Changelog, Artefakte)
- [ ] Linux `.deb` und `.AppImage` vorhanden
- [ ] macOS `.dmg` vorhanden
- [ ] Download-Links funktionieren
- [ ] App startet korrekt nach Installation (Smoke-Test)
- [ ] Datenbank-Migration laeuft korrekt (falls Schema-Aenderungen)
- [ ] Ollama-Integration funktioniert

## 5. Bekannte Einschraenkungen

| Einschraenkung | Details |
|----------------|---------|
| Kein macOS-CI-Runner | macOS-Build muss immer manuell erstellt werden |
| Gitea Act Runner | Nur `upload-artifact@v3` (nicht v4) |
| Kein Code-Signing | macOS `.dmg` ist nicht signiert (Gatekeeper-Warnung) |
| Kein Auto-Update | Manueller Download fuer Updates noetig |
| Doctest-Fehler | Einige Doctests in transaction.rs/headless.rs/string.rs sind bekannt fehlerhaft, betreffen nicht die Funktionalitaet |

## 6. Rollback

Falls ein Release Probleme verursacht:

```bash
# Tag loeschen (lokal und remote)
git tag -d vX.Y.Z
git push origin :refs/tags/vX.Y.Z

# Release in Gitea manuell loeschen
# Version zuruecksetzen und neuen Tag erstellen
```

## 7. Schnell-Checkliste (Kurzfassung)

```
[ ] Version in package.json, Cargo.toml, tauri.conf.json aktualisiert
[ ] npm run lint && npm run format:check
[ ] npm run rust:fmt:check && npm run rust:clippy
[ ] npm run test
[ ] cargo test --manifest-path src-tauri/Cargo.toml --lib --bins
[ ] npm run security:scan && npm run security:owasp
[ ] npm run security:audit
[ ] Dokumentation geprueft (README, CLAUDE.md, ANFORDERUNGEN)
[ ] git commit -m "chore: bump version to vX.Y.Z"
[ ] git tag vX.Y.Z && git push origin main && git push origin vX.Y.Z
[ ] CI-Pipeline + Release-Workflow erfolgreich
[ ] ./scripts/build-macos.sh --clean
[ ] macOS .dmg zum Gitea-Release hinzugefuegt
[ ] Smoke-Test auf beiden Plattformen
```
