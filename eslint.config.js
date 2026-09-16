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
        setTimeout: 'readonly',
        clearTimeout: 'readonly',
        process: 'readonly',
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
      // En Tauri v2 un comando que devuelve `Err(String)` rechaza la promesa con
      // un STRING plano, no con un `Error`. Chequear `err instanceof Error` tira
      // a la basura el mensaje del backend y termina mostrando "Error desconocido".
      'no-restricted-syntax': [
        'error',
        {
          selector:
            'BinaryExpression[operator="instanceof"][right.name="Error"]',
          message:
            'No uses `err instanceof Error`: en Tauri v2 los errores del backend llegan como string plano y se pierde el mensaje real. Usá getErrorMessage(err) de src/utils/errors.ts.',
        },
      ],
    },
  },
  {
    ignores: [
      'node_modules/**',
      'dist/**',
      'src-tauri/**',
      '.vite/**',
      'tailwind.config.js',
      // El snapshot histórico vive en docs/historico/win10/**, así que el patrón
      // 'win10/**' (relativo al cwd) nunca lo agarraba y ESLint terminaba
      // linteando código archivado que nadie va a arreglar.
      'win10/**',
      'docs/**',
    ],
  },
];
