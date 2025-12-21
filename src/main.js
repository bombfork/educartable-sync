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
    const result = await invoke("authenticate");
    loginMsgEl.textContent = result;
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
