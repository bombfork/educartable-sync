import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    // Enable browser-like environment for DOM APIs
    environment: 'happy-dom',

    // Global test setup
    globals: true,

    // Setup files to run before tests
    setupFiles: ['./src/__tests__/setup.ts'],

    // Coverage configuration
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      exclude: [
        'node_modules/',
        'src/__tests__/',
        '**/*.test.ts',
        '**/*.spec.ts',
      ],
    },

    // Test file patterns
    include: ['src/**/*.{test,spec}.{js,ts}'],

    // Exclude patterns
    exclude: [
      'node_modules',
      'dist',
      'src-tauri',
      'e2e-tests',
      '.git',
    ],
  },
});
