/**
 * Sample test file demonstrating Vitest setup with Tauri API mocking
 *
 * This file serves as a reference for writing unit tests for the frontend
 * and demonstrates how to mock Tauri APIs.
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { mockIPC } from '@tauri-apps/api/mocks';

describe('Vitest Configuration', () => {
  it('should run a basic test', () => {
    expect(1 + 1).toBe(2);
  });

  it('should have access to DOM APIs', () => {
    const div = document.createElement('div');
    div.textContent = 'Hello, Vitest!';
    expect(div.textContent).toBe('Hello, Vitest!');
  });
});

describe('Tauri API Mocking', () => {
  beforeEach(() => {
    // Clear mocks before each test
    vi.clearAllMocks();
  });

  it('should be able to mock Tauri IPC calls', async () => {
    // Mock a Tauri command response
    mockIPC((cmd, args) => {
      if (cmd === 'test_command') {
        return { success: true, message: 'Mocked response' };
      }
    });

    // Example: In your actual code, you would call a Tauri command like:
    // import { invoke } from '@tauri-apps/api/core';
    // const result = await invoke('test_command');

    // For this test, we just verify the mock was set up
    expect(mockIPC).toBeDefined();
  });

  it('should demonstrate async test support', async () => {
    const promise = Promise.resolve('async value');
    const result = await promise;
    expect(result).toBe('async value');
  });
});

describe('DOM Manipulation', () => {
  it('should support querySelector', () => {
    document.body.innerHTML = '<div id="test">Test Content</div>';
    const element = document.querySelector('#test');
    expect(element?.textContent).toBe('Test Content');
  });

  it('should support addEventListener', () => {
    const button = document.createElement('button');
    let clicked = false;

    button.addEventListener('click', () => {
      clicked = true;
    });

    button.click();
    expect(clicked).toBe(true);
  });
});
