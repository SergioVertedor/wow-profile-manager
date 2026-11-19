import js from "@eslint/js";
import tseslint from "typescript-eslint";
import sonarjs from "eslint-plugin-sonarjs";
import reactHooks from "eslint-plugin-react-hooks";
import globals from "globals";

export default tseslint.config(
  { ignores: ["dist", "src-tauri", "node_modules"] },
  js.configs.recommended,
  ...tseslint.configs.strict,
  sonarjs.configs.recommended,
  {
    files: ["src/**/*.{ts,tsx}"],
    languageOptions: { globals: globals.browser },
    plugins: { "react-hooks": reactHooks },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "no-console": "error",
      "@typescript-eslint/no-explicit-any": "error",
      "sonarjs/cognitive-complexity": ["error", 15],
    },
  },
  {
    // Herramientas de desarrollo que corren con Node (p. ej. `scripts/capturas.mjs`).
    files: ["scripts/**/*.mjs"],
    languageOptions: { globals: globals.node },
    rules: { "no-console": "error", "sonarjs/cognitive-complexity": ["error", 15] },
  },
);
