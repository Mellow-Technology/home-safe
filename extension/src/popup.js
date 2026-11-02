const statusEl = document.getElementById("status");
const statusText = document.getElementById("status-text");
const portInput = document.getElementById("port");

async function check() {
  statusEl.className = "status";
  statusText.textContent = "Checking…";
  const res = await chrome.runtime.sendMessage({ type: "health" });
  statusEl.classList.add(res?.ok ? "ok" : "err");
  statusText.textContent = res?.ok ? `Connected (v${res.data.version})` : "Desktop app not running";
}

chrome.storage.local.get({ port: 47321 }).then(({ port }) => (portInput.value = port));
portInput.addEventListener("change", async () => {
  const port = Number(portInput.value);
  if (port > 0 && port < 65536) {
    await chrome.storage.local.set({ port });
    check();
  }
});
check();
