use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::Utc;
use tokio::sync::Mutex;
use turso::{Builder, Connection, Row, Value};

use crate::model::{Attribute, Home, HomeCapture, HomeImage, ImageCapture};

/// Builds a positional parameter list of turso values.
macro_rules! vals {
    ($($v:expr),* $(,)?) => { Vec::<Value>::from([$($v),*]) };
}

/// Ordered schema migrations. Append new entries; never edit applied ones.
const MIGRATIONS: &[&str] = &[
    // 1: core tables
    r#"
    CREATE TABLE homes (
        id              INTEGER PRIMARY KEY AUTOINCREMENT,
        source          TEXT NOT NULL,
        source_id       TEXT NOT NULL,
        url             TEXT,
        address         TEXT,
        city            TEXT,
        state           TEXT,
        zip             TEXT,
        price           INTEGER,
        beds            REAL,
        baths           REAL,
        sqft            INTEGER,
        lot_sqft        INTEGER,
        year_built      INTEGER,
        home_type       TEXT,
        status          TEXT,
        latitude        REAL,
        longitude       REAL,
        zestimate       INTEGER,
        rent_zestimate  INTEGER,
        hoa_monthly     INTEGER,
        days_on_market  INTEGER,
        description     TEXT,
        notes           TEXT,
        created_at      TEXT NOT NULL,
        updated_at      TEXT NOT NULL
    );
    CREATE UNIQUE INDEX homes_source_id ON homes (source, source_id);

    CREATE TABLE home_images (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        home_id     INTEGER NOT NULL REFERENCES homes (id) ON DELETE CASCADE,
        position    INTEGER NOT NULL,
        source_url  TEXT NOT NULL,
        local_path  TEXT,
        width       INTEGER,
        height      INTEGER,
        caption     TEXT
    );
    CREATE UNIQUE INDEX home_images_url ON home_images (home_id, source_url);

    -- Every raw payload ever captured for a home, from listing sites or
    -- enrichment sources. Lets us re-derive fields and track price history.
    CREATE TABLE home_snapshots (
        id           INTEGER PRIMARY KEY AUTOINCREMENT,
        home_id      INTEGER NOT NULL REFERENCES homes (id) ON DELETE CASCADE,
        source       TEXT NOT NULL,
        captured_at  TEXT NOT NULL,
        price        INTEGER,
        status       TEXT,
        payload      TEXT
    );
    CREATE INDEX home_snapshots_home ON home_snapshots (home_id, captured_at);

    -- Open-ended enrichment data, one namespace per enricher.
    CREATE TABLE home_attributes (
        home_id     INTEGER NOT NULL REFERENCES homes (id) ON DELETE CASCADE,
        namespace   TEXT NOT NULL,
        key         TEXT NOT NULL,
        value       TEXT NOT NULL,
        source      TEXT,
        updated_at  TEXT NOT NULL,
        PRIMARY KEY (home_id, namespace, key)
    );
    "#,
];

/// Columns selected for [`Home`], in the order [`home_from_row`] reads them.
const HOME_COLUMNS: &str = "h.id, h.source, h.source_id, h.url, h.address, h.city, h.state, h.zip, \
    h.price, h.beds, h.baths, h.sqft, h.lot_sqft, h.year_built, h.home_type, h.status, \
    h.latitude, h.longitude, h.zestimate, h.rent_zestimate, h.hoa_monthly, h.days_on_market, \
    h.description, h.notes, h.created_at, h.updated_at, \
    (SELECT local_path FROM home_images i WHERE i.home_id = h.id AND i.local_path IS NOT NULL ORDER BY position LIMIT 1), \
    (SELECT COUNT(*) FROM home_images i WHERE i.home_id = h.id)";

/// Turso-backed storage. Cheap to clone; all clones share one connection.
#[derive(Clone)]
pub struct Store {
    conn: Arc<Mutex<Connection>>,
    // Keeps the database alive for the connection's lifetime.
    _db: Arc<turso::Database>,
}

impl Store {
    pub async fn open(path: &Path) -> Result<Self> {
        let path = path.to_str().context("database path is not valid UTF-8")?;
        Self::open_url(path).await
    }

    pub async fn open_in_memory() -> Result<Self> {
        Self::open_url(":memory:").await
    }

    async fn open_url(path: &str) -> Result<Self> {
        let db = Builder::new_local(path)
            .build()
            .await
            .with_context(|| format!("opening database {path}"))?;
        let conn = db.connect()?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
            _db: Arc::new(db),
        };
        store.migrate().await?;
        Ok(store)
    }

    async fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL)",
            (),
        )
        .await?;
        let mut rows = conn
            .query("SELECT COALESCE(MAX(version), 0) FROM schema_migrations", ())
            .await?;
        let current = match rows.next().await? {
            Some(row) => int(&row, 0)?.unwrap_or(0),
            None => 0,
        };
        drop(rows);
        for (ix, sql) in MIGRATIONS.iter().enumerate() {
            let version = ix as i64 + 1;
            if version <= current {
                continue;
            }
            conn.execute("BEGIN", ()).await?;
            let applied = async {
                conn.execute_batch(sql).await?;
                conn.execute(
                    "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
                    (version, now()),
                )
                .await?;
                anyhow::Ok(())
            }
            .await;
            match applied {
                Ok(()) => conn.execute("COMMIT", ()).await?,
                Err(err) => {
                    conn.execute("ROLLBACK", ()).await.ok();
                    return Err(err.context(format!("applying migration {version}")));
                }
            };
        }
        Ok(())
    }

    /// Insert or update a home from a capture, record a snapshot, and register
    /// its images (not yet downloaded). Returns the home id.
    pub async fn save_capture(&self, capture: &HomeCapture) -> Result<i64> {
        anyhow::ensure!(!capture.source.is_empty(), "capture is missing a source");
        anyhow::ensure!(!capture.source_id.is_empty(), "capture is missing a source id");

        let conn = self.conn.lock().await;
        let ts = now();
        conn.execute("BEGIN", ()).await?;
        let result = async {
            let c = capture;
            let params: Vec<Value> = vec![
                c.source.clone().into(),
                c.source_id.clone().into(),
                c.url.clone().into(),
                c.address.clone().into(),
                c.city.clone().into(),
                c.state.clone().into(),
                c.zip.clone().into(),
                c.price.into(),
                c.beds.into(),
                c.baths.into(),
                c.sqft.into(),
                c.lot_sqft.into(),
                c.year_built.into(),
                c.home_type.clone().into(),
                c.status.clone().into(),
                c.latitude.into(),
                c.longitude.into(),
                c.zestimate.into(),
                c.rent_zestimate.into(),
                c.hoa_monthly.into(),
                c.days_on_market.into(),
                c.description.clone().into(),
                ts.clone().into(),
            ];
            // Re-saving refreshes listing data but keeps anything the capture
            // lacks, so a sparse fallback parse never erases good values.
            conn.execute(
                r#"
                INSERT INTO homes (
                    source, source_id, url, address, city, state, zip, price, beds, baths,
                    sqft, lot_sqft, year_built, home_type, status, latitude, longitude,
                    zestimate, rent_zestimate, hoa_monthly, days_on_market, description,
                    created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                          ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?23)
                ON CONFLICT (source, source_id) DO UPDATE SET
                    url = COALESCE(excluded.url, url),
                    address = COALESCE(excluded.address, address),
                    city = COALESCE(excluded.city, city),
                    state = COALESCE(excluded.state, state),
                    zip = COALESCE(excluded.zip, zip),
                    price = COALESCE(excluded.price, price),
                    beds = COALESCE(excluded.beds, beds),
                    baths = COALESCE(excluded.baths, baths),
                    sqft = COALESCE(excluded.sqft, sqft),
                    lot_sqft = COALESCE(excluded.lot_sqft, lot_sqft),
                    year_built = COALESCE(excluded.year_built, year_built),
                    home_type = COALESCE(excluded.home_type, home_type),
                    status = COALESCE(excluded.status, status),
                    latitude = COALESCE(excluded.latitude, latitude),
                    longitude = COALESCE(excluded.longitude, longitude),
                    zestimate = COALESCE(excluded.zestimate, zestimate),
                    rent_zestimate = COALESCE(excluded.rent_zestimate, rent_zestimate),
                    hoa_monthly = COALESCE(excluded.hoa_monthly, hoa_monthly),
                    days_on_market = COALESCE(excluded.days_on_market, days_on_market),
                    description = COALESCE(excluded.description, description),
                    updated_at = excluded.updated_at
                "#,
                params,
            )
            .await?;

            let home_id = {
                let mut rows = conn
                    .query(
                        "SELECT id FROM homes WHERE source = ?1 AND source_id = ?2",
                        (c.source.as_str(), c.source_id.as_str()),
                    )
                    .await?;
                let row = rows.next().await?.context("saved home not found")?;
                int(&row, 0)?.context("home id is null")?
            };

            let payload = c.raw.as_ref().map(|v| v.to_string());
            conn.execute(
                "INSERT INTO home_snapshots (home_id, source, captured_at, price, status, payload) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                vals![
                    home_id.into(),
                    c.source.clone().into(),
                    ts.clone().into(),
                    c.price.into(),
                    c.status.clone().into(),
                    payload.into(),
                ],
            )
            .await?;

            for (position, image) in c.images.iter().enumerate() {
                insert_image(&conn, home_id, position as i64, image).await?;
            }
            anyhow::Ok(home_id)
        }
        .await;

        match result {
            Ok(id) => {
                conn.execute("COMMIT", ()).await?;
                Ok(id)
            }
            Err(err) => {
                conn.execute("ROLLBACK", ()).await.ok();
                Err(err)
            }
        }
    }

    pub async fn list_homes(&self) -> Result<Vec<Home>> {
        let conn = self.conn.lock().await;
        let mut rows = conn
            .query(
                format!("SELECT {HOME_COLUMNS} FROM homes h ORDER BY h.updated_at DESC"),
                (),
            )
            .await?;
        let mut homes = Vec::new();
        while let Some(row) = rows.next().await? {
            homes.push(home_from_row(&row)?);
        }
        Ok(homes)
    }

    pub async fn get_home(&self, id: i64) -> Result<Option<Home>> {
        let conn = self.conn.lock().await;
        let mut rows = conn
            .query(format!("SELECT {HOME_COLUMNS} FROM homes h WHERE h.id = ?1"), (id,))
            .await?;
        rows.next().await?.map(|row| home_from_row(&row)).transpose()
    }

    pub async fn find_home(&self, source: &str, source_id: &str) -> Result<Option<Home>> {
        let conn = self.conn.lock().await;
        let mut rows = conn
            .query(
                format!("SELECT {HOME_COLUMNS} FROM homes h WHERE h.source = ?1 AND h.source_id = ?2"),
                (source, source_id),
            )
            .await?;
        rows.next().await?.map(|row| home_from_row(&row)).transpose()
    }

    pub async fn delete_home(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().await;
        // Explicit child deletes: don't rely on foreign key enforcement.
        for sql in [
            "DELETE FROM home_images WHERE home_id = ?1",
            "DELETE FROM home_snapshots WHERE home_id = ?1",
            "DELETE FROM home_attributes WHERE home_id = ?1",
            "DELETE FROM homes WHERE id = ?1",
        ] {
            conn.execute(sql, (id,)).await?;
        }
        Ok(())
    }

    pub async fn set_notes(&self, id: i64, notes: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "UPDATE homes SET notes = ?1, updated_at = ?2 WHERE id = ?3",
            vals![notes.map(str::to_string).into(), now().into(), id.into()],
        )
        .await?;
        Ok(())
    }

    pub async fn images(&self, home_id: i64) -> Result<Vec<HomeImage>> {
        let conn = self.conn.lock().await;
        let mut rows = conn
            .query(
                "SELECT id, home_id, position, source_url, local_path, width, height, caption \
                 FROM home_images WHERE home_id = ?1 ORDER BY position",
                (home_id,),
            )
            .await?;
        let mut images = Vec::new();
        while let Some(row) = rows.next().await? {
            images.push(HomeImage {
                id: int(&row, 0)?.unwrap_or_default(),
                home_id: int(&row, 1)?.unwrap_or_default(),
                position: int(&row, 2)?.unwrap_or_default(),
                source_url: text(&row, 3)?.unwrap_or_default(),
                local_path: text(&row, 4)?,
                width: int(&row, 5)?,
                height: int(&row, 6)?,
                caption: text(&row, 7)?,
            });
        }
        Ok(images)
    }

    pub async fn set_image_path(&self, image_id: i64, local_path: &str) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "UPDATE home_images SET local_path = ?1 WHERE id = ?2",
            (local_path, image_id),
        )
        .await?;
        Ok(())
    }

    pub async fn upsert_attribute(&self, home_id: i64, attr: &Attribute) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO home_attributes (home_id, namespace, key, value, source, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT (home_id, namespace, key) DO UPDATE SET \
                value = excluded.value, source = excluded.source, updated_at = excluded.updated_at",
            vals![
                home_id.into(),
                attr.namespace.clone().into(),
                attr.key.clone().into(),
                attr.value.to_string().into(),
                attr.source.clone().into(),
                now().into(),
            ],
        )
        .await?;
        Ok(())
    }

    pub async fn attributes(&self, home_id: i64) -> Result<Vec<Attribute>> {
        let conn = self.conn.lock().await;
        let mut rows = conn
            .query(
                "SELECT namespace, key, value, source, updated_at FROM home_attributes \
                 WHERE home_id = ?1 ORDER BY namespace, key",
                (home_id,),
            )
            .await?;
        let mut attrs = Vec::new();
        while let Some(row) = rows.next().await? {
            let raw = text(&row, 2)?.unwrap_or_default();
            attrs.push(Attribute {
                namespace: text(&row, 0)?.unwrap_or_default(),
                key: text(&row, 1)?.unwrap_or_default(),
                value: serde_json::from_str(&raw).unwrap_or(serde_json::Value::String(raw)),
                source: text(&row, 3)?,
                updated_at: text(&row, 4)?,
            });
        }
        Ok(attrs)
    }
}

async fn insert_image(
    conn: &Connection,
    home_id: i64,
    position: i64,
    image: &ImageCapture,
) -> Result<()> {
    if image.url.is_empty() {
        return Ok(());
    }
    conn.execute(
        "INSERT INTO home_images (home_id, position, source_url, width, height, caption) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
         ON CONFLICT (home_id, source_url) DO UPDATE SET position = excluded.position",
        vals![
            home_id.into(),
            position.into(),
            image.url.clone().into(),
            image.width.into(),
            image.height.into(),
            image.caption.clone().into(),
        ],
    )
    .await?;
    Ok(())
}

fn home_from_row(row: &Row) -> Result<Home> {
    Ok(Home {
        id: int(row, 0)?.unwrap_or_default(),
        source: text(row, 1)?.unwrap_or_default(),
        source_id: text(row, 2)?.unwrap_or_default(),
        url: text(row, 3)?,
        address: text(row, 4)?,
        city: text(row, 5)?,
        state: text(row, 6)?,
        zip: text(row, 7)?,
        price: int(row, 8)?,
        beds: real(row, 9)?,
        baths: real(row, 10)?,
        sqft: int(row, 11)?,
        lot_sqft: int(row, 12)?,
        year_built: int(row, 13)?,
        home_type: text(row, 14)?,
        status: text(row, 15)?,
        latitude: real(row, 16)?,
        longitude: real(row, 17)?,
        zestimate: int(row, 18)?,
        rent_zestimate: int(row, 19)?,
        hoa_monthly: int(row, 20)?,
        days_on_market: int(row, 21)?,
        description: text(row, 22)?,
        notes: text(row, 23)?,
        created_at: text(row, 24)?.unwrap_or_default(),
        updated_at: text(row, 25)?.unwrap_or_default(),
        cover_image: text(row, 26)?,
        image_count: int(row, 27)?.unwrap_or_default(),
    })
}

fn int(row: &Row, ix: usize) -> Result<Option<i64>> {
    Ok(match row.get_value(ix)? {
        Value::Integer(v) => Some(v),
        Value::Real(v) => Some(v as i64),
        Value::Text(s) => s.parse().ok(),
        Value::Null | Value::Blob(_) => None,
    })
}

fn real(row: &Row, ix: usize) -> Result<Option<f64>> {
    Ok(match row.get_value(ix)? {
        Value::Integer(v) => Some(v as f64),
        Value::Real(v) => Some(v),
        Value::Text(s) => s.parse().ok(),
        Value::Null | Value::Blob(_) => None,
    })
}

fn text(row: &Row, ix: usize) -> Result<Option<String>> {
    Ok(match row.get_value(ix)? {
        Value::Text(s) => Some(s),
        Value::Integer(v) => Some(v.to_string()),
        Value::Real(v) => Some(v.to_string()),
        Value::Null | Value::Blob(_) => None,
    })
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capture(price: Option<i64>) -> HomeCapture {
        HomeCapture {
            source: "zillow".into(),
            source_id: "123".into(),
            address: Some("1 Main St".into()),
            city: Some("Springfield".into()),
            price,
            sqft: Some(1000),
            images: vec![
                ImageCapture { url: "https://example.com/a.jpg".into(), ..Default::default() },
                ImageCapture { url: "https://example.com/b.jpg".into(), ..Default::default() },
            ],
            raw: Some(serde_json::json!({"zpid": 123})),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn save_is_idempotent_and_merges() -> Result<()> {
        let store = Store::open_in_memory().await?;
        let id = store.save_capture(&capture(Some(500_000))).await?;
        let mut sparse = capture(Some(450_000));
        sparse.address = None;
        let id2 = store.save_capture(&sparse).await?;
        assert_eq!(id, id2);

        let homes = store.list_homes().await?;
        assert_eq!(homes.len(), 1);
        let home = &homes[0];
        assert_eq!(home.price, Some(450_000));
        assert_eq!(home.address.as_deref(), Some("1 Main St"));
        assert_eq!(home.price_per_sqft(), Some(450));
        assert_eq!(home.image_count, 2);
        assert_eq!(home.cover_image, None);

        let images = store.images(id).await?;
        store.set_image_path(images[0].id, "/tmp/a.jpg").await?;
        let home = store.get_home(id).await?.unwrap();
        assert_eq!(home.cover_image.as_deref(), Some("/tmp/a.jpg"));
        Ok(())
    }

    #[tokio::test]
    async fn attributes_and_delete() -> Result<()> {
        let store = Store::open_in_memory().await?;
        let id = store.save_capture(&capture(Some(1))).await?;
        let attr = Attribute {
            namespace: "schools".into(),
            key: "elementary".into(),
            value: serde_json::json!({"name": "Lincoln", "rating": 8}),
            source: Some("test".into()),
            updated_at: None,
        };
        store.upsert_attribute(id, &attr).await?;
        store.upsert_attribute(id, &attr).await?;
        let attrs = store.attributes(id).await?;
        assert_eq!(attrs.len(), 1);
        assert_eq!(attrs[0].value["rating"], 8);

        store.delete_home(id).await?;
        assert!(store.list_homes().await?.is_empty());
        assert!(store.images(id).await?.is_empty());
        Ok(())
    }
}
