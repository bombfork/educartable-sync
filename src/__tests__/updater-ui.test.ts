/**
 * Updater UI Test Suite
 *
 * This file tests the frontend UI behavior of the updater module.
 * It verifies that the UI correctly displays update status and handles user interactions.
 *
 * Test scenarios:
 * 1. When a new version is available
 * 2. When user clicks update button (download/install state)
 * 3. When no new version is available
 */

import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { mockIPC, clearMocks } from '@tauri-apps/api/mocks';
import { invoke } from '@tauri-apps/api/core';
import { checkForUpdatesSilently, handleUpdateButtonClick } from '../updater.js';

describe('Updater UI - Update Status Display', () => {
  let statusElement: HTMLElement;
  let buttonElement: HTMLButtonElement;

  beforeEach(() => {
    // Clear all mocks before each test
    clearMocks();

    // Mock window.__TAURI__ to make it available to the updater module
    (window as any).__TAURI__ = {
      core: { invoke },
      process: {
        relaunch: vi.fn()
      }
    };

    // Create DOM elements that the updater expects
    statusElement = document.createElement('span');
    statusElement.id = 'update-status';
    statusElement.style.display = 'none';

    buttonElement = document.createElement('button');
    buttonElement.id = 'check-updates-btn';
    buttonElement.textContent = '🔄 Vérifier les mises à jour';

    document.body.appendChild(statusElement);
    document.body.appendChild(buttonElement);
  });

  afterEach(() => {
    // Clean up DOM elements after each test
    document.body.removeChild(statusElement);
    document.body.removeChild(buttonElement);
  });

  describe('New version available', () => {
    it('should display update available status with version', async () => {
      const updateInfo = {
        available: true,
        current_version: '1.0.0',
        latest_version: '1.1.0'
      };

      mockIPC((cmd) => {
        if (cmd === 'check_for_updates') {
          return updateInfo;
        }
      });

      await checkForUpdatesSilently();

      // Verify status text
      expect(statusElement.textContent).toBe('Mise à jour disponible : 1.1.0');
      expect(statusElement.style.display).toBe('inline');
      expect(statusElement.style.color).toBe('#ef4444'); // red color

      // Verify button text
      expect(buttonElement.textContent).toBe('⬇️ Télécharger et installer la nouvelle version');
    });

    it('should show status text when update is available', async () => {
      const updateInfo = {
        available: true,
        current_version: '0.5.0',
        latest_version: '2.0.0'
      };

      mockIPC((cmd) => {
        if (cmd === 'check_for_updates') {
          return updateInfo;
        }
      });

      // Initially hidden
      expect(statusElement.style.display).toBe('none');

      await checkForUpdatesSilently();

      // Should be visible after check
      expect(statusElement.style.display).toBe('inline');
      expect(statusElement.textContent).toContain('2.0.0');
    });

    it('should update button to download action when update is available', async () => {
      const updateInfo = {
        available: true,
        current_version: '1.0.0',
        latest_version: '1.2.5'
      };

      mockIPC((cmd) => {
        if (cmd === 'check_for_updates') {
          return updateInfo;
        }
      });

      const originalButtonText = buttonElement.textContent;

      await checkForUpdatesSilently();

      // Button text should change to download action
      expect(buttonElement.textContent).not.toBe(originalButtonText);
      expect(buttonElement.textContent).toContain('Télécharger');
      expect(buttonElement.textContent).toContain('installer');
    });
  });

  describe('No update available', () => {
    it('should display up-to-date status with current version', async () => {
      const updateInfo = {
        available: false,
        current_version: '1.0.0',
        latest_version: '1.0.0'
      };

      mockIPC((cmd) => {
        if (cmd === 'check_for_updates') {
          return updateInfo;
        }
      });

      await checkForUpdatesSilently();

      // Verify status text
      expect(statusElement.textContent).toBe('✓ Vous utilisez déjà la dernière version (1.0.0)');
      expect(statusElement.style.display).toBe('inline');
      expect(statusElement.style.color).toBe('#10b981'); // green color

      // Verify button text
      expect(buttonElement.textContent).toBe('🔄 Vérifier les mises à jour');
    });

    it('should show green checkmark when up to date', async () => {
      const updateInfo = {
        available: false,
        current_version: '2.5.3',
        latest_version: '2.5.3'
      };

      mockIPC((cmd) => {
        if (cmd === 'check_for_updates') {
          return updateInfo;
        }
      });

      await checkForUpdatesSilently();

      expect(statusElement.textContent).toContain('✓');
      expect(statusElement.textContent).toContain('2.5.3');
      expect(statusElement.style.color).toBe('#10b981');
    });

    it('should propose to check again when no update available', async () => {
      const updateInfo = {
        available: false,
        current_version: '1.5.0',
        latest_version: '1.5.0'
      };

      mockIPC((cmd) => {
        if (cmd === 'check_for_updates') {
          return updateInfo;
        }
      });

      await checkForUpdatesSilently();

      // Button should propose to check for updates
      expect(buttonElement.textContent).toBe('🔄 Vérifier les mises à jour');
    });
  });

  describe('Silent update check error handling', () => {
    it('should not throw error when update check fails silently', async () => {
      // Mock console.warn to avoid cluttering test output
      const consoleWarnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

      mockIPC((cmd) => {
        if (cmd === 'check_for_updates') {
          throw new Error('Network error');
        }
      });

      // Should not throw
      await expect(checkForUpdatesSilently()).resolves.not.toThrow();

      // Should log warning
      expect(consoleWarnSpy).toHaveBeenCalled();

      consoleWarnSpy.mockRestore();
    });

    it('should not update UI when silent check fails', async () => {
      vi.spyOn(console, 'warn').mockImplementation(() => {});

      mockIPC((cmd) => {
        if (cmd === 'check_for_updates') {
          throw new Error('Network error');
        }
      });

      const originalStatusDisplay = statusElement.style.display;
      const originalButtonText = buttonElement.textContent;

      await checkForUpdatesSilently();

      // UI should remain unchanged
      expect(statusElement.style.display).toBe(originalStatusDisplay);
      expect(buttonElement.textContent).toBe(originalButtonText);

      vi.restoreAllMocks();
    });
  });
});

describe('Updater UI - Button Click Behavior', () => {
  let statusElement: HTMLElement;
  let buttonElement: HTMLButtonElement;

  beforeEach(() => {
    clearMocks();

    // Mock window.__TAURI__ to make it available to the updater module
    (window as any).__TAURI__ = {
      core: { invoke },
      process: {
        relaunch: vi.fn()
      }
    };

    statusElement = document.createElement('span');
    statusElement.id = 'update-status';
    statusElement.style.display = 'none';

    buttonElement = document.createElement('button');
    buttonElement.id = 'check-updates-btn';
    buttonElement.textContent = '🔄 Vérifier les mises à jour';

    document.body.appendChild(statusElement);
    document.body.appendChild(buttonElement);
  });

  afterEach(() => {
    document.body.removeChild(statusElement);
    document.body.removeChild(buttonElement);
  });

  describe('Downloading and installing update', () => {
    it('should call download_and_install_update when update is available and button is clicked', async () => {
      const mockFn = vi.fn();

      // First, set up state where update is available
      mockIPC((cmd) => {
        mockFn(cmd);
        if (cmd === 'check_for_updates') {
          return {
            available: true,
            current_version: '1.0.0',
            latest_version: '1.1.0'
          };
        }
        if (cmd === 'download_and_install_update') {
          return null;
        }
      });

      // Check for updates to set the state
      await checkForUpdatesSilently();

      // Reset mock to track only the download call
      mockFn.mockClear();

      // Click the button (which should now trigger download)
      await handleUpdateButtonClick();

      // Verify download command was called
      expect(mockFn).toHaveBeenCalledWith('download_and_install_update');
    });

    it('should check for updates when button is clicked and no update is available', async () => {
      const mockFn = vi.fn();

      mockIPC((cmd) => {
        mockFn(cmd);
        if (cmd === 'check_for_updates') {
          return {
            available: false,
            current_version: '1.0.0',
            latest_version: '1.0.0'
          };
        }
      });

      // Set initial state (no update available)
      await checkForUpdatesSilently();

      // Reset mock to track only the manual check
      mockFn.mockClear();

      // Click the button (should trigger manual check)
      await handleUpdateButtonClick();

      // Verify check command was called again
      expect(mockFn).toHaveBeenCalledWith('check_for_updates');
    });
  });

  describe('Update state transitions', () => {
    it('should transition from "no update" to "update available" when new version is detected', async () => {
      const mockFn = vi.fn();
      let callCount = 0;

      mockIPC((cmd) => {
        mockFn(cmd);
        if (cmd === 'check_for_updates') {
          callCount++;
          // First call: no update
          if (callCount === 1) {
            return {
              available: false,
              current_version: '1.0.0',
              latest_version: '1.0.0'
            };
          }
          // Second call: update available
          return {
            available: true,
            current_version: '1.0.0',
            latest_version: '1.1.0'
          };
        }
      });

      // First check - no update
      await checkForUpdatesSilently();
      expect(buttonElement.textContent).toBe('🔄 Vérifier les mises à jour');
      expect(statusElement.textContent).toContain('✓');

      // Second check via button click - update found
      await handleUpdateButtonClick();

      // Status should be updated to show new version available
      expect(statusElement.textContent).toContain('Mise à jour disponible');
      expect(statusElement.style.color).toBe('#ef4444');

      // Button text should now show download action
      expect(buttonElement.textContent).toBe('⬇️ Télécharger et installer la nouvelle version');
    });

    it('should maintain "update available" state across multiple checks', async () => {
      mockIPC((cmd) => {
        if (cmd === 'check_for_updates') {
          return {
            available: true,
            current_version: '1.0.0',
            latest_version: '1.5.0'
          };
        }
      });

      // First check
      await checkForUpdatesSilently();
      const firstButtonText = buttonElement.textContent;
      const firstStatusText = statusElement.textContent;

      // Second check
      await checkForUpdatesSilently();

      // State should be consistent
      expect(buttonElement.textContent).toBe(firstButtonText);
      expect(statusElement.textContent).toBe(firstStatusText);
      expect(buttonElement.textContent).toContain('Télécharger');
    });
  });

  describe('Version display formatting', () => {
    it('should correctly display various version number formats', async () => {
      const testVersions = [
        '1.0.0',
        '2.5.3-beta',
        '10.20.30',
        '0.0.1-alpha.1'
      ];

      for (const version of testVersions) {
        clearMocks();

        mockIPC((cmd) => {
          if (cmd === 'check_for_updates') {
            return {
              available: true,
              current_version: '1.0.0',
              latest_version: version
            };
          }
        });

        await checkForUpdatesSilently();

        expect(statusElement.textContent).toContain(version);
        expect(statusElement.textContent).toBe(`Mise à jour disponible : ${version}`);
      }
    });
  });
});

describe('Updater UI - Integration Scenarios', () => {
  let statusElement: HTMLElement;
  let buttonElement: HTMLButtonElement;

  beforeEach(() => {
    clearMocks();

    // Mock window.__TAURI__ to make it available to the updater module
    (window as any).__TAURI__ = {
      core: { invoke },
      process: {
        relaunch: vi.fn()
      }
    };

    statusElement = document.createElement('span');
    statusElement.id = 'update-status';
    statusElement.style.display = 'none';

    buttonElement = document.createElement('button');
    buttonElement.id = 'check-updates-btn';
    buttonElement.textContent = '🔄 Vérifier les mises à jour';

    document.body.appendChild(statusElement);
    document.body.appendChild(buttonElement);
  });

  afterEach(() => {
    document.body.removeChild(statusElement);
    document.body.removeChild(buttonElement);
  });

  it('should handle complete update flow from check to download', async () => {
    const mockFn = vi.fn();

    mockIPC((cmd) => {
      mockFn(cmd);
      if (cmd === 'check_for_updates') {
        return {
          available: true,
          current_version: '1.0.0',
          latest_version: '2.0.0'
        };
      }
      if (cmd === 'download_and_install_update') {
        return null;
      }
    });

    // Step 1: Initial check finds update
    await checkForUpdatesSilently();
    expect(statusElement.textContent).toBe('Mise à jour disponible : 2.0.0');
    expect(buttonElement.textContent).toBe('⬇️ Télécharger et installer la nouvelle version');

    // Step 2: User clicks to download
    mockFn.mockClear();
    await handleUpdateButtonClick();
    expect(mockFn).toHaveBeenCalledWith('download_and_install_update');
  });

  it('should handle startup scenario with no update', async () => {
    mockIPC((cmd) => {
      if (cmd === 'check_for_updates') {
        return {
          available: false,
          current_version: '2.3.1',
          latest_version: '2.3.1'
        };
      }
    });

    // Simulate app startup
    await checkForUpdatesSilently();

    // Should show user is up to date
    expect(statusElement.style.display).toBe('inline');
    expect(statusElement.textContent).toContain('✓');
    expect(statusElement.textContent).toContain('2.3.1');
    expect(buttonElement.textContent).toBe('🔄 Vérifier les mises à jour');
  });

  it('should handle startup scenario with update available', async () => {
    mockIPC((cmd) => {
      if (cmd === 'check_for_updates') {
        return {
          available: true,
          current_version: '1.0.0',
          latest_version: '1.5.0'
        };
      }
    });

    // Simulate app startup
    await checkForUpdatesSilently();

    // Should notify user of available update
    expect(statusElement.style.display).toBe('inline');
    expect(statusElement.style.color).toBe('#ef4444');
    expect(statusElement.textContent).toBe('Mise à jour disponible : 1.5.0');
    expect(buttonElement.textContent).toContain('Télécharger');
  });
});
