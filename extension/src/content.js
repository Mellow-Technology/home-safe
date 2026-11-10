// Injects the "Save in Home Safe" control on Zillow home detail pages, with
// a "Remove" button alongside it once the home is saved.
//
// After a save the app downloads the listing's photos in the background; the
// button shows a spinner and "Downloading photos 3 of 24" until it finishes.
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
  let progressRun = 0; // bumped to stop an earlier progress poll

  const POLL_MS = 600;

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
        button[data-state="saving"], button[data-state="downloading"] { cursor: progress; }
        button[data-state="saving"] { opacity: .8; }
        .spinner {
          display: inline-block; width: 12px; height: 12px; margin-right: 8px;
          vertical-align: -1px; border: 2px solid rgba(255,255,255,.35);
          border-top-color: #fff; border-radius: 50%;
          animation: spin .8s linear infinite;
        }
        @keyframes spin { to { transform: rotate(360deg); } }
        @media (prefers-reduced-motion: reduce) { .spinner { animation-duration: 2.4s; } }
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

  function setState(state, label, title, { spinner = false } = {}) {
    if (!button) return;
    button.dataset.state = state;
    button.textContent = label;
    if (spinner) {
      const s = document.createElement("span");
      s.className = "spinner";
      s.setAttribute("aria-hidden", "true");
      button.prepend(s);
    }
    if (title) button.title = title;
  }

  const busy = () => ["saving", "downloading"].includes(button?.dataset.state);

  // Shows photo download progress until the app reports it finished.
  async function followDownload(id, zpid) {
    const run = ++progressRun;
    const stale = () => run !== progressRun || zpid !== currentZpid;
    if (removeButton) removeButton.hidden = true;
    for (;;) {
      let p;
      try {
        p = await send({ type: "progress", id });
      } catch (err) {
        if (stale()) return;
        setState("error", "Home Safe app isn't running", String(err.message || err));
        setSavedId(savedId);
        return;
      }
      if (stale()) return;
      if (p.done) return showSaved(p);
      const count = p.total ? ` ${Math.min(p.downloaded + p.failed + 1, p.total)} of ${p.total}` : "…";
      setState("downloading", `Downloading photos${count}`,
        `${p.downloaded} of ${p.total} photos saved so far.`, { spinner: true });
      await new Promise((r) => setTimeout(r, POLL_MS));
      if (stale()) return;
    }
  }

  function showSaved({ total, downloaded, failed }) {
    setSavedId(savedId);
    if (!total) {
      return setState("saved", "✓ Saved in Home Safe", "Saved. Click to update.");
    }
    if (downloaded < total) {
      const why = failed ? `${failed} couldn't be downloaded.` : "Some aren't downloaded yet.";
      return setState("saved", `✓ Saved · ${downloaded} of ${total} photos`,
        `${why} Click to update and retry.`);
    }
    setState("saved", `✓ Saved in Home Safe · ${total} photos`, "Saved with all photos. Click to update.");
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
    if (busy()) return;
    const zpid = currentZpid;
    progressRun++; // stop any earlier poll
    setState("saving", "Saving…", undefined, { spinner: true });
    if (removeButton) removeButton.hidden = true;
    try {
      const data = await capture();
      if (!data) throw new Error("Couldn't find home data on this page");
      const result = await send({ type: "save", capture: data });
      if (zpid !== currentZpid) return;
      savedId = result.id;
      await followDownload(result.id, zpid);
    } catch (err) {
      if (zpid !== currentZpid) return;
      const msg = String(err.message || err);
      const offline = /fetch|network|Failed/i.test(msg);
      setState("error", offline ? "Home Safe app isn't running" : "Save failed — retry", msg);
      setSavedId(savedId);
    }
  }

  async function remove() {
    if (savedId == null || busy()) return;
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
      if (!saved) return;
      setState("saved", "✓ Saved · Update in Home Safe", "Already saved. Click to refresh its data.");
      // Pick up a download that's still running (e.g. after a page reload).
      const p = await send({ type: "progress", id: home.id });
      if (zpid === currentZpid && !p.done) followDownload(home.id, zpid);
    } catch {
      // App offline: leave the default label; saving will report the error.
    }
  }

  function onLocationChange() {
    const zpid = location.pathname.includes("/homedetails/") ? zpidFromUrl(location.href) : null;
    if (zpid === currentZpid) return;
    currentZpid = zpid;
    progressRun++; // stop following the previous home's download
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
