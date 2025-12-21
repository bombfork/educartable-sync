const { invoke } = window.__TAURI__.core;

let greetInputEl;
let greetMsgEl;
let loginMsgEl;

async function greet() {
  // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
  greetMsgEl.textContent = await invoke("greet", { name: greetInputEl.value });
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
  } catch (error) {
    loginMsgEl.textContent = `Error: ${error}`;
  }
}

window.addEventListener("DOMContentLoaded", () => {
  greetInputEl = document.querySelector("#greet-input");
  greetMsgEl = document.querySelector("#greet-msg");
  loginMsgEl = document.querySelector("#login-msg");

  document.querySelector("#greet-form").addEventListener("submit", (e) => {
    e.preventDefault();
    greet();
  });

  document.querySelector("#login-btn").addEventListener("click", () => {
    authenticate();
  });
});
