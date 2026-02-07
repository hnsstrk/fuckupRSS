# TODO

## Linux-Rechner (RTX 3080Ti)

- [ ] **Ollama Config bereinigen** - `OLLAMA_NUM_PARALLEL` entfernen (nicht mehr verwendet)
  - Befehl: `sudo systemctl edit ollama.service`
  - Nur `OLLAMA_MAX_LOADED_MODELS=2` und `OLLAMA_FLASH_ATTENTION=1` behalten
  - Siehe: `docs/guides/HARDWARE_OPTIMIZATION.md`
