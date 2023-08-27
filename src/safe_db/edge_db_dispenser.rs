use crate::types::edge::EdgeDB;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct EdgeDBVersion {
    pub version_number: u64,
    pub edges: Arc<EdgeDB>,
}

pub struct EdgeDbDispenser {
    versions: Mutex<HashMap<u64, Arc<EdgeDB>>>,
    counter: Mutex<u64>,
    refs: Mutex<HashMap<u64, usize>>,
}

impl EdgeDbDispenser {
    pub(crate) fn new() -> Self {
        Self {
            versions: Mutex::new(HashMap::new()),
            counter: Mutex::new(0),
            refs: Mutex::new(HashMap::new()),
        }
    }

    pub fn get_latest_version(&self) -> Option<EdgeDBVersion> {
        let counter = self.counter.lock().unwrap();
        let versions = self.versions.lock().unwrap();

        if !versions.contains_key(&*counter) {
            return None;
        }

        let edges = versions.get(&*counter).unwrap().clone();
        drop(versions); // release the lock on versions as soon as we're done with it.

        let mut refs = self.refs.lock().unwrap();
        *refs.entry(*counter).or_insert(0) += 1;

        Some(EdgeDBVersion {
            version_number: *counter,
            edges,
        })
    }

    pub fn update(&self, new_edgedb: EdgeDB) {
        let mut counter = self.counter.lock().unwrap();
        let mut versions = self.versions.lock().unwrap();

        *counter += 1;
        versions.insert(*counter, Arc::new(new_edgedb));

        // Clean up any version that has no reference and is not the latest one.
        let old_versions: Vec<u64> = versions
            .keys()
            .filter(|&&version| {
                version != *counter && !self.refs.lock().unwrap().contains_key(&version)
            })
            .cloned()
            .collect();

        for version in old_versions {
            let mut v = versions.remove(&version);
            let db = v.take().unwrap();
            drop(db);
        }
    }

    pub fn try_release_version(&self, version: EdgeDBVersion) {
        let mut refs = self.refs.lock().unwrap();

        match refs.get_mut(&version.version_number) {
            Some(count) => {
                *count -= 1;
                if *count == 0 {
                    refs.remove(&version.version_number);
                }
            }
            _ => {
                println!(
                    "Error: version {} not found in refs.",
                    version.version_number
                );
            }
        }
    }
}
