# Quality Checklist: Systematische Problemsuche

Diese Checkliste dient der systematischen Überprüfung der Code-Qualität, insbesondere der Frontend-Backend-Kommunikation.

**Erstellt:** 2026-01-14
**Basierend auf:** Analyse-Session mit 78 invoke-Calls und 6 Dateien

---

## 1. State-Konsistenz nach Backend-Operationen

### Checkliste für jeden `invoke()` Call

- [ ] **Wird State korrekt aktualisiert?**
  - Nach Daten-Änderungen: Relevante State-Variablen neu laden
  - Nach Löschungen: Abhängige Daten bereinigen
  - Nach Sync: Counts und Listen aktualisieren

- [ ] **Welche State-Variablen sind betroffen?**
  | Operation | Zu aktualisierende State |
  |-----------|-------------------------|
  | Feed löschen | `pentacles`, `fnords`, `changedFnords`, `unprocessedCount` |
  | Feed sync | `pentacles`, `fnords`, `unprocessedCount` |
  | Artikel verarbeiten | `fnords` (einzeln), `unprocessedCount` |
  | Volltext abrufen | `unprocessedCount` |
  | Status ändern | `fnords` (einzeln), `pentacles` (unread_count) |

### Bekannte Problemstellen (behoben)

```typescript
// PATTERN: Nach Backend-Operation immer relevanten State aktualisieren
async syncAllFeeds() {
  const result = await invoke<SyncAllResponse>("sync_all_feeds");
  await this.loadPentacles();        // Feed-Counts
  await this.loadFnords();           // Artikel-Liste
  await this.loadUnprocessedCount(); // Batch-Button Badge ← WICHTIG!
}
```

---

## 2. Error Handling

### Mindestanforderungen für jeden `invoke()` Call

- [ ] **Try-Catch Block vorhanden?**
- [ ] **Fehler geloggt?** (`console.error`)
- [ ] **User-Feedback?** (Toast bei user-initiierten Aktionen)
- [ ] **Graceful Degradation?** (App bleibt benutzbar)

### Error-Handling Pattern

```typescript
async function handleUserAction() {
  try {
    await invoke("backend_command", { param });
    toasts.success($_('action.success'));
  } catch (e) {
    console.error("Failed to perform action:", e);
    toasts.error($_('action.error'));
  }
}
```

### Wann ist Error-Handling kritisch?

| Priorität | Situation | Beispiel |
|-----------|-----------|----------|
| **HOCH** | User-initiierte Aktion | Button-Klick, Formular-Submit |
| **HOCH** | Datenverändernde Operation | Löschen, Speichern, Sync |
| **MITTEL** | Hintergrund-Ladeoperation | Initial Load, Refresh |
| **NIEDRIG** | Optionale Features | Statistiken, Vorschläge |

---

## 3. Event Listener Management

### Checkliste für `listen()` und `addEventListener()`

- [ ] **Cleanup in `onDestroy` / `onMount` return?**
- [ ] **Unlisten-Funktion gespeichert?**
- [ ] **Keine doppelten Listener?**

### Korrektes Pattern (Tauri Events)

```typescript
onMount(() => {
  const unlistenTauri = listen("event-name", handler);
  const unlistenCustom = listen("custom-event", handler);

  return async () => {
    (await unlistenTauri)();
    (await unlistenCustom)();
  };
});
```

### Korrektes Pattern (CustomEvents fuer Daten-Refresh)

Komponenten die Backend-Daten anzeigen MUESSEN auf Aenderungs-Events lauschen:

```typescript
// Refresh-Handler
async function handleRefresh() {
  await loadData();
}

onMount(() => {
  window.addEventListener('batch-complete', handleRefresh);
  window.addEventListener('keywords-changed', handleRefresh);
  // ... bestehender onMount Code ...
});

onDestroy(() => {
  window.removeEventListener('batch-complete', handleRefresh);
  window.removeEventListener('keywords-changed', handleRefresh);
});
```

### Verfuegbare CustomEvents

| Event | Ausgelöst von | Wann |
|-------|---------------|------|
| `batch-complete` | `state.svelte.ts` | Nach Batch-Processing Abschluss |
| `keywords-changed` | `state.svelte.ts`, `networkStore` | Nach Keyword-Mutationen (create, merge, rename, delete, batch) |

### Welche Komponenten muessen auf welche Events lauschen?

| Komponente | `batch-complete` | `keywords-changed` |
|------------|:-:|:-:|
| KeywordNetwork (via networkStore) | Ja | Ja |
| FnordView | Ja | Ja |
| KeywordTable | Ja | Ja |
| CompoundKeywordManager | Ja | Ja |
| ArticleView | Ja | - |
| ArticleCategories | Ja | - |
| ArticleKeywords | - | Ja |
| KeywordNetworkSynonyms | - | Ja |

### Memory Leak Indikatoren

- Event Handler ohne Cleanup
- Listener in `$effect` ohne Cleanup-Logic
- Globale Event-Registrierung ohne Deregistrierung

---

## 4. Systematische Code-Review Schritte

### Schritt 1: Invoke-Calls analysieren

```bash
# Alle invoke-Calls finden
grep -rn "invoke(" src/lib --include="*.svelte" --include="*.ts"

# Nach Dateien gruppieren
grep -l "invoke(" src/lib --include="*.svelte" --include="*.ts"
```

**Prüfen für jeden Call:**
1. Ist der Call in try-catch?
2. Wird relevanter State danach aktualisiert?
3. Gibt es User-Feedback bei Fehlern?

### Schritt 2: Event-Listener analysieren

```bash
# Tauri listen() Calls
grep -rn "listen(" src/lib --include="*.svelte" --include="*.ts"

# DOM Event Listener
grep -rn "addEventListener" src/lib --include="*.svelte" --include="*.ts"
```

**Prüfen für jeden Listener:**
1. Gibt es einen Cleanup?
2. Ist der Cleanup im richtigen Lifecycle-Hook?

### Schritt 3: State-Flows analysieren

```bash
# State-Mutations finden
grep -rn "appState\." src/lib --include="*.svelte" | grep -v "appState\.\w\+\s*[^=]"
```

**Prüfen für jede Mutation:**
1. Werden abhängige States auch aktualisiert?
2. Ist die Update-Reihenfolge korrekt?

---

## 5. Test-Anforderungen

### Unit Tests (Vitest)

| Was testen? | Wie? |
|-------------|------|
| State-Updates nach Invoke | Mock invoke, prüfe Call-Reihenfolge |
| Error-Handling | Mock reject, prüfe Toast/Console |
| Computed Values | Teste $derived Logik |

### E2E Tests (Playwright)

| Was testen? | Wie? |
|-------------|------|
| User Flows | Interagiere wie User |
| UI-Feedback | Prüfe Toasts, Loading States |
| Fehlerzustände | Simuliere Netzwerk-Fehler |

### Test-Limitations

⚠️ **Bekannte Einschränkung:** Svelte 5 Runes ($state) reagieren nicht auf gemockte invoke-Calls in Tests. Daher:
- UI-State-Tests markieren als `.skip` oder
- Nur API-Level Tests (Call-Reihenfolge) schreiben

---

## 6. Komponenten-spezifische Checklisten

### SettingsView.svelte

- [ ] Alle `set_setting` Calls haben Error-Handling
- [ ] Model-Wechsel triggert UI-Feedback
- [ ] Prompt-Änderungen werden bestätigt

### ArticleView.svelte

- [ ] Status-Änderungen aktualisieren Feed-Counts
- [ ] Volltext-Abruf aktualisiert `unprocessedCount`
- [ ] Analyse aktualisiert Artikel und `unprocessedCount`

### Sidebar.svelte

- [ ] Feed-Löschung bereinigt alle abhängigen States
- [ ] Sync aktualisiert Counts korrekt

### KeywordNetwork.svelte

- [ ] Nutzt `networkStore` fuer State-Management (nicht lokalen State!)
- [ ] Event-Listener via `networkStore.setupEventListeners()` / `teardownEventListeners()`
- [ ] Error-Handling für alle Keyword-Operationen
- [ ] Nach Keyword-Mutationen wird `keywords-changed` Event dispatched

---

## 7. Automatisierte Checks (TODO)

### Potenzielle ESLint Rules

```javascript
// .eslintrc.js (Konzept)
rules: {
  // Warnung bei invoke ohne try-catch
  "no-unhandled-invoke": "warn",

  // Warnung bei listen ohne cleanup
  "require-listener-cleanup": "warn"
}
```

### CI Pipeline Checks

```yaml
# .github/workflows/quality.yml (Konzept)
- name: Check invoke error handling
  run: |
    # Finde invoke ohne try-catch
    grep -rn "invoke(" src/lib | grep -v "try" | wc -l
```

---

## 8. Bekannte Ausnahmen

### Akzeptable fehlende Error-Handler

| Datei | Funktion | Grund |
|-------|----------|-------|
| `state.svelte.ts` | Initial loads | Fehler werden im UI als leere Listen angezeigt |
| `KeywordNetwork` | Interne Operationen | Hat eigenes Fehler-Management |

### Bewusst isolierte States

| Komponente | State | Grund |
|------------|-------|-------|
| `KeywordTrendChart` | Chart-Daten | Visualisierungs-spezifisch |

**Hinweis:** KeywordNetwork nutzt seit 2026-02 den zentralen `networkStore` statt lokalen State.

---

## Änderungshistorie

| Datum | Änderung |
|-------|----------|
| 2026-01-14 | Initiale Version basierend auf Code-Review |
| 2026-02-12 | Event-Refresh Pattern dokumentiert (batch-complete, keywords-changed), KeywordNetwork Store-Konsolidierung |

