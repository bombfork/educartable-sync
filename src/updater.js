/**
 * Educartable Sync - Updater Module
 *
 * Handles automatic update checking and installation with French UI
 */

// Import notification system
import { showError, showSuccess, showLoading, showInfo, handleError } from './notifications.js';

// Track current update state
let currentUpdateInfo = null;

/**
 * Check for updates silently (used on startup)
 * Updates the UI to show current update status
 */
export async function checkForUpdatesSilently() {
    try {
        console.log('Checking for updates silently...');
        const { invoke } = window.__TAURI__.core;
        const updateInfo = await invoke('check_for_updates');

        if (updateInfo.available) {
            console.log('Update available:', updateInfo.latest_version);
            setUpdateAvailableUI(updateInfo);
        } else {
            console.log('No updates available');
            setUpToDateUI(updateInfo.current_version);
        }
    } catch (error) {
        // Silent failure - don't bother user with update check errors on startup
        console.warn('Silent update check failed:', error);
    }
}

/**
 * Handle update button click - either checks for updates or downloads available update
 * This is the main handler connected to the button
 */
export async function handleUpdateButtonClick() {
    if (currentUpdateInfo && currentUpdateInfo.available) {
        // Update is available, download it
        await downloadAndInstallUpdate();
    } else {
        // No update available, check for updates
        await checkForUpdates();
    }
}

/**
 * Check for updates (called when button is clicked in check mode)
 */
async function checkForUpdates() {
    const checkBtn = document.getElementById('check-updates-btn');

    // Set button to loading state
    checkBtn.disabled = true;
    checkBtn.setAttribute('aria-busy', 'true');
    const originalText = checkBtn.textContent;
    checkBtn.textContent = 'Vérification...';

    const loadingNotification = showLoading('Vérification des mises à jour...');

    try {
        console.log('Checking for updates manually...');
        const { invoke } = window.__TAURI__.core;
        const updateInfo = await invoke('check_for_updates');

        // Dismiss loading notification
        loadingNotification.dismiss();

        if (updateInfo.available) {
            console.log('Update available:', updateInfo.latest_version);
            setUpdateAvailableUI(updateInfo);
        } else {
            console.log('No updates available');
            setUpToDateUI(updateInfo.current_version);
        }
    } catch (error) {
        console.error('Update check failed:', error);

        // Dismiss loading notification
        loadingNotification.dismiss();

        // Show error to user
        handleError(error, 'update check');
    } finally {
        // Reset button state
        checkBtn.disabled = false;
        checkBtn.removeAttribute('aria-busy');
        checkBtn.textContent = originalText;
    }
}

/**
 * Show notification when update is available
 * Creates a custom notification with action buttons
 */
function showUpdateAvailableNotification(updateInfo) {
    const notification = createUpdateNotification(updateInfo);
    showCustomNotification(notification);
}

/**
 * Create update notification element with action buttons
 */
function createUpdateNotification(updateInfo) {
    const notification = document.createElement('div');
    notification.className = 'notification notification-info update-notification';
    notification.setAttribute('role', 'alert');

    notification.innerHTML = `
        <div class="notification-icon">🔄</div>
        <div class="notification-content">
            <div class="notification-title">Mise à jour disponible</div>
            <div class="notification-message">Une nouvelle version (${escapeHtml(updateInfo.latest_version)}) est disponible</div>
            <div class="update-actions">
                <button class="update-download-btn" style="margin: 0.5rem 0.5rem 0 0; padding: 0.25rem 0.75rem; font-size: 0.9rem;">
                    Télécharger
                </button>
                <button class="update-later-btn secondary" style="margin: 0.5rem 0 0 0; padding: 0.25rem 0.75rem; font-size: 0.9rem;">
                    Plus tard
                </button>
            </div>
        </div>
        <button class="notification-close" aria-label="Close">&times;</button>
    `;

    // Add download button handler
    const downloadBtn = notification.querySelector('.update-download-btn');
    downloadBtn.addEventListener('click', () => {
        dismissCustomNotification(notification);
        downloadAndInstallUpdate();
    });

    // Add "later" button handler
    const laterBtn = notification.querySelector('.update-later-btn');
    laterBtn.addEventListener('click', () => {
        dismissCustomNotification(notification);
    });

    // Add close button handler
    const closeBtn = notification.querySelector('.notification-close');
    closeBtn.addEventListener('click', () => {
        dismissCustomNotification(notification);
    });

    return notification;
}

/**
 * Download and install the update
 */
async function downloadAndInstallUpdate() {
    const loadingNotification = showLoading('Téléchargement de la mise à jour... L\'application redémarrera automatiquement.');

    try {
        console.log('Downloading and installing update...');
        const { invoke } = window.__TAURI__.core;
        await invoke('download_and_install_update');

        // Dismiss loading notification
        loadingNotification.dismiss();

        console.log('Update downloaded and ready to install');

        // Show restart prompt
        showRestartPromptNotification();
    } catch (error) {
        console.error('Update download/install failed:', error);

        // Dismiss loading notification
        loadingNotification.dismiss();

        // Show error to user
        handleError(error, 'update download');
    }
}

/**
 * Show notification prompting user to restart
 */
function showRestartPromptNotification() {
    const notification = document.createElement('div');
    notification.className = 'notification notification-success update-notification';
    notification.setAttribute('role', 'alert');

    notification.innerHTML = `
        <div class="notification-icon">✓</div>
        <div class="notification-content">
            <div class="notification-title">Mise à jour prête</div>
            <div class="notification-message">La mise à jour a été téléchargée avec succès</div>
            <div class="update-actions">
                <button class="restart-now-btn" style="margin: 0.5rem 0.5rem 0 0; padding: 0.25rem 0.75rem; font-size: 0.9rem;">
                    Redémarrer maintenant
                </button>
                <button class="restart-later-btn secondary" style="margin: 0.5rem 0 0 0; padding: 0.25rem 0.75rem; font-size: 0.9rem;">
                    Plus tard
                </button>
            </div>
        </div>
        <button class="notification-close" aria-label="Close">&times;</button>
    `;

    // Add restart now button handler
    const restartBtn = notification.querySelector('.restart-now-btn');
    restartBtn.addEventListener('click', async () => {
        console.log('Restarting application to apply update...');
        try {
            const { relaunch } = window.__TAURI__.process;
            await relaunch();
        } catch (error) {
            console.error('Failed to restart:', error);
            handleError(error, 'restart');
        }
    });

    // Add "later" button handler
    const laterBtn = notification.querySelector('.restart-later-btn');
    laterBtn.addEventListener('click', () => {
        dismissCustomNotification(notification);
        showInfo('Redémarrage différé', 'La mise à jour sera installée au prochain démarrage');
    });

    // Add close button handler
    const closeBtn = notification.querySelector('.notification-close');
    closeBtn.addEventListener('click', () => {
        dismissCustomNotification(notification);
        showInfo('Redémarrage différé', 'La mise à jour sera installée au prochain démarrage');
    });

    showCustomNotification(notification);
}

/**
 * Show a custom notification element
 */
function showCustomNotification(notification) {
    const container = getOrCreateContainer();
    container.appendChild(notification);

    // Trigger animation
    setTimeout(() => {
        notification.classList.add('notification-show');
    }, 10);
}

/**
 * Dismiss a custom notification element
 */
function dismissCustomNotification(notification) {
    notification.classList.remove('notification-show');
    notification.classList.add('notification-hide');

    // Remove from DOM after animation
    setTimeout(() => {
        if (notification.parentNode) {
            notification.parentNode.removeChild(notification);
        }
    }, 300);
}

/**
 * Get or create the notification container
 */
function getOrCreateContainer() {
    let container = document.getElementById('notification-container');

    if (!container) {
        container = document.createElement('div');
        container.id = 'notification-container';
        container.className = 'notification-container';
        document.body.appendChild(container);
    }

    return container;
}

/**
 * Escape HTML to prevent XSS
 */
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

/**
 * Set UI to show "up to date" state
 */
function setUpToDateUI(currentVersion) {
    // Store state
    currentUpdateInfo = {
        available: false,
        current_version: currentVersion
    };

    const statusElement = document.getElementById('update-status');
    const checkBtn = document.getElementById('check-updates-btn');

    if (statusElement) {
        statusElement.textContent = `✓ Vous utilisez déjà la dernière version (${currentVersion})`;
        statusElement.style.display = 'inline';
        statusElement.style.color = '#10b981';
    }

    if (checkBtn) {
        checkBtn.textContent = '🔄 Vérifier les mises à jour';
    }
}

/**
 * Set UI to show "update available" state
 */
function setUpdateAvailableUI(updateInfo) {
    // Store state
    currentUpdateInfo = updateInfo;

    const statusElement = document.getElementById('update-status');
    const checkBtn = document.getElementById('check-updates-btn');

    if (statusElement) {
        statusElement.textContent = `Mise à jour disponible : ${updateInfo.latest_version}`;
        statusElement.style.display = 'inline';
        statusElement.style.color = '#ef4444';
    }

    if (checkBtn) {
        checkBtn.textContent = '⬇️ Télécharger la mise à jour';
    }
}
