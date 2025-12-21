/**
 * Educartable Sync - Notification System
 *
 * Provides user-friendly error messages, success notifications, and loading states.
 * Implements toast notifications for better user experience.
 */

/**
 * Show an error notification with user-friendly message and actionable advice
 * @param {string} title - Short error title
 * @param {string} message - Detailed error message
 * @param {string} [action] - Optional actionable advice for the user
 */
export function showError(title, message, action = '') {
    console.error(`Error: ${title} - ${message}`);

    const notification = createNotification('error', title, message, action);
    showNotification(notification);
}

/**
 * Show a success notification
 * @param {string} message - Success message to display
 */
export function showSuccess(message) {
    console.log(`Success: ${message}`);

    const notification = createNotification('success', 'Success', message);
    showNotification(notification);
}

/**
 * Show a loading notification (returns handle for dismissal)
 * @param {string} message - Loading message to display
 * @returns {object} Handle with dismiss() method
 */
export function showLoading(message) {
    console.log(`Loading: ${message}`);

    const notification = createNotification('loading', 'Please wait', message);
    showNotification(notification);

    // Return handle for manual dismissal
    return {
        dismiss: () => dismissNotification(notification)
    };
}

/**
 * Show an info notification
 * @param {string} title - Info title
 * @param {string} message - Info message
 */
export function showInfo(title, message) {
    console.log(`Info: ${title} - ${message}`);

    const notification = createNotification('info', title, message);
    showNotification(notification);
}

/**
 * Create a notification element
 * @private
 */
function createNotification(type, title, message, action = '') {
    const notification = document.createElement('div');
    notification.className = `notification notification-${type}`;
    notification.setAttribute('role', 'alert');

    // Choose icon based on type
    let icon;
    switch (type) {
        case 'error':
            icon = '❌';
            break;
        case 'success':
            icon = '✓';
            break;
        case 'loading':
            icon = '⏳';
            break;
        case 'info':
            icon = 'ℹ️';
            break;
        default:
            icon = '📢';
    }

    notification.innerHTML = `
        <div class="notification-icon">${icon}</div>
        <div class="notification-content">
            <div class="notification-title">${escapeHtml(title)}</div>
            <div class="notification-message">${escapeHtml(message)}</div>
            ${action ? `<div class="notification-action">${escapeHtml(action)}</div>` : ''}
        </div>
        <button class="notification-close" aria-label="Close">&times;</button>
    `;

    // Add close button handler (if present)
    const closeBtn = notification.querySelector('.notification-close');
    if (closeBtn) {
        closeBtn.addEventListener('click', () => {
            dismissNotification(notification);
        });
    }

    return notification;
}

/**
 * Show a notification element
 * @private
 */
function showNotification(notification) {
    const container = getOrCreateContainer();
    container.appendChild(notification);

    // Trigger animation
    setTimeout(() => {
        notification.classList.add('notification-show');
    }, 10);
}

/**
 * Dismiss a notification element
 * @private
 */
function dismissNotification(notification) {
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
 * @private
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
 * @private
 */
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

/**
 * Parse backend error messages and make them user-friendly
 * @param {string|Error} error - Error from backend
 * @returns {object} Object with title, message, and action
 */
export function parseError(error) {
    const errorStr = typeof error === 'string' ? error : error.toString();
    const errorLower = errorStr.toLowerCase();

    // Authentication errors
    if (errorLower.includes('not authenticated') || errorLower.includes('failed to load tokens')) {
        return {
            title: 'Not Logged In',
            message: 'You need to log in to Educartable first.',
            action: 'Click the "Login to Educartable" button to continue.'
        };
    }

    if (errorLower.includes('login timeout') || errorLower.includes('no response within')) {
        return {
            title: 'Login Timeout',
            message: 'Login took too long.',
            action: 'Please try logging in again.'
        };
    }

    if (errorLower.includes('tokens not found') || errorLower.includes('invalid token')) {
        return {
            title: 'Login Failed',
            message: 'Could not complete login.',
            action: 'Please check your credentials and try again.'
        };
    }

    if (errorLower.includes('token refresh failed')) {
        return {
            title: 'Session Expired',
            message: 'Your login session has expired.',
            action: 'Please log in again to continue.'
        };
    }

    // Network errors
    if (errorLower.includes('network') || errorLower.includes('connection') ||
        errorLower.includes('timed out') || errorLower.includes('timeout')) {
        return {
            title: 'Connection Error',
            message: 'Cannot connect to Educartable.',
            action: 'Please check your internet connection and try again.'
        };
    }

    if (errorLower.includes('rate limit')) {
        return {
            title: 'Too Many Requests',
            message: 'You are sending requests too quickly.',
            action: 'Please wait a moment and try again.'
        };
    }

    // Configuration errors
    if (errorLower.includes('sync directory not configured') ||
        errorLower.includes('no folder selected')) {
        return {
            title: 'No Folder Selected',
            message: 'You need to choose where to save your photos.',
            action: 'Click the "Browse" button to select a folder.'
        };
    }

    if (errorLower.includes('permission denied') || errorLower.includes('access denied')) {
        return {
            title: 'Permission Denied',
            message: 'Cannot write to the selected folder.',
            action: 'Choose a different folder that you have permission to write to.'
        };
    }

    if (errorLower.includes('no space') || errorLower.includes('disk full') ||
        errorLower.includes('insufficient space')) {
        return {
            title: 'Not Enough Space',
            message: 'Your disk is full or running low on space.',
            action: 'Free up some space and try again.'
        };
    }

    // Sync errors
    if (errorLower.includes('failed to get user info')) {
        return {
            title: 'Cannot Access Account',
            message: 'Could not retrieve your account information.',
            action: 'Please try logging in again.'
        };
    }

    if (errorLower.includes('failed to fetch activities')) {
        return {
            title: 'Cannot Load Activities',
            message: 'Could not retrieve your activities from Educartable.',
            action: 'Please check your connection and try again.'
        };
    }

    if (errorLower.includes('server error') || errorLower.includes('500') ||
        errorLower.includes('503') || errorLower.includes('502')) {
        return {
            title: 'Server Error',
            message: 'Educartable servers are having issues.',
            action: 'Please try again later.'
        };
    }

    // Generic errors
    if (errorLower.includes('failed to') || errorLower.includes('error:')) {
        // Try to extract a more specific message
        const colonIndex = errorStr.indexOf(':');
        const specificMsg = colonIndex > 0 ? errorStr.substring(colonIndex + 1).trim() : errorStr;

        return {
            title: 'Operation Failed',
            message: specificMsg,
            action: 'Please try again or contact support if the problem persists.'
        };
    }

    // Unknown error
    return {
        title: 'Unexpected Error',
        message: errorStr,
        action: 'Please try again. If the problem persists, contact support.'
    };
}

/**
 * Handle errors in a user-friendly way
 * @param {string|Error} error - Error from backend
 * @param {string} [context] - Optional context (e.g., "login", "sync")
 */
export function handleError(error, context = '') {
    const parsed = parseError(error);

    // Log full error for debugging
    console.error(`Error in ${context}:`, error);

    // Show user-friendly message
    showError(parsed.title, parsed.message, parsed.action);
}
