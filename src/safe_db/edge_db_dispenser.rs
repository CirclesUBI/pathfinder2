use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::types::edge::EdgeDB;

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
        println!("Tracing: get_latest_version - Acquiring lock on versions.");
        let counter = self.counter.lock().unwrap();
        let versions = self.versions.lock().unwrap();

        if !versions.contains_key(&*counter) {
            println!("Tracing: get_latest_version - No version found for counter {}.", *counter);
            return None;
        }

        let edges = versions.get(&*counter).unwrap().clone();
        drop(versions); // release the lock on versions as soon as we're done with it.

        println!("Tracing: get_latest_version - Acquired lock on refs.");

        let mut refs = self.refs.lock().unwrap();
        *refs.entry(*counter).or_insert(0) += 1;

        println!("Tracing: get_latest_version - Increased reference count for version {}.", *counter);

        Some(EdgeDBVersion { version_number: *counter, edges })
    }

    pub fn update(&self, new_edgedb: EdgeDB) {
        let mut counter = self.counter.lock().unwrap();
        let mut versions = self.versions.lock().unwrap();

        *counter += 1;
        versions.insert(*counter, Arc::new(new_edgedb));

        println!("Tracing: update - Updated version counter to {}.", *counter);

        // Clean up any version that has no reference and is not the latest one.
        let old_versions: Vec<u64> = versions
            .keys()
            .filter(|&&version| version != *counter && !self.refs.lock().unwrap().contains_key(&version))
            .cloned()
            .collect();

        println!("Tracing: update - Found {} old versions to clean up.", old_versions.len());

        for version in old_versions {
            let mut v = versions.remove(&version);
            let db = v.take().unwrap();
            println!("Tracing: update - Take and drop old version {}", version);
            drop(db);
            println!("Tracing: update - Cleaned up old version {}.", version);
        }
    }

    pub fn try_release_version(&self, version: EdgeDBVersion) {
        let mut refs = self.refs.lock().unwrap();

        match refs.get_mut(&version.version_number) {
            Some(count) => {
                *count -= 1;
                if *count == 0 {
                    refs.remove(&version.version_number);
                    println!("Tracing: try_release_version - Removed version {} from refs.", version.version_number);
                }
            }
            _ => {
                println!("Error: version {} not found in refs.", version.version_number);
            }
        }

        println!("Tracing: total refs: {}.", refs.len());
        println!("Total versions: {}.", self.versions.lock().unwrap().len());
    }
}
