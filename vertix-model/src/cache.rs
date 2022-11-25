use aragog::{Record, DatabaseRecord, DatabaseAccess};
use async_cell::sync::AsyncCell;
use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::sync::Arc;

/// An asynchronous cache meant to be used for unifying requests to other objects over a short
/// period of time. For example, fetching related models over the lifetime of a request.
///
/// It has no eviction policy; stored data will not be dropped until the cache is no longer alive.
#[derive(Clone)]
pub struct ShortLivedCache<K, V> {
    cache: Arc<AsyncCell<BTreeMap<K, Arc<AsyncCell<V>>>>>
}

impl<K, V> fmt::Debug for ShortLivedCache<K, V>
where
    K: fmt::Debug + Clone,
    V: fmt::Debug + Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ShortLivedCache").field("cache", &self.cache).finish()
    }
}

impl<K, V> Default for ShortLivedCache<K, V>
where
    K: Ord,
    V: Clone,
{
    fn default() -> Self {
        ShortLivedCache::new()
    }
}

impl<K, V> ShortLivedCache<K, V>
where
    K: Ord,
    V: Clone,
{
    pub fn new() -> ShortLivedCache<K, V> {
        ShortLivedCache { cache: AsyncCell::new_with(BTreeMap::default()).into() }
    }

    // Pre-initialize a cache with the given pairs
    pub fn new_with(pairs: impl IntoIterator<Item=(K, V)>) -> ShortLivedCache<K, V> {
        let mut map = BTreeMap::new();
        map.extend(pairs.into_iter().map(|(key, value)| (key, AsyncCell::new_with(value).into())));
        ShortLivedCache { cache: AsyncCell::new_with(map).into() }
    }

    /// Get the value from the cache if it already exists or is in progress, or run the provided
    /// generate function in order to start working on it if it hasn't been requested yet.
    pub async fn get<Q, Fut>(&self, key: &Q, generate: impl FnOnce() -> Fut) -> V
    where
        Q: ToOwned<Owned=K> + Ord + ?Sized,
        K: Borrow<Q>,
        Fut: Future<Output=V>,
    {
        // While have the map, nobody else can use it, so it's important that we put it away quickly
        let mut map = self.cache.take().await;

        if let Some(cell) = map.get(key).cloned() {
            // Put the map back and get/wait for the value
            self.cache.set(map);
            cell.get().await
        } else {
            // Create a new cell for the output, insert and put the map back
            let new_cell = AsyncCell::shared();
            let out = new_cell.clone();
            map.insert(key.to_owned(), new_cell);
            self.cache.set(map);

            // Wait for the output, put it in the cell, and then return it
            let value = generate().await;
            out.set(value.clone());
            value
        }
    }

    /// Insert a value into the cache
    pub async fn put(&self, key: K, value: V) {
        let mut map = self.cache.take().await;
        map.insert(key, AsyncCell::new_with(value).into());
        self.cache.set(map);
    }
}

/// A cache for db records
#[derive(Debug, Clone)]
pub struct RecordCache<T> {
    cache: ShortLivedCache<String, Result<Arc<DatabaseRecord<T>>, Arc<aragog::Error>>>,
}

impl<T> RecordCache<T> where T: Record + Send {
    pub fn new() -> RecordCache<T> {
        RecordCache { cache: ShortLivedCache::new() }
    }

    /// Pre-populate a record cache with the given records.
    pub fn new_with(records: impl IntoIterator<Item=Arc<DatabaseRecord<T>>>) -> RecordCache<T> {
        RecordCache {
            cache: ShortLivedCache::new_with(
                records.into_iter().map(|record| (record.key().to_owned(), Ok(record)))
            )
        }
    }

    /// Get a record from the cache, looking it up if not present.
    pub async fn get<D>(&self, key: &str, db: &D)
        -> Result<Arc<DatabaseRecord<T>>, Arc<aragog::Error>>
    where
        D: DatabaseAccess,
    {
        self.cache.get(key, || async {
            T::find(key, db).await.map(Arc::from).map_err(Arc::from)
        }).await
    }

    /// Insert a record into the cache
    pub async fn put(&self, record: Arc<DatabaseRecord<T>>) {
        self.cache.put(record.key().to_owned(), Ok(record)).await
    }

    /// Insert many records into the cache
    pub async fn put_many<A>(&self, records: impl IntoIterator<Item = A>)
        -> Vec<Arc<DatabaseRecord<T>>>
    where
        A: Into<Arc<DatabaseRecord<T>>>,
    {
        let mut out = vec![];

        for record in records {
            let arc = record.into();
            self.put(arc.clone()).await;
            out.push(arc);
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use actix_rt::time::sleep;
    use std::time::Duration;
    use std::sync::atomic::{AtomicU32, Ordering};
    use futures::join;
    use super::*;
    use Ordering::SeqCst;

    #[actix_rt::test]
    async fn calling_get_on_empty_cache_does_the_operation() {
        let counter = AtomicU32::new(0);
        let cache = ShortLivedCache::<&'static str, &'static str>::new();
        let output = cache.get(&"hello", || async {
            sleep(Duration::from_millis(100)).await;
            counter.store(1, SeqCst);
            "world"
        }).await;
        assert_eq!(counter.load(SeqCst), 1);
        assert_eq!(output, "world");
    }

    #[actix_rt::test]
    async fn parallel_get_calls_return_the_same_future() {
        let counter = AtomicU32::new(0);
        let cache = ShortLivedCache::<&'static str, &'static str>::new();
        let operation = || async {
            sleep(Duration::from_millis(100)).await;
            counter.fetch_add(1, SeqCst);
            "world"
        };
        let (out1, out2, out3) = join!(
            cache.get(&"hello", &operation),
            cache.get(&"hello", &operation),
            cache.get(&"hello", &operation),
        );
        assert_eq!(counter.load(SeqCst), 1);
        assert_eq!(out1, "world");
        assert_eq!(out2, "world");
        assert_eq!(out3, "world");
    }

    #[actix_rt::test]
    async fn subsequent_get_calls_return_cached_result() {
        let counter = AtomicU32::new(0);
        let cache = ShortLivedCache::<&'static str, &'static str>::new();
        let operation = || async {
            sleep(Duration::from_millis(100)).await;
            counter.fetch_add(1, SeqCst);
            "world"
        };
        assert_eq!(cache.get(&"hello", &operation).await, "world");
        assert_eq!(counter.load(SeqCst), 1);
        assert_eq!(cache.get(&"hello", &operation).await, "world");
        assert_eq!(counter.load(SeqCst), 1);
    }
}
