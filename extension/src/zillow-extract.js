// Extracts a HomeCapture from a Zillow home details page.
//
// Zillow's markup changes often, so this leans on the JSON the page embeds
// for its own hydration rather than on DOM structure:
//   1. Any embedded JSON (Next.js __NEXT_DATA__, including the stringified
//      gdpClientCache inside it) searched for the property object whose zpid
//      matches the URL.
//   2. schema.org JSON-LD for address/geo when (1) comes up short.
//   3. Visible DOM text as a last resort for price/address.
// Exposed as globalThis.HomeSafeZillow so content.js and tests can share it.

(function () {
  const ZPID_RE = /\/(\d+)_zpid/;

  function zpidFromUrl(url) {
    const m = String(url).match(ZPID_RE);
    return m ? m[1] : null;
  }

  function parseJson(text) {
    try {
      return JSON.parse(text);
    } catch {
      return null;
    }
  }

  // Walks a JSON tree, also descending into strings that hold JSON
  // (Zillow stores gdpClientCache as a string).
  function* walk(node, depth = 0) {
    if (depth > 40 || node == null) return;
    if (typeof node === "string") {
      const t = node.trimStart();
      if (t.length > 50 && (t[0] === "{" || t[0] === "[")) {
        const parsed = parseJson(t);
        if (parsed) yield* walk(parsed, depth + 1);
      }
      return;
    }
    if (typeof node !== "object") return;
    yield node;
    for (const value of Array.isArray(node) ? node : Object.values(node)) {
      if (value && (typeof value === "object" || typeof value === "string")) {
        yield* walk(value, depth + 1);
      }
    }
  }

  const SIGNAL_KEYS = [
    "price", "bedrooms", "bathrooms", "livingArea", "yearBuilt", "homeType",
    "homeStatus", "zestimate", "description", "responsivePhotos", "address",
  ];

  // Picks the richest object describing this zpid.
  function findProperty(roots, zpid) {
    let best = null;
    let bestScore = 0;
    for (const root of roots) {
      for (const obj of walk(root)) {
        if (Array.isArray(obj) || obj.zpid == null) continue;
        if (zpid && String(obj.zpid) !== zpid) continue;
        const score = SIGNAL_KEYS.reduce((n, k) => n + (obj[k] != null ? 1 : 0), 0);
        if (score > bestScore) {
          best = obj;
          bestScore = score;
        }
      }
    }
    return bestScore >= 3 ? best : null;
  }

  function embeddedJson(doc) {
    const roots = [];
    for (const script of doc.querySelectorAll('script[type="application/json"], script#__NEXT_DATA__')) {
      const parsed = parseJson(script.textContent);
      if (parsed) roots.push(parsed);
    }
    return roots;
  }

  function jsonLd(doc) {
    const items = [];
    for (const script of doc.querySelectorAll('script[type="application/ld+json"]')) {
      const parsed = parseJson(script.textContent);
      if (!parsed) continue;
      for (const item of [parsed, ...(parsed["@graph"] || [])].flat()) {
        if (item && typeof item === "object") items.push(item);
      }
    }
    return items.find((i) => i.address && (i.geo || i.floorSize || i["@type"] !== "Organization")) || null;
  }

  const num = (v) => {
    if (v == null || v === "") return null;
    const n = typeof v === "number" ? v : Number(String(v).replace(/[^0-9.\-]/g, ""));
    return Number.isFinite(n) ? n : null;
  };
  const int = (v) => {
    const n = num(v);
    return n == null ? null : Math.round(n);
  };
  const str = (v) => (v == null || v === "" ? null : String(v));

  function lotSqft(p) {
    const value = num(p.lotAreaValue ?? p.resoFacts?.lotSize);
    const units = String(p.lotAreaUnits || p.resoFacts?.lotSize || "").toLowerCase();
    if (value != null) return Math.round(units.includes("acre") ? value * 43560 : value);
    return int(p.lotSize);
  }

  function hoaMonthly(p) {
    const direct = int(p.monthlyHoaFee ?? p.hoaFee);
    if (direct != null) return direct;
    const fee = p.resoFacts?.hoaFee; // e.g. "$250 monthly"
    if (!fee) return null;
    const amount = num(String(fee).split(/\s/)[0]);
    if (amount == null) return null;
    return /annual|year/i.test(fee) ? Math.round(amount / 12) : Math.round(amount);
  }

  // Largest JPEG per photo; falls back to whatever URL is present.
  function photos(p) {
    const list = p.responsivePhotos || p.originalPhotos || p.photos || p.hugePhotos || [];
    const out = [];
    const seen = new Set();
    for (const photo of list) {
      const sources = photo?.mixedSources?.jpeg || photo?.mixedSources?.webp || [];
      const largest = sources.reduce((a, b) => ((b?.width || 0) > (a?.width || 0) ? b : a), null);
      const url = largest?.url || photo?.url || photo?.fullUrl;
      if (!url || seen.has(url)) continue;
      seen.add(url);
      out.push({
        url,
        width: int(largest?.width),
        height: int(largest?.height),
        caption: str(photo?.caption),
      });
    }
    return out;
  }

  function domPhotos(doc) {
    const out = [];
    const seen = new Set();
    for (const img of doc.querySelectorAll('img[src*="photos.zillowstatic.com"], source[srcset*="photos.zillowstatic.com"]')) {
      const candidates = (img.getAttribute("srcset") || img.getAttribute("src") || "")
        .split(",")
        .map((s) => s.trim().split(/\s+/))
        .map(([url, w]) => ({ url, w: parseInt(w) || 0 }))
        .filter((c) => c.url);
      const best = candidates.sort((a, b) => b.w - a.w)[0];
      if (best && !seen.has(best.url)) {
        seen.add(best.url);
        out.push({ url: best.url, width: best.w || null });
      }
    }
    return out;
  }

  function fromProperty(p, zpid, url) {
    const a = p.address || {};
    return {
      source: "zillow",
      sourceId: String(p.zpid ?? zpid),
      url: p.hdpUrl ? new URL(p.hdpUrl, "https://www.zillow.com").href : url,
      address: str(a.streetAddress ?? p.streetAddress),
      city: str(a.city ?? p.city),
      state: str(a.state ?? p.state),
      zip: str(a.zipcode ?? p.zipcode),
      price: int(p.price ?? p.listPrice ?? p.unformattedPrice),
      beds: num(p.bedrooms ?? p.beds),
      baths: num(p.bathrooms ?? p.baths),
      sqft: int(p.livingArea ?? p.livingAreaValue ?? p.area),
      lotSqft: lotSqft(p),
      yearBuilt: int(p.yearBuilt ?? p.resoFacts?.yearBuilt),
      homeType: str(p.homeType),
      status: str(p.homeStatus),
      latitude: num(p.latitude ?? p.latLong?.latitude),
      longitude: num(p.longitude ?? p.latLong?.longitude),
      zestimate: int(p.zestimate),
      rentZestimate: int(p.rentZestimate),
      hoaMonthly: hoaMonthly(p),
      daysOnMarket: int(p.daysOnZillow),
      description: str(p.description),
      images: photos(p),
    };
  }

  function fillFromJsonLd(capture, ld) {
    if (!ld) return;
    const a = ld.address || {};
    capture.address ??= str(a.streetAddress);
    capture.city ??= str(a.addressLocality);
    capture.state ??= str(a.addressRegion);
    capture.zip ??= str(a.postalCode);
    capture.latitude ??= num(ld.geo?.latitude);
    capture.longitude ??= num(ld.geo?.longitude);
    capture.sqft ??= int(ld.floorSize?.value);
    capture.price ??= int(ld.offers?.price);
  }

  function fillFromDom(capture, doc) {
    if (capture.price == null) {
      const el = doc.querySelector('[data-testid="price"], [data-test="property-card-price"]');
      capture.price = int(el?.textContent);
    }
    if (capture.address == null) {
      const h1 = doc.querySelector("h1");
      const parts = h1?.textContent?.split(",").map((s) => s.trim()) || [];
      if (parts.length) capture.address = parts[0];
    }
    if (!capture.images?.length) capture.images = domPhotos(doc);
  }

  /**
   * Returns a capture for the home in `doc`, or null if `doc` doesn't
   * describe the home at `url` (e.g. stale SPA data after navigation).
   */
  function extract(doc, url) {
    const zpid = zpidFromUrl(url);
    if (!zpid) return null;
    const property = findProperty(embeddedJson(doc), zpid);
    const ld = jsonLd(doc);
    if (!property && !ld) return null;

    const capture = property
      ? fromProperty(property, zpid, url)
      : { source: "zillow", sourceId: zpid, url, images: [] };
    fillFromJsonLd(capture, ld);
    fillFromDom(capture, doc);
    capture.raw = { extractor: "zillow-v1", capturedFrom: url, property, jsonLd: ld };
    return capture;
  }

  globalThis.HomeSafeZillow = { extract, zpidFromUrl };
})();
