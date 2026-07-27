import js from "@eslint/js";
import ts from "typescript-eslint";
import svelte from "eslint-plugin-svelte";
import prettier from "eslint-config-prettier";
import globals from "globals";

export default ts.config(
  js.configs.recommended,
  ...ts.configs.recommended,
  ...svelte.configs["flat/recommended"],
  prettier,
  ...svelte.configs["flat/prettier"],
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
        __TEPUB_TARGET__: "readonly",
      },
    },
  },
  {
    files: ["**/*.svelte"],
    languageOptions: {
      parserOptions: {
        parser: ts.parser,
      },
    },
  },
  {
    // 存量代码基线：先关掉噪声最大的规则，新代码逐步收紧。
    rules: {
      "@typescript-eslint/no-explicit-any": "off",
      "@typescript-eslint/no-unused-vars": [
        "warn",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
      "no-empty": ["warn", { allowEmptyCatch: true }],
      "svelte/no-at-html-tags": "warn",
    },
  },
  {
    ignores: [
      "node_modules/",
      "build/",
      ".svelte-kit/",
      "src-tauri/",
      "flutter/",
      "server/",
      "static/",
      ".dev-logs/",
    ],
  },
);
