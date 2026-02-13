# Testing Guide

> **This is the dedicated Testing Guide for fuckupRSS.**
> For the full developer context, see [CLAUDE.md](../../CLAUDE.md).

---

## Overview

**WICHTIG:** Alle neuen Features und Bugfixes MUESSEN mit Tests abgedeckt werden. Code ohne Tests wird nicht akzeptiert.

This project maintains a comprehensive test suite covering:
- **Rust Backend** - Unit and integration tests
- **Svelte Frontend** - Component and store tests with Vitest
- **End-to-End** - Full user flow tests with Playwright

---

## Test Commands

### Running Tests

```bash
# All tests by area
npm run test           # Frontend (Vitest)
npm run test:e2e       # E2E Tests (Playwright)
cargo test --manifest-path src-tauri/Cargo.toml  # Backend (Rust)

# Watch mode for development
npm run test:watch

# Test coverage
npm run test:coverage
cargo tarpaulin --manifest-path src-tauri/Cargo.toml
```

### Quick Reference

| Command | Description |
|---------|-------------|
| `npm run test` | Run all frontend unit tests |
| `npm run test:watch` | Frontend tests in watch mode |
| `npm run test:coverage` | Frontend coverage report |
| `npm run test:e2e` | Run Playwright E2E tests |
| `npm run pw:open` | Playwright CLI: Browser oeffnen |
| `npm run pw:snapshot` | Playwright CLI: Accessibility Snapshot |
| `npm run pw:screenshot` | Playwright CLI: Screenshot |
| `npm run pw:close` | Playwright CLI: Browser schliessen |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Run all Rust backend tests |
| `cargo tarpaulin --manifest-path src-tauri/Cargo.toml` | Rust coverage report |

---

## Test Overview

| Area | Test Count | Tool |
|------|------------|------|
| Rust Backend | 160 Tests | `cargo test` |
| Frontend (Vitest) | 95 Tests | `npm run test` |
| E2E (Playwright) | 32 Tests (9 skipped) | `npm run test:e2e` |
| **Total** | **287 Tests** | |

---

## Playwright CLI (Interaktives Browser-Testing)

### Was ist Playwright CLI?

Playwright CLI (`@playwright/cli`) ist ein interaktives Browser-Automatisierungstool, das sich von `@playwright/test` unterscheidet:

| Aspekt | `@playwright/test` | `@playwright/cli` (Playwright CLI) |
|--------|--------------------|------------------------------------|
| **Zweck** | Automatisierte Test-Suites | Interaktive Browser-Steuerung |
| **Ausfuehrung** | Headless, CI/CD-tauglich | Headed, explorativ |
| **Ergebnisse** | Pass/Fail, Reports | Snapshots, Screenshots, generierten Test-Code |
| **Anwendung** | Regressions-Tests | Debugging, Exploration, Test-Generierung |
| **Claude Code** | `npm run test:e2e` | Skills via `playwright-cli` Commands |

### Installation und Setup

Playwright CLI ist bereits installiert und konfiguriert:

```bash
# Bereits in devDependencies
npm install  # installiert @playwright/cli

# Skills fuer Claude Code
# Skills liegen in .claude/skills/playwright-cli/
```

### Konfiguration

Die CLI-Konfiguration liegt in `playwright-cli.json`:

```json
{
  "browser": "chrome",
  "baseURL": "http://localhost:1420",
  "timeout": 30000
}
```

**Voraussetzung:** Der Vite Dev-Server muss laufen (`npm run dev` oder `npm run tauri dev`).

### npm Scripts

| Script | Beschreibung |
|--------|-------------|
| `npm run pw:open` | Browser oeffnen mit Projekt-Konfiguration |
| `npm run pw:snapshot` | Accessibility Snapshot der aktuellen Seite |
| `npm run pw:screenshot` | Screenshot der aktuellen Seite |
| `npm run pw:close` | Browser schliessen |

### Skills fuer Claude Code

Playwright CLI stellt Skills bereit, die Claude Code direkt nutzen kann:

| Skill-Kategorie | Beispiel-Commands | Zweck |
|-----------------|-------------------|-------|
| **Navigation** | `playwright-cli open`, `goto`, `go-back` | Seiten aufrufen |
| **Interaktion** | `click`, `fill`, `type`, `select` | UI-Elemente bedienen |
| **Inspektion** | `snapshot`, `screenshot`, `console`, `network` | Seite analysieren |
| **Test-Generierung** | Jede Aktion generiert Playwright-Code | Tests aus Interaktionen erstellen |
| **DevTools** | `tracing-start`, `tracing-stop`, `video-start` | Debugging und Aufzeichnung |
| **Storage** | `cookie-list`, `localstorage-get`, `state-save` | Browser-State verwalten |
| **Network** | `route`, `unroute` | Request-Mocking |

### Typische Workflows

**1. App interaktiv testen:**
```bash
npm run tauri dev              # App starten
npm run pw:open                # Browser oeffnen
npm run pw:snapshot            # Accessibility-Tree anzeigen
playwright-cli click e5        # Element interagieren
npm run pw:screenshot          # Ergebnis festhalten
npm run pw:close               # Browser schliessen
```

**2. Test-Code generieren:**
```bash
playwright-cli open http://localhost:1420
playwright-cli snapshot                        # Elemente identifizieren
playwright-cli fill e1 "https://feed.url"      # Generiert: await page.getByRole('textbox'...).fill(...)
playwright-cli click e3                        # Generiert: await page.getByRole('button'...).click()
# Generierten Code in E2E-Test-Datei uebernehmen
```

**3. Debugging mit Tracing:**
```bash
playwright-cli open http://localhost:1420
playwright-cli tracing-start
# Aktionen ausfuehren...
playwright-cli tracing-stop    # Trace-Datei zum Analysieren
```

### Wann CLI vs. klassische E2E Tests verwenden?

| Situation | Empfohlenes Tool |
|-----------|-----------------|
| Neues Feature explorativ testen | **Playwright CLI** |
| Automatisierte Regressions-Tests | **@playwright/test** (`npm run test:e2e`) |
| Bug reproduzieren und untersuchen | **Playwright CLI** |
| CI/CD Pipeline | **@playwright/test** |
| Test-Code fuer neue E2E Tests generieren | **Playwright CLI** (dann in Test-Datei uebernehmen) |
| Accessibility-Struktur pruefen | **Playwright CLI** (`snapshot`) |
| Screenshots fuer Dokumentation | **Playwright CLI** (`screenshot`) |

### Referenz-Dokumentation

Detaillierte Anleitungen finden sich in `.claude/skills/playwright-cli/references/`:

| Datei | Thema |
|-------|-------|
| `test-generation.md` | Test-Code aus Interaktionen generieren |
| `running-code.md` | Playwright-Code direkt ausfuehren |
| `request-mocking.md` | Netzwerk-Requests mocken |
| `session-management.md` | Browser-Sessions verwalten |
| `storage-state.md` | Cookies, LocalStorage |
| `tracing.md` | Tracing fuer Debugging |
| `video-recording.md` | Video-Aufzeichnung |

---

## Bewertung der skipped E2E Tests

### Ueberblick

Aktuell sind **5 von 14 E2E Tests skipped** in `app.spec.ts` und **4 von 18 Tests skipped** in `erisian-archives.spec.ts`. Alle aus dem gleichen Grund: **Svelte-Reaktivitaet funktioniert nicht zuverlaessig mit gemockten Tauri APIs.**

### Ursache

Die E2E Tests verwenden `e2e/fixtures.ts`, das `window.__TAURI_INTERNALS__` mockt. Das Mock-System funktioniert fuer:
- Initiales Laden von Daten (GET-artige Aufrufe)
- Verifizierung von API-Aufrufen (Invoke-Tracking)

Es funktioniert **nicht** fuer:
- Svelte-State-Updates nach Aktionen (z.B. Button-Click -> Dialog oeffnen)
- UI-Reaktionen auf State-Aenderungen (z.B. Badge-Update nach Sync)
- Tab-Wechsel mit CSS-Klassen-Updates

### Kann Playwright CLI helfen?

| Problem | CLI-Loesung? | Begruendung |
|---------|-------------|-------------|
| Dialog oeffnet nicht nach Click | **Teilweise** | CLI kann den tatsaechlichen App-State (mit `npm run tauri dev`) testen, aber nicht die automatisierten Mock-Tests ersetzen |
| Badge-Update nach Sync | **Nein** | Braucht echtes Tauri-Backend fuer Sync |
| Tab-Wechsel CSS-Updates | **Ja** | CLI kann gegen die laufende App testen und Tab-Wechsel verifizieren |
| State-Updates nach Invoke | **Nein** | Grundlegendes Mock-Limitierungsproblem |

### Empfehlungen

1. **Playwright CLI als Explorationstools nutzen:** Fuer manuelle Verifikation der skipped Szenarien gegen die laufende App (`npm run tauri dev`)
2. **Skipped Tests beibehalten:** Die Tests dokumentieren erwartetes Verhalten - sie koennen aktiviert werden, wenn das Mock-System verbessert wird
3. **Aktive Tests priorisieren:** Die 9+14 aktiven Tests decken API-Integration und Basis-Layout zuverlaessig ab
4. **Langfristig:** Evaluieren ob `@tauri-apps/api/mocks` oder ein eigenes Mock-Framework die Svelte-Reaktivitaet besser unterstuetzen koennte

---

## Test Structure

```
fuckupRSS/
├── src/
│   └── lib/
│       └── __tests__/           # Frontend Unit Tests (Vitest)
│           ├── setup.ts         # Test-Setup mit Mocks
│           ├── stores/          # Store Tests
│           │   ├── state.test.ts      # State Management Tests (18 Tests)
│           │   ├── network.test.ts    # Immanentize Network Tests (31 Tests)
│           │   └── navigation.test.ts # Navigation Events Tests (21 Tests)
│           └── components/      # Component Tests
│               └── Toast.test.ts      # Toast Component Tests (19 Tests)
├── e2e/                         # E2E Tests (Playwright)
│   ├── fixtures.ts              # Tauri API Mocks
│   ├── app.spec.ts              # App-Layout, Sidebar, Settings, Theme, Accessibility
│   └── erisian-archives.spec.ts # ErisianArchives: Tabs, Stats, Artikel-Liste
├── src-tauri/
│   └── src/
│       ├── db/
│       │   └── tests.rs         # DB Unit Tests (14 Tests)
│       ├── sync/
│       │   └── tests.rs         # Sync Unit Tests (14 Tests)
│       ├── retrieval/
│       │   └── tests.rs         # Retrieval Unit Tests (22 Tests)
│       ├── ollama/
│       │   └── tests.rs         # Ollama Unit Tests (33 Tests)
│       └── commands/
│           ├── tests.rs         # Batch-Analyse Unit Tests (31 Tests)
│           └── batch_integration_tests.rs  # DB-Integration (9 Tests)
```

---

## Test Requirements

| Area | Requirement | Tool |
|------|-------------|------|
| Rust Backend | Unit Tests for all modules | `cargo test` |
| Tauri Commands | Integration Tests | `cargo test` |
| Svelte Stores | Unit Tests for State Logic | Vitest |
| Svelte Components | Component Tests | Vitest + Testing Library |
| User Flows | E2E Tests | Playwright |

---

## Test Patterns

### Rust Unit Test

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        // Arrange
        let input = ...;

        // Act
        let result = function_name(input);

        // Assert
        assert_eq!(result, expected);
    }
}
```

**Key points:**
- Use `#[cfg(test)]` to compile tests only in test mode
- Follow Arrange-Act-Assert pattern
- Name tests descriptively: `test_<function>_<scenario>_<expected_result>`

### Frontend Component Test (Vitest)

```typescript
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import Component from './Component.svelte';

describe('Component', () => {
  it('renders correctly', () => {
    render(Component, { props: { ... } });
    expect(screen.getByText('...')).toBeInTheDocument();
  });
});
```

**Key points:**
- Use `@testing-library/svelte` for component rendering
- Query elements using accessible selectors (`getByText`, `getByRole`, etc.)
- Test user-visible behavior, not implementation details

### Frontend Store Test (Vitest)

```typescript
import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { myStore, resetStore } from '$lib/stores/myStore.svelte';

describe('myStore', () => {
  beforeEach(() => {
    resetStore();
  });

  it('updates state correctly', () => {
    myStore.someAction();
    expect(get(myStore).value).toBe(expected);
  });
});
```

### E2E Test (Playwright)

```typescript
import { test, expect } from '@playwright/test';

test('user can add a feed', async ({ page }) => {
  await page.goto('/');
  await page.fill('[data-testid="feed-url"]', 'https://example.com/feed.xml');
  await page.click('[data-testid="add-feed"]');
  await expect(page.locator('.feed-item')).toBeVisible();
});
```

**Key points:**
- Use `data-testid` attributes for reliable element selection
- Test complete user flows
- Include assertions for expected outcomes
- Use Playwright's auto-waiting capabilities

---

## When to Write Tests

| Timing | Description |
|--------|-------------|
| **BEFORE implementing** | TDD preferred - write the test first |
| **DURING implementation** | For complex logic that needs verification |
| **AFTER implementation** | Minimum: all public APIs must be tested |
| **For bugfixes** | Write a test that reproduces the bug, then fix it |

### TDD Workflow (Recommended)

1. Write a failing test that describes the expected behavior
2. Implement the minimum code to make the test pass
3. Refactor while keeping tests green
4. Repeat

### Bugfix Workflow

1. Write a test that reproduces the bug
2. Verify the test fails
3. Fix the bug
4. Verify the test passes
5. Add edge case tests if needed

---

## Coverage Requirements

While no strict coverage percentage is enforced, the following guidelines apply:

| Component Type | Expected Coverage |
|----------------|-------------------|
| Public API functions | 100% |
| Critical business logic | 90%+ |
| Edge cases and error paths | High priority |
| UI components | Key interactions covered |

### Generating Coverage Reports

**Frontend (Istanbul/Vitest):**
```bash
npm run test:coverage
# Report generated in coverage/ directory
```

**Rust Backend (Tarpaulin):**
```bash
cargo tarpaulin --manifest-path src-tauri/Cargo.toml
# Summary printed to console
```

---

## Test Configuration Files

| File | Purpose |
|------|---------|
| `vitest.config.ts` | Vitest configuration |
| `playwright.config.ts` | Playwright E2E configuration (`@playwright/test`) |
| `playwright-cli.json` | Playwright CLI Konfiguration (Browser, baseURL, Timeout) |
| `src/lib/__tests__/setup.ts` | Vitest global setup and mocks |
| `e2e/fixtures.ts` | Playwright fixtures and Tauri API mocks |
| `.claude/skills/playwright-cli/` | Claude Code Skills fuer Playwright CLI |

---

## Mocking in Tests

### Mocking Tauri APIs (Frontend)

The test setup includes mocks for Tauri's `invoke` function. See `src/lib/__tests__/setup.ts` for the mock configuration.

```typescript
// Example: Mocking a Tauri command
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn((cmd: string) => {
    if (cmd === 'get_pentacles') {
      return Promise.resolve([...mockPentacles]);
    }
    // ... other commands
  }),
}));
```

### Mocking in Rust

Use conditional compilation and mock structs:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Create mock implementations for external dependencies
    struct MockOllamaClient {
        response: String,
    }

    impl MockOllamaClient {
        fn generate(&self, _prompt: &str) -> Result<String> {
            Ok(self.response.clone())
        }
    }
}
```

---

## Best Practices

1. **Keep tests independent** - Each test should be able to run in isolation
2. **Use descriptive names** - Test names should describe what is being tested and expected outcome
3. **Avoid test interdependence** - Don't rely on test execution order
4. **Clean up after tests** - Reset state, close connections, remove test data
5. **Test edge cases** - Empty inputs, null values, boundary conditions
6. **Test error handling** - Verify error messages and recovery paths
7. **Keep tests fast** - Mock external dependencies to avoid slow I/O

---

## Related Documentation

- [CLAUDE.md](../../CLAUDE.md) - Full developer context and project guidelines
- [docs/ANFORDERUNGEN.md](../../docs/ANFORDERUNGEN.md) - Technical specification
- [README.md](../../README.md) - Project overview and installation
