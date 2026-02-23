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

## Linux-Rechner (RTX 3080Ti)

- [ ] **Ollama Config bereinigen** - `OLLAMA_NUM_PARALLEL` entfernen (nicht mehr verwendet)
  - Befehl: `sudo systemctl edit ollama.service`
  - Nur `OLLAMA_MAX_LOADED_MODELS=2` und `OLLAMA_FLASH_ATTENTION=1` behalten
  - Siehe: `docs/guides/HARDWARE_OPTIMIZATION.md`
