use std::sync::Arc;
use std::time::SystemTime;
use crate::safe_db::edge_db_dispenser::{EdgeDbDispenser, EdgeDBVersion};

pub struct CallContext {
    pub start_time: std::time::Instant,
    pub dispenser: Arc<EdgeDbDispenser>,
    pub version: Option<EdgeDBVersion>,
}

impl CallContext {
    pub fn new(dispenser: &Arc<EdgeDbDispenser>) -> Self {
        let version = dispenser.get_latest_version();
        let context = CallContext {
            start_time: std::time::Instant::now(),
            dispenser: dispenser.clone(),
            version: version,
        };

        context.log("->", None, None);
        context
    }

    pub fn log(&self, prefix: &str, suffix: Option<&str>, version: Option<&str>) {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let suffix_str = suffix.unwrap_or("");

        let thread_id_string = format!("{:?}", std::thread::current().id());
        let version_number = self.version.as_ref().map(|v| v.version_number).unwrap_or(0);

        println!(
            "{} {} [{}] [{}] {}",
            prefix, timestamp, thread_id_string, version.unwrap_or(&version_number.to_string()), suffix_str
        );
    }

    pub fn log_message(&self, message: &str) {
        self.log("  ", Some(&format!(" {}", message)), None);
    }
}

impl Drop for CallContext {
    fn drop(&mut self) {
        let version_number = self.version.as_ref().map(|v| v.version_number).unwrap_or(0);

        if let Some(version) = self.version.take() {
            self.dispenser.try_release_version(version);
        }

        let call_duration = self.start_time.elapsed().as_millis();
        self.log("<-", Some(&format!(" (took {} ms)", call_duration)), Some(&version_number.to_string()));
    }
}