# TypeScript/Svelte Tech Debt Analysis

**Generated:** 2026-01-19
**Tool:** `npx svelte-check --threshold error`
**Total Errors:** 94 errors in 34 files

---

## Summary by Category

| Category | Error Count | Files Affected |
|----------|-------------|----------------|
| Module Import Issues (allowSyntheticDefaultImports) | 5 | 4 |
| Missing Module `$lib/types` | 4 | 4 |
| Missing Module `$lib/utils/sanitizer` | 1 | 1 |
| Unused Variables/Declarations | 12 | 8 |
| Type Mismatches | 8 | 5 |
| Test File Type Issues (`result is unknown`) | 61 | 8 |
| Implicit `any` Type | 3 | 3 |

---

## 1. Module Import Issues (allowSyntheticDefaultImports)

### Root Cause
TypeScript's `esModuleInterop` and `allowSyntheticDefaultImports` flags are not enabled, preventing default imports from CommonJS modules that don't have a default export.

### Affected Files (5 errors in 4 files)

| File | Import |
|------|--------|
| `src/lib/i18n/index.ts` | `import de from './de.json'` |
| `src/lib/i18n/index.ts` | `import en from './en.json'` |
| `src/lib/utils/sanitizer.ts` | `import DOMPurify from 'dompurify'` |
| `src/lib/components/NeighborGraph.svelte` | `import cytoscape from 'cytoscape'` |

### Suggested Fix

**Option A (Recommended):** Update `tsconfig.json` to enable synthetic default imports:
```json
{
  "compilerOptions": {
    "esModuleInterop": true,
    "allowSyntheticDefaultImports": true
  }
}
```

**Option B:** Change imports to namespace imports:
```typescript
// Before
import de from './de.json';
// After
import * as de from './de.json';
```

---

## 2. Missing Module `$lib/types`

### Root Cause
The file `src/lib/types.ts` (or `src/lib/types/index.ts`) does not exist. Components are importing types that should be defined in a central types file.

### Affected Files (4 errors)

| File | Missing Types |
|------|---------------|
| `src/lib/components/article/ArticleKeywords.svelte` | `ArticleKeyword`, `KeywordType`, `ExtractionMethod`, `CorrectionInput` |
| `src/lib/components/article/ArticleCategories.svelte` | `ArticleCategoryDetailed`, `Sephiroth`, `CorrectionInput` |
| `src/lib/components/article/StatisticalPreview.svelte` | `StatisticalAnalysis`, `KeywordCandidateResult`, `CategoryScoreResult` |
| `src/lib/components/ArticleView.svelte` | `ArticleKeyword`, `ArticleCategoryDetailed` |

### Suggested Fix

Create `src/lib/types.ts` or `src/lib/types/index.ts` with all required type definitions:
```typescript
// src/lib/types.ts
export interface ArticleKeyword {
  id: number;
  keyword: string;
  keyword_type: KeywordType;
  extraction_method: ExtractionMethod;
  // ... other fields
}

export type KeywordType = 'topic' | 'entity' | 'concept';
export type ExtractionMethod = 'llm' | 'statistical' | 'manual';

export interface CorrectionInput {
  // ... fields
}

// ... etc
```

---

## 3. Missing Module `$lib/utils/sanitizer`

### Root Cause
The file exists at `src/lib/utils/sanitizer.ts` but TypeScript cannot resolve it, likely due to the same `allowSyntheticDefaultImports` issue affecting the DOMPurify import within that file.

### Affected Files (1 error)

| File | Import |
|------|--------|
| `src/lib/components/ArticleView.svelte` | `import { sanitizeArticleContent } from '$lib/utils/sanitizer'` |

### Suggested Fix

Fix the DOMPurify import in `src/lib/utils/sanitizer.ts` (see Category 1), and this error should resolve.

---

## 4. Unused Variables/Declarations

### Root Cause
Variables or type declarations that are defined but never used in the code.

### Affected Files (12 errors in 8 files)

| File | Unused Declaration | Type |
|------|-------------------|------|
| `src/lib/components/article/ArticleCategories.svelte` | `GroupedCategory` | interface |
| `src/lib/components/article/StatisticalPreview.svelte` | `KeywordCandidateResult` | type import |
| `src/lib/components/article/StatisticalPreview.svelte` | `CategoryScoreResult` | type import |
| `src/lib/components/ArticleView.svelte` | `formatShortDate` | function |
| `src/lib/components/ErisianArchives.svelte` | `failedCount` | variable |
| `src/lib/components/ErisianArchives.svelte` | `hopelessCount` | variable |
| `src/lib/utils/sanitizer.ts` | `data` | parameter |
| `src/lib/components/ArticleList.svelte` | `listContainer` | variable |
| `src/lib/__tests__/components/ErisianArchives.test.ts` | `mockFnords` | variable |
| `src/lib/__tests__/components/ErisianArchives.test.ts` | `invokeHandler` | function |
| `src/lib/__tests__/components/Sidebar.test.ts` | `mockPentacles` | variable |
| `src/lib/__tests__/components/Sidebar.test.ts` | `invokeHandler` | function |

### Suggested Fix

**For unused imports:** Remove the unused type imports
```typescript
// Before
import type { StatisticalAnalysis, KeywordCandidateResult, CategoryScoreResult } from '$lib/types';
// After
import type { StatisticalAnalysis } from '$lib/types';
```

**For unused variables:** Either use them or remove them. If planned for future use, prefix with underscore:
```typescript
// Signals to TypeScript this is intentionally unused
let _failedCount = $state(0);
```

**For unused parameters:** Use underscore prefix
```typescript
DOMPurify.addHook('uponSanitizeElement', (node: Element, _data) => {
```

---

## 5. Type Mismatches

### Root Cause
Incompatible types between component props and the data being passed. Most commonly `null` vs `undefined` for optional string fields.

### Affected Files (8 errors in 5 files)

| File | Error | Issue |
|------|-------|-------|
| `src/lib/components/ArticleView.svelte:754` | `SimilarArticleCategory[]` not assignable to `Category[]` | `color: string \| null` vs `color: string \| undefined` |
| `src/lib/components/ArticleList.svelte:109` | `FnordCategoryInfo[]` not assignable to `Category[]` | Same null vs undefined issue |
| `src/lib/components/ErisianArchives.svelte:123` | Object literal not assignable to `Fnord` | Type structure mismatch |
| `src/lib/components/ErisianArchives.svelte:139` | Same as above | |
| `src/lib/components/NeighborGraph.svelte` | Type errors with cytoscape | Multiple errors |
| `src/lib/__tests__/components/MindfuckView.test.ts` | Type mismatches in test mocks | |

### Suggested Fix

**Option A:** Standardize on `null` OR `undefined` across all types (recommended: use `null` to match Rust/SQLite):
```typescript
// In Category type definition
interface Category {
  color: string | null;  // Use null consistently
}
```

**Option B:** Add type assertion or mapping where data is passed:
```typescript
categories={article.categories.map(c => ({ ...c, color: c.color ?? undefined }))}
```

---

## 6. Test File Type Issues (`result is unknown`)

### Root Cause
The mocked `invoke` function returns `unknown` type, and test code accesses properties without type assertions.

### Affected Files (61 errors in 8 test files)

| File | Error Count |
|------|-------------|
| `src/lib/__tests__/stores/state.test.ts` | 15 |
| `src/lib/__tests__/stores/network.test.ts` | 10 |
| `src/lib/__tests__/stores/batch.test.ts` | 15 |
| `src/lib/__tests__/stores/immanentize.test.ts` | 6 |
| `src/lib/__tests__/components/ErisianArchives.test.ts` | 5 |
| `src/lib/__tests__/components/Sidebar.test.ts` | 3 |
| `src/lib/__tests__/components/ArticleList.test.ts` | 4 |
| `src/lib/__tests__/components/ArticleView.test.ts` | 3 |

### Suggested Fix

**Option A (Recommended):** Create typed wrapper for invoke in tests:
```typescript
// src/lib/__tests__/helpers/invoke.ts
import { invoke } from '@tauri-apps/api/core';

export async function typedInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return invoke(cmd, args) as Promise<T>;
}

// In tests
const result = await typedInvoke<OllamaStatus>('check_ollama');
expect(result.available).toBe(true);
```

**Option B:** Add type assertions in each test:
```typescript
const result = await invoke('check_ollama') as OllamaStatus;
expect(result.available).toBe(true);
```

**Option C:** Define return types in mock setup:
```typescript
vi.mocked(invoke).mockResolvedValue({ available: true, models: [] } as OllamaStatus);
```

---

## 7. Implicit `any` Type

### Root Cause
Parameters without explicit type annotations in contexts where TypeScript cannot infer the type.

### Affected Files (3 errors)

| File | Line | Parameter |
|------|------|-----------|
| `src/lib/utils/sanitizer.ts` | 84 | `data` parameter in DOMPurify hook |
| `src/lib/__tests__/components/ErisianArchives.test.ts` | - | Various mock handlers |
| `src/lib/__tests__/components/Sidebar.test.ts` | - | Various mock handlers |

### Suggested Fix

Add explicit type annotations:
```typescript
// Before
DOMPurify.addHook('uponSanitizeElement', (node: Element, data) => {
// After
DOMPurify.addHook('uponSanitizeElement', (node: Element, data: DOMPurify.SanitizeElementHookEvent) => {
```

---

## Recommended Fix Priority

### High Priority (Blocking/Breaking)
1. **Enable `allowSyntheticDefaultImports`** - Quick fix that resolves 5 errors
2. **Create `$lib/types.ts`** - Resolves 4 errors and improves type safety

### Medium Priority (Code Quality)
3. **Fix type mismatches (null vs undefined)** - 8 errors, improves prop consistency
4. **Remove unused variables** - 12 errors, cleaner code

### Low Priority (Test-Only)
5. **Add type assertions to tests** - 61 errors, only affects test files
6. **Fix implicit any types** - 3 errors, minor improvement

---

## Quick Wins

### 1. tsconfig.json update (resolves ~5 errors immediately)
```json
{
  "compilerOptions": {
    "esModuleInterop": true,
    "allowSyntheticDefaultImports": true
  }
}
```

### 2. Remove unused imports (resolves ~5 errors)
Simple deletion of unused type imports from:
- `StatisticalPreview.svelte`
- `ArticleCategories.svelte`

### 3. Prefix unused variables with underscore (resolves ~7 errors)
```typescript
let _failedCount = $state(0);
let _hopelessCount = $state(0);
let _listContainer: HTMLDivElement;
```

---

## Files Requiring Most Attention

| File | Error Count | Categories |
|------|-------------|------------|
| `src/lib/__tests__/stores/state.test.ts` | 15 | Test types |
| `src/lib/__tests__/stores/batch.test.ts` | 15 | Test types |
| `src/lib/__tests__/stores/network.test.ts` | 10 | Test types |
| `src/lib/components/ErisianArchives.svelte` | 5 | Unused vars, type mismatch |
| `src/lib/__tests__/components/ErisianArchives.test.ts` | 6 | Test types, unused vars |
| `src/lib/utils/sanitizer.ts` | 3 | Import, unused param, implicit any |
