use crate::{KvError, Kvpair, Value};

mod memory;
mod sleddb;
pub use memory::MemTable;
pub use sleddb::SledDb;

/// 对存储的抽象，定义了外界如何与后端打交道
pub trait Storage {
    /// 从一个 HashTable 里获得一个 key 的 value
    fn get(&self, table: &str, key: &str) -> Result<Option<Value>, KvError>;

    /// 从一个 HashTable 设置一个 key 的 value 并返回旧的 value
    fn set(&self, table: &str, key: String, value: Value) -> Result<Option<Value>, KvError>;

    /// 从 HashTable 中删除一个 key
    fn del(&self, table: &str, key: &str) -> Result<Option<Value>, KvError>;

    /// 判断 HashTable 中是否含有 key
    fn contains(&self, table: &str, key: &str) -> Result<bool, KvError>;

    /// 遍历 HashTable，返回所有的 kv pair，不好的接口
    fn get_all(&self, table: &str) -> Result<Vec<Kvpair>, KvError>;

    /// 遍历 HashTable，返回 Iterator
    fn get_iter(&self, table: &str) -> Result<Box<dyn Iterator<Item = Kvpair>>, KvError>;
}

/// 提供 Storage Iterator 来对 iter 进行抽象，这样 trait 的实现者
/// 只需要将它们的 iterator 提供了 StorageIter，然后它们保证 next()
/// 传出的类型实现了 Into<Kvpair> 即可
pub struct StorageIter<T> {
    iter: T,
}

impl<T> StorageIter<T> {
    pub fn new(iter: T) -> Self {
        Self { iter }
    }
}

impl<T> Iterator for StorageIter<T>
where
    T: Iterator,
    T::Item: Into<Kvpair>,
{
    type Item = Kvpair;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|v| v.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn memtable_basic_interfaces_should_work() {
        let store = MemTable::new();
        test_basic_interfaces_should_work(store);
    }

    #[test]
    fn memtable_get_all_should_work() {
        let store = MemTable::new();
        test_get_all_should_work(store);
    }

    #[test]
    fn memtable_get_iter_should_work() {
        let store = MemTable::new();
        test_get_iter_should_work(store);
    }

    #[test]
    fn sleddb_basic_interface_should_work() {
        let dir = tempdir().unwrap();
        let store = SledDb::new(dir);
        test_basic_interfaces_should_work(store);
    }

    #[test]
    fn sleddb_get_all_should_work() {
        let dir = tempdir().unwrap();
        let store = SledDb::new(dir);
        test_get_all_should_work(store);
    }
    #[test]
    fn sleddb_iter_should_work() {
        let dir = tempdir().unwrap();
        let store = SledDb::new(dir);
        test_get_iter_should_work(store);
    }

    fn test_basic_interfaces_should_work(store: impl Storage) {
        // 首次插入会返回 None
        let v = store.set("t1", "k1".into(), "v1".into()).unwrap();
        assert!(v.is_none());

        let v = store.set("t1", "k1".into(), "v2".into()).unwrap();
        assert_eq!(v, Some("v1".into()));

        let v = store.get("t1", "k1").unwrap();
        assert_eq!(v, Some("v2".into()));

        let v = store.get("t2", "k1").unwrap();
        assert!(v.is_none());

        let v = store.get("t1", "k2").unwrap();
        assert!(v.is_none());

        assert_eq!(store.contains("t1", "k1"), Ok(true));
        assert_eq!(store.contains("t1", "k3"), Ok(false));
        assert_eq!(store.contains("t3", "k1"), Ok(false));

        let v = store.del("t1", "k1").unwrap();
        assert_eq!(v, Some("v2".into()));
        assert_eq!(store.contains("t1", "k1"), Ok(false));

        let v = store.del("t2", "k1").unwrap();
        assert!(v.is_none());

        let v = store.del("t1", "k2").unwrap();
        assert!(v.is_none());
    }

    fn test_get_all_should_work(store: impl Storage) {
        store.set("t1", "k1".into(), "v1".into()).unwrap();
        store.set("t1", "k2".into(), "v2".into()).unwrap();
        let mut data: Vec<_> = store.get_all("t1").unwrap();
        data.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(
            data,
            vec![
                Kvpair::new("k1", "v1".into()),
                Kvpair::new("k2", "v2".into()),
            ]
        );
    }

    fn test_get_iter_should_work(store: impl Storage) {
        store.set("t1", "k1".into(), "v1".into()).unwrap();
        store.set("t1", "k2".into(), "v2".into()).unwrap();
        let mut data: Vec<_> = store.get_iter("t1").unwrap().collect();
        data.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(
            data,
            vec![
                Kvpair::new("k1", "v1".into()),
                Kvpair::new("k2", "v2".into()),
            ]
        );
    }
}
