# Design: Ollama-Server Dropdown mit History

**Datum:** 2026-03-01
**Status:** Genehmigt

## Ziel

Das bestehende Text-Input-Feld fuer die Ollama-Server-URL durch ein Combobox-Dropdown ersetzen, das:
- Frueher erfolgreich verbundene Server-Adressen speichert
- Den Verbindungsstatus jedes Servers als farbigen Punkt (gruen/rot) anzeigt
- Das Loeschen gespeicherter Adressen ermoeglicht
- Weiterhin freie Eingabe neuer URLs erlaubt

## Datenmodell

- **Speicherort:** `settings`-Tabelle, Key `ollama_server_history`
- **Format:** JSON-Array von URL-Strings, z.B. `["http://localhost:11434", "http://192.168.177.22:11434"]`
- **Aktive URL:** Bleibt unter Key `ollama_url`
- **Default:** `http://localhost:11434` ist immer vorhanden und nicht loeschbar

## UI-Design

```
Ollama-Server

┌──────────────────────────────────┐ ┌────────────────┐
│ http://192.168.177.22:11434  ▼  │ │ Verb. testen   │
└──────────────────────────────────┘ └────────────────┘
  ├─ http://localhost:11434        🟢
  ├─ http://192.168.177.22:11434   🔴  ✕
  └─ (freie Eingabe moeglich)

URL des Ollama-Servers. Fuer lokale Nutzung: http://localhost:11434
```

## Verhalten

1. **Dropdown** zeigt alle gespeicherten Server mit Status-Punkt (gruen=erreichbar, rot=nicht erreichbar)
2. **Freie Eingabe** weiterhin moeglich (Combobox-Pattern, kein reines Select)
3. **Status-Check** aller Server beim Oeffnen der Settings (parallel, nicht-blockierend)
4. **Loeschen** ueber ein X-Icon pro Eintrag im Dropdown (ausser localhost-Default)
5. **Hinzufuegen** automatisch bei erfolgreichem "Verbindung testen"
6. `http://localhost:11434` ist immer vorhanden (nicht loeschbar)

## Trigger: Wann wird eine URL zur History hinzugefuegt?

Nur bei erfolgreichem Verbindungstest ("Verbindung testen" Button). Verhindert Tippfehler in der Liste.

## Betroffene Dateien

| Datei | Aenderung |
|-------|-----------|
| `src/lib/components/settings/SettingsOllamaProvider.svelte` | Input → Combobox-Dropdown mit History, Status-Dots, Delete-Buttons |
| `src/lib/components/settings/SettingsOllama.svelte` | Server-History laden/speichern, Status-Check-Logik bei init() |
| `src/lib/i18n/de.json` | Neue i18n-Keys (Tooltips fuer Loeschen, Status) |
| `src/lib/i18n/en.json` | Neue i18n-Keys (englisch) |

## Kein Backend-Change noetig

Die gesamte Logik wird im Frontend umgesetzt:
- History speichern: `set_setting("ollama_server_history", JSON.stringify(urls))`
- History laden: `JSON.parse(get_setting("ollama_server_history"))`
- Status pruefen: bestehender `test_ai_provider` Command pro URL
