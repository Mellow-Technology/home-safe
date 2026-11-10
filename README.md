# Home Safe

A desktop inventory of homes you're considering. A browser extension adds a
**Save in Home Safe** button to Zillow listings; clicking it sends the listing
to the desktop app, which stores it in a local Turso database, downloads the
photos, and shows everything as photo cards or a sortable table.

```
Zillow page ──(content script: extract)──▶ extension background
     ──POST http://127.0.0.1:47321/api/homes──▶ Home Safe app
          ├─ Turso DB (homes, images, snapshots, attributes)
          ├─ photo downloader → <data dir>/images/<home id>/
          └─ GPUI cards / table (live-updates on every save)
```

## Layout

| Path | What |
| --- | --- |
| `crates/core` | Domain model, Turso `Store` with migrations, image downloader |
| `crates/server` | axum REST API (+ `home-safe-server` headless binary) |
| `crates/app` | GPUI Kit desktop app; runs the API in-process |
| `extension` | Manifest V3 WebExtension (Chrome and Firefox) |
| `site` | The website, published to GitHub Pages |
| `scripts` | Icon generator and the macOS app bundler |

## Running

```bash
cargo run -p home-safe
```

On macOS, Home Safe also puts a house icon in the menu bar. Closing the window
(or ⌘W) leaves the app running there, without a Dock icon, so the extension can
keep saving homes. Reopen the window from the menu bar icon (or, for the bundled
app, by opening Home Safe again); quit from the icon's menu, or with ⌘Q while
the window is open.

Turn on **Start at Login** (in the menu bar icon's menu or the Home Safe menu)
to have it start in the menu bar, without opening the window, whenever you log
in. It adds a LaunchAgent at `~/Library/LaunchAgents/com.mellow.HomeSafe.plist`
that runs the copy of the app you turned it on from, so turn it on from the
bundled app rather than a `cargo run` build.

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
page, or open it in the app and click the trash icon on its page.
Either way the home, its history, and its downloaded photos are deleted.

## Using the app

Homes show as cards (the default) or a sortable table; switch with the buttons
at the right of the toolbar. The status bar shows how many homes match the
search and the API address the extension talks to. Click a card, or double-click a table row, to open the
home's page; **All homes** or Esc goes back. Click any photo to browse them
full size: ← and → step through them (wrapping around at the ends), and Esc
or a click outside the photo closes the viewer.

## Icons

The app and extension icons (a house with a heart inside) come from
`scripts/make-icons.py`, which has no dependencies. Run
`python3 scripts/make-icons.py` to regenerate `extension/icons/*.png`,
`crates/app/assets/icon.png` (the app sets it as its Dock icon at runtime),
`crates/app/assets/logo.png` (the toolbar logo),
`crates/app/assets/tray.png` (the menu bar icon, a template image macOS tints) and
`crates/app/assets/AppIcon.icns` (the bundled app's icon).

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
| GET | `/api/homes/{id}/photos/progress` | photo download `{ total, downloaded, failed, done }` |
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

## Releases

`scripts/bundle-macos.sh` builds `Home Safe.app` as a universal binary (Apple
silicon and Intel; needs `rustup target add aarch64-apple-darwin
x86_64-apple-darwin`) and writes a `.dmg`, a zipped app and a zipped
extension to `target/release-macos/`. `ARCHS=aarch64 scripts/bundle-macos.sh`
builds for just Apple silicon, which is quicker.

To release, bump the version in `Cargo.toml` and `extension/manifest.json`,
then push a matching tag:

```bash
git tag v0.2.0 && git push origin v0.2.0
```

The Release workflow checks the versions match, runs the tests, builds the
bundle and publishes a GitHub release with it. Running the workflow by hand
from the Actions tab builds the files (as a workflow artifact) without
releasing.

The app is ad-hoc signed, so the first time it's opened macOS says it can't
check it for malware: open it from **System Settings → Privacy & Security →
Open Anyway**. To sign and notarize instead, add these repository secrets:
`MACOS_CERTIFICATE` (a base64-encoded Developer ID Application `.p12`),
`MACOS_CERTIFICATE_PASSWORD`, and for notarizing `APPLE_ID`, `APPLE_TEAM_ID`
and `APPLE_APP_PASSWORD` (an app-specific password).

## Website

`site/` is the project's website: a Next.js static export built with
[gui-mellow](https://github.com/Mellow-Technology/gui-mellow) and its own warm
theme (`site/app/theme.scss`). `.github/workflows/pages.yml` publishes it to
GitHub Pages on every push to `main` that changes it; in the repository's
settings, under Pages, set the source to "GitHub Actions".

```bash
cd site
bun install
bun dev        # http://localhost:3002
bun run build  # static site in site/out
```

## License

Home Safe is licensed under the GNU Lesser General Public License, version 3
only (`LGPL-3.0-only`). See [COPYING.LESSER](COPYING.LESSER) for the LGPL and
[COPYING](COPYING) for the GNU GPL it builds on.
