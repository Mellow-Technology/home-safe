use serde::{Deserialize, Serialize};

/// A property as captured from a listing site by the browser extension.
///
/// Normalized fields are optional because sites omit them freely. `raw` holds
/// the untouched source payload so later enrichment or re-parsing never needs
/// to revisit the page.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HomeCapture {
    /// Listing source, e.g. `"zillow"`.
    pub source: String,
    /// The source's own id for the property (Zillow's zpid).
    pub source_id: String,
    pub url: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
    pub price: Option<i64>,
    pub beds: Option<f64>,
    pub baths: Option<f64>,
    pub sqft: Option<i64>,
    pub lot_sqft: Option<i64>,
    pub year_built: Option<i64>,
    pub home_type: Option<String>,
    pub status: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub zestimate: Option<i64>,
    pub rent_zestimate: Option<i64>,
    pub hoa_monthly: Option<i64>,
    pub days_on_market: Option<i64>,
    pub description: Option<String>,
    pub images: Vec<ImageCapture>,
    pub raw: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ImageCapture {
    pub url: String,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub caption: Option<String>,
}

/// A saved home as stored in the database.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Home {
    pub id: i64,
    pub source: String,
    pub source_id: String,
    pub url: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
    pub price: Option<i64>,
    pub beds: Option<f64>,
    pub baths: Option<f64>,
    pub sqft: Option<i64>,
    pub lot_sqft: Option<i64>,
    pub year_built: Option<i64>,
    pub home_type: Option<String>,
    pub status: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub zestimate: Option<i64>,
    pub rent_zestimate: Option<i64>,
    pub hoa_monthly: Option<i64>,
    pub days_on_market: Option<i64>,
    pub description: Option<String>,
    pub notes: Option<String>,
    /// Local path of the first downloaded image, for thumbnails.
    pub cover_image: Option<String>,
    pub image_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl Home {
    pub fn price_per_sqft(&self) -> Option<i64> {
        match (self.price, self.sqft) {
            (Some(p), Some(s)) if s > 0 => Some(p / s),
            _ => None,
        }
    }

    pub fn full_address(&self) -> String {
        let locality = [self.city.as_deref(), self.state.as_deref(), self.zip.as_deref()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ");
        match self.address.as_deref() {
            Some(a) if !locality.is_empty() => format!("{a}, {locality}"),
            Some(a) => a.to_string(),
            None => locality,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeImage {
    pub id: i64,
    pub home_id: i64,
    pub position: i64,
    pub source_url: String,
    pub local_path: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub caption: Option<String>,
}

/// A piece of data attached to a home by an enrichment source (schools,
/// commute times, flood zone, ...). Keyed by `(namespace, key)` so each
/// enricher owns its namespace; `value` is arbitrary JSON.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attribute {
    pub namespace: String,
    pub key: String,
    pub value: serde_json::Value,
    pub source: Option<String>,
    pub updated_at: Option<String>,
}
