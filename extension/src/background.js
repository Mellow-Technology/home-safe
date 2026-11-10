// Relays requests from content scripts to the Home Safe desktop app.
// Content scripts can't call the local API directly: Zillow's page origin is
// rejected by the app, and https -> http://127.0.0.1 is mixed content.

const DEFAULT_PORT = 47321;

async function apiBase() {
  const { port } = await chrome.storage.local.get({ port: DEFAULT_PORT });
  return `http://127.0.0.1:${port}/api`;
}

async function api(path, init = {}) {
  const res = await fetch(`${await apiBase()}${path}`, {
    ...init,
    headers: { "Content-Type": "application/json", ...(init.headers || {}) },
  });
  const text = await res.text();
  const body = text ? JSON.parse(text) : null;
  if (!res.ok) throw new Error(body?.error || `HTTP ${res.status}`);
  return body;
}

const handlers = {
  health: () => api("/health"),
  lookup: ({ source, sourceId }) =>
    api(`/homes/lookup?source=${encodeURIComponent(source)}&sourceId=${encodeURIComponent(sourceId)}`),
  save: ({ capture }) => api("/homes", { method: "POST", body: JSON.stringify(capture) }),
  remove: ({ id }) => api(`/homes/${encodeURIComponent(id)}`, { method: "DELETE" }),
  progress: ({ id }) => api(`/homes/${encodeURIComponent(id)}/photos/progress`),
};

chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  const handler = handlers[msg?.type];
  if (!handler) return false;
  handler(msg)
    .then((data) => sendResponse({ ok: true, data }))
    .catch((err) => sendResponse({ ok: false, error: String(err?.message || err) }));
  return true; // async response
});
