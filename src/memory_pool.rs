use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use axum_session::{DatabaseError, DatabasePool};
use chrono::{TimeZone, Utc};

#[derive(Clone, Debug, Default)]
struct SessionValue {
    id: String,
    session: String,
    expires: i64,
}

#[derive(Clone, Debug, Default)]
pub struct MemoryPool {
    entries: Arc<RwLock<HashMap<String, SessionValue>>>,
    expires: Arc<RwLock<HashMap<i64, Vec<String>>>>,
}

impl MemoryPool {
    pub fn new() -> MemoryPool {
        MemoryPool::default()
    }
}

#[async_trait::async_trait]
impl DatabasePool for MemoryPool {
    #[inline(always)]
    async fn initiate(&self, _table_name: &str) -> Result<(), DatabaseError> {
        Ok(())
    }

    #[inline(always)]
    async fn delete_by_expiry(&self, _table_name: &str) -> Result<Vec<String>, DatabaseError> {
        let mut expired = self
            .expires
            .write()
            .map_err(|_| DatabaseError::GenericCreateError("Lock poisoned".into()))?;
        let now = Utc::now().timestamp();
        let expired_entries: Vec<String> = expired
            .iter()
            .filter(|(&k, _)| k < now)
            .flat_map(|(_, v)| v.clone())
            .collect();
        expired.retain(|&k, _| k >= now);

        let mut entries = self
            .entries
            .write()
            .map_err(|_| DatabaseError::GenericCreateError("Lock poisoned".into()))?;
        entries.retain(|_, v| !expired_entries.contains(&v.id));

        Ok(expired_entries)
    }

    #[inline(always)]
    async fn count(&self, _table_name: &str) -> Result<i64, DatabaseError> {
        Ok(self
            .entries
            .read()
            .map_err(|_| DatabaseError::GenericCreateError("Lock poisoned".into()))?
            .len() as i64)
    }

    #[inline(always)]
    async fn store(
        &self,
        id: &str,
        session: &str,
        expires: i64,
        _table_name: &str,
    ) -> Result<(), DatabaseError> {
        let expiry = chrono::DateTime::from_timestamp(expires, 0)
            .map(|expires| Utc.from_utc_datetime(&expires.naive_utc()))
            .map(|dt| dt.timestamp())
            .unwrap_or(0);

        let model = SessionValue {
            id: id.to_owned(),
            session: session.to_string(),
            expires: expiry,
        };

        let mut entries = self
            .entries
            .write()
            .map_err(|_| DatabaseError::GenericCreateError("Lock poisoned".into()))?;
        entries.insert(id.to_owned(), model.clone());

        let mut expires = self
            .expires
            .write()
            .map_err(|_| DatabaseError::GenericCreateError("Lock poisoned".into()))?;
        expires.entry(expiry).or_default().push(id.to_owned());

        Ok(())
    }

    #[inline(always)]
    async fn load(&self, id: &str, _table_name: &str) -> Result<Option<String>, DatabaseError> {
        let entries = self
            .entries
            .read()
            .map_err(|_| DatabaseError::GenericCreateError("Lock poisoned".into()))?;
        let maybe_model = entries.get(id);

        Ok(maybe_model.map(|model| model.session.clone()))
    }

    #[inline(always)]
    async fn delete_one_by_id(&self, id: &str, _table_name: &str) -> Result<(), DatabaseError> {
        let mut entries = self
            .entries
            .write()
            .map_err(|_| DatabaseError::GenericCreateError("Lock poisoned".into()))?;
        if let Some(entry) = entries.remove(id) {
            let mut expires = self
                .expires
                .write()
                .map_err(|_| DatabaseError::GenericCreateError("Lock poisoned".into()))?;
            expires.entry(entry.expires).and_modify(|v| {
                v.retain(|e| e != id);
            });
        }

        Ok(())
    }

    #[inline(always)]
    async fn exists(&self, id: &str, _table_name: &str) -> Result<bool, DatabaseError> {
        let entries = self
            .entries
            .read()
            .map_err(|_| DatabaseError::GenericCreateError("Lock poisoned".into()))?;
        Ok(entries.contains_key(id))
    }

    #[inline(always)]
    async fn delete_all(&self, _table_name: &str) -> Result<(), DatabaseError> {
        let mut entries = self
            .entries
            .write()
            .map_err(|_| DatabaseError::GenericCreateError("Lock poisoned".into()))?;
        entries.clear();
        let mut expires = self
            .expires
            .write()
            .map_err(|_| DatabaseError::GenericCreateError("Lock poisoned".into()))?;
        expires.clear();
        Ok(())
    }

    #[inline(always)]
    async fn get_ids(&self, _table_name: &str) -> Result<Vec<String>, DatabaseError> {
        let entries = self
            .entries
            .read()
            .map_err(|_| DatabaseError::GenericCreateError("Lock poisoned".into()))?;
        Ok(entries.keys().cloned().collect())
    }

    #[inline(always)]
    fn auto_handles_expiry(&self) -> bool {
        false
    }
}
