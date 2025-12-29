/**
 * Vitest setup file for Tauri frontend unit tests
 *
 * This file is executed before running tests and sets up:
 * - Tauri API mocks
 * - Global test utilities
 * - DOM environment configuration
 */

import { beforeAll, afterAll, beforeEach, afterEach, vi } from 'vitest';

// Mock the Tauri API
// Note: Individual test files should import and configure specific mocks
// from '@tauri-apps/api/mocks' as needed

// Global test setup
beforeAll(() => {
  // Setup code that runs once before all tests
});

afterAll(() => {
  // Cleanup code that runs once after all tests
});

beforeEach(() => {
  // Setup code that runs before each test
  // Clear all mocks before each test
  vi.clearAllMocks();
});

afterEach(() => {
  // Cleanup code that runs after each test
});

// Mock window.matchMedia if needed for tests
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation(query => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});
