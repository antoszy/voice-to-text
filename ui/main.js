const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const ring = document.getElementById("status-ring");
const statusText = document.getElementById("status-text");
const modelWarning = document.getElementById("model-warning");
const langSelect = document.getElementById("lang-select");
const modeSelect = document.getElementById("mode-select");

const STATUS_MAP = {
  idle: { class: "idle", text: "Gotowy" },
  recording: { class: "recording", text: "Nagrywanie..." },
  transcribing: { class: "transcribing", text: "Transkrypcja..." },
};

function updateUI(status) {
  const s = STATUS_MAP[status] || STATUS_MAP.idle;
  ring.className = s.class;
  statusText.textContent = s.text;
}

async function saveSettings() {
  const settings = await invoke("get_settings");
  settings.language = langSelect.value;
  settings.mode = modeSelect.value;
  try {
    await invoke("update_settings", { settings });
  } catch (e) {
    console.error("Settings error:", e);
  }
}

async function init() {
  const hasModel = await invoke("check_model");
  if (!hasModel) {
    modelWarning.classList.remove("hidden");
  }

  const settings = await invoke("get_settings");
  langSelect.value = settings.language;
  modeSelect.value = settings.mode;

  const status = await invoke("get_status");
  updateUI(status);

  await listen("status-changed", (event) => updateUI(event.payload));
  await listen("error", (event) => {
    statusText.textContent = event.payload;
    setTimeout(() => updateUI("idle"), 3000);
  });

  ring.addEventListener("click", async () => {
    try { await invoke("toggle_recording"); } catch (e) { console.error(e); }
  });

  langSelect.addEventListener("change", saveSettings);
  modeSelect.addEventListener("change", saveSettings);
}

init();
