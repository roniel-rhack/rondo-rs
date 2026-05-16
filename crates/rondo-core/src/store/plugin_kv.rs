use crate::error::Result;
use crate::store::sqlite::SqliteStore;
use rusqlite::params;

impl SqliteStore {
    /// Read a plugin-scoped value. `Ok(None)` if the key is unset.
    pub fn kv_get(&self, plugin_id: &str, key: &str) -> Result<Option<Vec<u8>>> {
        let conn = self.conn.lock().unwrap();
        let res: rusqlite::Result<Vec<u8>> = conn.query_row(
            super::queries::KV_GET,
            params![plugin_id, key],
            |r| r.get::<_, Vec<u8>>(0),
        );
        match res {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Write a plugin-scoped value. Upserts on `(plugin_id, key)`.
    pub fn kv_set(&self, plugin_id: &str, key: &str, value: &[u8]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(super::queries::KV_SET, params![plugin_id, key, value, now])?;
        Ok(())
    }

    /// Remove a plugin-scoped key. No-op if missing.
    pub fn kv_delete(&self, plugin_id: &str, key: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(super::queries::KV_DELETE, params![plugin_id, key])?;
        Ok(())
    }

    /// List every key currently set for `plugin_id`, sorted ascending.
    pub fn kv_list_keys(&self, plugin_id: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(super::queries::KV_LIST_FOR_PLUGIN)?;
        let rows = stmt.query_map(params![plugin_id], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}
