# Quality Checklist: Systematic Problem Investigation

This checklist is used for systematic code quality verification, particularly for frontend-backend communication.

**Created:** 2026-01-14
**Based on:** Analysis session with 78 invoke calls and 6 files

---

## 1. State Consistency After Backend Operations

### Checklist for Every `invoke()` Call

- [ ] **Is state updated correctly?**
  - After data changes: Reload relevant state variables
  - After deletions: Clean up dependent data
  - After sync: Update counts and lists

- [ ] **Which state variables are affected?**
  | Operation | State to Update |
  |-----------|-------------------------|
  | Delete feed | `pentacles`, `fnords`, `changedFnords`, `unprocessedCount` |
  | Sync feed | `pentacles`, `fnords`, `unprocessedCount` |
  | Process article | `fnords` (individual), `unprocessedCount` |
  | Fetch full text | `unprocessedCount` |
  | Change status | `fnords` (individual), `pentacles` (unread_count) |

### Known Problem Areas (fixed)

```typescript
// PATTERN: Always update relevant state after backend operations
async syncAllFeeds() {
  const result = await invoke<SyncAllResponse>("sync_all_feeds");
  await this.loadPentacles();        // Feed counts
  await this.loadFnords();           // Article list
  await this.loadUnprocessedCount(); // Batch button badge ← IMPORTANT!
}
```

---

## 2. Error Handling

### Minimum Requirements for Every `invoke()` Call

- [ ] **Try-catch block present?**
- [ ] **Error logged?** (`console.error`)
- [ ] **User feedback?** (Toast for user-initiated actions)
- [ ] **Graceful degradation?** (App remains usable)

### Error Handling Pattern

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

### When Is Error Handling Critical?

| Priority | Situation | Example |
|----------|-----------|---------|
| **HIGH** | User-initiated action | Button click, form submit |
| **HIGH** | Data-modifying operation | Delete, save, sync |
| **MEDIUM** | Background loading operation | Initial load, refresh |
| **LOW** | Optional features | Statistics, suggestions |

---

## 3. Event Listener Management

### Checklist for `listen()` and `addEventListener()`

- [ ] **Cleanup in `onDestroy` / `onMount` return?**
- [ ] **Unlisten function stored?**
- [ ] **No duplicate listeners?**

### Correct Pattern (Tauri Events)

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

### Correct Pattern (CustomEvents for Data Refresh)

Components that display backend data MUST listen for change events:

```typescript
// Refresh handler
async function handleRefresh() {
  await loadData();
}

onMount(() => {
  window.addEventListener('batch-complete', handleRefresh);
  window.addEventListener('keywords-changed', handleRefresh);
  // ... existing onMount code ...
});

onDestroy(() => {
  window.removeEventListener('batch-complete', handleRefresh);
  window.removeEventListener('keywords-changed', handleRefresh);
});
```

### Available CustomEvents

| Event | Triggered by | When |
|-------|--------------|------|
| `batch-complete` | `state.svelte.ts` | After batch processing completes |
| `keywords-changed` | `state.svelte.ts`, `networkStore` | After keyword mutations (create, merge, rename, delete, batch) |

### Which Components Must Listen for Which Events?

| Component | `batch-complete` | `keywords-changed` |
|-----------|:-:|:-:|
| KeywordNetwork (via networkStore) | Yes | Yes |
| FnordView | Yes | Yes |
| KeywordTable | Yes | Yes |
| CompoundKeywordManager | Yes | Yes |
| ArticleView | Yes | - |
| ArticleCategories | Yes | - |
| ArticleKeywords | - | Yes |
| KeywordNetworkSynonyms | - | Yes |

### Memory Leak Indicators

- Event handlers without cleanup
- Listeners in `$effect` without cleanup logic
- Global event registration without deregistration

---

## 4. Systematic Code Review Steps

### Step 1: Analyze Invoke Calls

```bash
# Find all invoke calls
grep -rn "invoke(" src/lib --include="*.svelte" --include="*.ts"

# Group by files
grep -l "invoke(" src/lib --include="*.svelte" --include="*.ts"
```

**Check for each call:**
1. Is the call in a try-catch?
2. Is relevant state updated afterwards?
3. Is there user feedback on errors?

### Step 2: Analyze Event Listeners

```bash
# Tauri listen() calls
grep -rn "listen(" src/lib --include="*.svelte" --include="*.ts"

# DOM event listeners
grep -rn "addEventListener" src/lib --include="*.svelte" --include="*.ts"
```

**Check for each listener:**
1. Is there a cleanup?
2. Is the cleanup in the correct lifecycle hook?

### Step 3: Analyze State Flows

```bash
# Find state mutations
grep -rn "appState\." src/lib --include="*.svelte" | grep -v "appState\.\w\+\s*[^=]"
```

**Check for each mutation:**
1. Are dependent states also updated?
2. Is the update order correct?

---

## 5. Test Requirements

### Unit Tests (Vitest)

| What to test? | How? |
|---------------|------|
| State updates after invoke | Mock invoke, verify call order |
| Error handling | Mock reject, verify toast/console |
| Computed values | Test $derived logic |

### E2E Tests (Playwright)

| What to test? | How? |
|---------------|------|
| User flows | Interact like a user |
| UI feedback | Verify toasts, loading states |
| Error states | Simulate network errors |

### Test Limitations

⚠️ **Known limitation:** Svelte 5 Runes ($state) do not react to mocked invoke calls in tests. Therefore:
- Mark UI state tests as `.skip` or
- Write only API-level tests (call order)

---

## 6. Component-Specific Checklists

### SettingsView.svelte

- [ ] All `set_setting` calls have error handling
- [ ] Model switching triggers UI feedback
- [ ] Prompt changes are confirmed

### ArticleView.svelte

- [ ] Status changes update feed counts
- [ ] Full text retrieval updates `unprocessedCount`
- [ ] Analysis updates article and `unprocessedCount`

### Sidebar.svelte

- [ ] Feed deletion cleans up all dependent states
- [ ] Sync updates counts correctly

### KeywordNetwork.svelte

- [ ] Uses `networkStore` for state management (not local state!)
- [ ] Event listeners via `networkStore.setupEventListeners()` / `teardownEventListeners()`
- [ ] Error handling for all keyword operations
- [ ] After keyword mutations, `keywords-changed` event is dispatched

---

## 7. Automated Checks (TODO)

### Potential ESLint Rules

```javascript
// .eslintrc.js (concept)
rules: {
  // Warning on invoke without try-catch
  "no-unhandled-invoke": "warn",

  // Warning on listen without cleanup
  "require-listener-cleanup": "warn"
}
```

### CI Pipeline Checks

```yaml
# .github/workflows/quality.yml (concept)
- name: Check invoke error handling
  run: |
    # Find invoke without try-catch
    grep -rn "invoke(" src/lib | grep -v "try" | wc -l
```

---

## 8. Known Exceptions

### Acceptable Missing Error Handlers

| File | Function | Reason |
|------|----------|--------|
| `state.svelte.ts` | Initial loads | Errors are shown in the UI as empty lists |
| `KeywordNetwork` | Internal operations | Has its own error management |

### Intentionally Isolated States

| Component | State | Reason |
|-----------|-------|--------|
| `KeywordTrendChart` | Chart data | Visualization-specific |

**Note:** KeywordNetwork has been using the centralized `networkStore` instead of local state since 2026-02.

---

## Change History

| Date | Change |
|------|--------|
| 2026-01-14 | Initial version based on code review |
| 2026-02-12 | Documented event refresh pattern (batch-complete, keywords-changed), KeywordNetwork store consolidation |
