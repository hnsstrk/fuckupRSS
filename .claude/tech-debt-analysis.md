# Technical Debt Analysis

**Status:** Iteration 3 Completed
**Started:** 2026-01-19
**Progress:** 95 → 54 svelte-check errors (41 fixed)

## Summary

| Iteration | Errors Fixed | Errors Remaining |
|-----------|--------------|------------------|
| Start | - | 95 |
| 1 | 8 | 87 |
| 2 | 14 | 73 |
| 3 | 19 | 54 |

## Iteration 1 - Completed Fixes

| Fix | Files | Impact |
|-----|-------|--------|
| allowSyntheticDefaultImports | tsconfig.json | -5 import errors |
| Remove formatShortDate | ArticleView.svelte | -1 unused var |
| Remove failedCount/hopelessCount | ErisianArchives.svelte | -2 unused vars |
| Add #![allow(dead_code)] | headless.rs | 0 Rust warnings |
| Remove toggleView, navigateToFnord, consumePendingKeyword | navigation.svelte.ts | Cleaner code |
| Remove sanitizeStrictContent, sanitizePlainText, containsHtml | sanitizer.ts | Cleaner code |
| Fix Category interface (null vs undefined) | ArticleCard.svelte, ArticleItemCompact.svelte | -2 type errors |

## Iteration 2 - Completed Fixes

| Fix | Files | Impact |
|-----|-------|--------|
| Add $lib/* path alias | tsconfig.json | -5 module errors |
| Remove listContainer | ArticleList.svelte | -1 unused var |
| Remove GroupedCategory interface | ArticleCategories.svelte | -1 unused type |
| Remove unused type imports | StatisticalPreview.svelte | -2 unused imports |
| Prefix keywordName param | KeywordTrendChart.svelte | -1 unused param |
| Prefix neighbors, trendDays | KeywordNetworkDetail.svelte | -2 unused vars |
| Remove KeywordTrendChart import | KeywordNetwork.svelte | -1 unused import |
| Prefix keywordName param | KeywordNetwork.svelte | -1 unused param |
| Remove unused imports/vars | MindfuckView.svelte, SettingsGeneral.svelte, SettingsMaintenance.svelte, StatusBar.svelte | -5 |

## Iteration 3 - Completed Fixes

| Fix | Files | Impact |
|-----|-------|--------|
| Fix toast API (wrong arg order) | SettingsStopwords.svelte | -1 type error |
| Fix Tooltip prop (text→content) | RecommendationCard.svelte | -1 type error |
| Remove unused loadRecommendationProgress | MindfuckView.svelte | -4 unused vars/funcs |
| Remove _getSachlichkeitLabel, _formatDate | MindfuckView.svelte | -2 unused funcs |
| Remove unused RecommendationProgress interface | MindfuckView.svelte | -1 unused type |
| Remove _abortController | RecommendationList.svelte | -1 unused var |
| Remove dead code in loadCounterPerspectives | MindfuckView.svelte | Cleaner code |

## Remaining Issues (54 errors)

### By Category

| Category | Count | Notes |
|----------|-------|-------|
| Test file type issues | ~45 | `result is unknown` - low priority |
| DOMPurify type issues | 6 | Needs @types/dompurify config |
| NetworkGraph/Cytoscape types | 4 | Complex type mismatches |
| Fnord type in ErisianArchives | 2 | Missing full_text_fetch_error |

### Not Fixed (requires larger changes)

1. **DOMPurify types** - Needs proper type declaration or @types package
2. **Cytoscape types** - Complex animation/layout type mismatches
3. **Test file types** - Would require major test refactoring

## Build Status

✅ Frontend build successful
✅ Rust build successful (0 warnings)

## Bundle Size

- JS: 1,167 kB (gzip: 363 kB)
- CSS: 215 kB (gzip: 31 kB)

## Detailed Reports

- [TypeScript Analysis](./tech-debt-typescript.md)
- [Unused Code Analysis](./tech-debt-unused-code.md)
- [Test Coverage Analysis](./tech-debt-tests.md)
- [Rust Analysis](./tech-debt-rust.md)
