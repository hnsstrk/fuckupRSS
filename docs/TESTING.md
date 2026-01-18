# Testing Guide

> **This is the dedicated Testing Guide for fuckupRSS.**
> For the full developer context, see [CLAUDE.md](../CLAUDE.md).

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
| `cargo test --manifest-path src-tauri/Cargo.toml` | Run all Rust backend tests |
| `cargo tarpaulin --manifest-path src-tauri/Cargo.toml` | Rust coverage report |

---

## Test Overview

| Area | Test Count | Tool |
|------|------------|------|
| Rust Backend | 160 Tests | `cargo test` |
| Frontend (Vitest) | 95 Tests | `npm run test` |
| E2E (Playwright) | 14 Tests | `npm run test:e2e` |
| **Total** | **269 Tests** | |

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
│   └── app.spec.ts              # App-Tests
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
| `playwright.config.ts` | Playwright E2E configuration |
| `src/lib/__tests__/setup.ts` | Vitest global setup and mocks |
| `e2e/fixtures.ts` | Playwright fixtures and Tauri API mocks |

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

- [CLAUDE.md](../CLAUDE.md) - Full developer context and project guidelines
- [fuckupRSS-Anforderungen.md](../fuckupRSS-Anforderungen.md) - Technical specification
- [README.md](../README.md) - Project overview and installation
