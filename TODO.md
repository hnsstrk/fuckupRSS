# TODO

## GTK3-Dependencies in cargo audit

- [ ] **GTK3-Bindings verursachen wiederkehrende cargo-audit Warnings** — alle sind transitive Dependencies von Tauri (wry, tao, gtk, glib, atk, etc.) und nicht direkt kontrollierbar
  - Betroffene Advisories: RUSTSEC-2024-0413 bis -0420 (unmaintained), RUSTSEC-2024-0429 (glib unsound), RUSTSEC-2024-0370
  - CI wurde angepasst: `--deny unsound` entfernt, da glib 0.18.5 nicht vermeidbar ist
- [ ] **Alternativen prüfen:**
  - GTK4-Migration in zukünftigen Tauri-Versionen (Tauri v2+ / wry Updates)
  - webkit2gtk Updates verfolgen
  - Tauri-Roadmap bzgl. Linux-Backend-Alternativen beobachten
- [ ] **Langfristiges Ziel:** cargo audit ohne GTK3-Ausnahmen bestehen

## CI/CD Verbesserungen

- [ ] **Rust-Toolchain cachen** — Aktuell wird bei jedem CI-Run `rustup.rs` frisch installiert. Tool-Cache konfigurieren oder Custom Runner-Image mit vorinstallierter Toolchain verwenden.
- [ ] **Semgrep und Cargo-Tools cachen** — `pip install semgrep` und `cargo install cargo-audit cargo-cyclonedx` laufen bei jedem Run. Entweder in Custom Runner-Image einbacken oder Cache erweitern.
- [ ] **Runner-Image pinnen** — Aktuell `:latest` Tag (`docker.gitea.com/runner-images:ubuntu-latest`). Für Reproduzierbarkeit auf spezifische Version pinnen.
- [ ] **SBOM-Validierung verbessern** — Aktuell nur `JSON.parse()`. Besser: CycloneDX Schema-Validierung. Das npm-Paket `@cyclonedx/cyclonedx-cli` existiert nicht in npm, Alternative suchen oder als Binary im Runner-Image vorinstallieren.
- [ ] **macOS-Build Checkliste** — macOS-Build ist manuell (`scripts/build-macos.sh`). Release-Checkliste erstellen damit der Schritt nicht vergessen wird.
- [ ] **Dependency-Update-Automation** — Kein Renovate/Dependabot konfiguriert. Für automatische Dependency-Updates evaluieren (Gitea hat Renovate-Support).

## Linux-Rechner (RTX 3080Ti)

- [ ] **Ollama Config bereinigen** - `OLLAMA_NUM_PARALLEL` entfernen (nicht mehr verwendet)
  - Befehl: `sudo systemctl edit ollama.service`
  - Nur `OLLAMA_MAX_LOADED_MODELS=2` und `OLLAMA_FLASH_ATTENTION=1` behalten
  - Siehe: `docs/guides/HARDWARE_OPTIMIZATION.md`
