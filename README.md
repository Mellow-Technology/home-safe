# Home Safe

A desktop inventory of homes you're considering. A browser extension adds a
**Save in Home Safe** button to Zillow listings; clicking it sends the listing
to the desktop app, which stores it in a local Turso database, downloads the
photos, and shows everything in a sortable table.

```
Zillow page ──(content script: extract)──▶ extension background
     ──POST http://127.0.0.1:47321/api/homes──▶ Home Safe app
          ├─ Turso DB (homes, images, snapshots, attributes)
          ├─ photo downloader → <data dir>/images/<home id>/
          └─ GPUI table (live-updates on every save)
```

## Layout

| Path | What |
| --- | --- |
| `crates/core` | Domain model, Turso `Store` with migrations, image downloader |
| `crates/server` | axum REST API (+ `home-safe-server` headless binary) |
| `crates/app` | GPUI Kit desktop app; runs the API in-process |
| `extension` | Manifest V3 WebExtension (Chrome and Firefox) |

## Running

```bash
cargo run -p home-safe
```

Data lives in `~/Library/Application Support/com.mellow.HomeSafe` (set
`HOME_SAFE_DATA_DIR` to override). The API listens on `127.0.0.1:47321`
(`HOME_SAFE_PORT` to override; set the same port in the extension popup).

Load the extension:

- **Chrome:** `chrome://extensions` → Developer mode → *Load unpacked* → `extension/`
- **Firefox:** `about:debugging` → This Firefox → *Load Temporary Add-on* → `extension/manifest.json`

Open any Zillow home page. The button sits in the bottom-right corner, and
reads "Saved" for homes you've already saved. Clicking it again refreshes the
saved data.

To remove a saved home, click **Remove** next to the button on its Zillow
page, or select it in the app and click the trash icon in the detail panel.
Either way the home, its history, and its downloaded photos are deleted.

## Icons

The app and extension icons (a house with a heart inside) come from
`scripts/make-icons.py`, which has no dependencies. Run
`python3 scripts/make-icons.py` to regenerate `extension/icons/*.png`,
`crates/app/assets/icon.png` (the app sets it as its Dock icon at runtime) and
`crates/app/assets/AppIcon.icns` (used by `cargo bundle --release -p home-safe`).

## Tests

```bash
cargo test --workspace
```

```bash
node --test extension/test
```

## How extraction works

`extension/src/zillow-extract.js` doesn't rely on Zillow's markup. Instead it
searches the JSON the page embeds for hydration (`__NEXT_DATA__`, including the
stringified `gdpClientCache`) for the richest object whose `zpid` matches the
URL. It falls back to schema.org JSON-LD and then to the DOM. When the live
page holds stale data after in-app navigation, it re-fetches the URL's
server-rendered HTML. Each capture also stores the untouched property JSON in
`home_snapshots.payload`, so fields can be re-derived later without
revisiting Zillow.

## API

Only extension origins (`chrome-extension://`, `moz-extension://`,
`safari-web-extension://`) and origin-less local clients are accepted. Requests
from web pages get 403.

| Method | Path | |
| --- | --- | --- |
| GET | `/api/health` | |
| GET | `/api/homes` | all homes |
| POST | `/api/homes` | save a `HomeCapture` (upsert by `source` + `sourceId`) |
| GET | `/api/homes/lookup?source=zillow&sourceId=…` | `{ saved, home }` |
| GET | `/api/homes/{id}` | home + images + attributes |
| DELETE | `/api/homes/{id}` | |
| PUT | `/api/homes/{id}/attributes` | upsert enrichment attributes |

## Enrichment

The schema is built to take on more data sources without migrations:

- `homes` holds normalized, queryable listing fields. Re-saving only
  overwrites fields the new capture actually has.
- `home_snapshots` keeps every raw payload with its price and status, which
  gives you price history for free.
- `home_attributes` is a `(namespace, key) → JSON` store per home. Give each
  enricher (schools, commute, flood zone, …) its own namespace and write
  through `Store::upsert_attribute` or `PUT /api/homes/{id}/attributes`.
  Promote an attribute to a real column once you want to sort or filter on it.
- `source` + `source_id` keep the data model open to listing sites other
  than Zillow.
