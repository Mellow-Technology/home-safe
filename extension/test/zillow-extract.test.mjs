// Run with: node --test extension/test
// Exercises the extractor against a fixture shaped like a Zillow home page
// (Next.js __NEXT_DATA__ holding a stringified gdpClientCache).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";

const source = readFileSync(new URL("../src/zillow-extract.js", import.meta.url), "utf8");
const ctx = { URL };
vm.createContext(ctx);
vm.runInContext(source, ctx);
const { extract, zpidFromUrl } = ctx.HomeSafeZillow;

// Minimal stand-in for a Document: the extractor only reads script tags,
// plus a few DOM fallbacks that return nothing here.
function fakeDoc(scripts) {
  return {
    querySelectorAll(selector) {
      if (selector.includes("ld+json")) return scripts.filter((s) => s.type === "application/ld+json");
      if (selector.includes("script")) return scripts.filter((s) => s.type !== "application/ld+json");
      return [];
    },
    querySelector: () => null,
  };
}

const URL_ = "https://www.zillow.com/homedetails/1-Main-St-Austin-TX-78701/12345_zpid/";

const property = {
  zpid: 12345,
  price: 525000,
  bedrooms: 3,
  bathrooms: 2.5,
  livingArea: 1850,
  lotAreaValue: 0.25,
  lotAreaUnits: "Acres",
  yearBuilt: 2004,
  homeType: "SINGLE_FAMILY",
  homeStatus: "FOR_SALE",
  zestimate: 530100,
  latitude: 30.26,
  longitude: -97.74,
  resoFacts: { hoaFee: "$600 annually" },
  daysOnZillow: 12,
  description: "Lovely home",
  hdpUrl: "/homedetails/1-Main-St-Austin-TX-78701/12345_zpid/",
  address: { streetAddress: "1 Main St", city: "Austin", state: "TX", zipcode: "78701" },
  responsivePhotos: [
    { caption: "Front", mixedSources: { jpeg: [{ url: "https://photos.zillowstatic.com/a-384.jpg", width: 384 }, { url: "https://photos.zillowstatic.com/a-1536.jpg", width: 1536 }] } },
    { mixedSources: { jpeg: [{ url: "https://photos.zillowstatic.com/b-1536.jpg", width: 1536 }] } },
  ],
};

const nextData = {
  props: {
    pageProps: {
      componentProps: {
        // A nearby home that must not be picked.
        nearby: { zpid: 999, price: 1, bedrooms: 1, bathrooms: 1, livingArea: 1 },
        gdpClientCache: JSON.stringify({ 'ForSaleQuery{"zpid":12345}': { property } }),
      },
    },
  },
};

test("zpid parsing", () => {
  assert.equal(zpidFromUrl(URL_), "12345");
  assert.equal(zpidFromUrl("https://www.zillow.com/homes/"), null);
});

test("extracts property from __NEXT_DATA__", () => {
  const doc = fakeDoc([{ type: "application/json", textContent: JSON.stringify(nextData) }]);
  const c = extract(doc, URL_);
  assert.equal(c.sourceId, "12345");
  assert.equal(c.price, 525000);
  assert.equal(c.address, "1 Main St");
  assert.equal(c.zip, "78701");
  assert.equal(c.baths, 2.5);
  assert.equal(c.lotSqft, 10890);
  assert.equal(c.hoaMonthly, 50);
  assert.equal(c.url, URL_);
  assert.deepEqual([...c.images].map((i) => i.url), [
    "https://photos.zillowstatic.com/a-1536.jpg",
    "https://photos.zillowstatic.com/b-1536.jpg",
  ]);
  assert.equal(c.images[0].caption, "Front");
});

test("falls back to JSON-LD", () => {
  const ld = {
    "@type": "SingleFamilyResidence",
    address: { streetAddress: "1 Main St", addressLocality: "Austin", addressRegion: "TX", postalCode: "78701" },
    geo: { latitude: 30.26, longitude: -97.74 },
    floorSize: { value: "1,850" },
  };
  const doc = fakeDoc([{ type: "application/ld+json", textContent: JSON.stringify(ld) }]);
  const c = extract(doc, URL_);
  assert.equal(c.city, "Austin");
  assert.equal(c.sqft, 1850);
  assert.equal(c.price, null);
});

test("returns null for stale data about another home", () => {
  const doc = fakeDoc([{ type: "application/json", textContent: JSON.stringify(nextData) }]);
  assert.equal(extract(doc, "https://www.zillow.com/homedetails/x/777_zpid/"), null);
});
