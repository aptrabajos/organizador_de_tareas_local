import js from '@eslint/js';
import typescript from '@typescript-eslint/eslint-plugin';
import typescriptParser from '@typescript-eslint/parser';
import solid from 'eslint-plugin-solid';

export default [
  js.configs.recommended,
  {
    files: ['**/*.{ts,tsx}'],
    plugins: {
      '@typescript-eslint': typescript,
      solid: solid,
    },
    languageOptions: {
      parser: typescriptParser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
        ecmaFeatures: {
          jsx: true,
        },
      },
      globals: {
        document: 'readonly',
        window: 'readonly',
        alert: 'readonly',
        confirm: 'readonly',
        console: 'readonly',
        HTMLInputElement: 'readonly',
        HTMLTextAreaElement: 'readonly',
        Element: 'readonly',
        InputEvent: 'readonly',
        SubmitEvent: 'readonly',
        Event: 'readonly',
        FileReader: 'readonly',
        Image: 'readonly',
        localStorage: 'readonly',
        writeTextFile: 'readonly',
      },
    },
    rules: {
      ...typescript.configs.recommended.rules,
      ...solid.configs.recommended.rules,
      '@typescript-eslint/no-unused-vars': [
        'warn',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
      ],
      '@typescript-eslint/no-explicit-any': 'warn',
    },
  },
  {
    ignores: ['node_modules/**', 'dist/**', 'src-tauri/**', '.vite/**'],
  },
];
