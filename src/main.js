const { invoke } = window.__TAURI__.core;

let greetInputEl;
let greetMsgEl;
let loginMsgEl;
let authStatusEl;
let loginBtn;
let logoutBtn;

async function greet() {
  // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
  greetMsgEl.textContent = await invoke("greet", { name: greetInputEl.value });
}

async function checkAuthStatus() {
  try {
    const isAuthenticated = await invoke("is_authenticated");
    updateUI(isAuthenticated);
  } catch (error) {
    console.error("Error checking auth status:", error);
    updateUI(false);
  }
}

function updateUI(isAuthenticated) {
  if (isAuthenticated) {
    // User is logged in
    loginBtn.style.display = "none";
    logoutBtn.style.display = "inline-block";
    authStatusEl.innerHTML = '<strong style="color: green;">✓ Logged in</strong>';
    loginMsgEl.textContent = "";
  } else {
    // User is not logged in
    loginBtn.style.display = "inline-block";
    logoutBtn.style.display = "none";
    authStatusEl.innerHTML = '<span style="color: gray;">Not logged in</span>';
    loginMsgEl.textContent = "";
  }
}

async function authenticate() {
  try {
    loginMsgEl.textContent = "Opening login window...";
    const tokens = await invoke("authenticate");

    // Display token information in a readable format
    loginMsgEl.innerHTML = `
      <strong>Login Successful!</strong><br>
      <span style="font-size: 0.9em;">
        Access Token: ${tokens.access_token.substring(0, 40)}...<br>
        Refresh Token: ${tokens.refresh_token.substring(0, 40)}...<br>
        ID Token: ${tokens.id_token.substring(0, 40)}...<br>
        Expires At: ${new Date(tokens.expires_at * 1000).toLocaleString()}<br>
        Session State: ${tokens.session_state}
      </span>
    `;

    // Update UI to show logged in state
    updateUI(true);
  } catch (error) {
    loginMsgEl.textContent = `Error: ${error}`;
    updateUI(false);
  }
}

async function logout() {
  try {
    loginMsgEl.textContent = "Logging out...";
    await invoke("logout");
    loginMsgEl.textContent = "Logged out successfully";

    // Update UI to show logged out state
    updateUI(false);

    // Clear message after 2 seconds
    setTimeout(() => {
      loginMsgEl.textContent = "";
    }, 2000);
  } catch (error) {
    loginMsgEl.textContent = `Logout error: ${error}`;
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  greetInputEl = document.querySelector("#greet-input");
  greetMsgEl = document.querySelector("#greet-msg");
  loginMsgEl = document.querySelector("#login-msg");
  authStatusEl = document.querySelector("#auth-status");
  loginBtn = document.querySelector("#login-btn");
  logoutBtn = document.querySelector("#logout-btn");

  // Check authentication status on startup
  await checkAuthStatus();

  document.querySelector("#greet-form").addEventListener("submit", (e) => {
    e.preventDefault();
    greet();
  });

  loginBtn.addEventListener("click", () => {
    authenticate();
  });

  logoutBtn.addEventListener("click", () => {
    logout();
  });
});
