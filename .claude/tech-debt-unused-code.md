# Unused Code Analysis

## Priority 1 - Unused Navigation Store Methods (3 functions)
- `toggleView()` - Never called
- `navigateToFnord()` - Never called
- `consumePendingKeyword()` - Never called

## Priority 2 - Unused State Store Functions (4 functions)
- `getMainCategories()` - Declared but no external usage
- `getSubcategories()` - Declared but no external usage
- `getSubcategoryNames()` - Declared but no external usage
- `getAllTags()` - Declared but no external usage

## Priority 3 - Unused Sanitizer Functions (3 functions)
- `sanitizeStrictContent()` - Not used
- `sanitizePlainText()` - Not used
- `containsHtml()` - Not used

## Minor Issues
- `KeywordStats` interface duplication (also defined locally in StatusBar.svelte)
- Possible unused diff utilities: `hasChanges()`, `visualizeWhitespace()`, `wrapWhitespaceChars()`
