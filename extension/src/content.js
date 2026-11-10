// Injects the "Save in Home Safe" control on Zillow home detail pages, with
// a "Remove" button alongside it once the home is saved.
//
// Zillow is a single-page app: opening a listing from search pushes a
// /homedetails/ URL without a full load, so we watch for URL changes and
// re-fetch the page HTML when the embedded data belongs to a different home.

(function () {
  const { extract, zpidFromUrl } = globalThis.HomeSafeZillow;

  let host = null; // shadow host for the control
  let button = null;
  let removeButton = null;
  let currentZpid = null;
  let savedId = null; // Home Safe id of the current home, if saved

  function send(msg) {
    return chrome.runtime.sendMessage(msg).then((res) => {
      if (!res?.ok) throw new Error(res?.error || "No response from extension");
      return res.data;
    });
  }

  function mount() {
    if (host) return;
    host = document.createElement("div");
    host.id = "home-safe-control";
    const root = host.attachShadow({ mode: "closed" });
    root.innerHTML = `
      <style>
        .bar {
          position: fixed; right: 24px; bottom: 24px; z-index: 2147483647;
          display: flex; align-items: center; gap: 8px;
        }
        button {
          padding: 10px 16px; border: 0; border-radius: 999px;
          font: 600 14px -apple-system, system-ui, sans-serif;
          color: #fff; background: #0f766e; cursor: pointer;
          box-shadow: 0 4px 14px rgba(0,0,0,.25);
        }
        button:hover { background: #115e59; }
        button[data-state="saving"] { opacity: .8; cursor: progress; }
        button[data-state="saved"] { background: #166534; }
        button[data-state="error"] { background: #b91c1c; }
        .remove { color: #b91c1c; background: #fff; }
        .remove:hover { background: #fef2f2; }
        .remove[hidden] { display: none; }
      </style>
      <div class="bar">
        <button type="button" class="remove" hidden title="Remove this home from Home Safe">Remove</button>
        <button type="button" class="save" data-state="idle" title="Save this home to the Home Safe desktop app"></button>
      </div>`;
    button = root.querySelector(".save");
    removeButton = root.querySelector(".remove");
    button.addEventListener("click", save);
    removeButton.addEventListener("click", remove);
    document.documentElement.appendChild(host);
  }

  function unmount() {
    host?.remove();
    host = button = removeButton = null;
  }

  function setSavedId(id) {
    savedId = id;
    if (removeButton) removeButton.hidden = id == null;
  }

  function setState(state, label, title) {
    if (!button) return;
    button.dataset.state = state;
    button.textContent = label;
    if (title) button.title = title;
  }

  async function capture() {
    const url = location.href;
    const fromDom = extract(document, url);
    if (fromDom?.price != null || fromDom?.images?.length) return fromDom;
    // The live DOM is stale or incomplete after SPA navigation; the server-
    // rendered HTML for this URL always embeds the right home.
    const res = await fetch(url, { credentials: "include" });
    const html = await res.text();
    const doc = new DOMParser().parseFromString(html, "text/html");
    return extract(doc, url) || fromDom;
  }

  async function save() {
    if (button?.dataset.state === "saving") return;
    setState("saving", "Saving…");
    if (removeButton) removeButton.hidden = true;
    try {
      const data = await capture();
      if (!data) throw new Error("Couldn't find home data on this page");
      const result = await send({ type: "save", capture: data });
      setSavedId(result.id);
      const photos = data.images.length ? ` · ${data.images.length} photos` : "";
      setState("saved", `✓ Saved in Home Safe${photos}`, `Saved as #${result.id}. Click to update.`);
    } catch (err) {
      const msg = String(err.message || err);
      const offline = /fetch|network|Failed/i.test(msg);
      setState("error", offline ? "Home Safe app isn't running" : "Save failed — retry", msg);
      setSavedId(savedId);
    }
  }

  async function remove() {
    if (savedId == null || button?.dataset.state === "saving") return;
    if (!confirm("Remove this home and its photos from Home Safe?")) return;
    const zpid = currentZpid;
    removeButton.disabled = true;
    try {
      await send({ type: "remove", id: savedId });
      if (zpid !== currentZpid) return;
      setSavedId(null);
      setState("idle", "Save in Home Safe", "Save this home to the Home Safe desktop app");
    } catch (err) {
      setState("error", "Remove failed — check the app", String(err.message || err));
    } finally {
      if (removeButton) removeButton.disabled = false;
    }
  }

  async function refreshSavedState(zpid) {
    try {
      const { saved, home } = await send({ type: "lookup", source: "zillow", sourceId: zpid });
      if (zpid !== currentZpid) return;
      setSavedId(saved ? home.id : null);
      if (saved) setState("saved", "✓ Saved · Update in Home Safe", "Already saved. Click to refresh its data.");
    } catch {
      // App offline: leave the default label; saving will report the error.
    }
  }

  function onLocationChange() {
    const zpid = location.pathname.includes("/homedetails/") ? zpidFromUrl(location.href) : null;
    if (zpid === currentZpid) return;
    currentZpid = zpid;
    if (!zpid) return unmount();
    mount();
    setSavedId(null);
    setState("idle", "Save in Home Safe");
    refreshSavedState(zpid);
  }

  // pushState isn't observable from an isolated world, so poll cheaply.
  let lastHref = "";
  setInterval(() => {
    if (location.href !== lastHref) {
      lastHref = location.href;
      onLocationChange();
    }
  }, 500);
  window.addEventListener("popstate", onLocationChange);
})();
