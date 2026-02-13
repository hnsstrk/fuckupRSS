import js from "@eslint/js";
import tseslint from "typescript-eslint";
import svelte from "eslint-plugin-svelte";
import globals from "globals";

export default tseslint.config(
  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...svelte.configs["flat/recommended"],
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
  },
  {
    files: ["**/*.svelte", "**/*.svelte.ts", "**/*.svelte.js"],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
      },
    },
  },
  {
    files: ["**/*.ts", "**/*.svelte"],
    rules: {
      // TypeScript handles undefined checks - disable JS no-undef
      "no-undef": "off",
    },
  },
  {
    rules: {
      "@typescript-eslint/no-explicit-any": "warn",
      "@typescript-eslint/no-unused-vars": [
        "warn",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
      "@typescript-eslint/ban-ts-comment": "warn",
      "svelte/no-at-html-tags": "warn",
      "svelte/require-each-key": "warn",
      "svelte/prefer-svelte-reactivity": "warn",
      "svelte/prefer-writable-derived": "warn",
      "svelte/no-unused-svelte-ignore": "warn",
      "no-console": ["warn", { allow: ["warn", "error"] }],
    },
  },
  {
    ignores: [
      "dist/",
      "node_modules/",
      "src-tauri/",
      "static/",
      "coverage/",
      "playwright-report/",
      "test-results/",
      "sbom/",
      "*.config.js",
      "*.config.ts",
    ],
  },
);
