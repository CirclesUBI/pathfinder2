use std::fmt;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::time::SystemTime;
use json::JsonValue;
use crate::safe_db::edge_db_dispenser::{EdgeDbDispenser, EdgeDBVersion};

pub struct CallContext {
    pub client_ip: String,
    pub request_id: JsonValue,
    pub rpc_function: String,
    pub start_time: std::time::Instant,
    pub dispenser: Arc<EdgeDbDispenser>,
    pub version: Option<EdgeDBVersion>,
}

impl CallContext {
    pub(crate) fn default() -> CallContext {
        CallContext {
            client_ip: "".to_string(),
            request_id: JsonValue::Null,
            rpc_function: "".to_string(),
            start_time: std::time::Instant::now(),
            dispenser: Arc::new(EdgeDbDispenser::new()),
            version: None,
        }
    }
}

impl CallContext {
    pub fn new(client_ip: &str, request_id: &JsonValue, rpc_function: &str, dispenser: &Arc<EdgeDbDispenser>) -> Self {
        let version = dispenser.get_latest_version();
        let context = CallContext {
            client_ip: client_ip.to_string(),
            request_id: request_id.clone(),
            rpc_function: rpc_function.to_string(),
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

        if self.client_ip.is_empty() && self.request_id.is_null() && self.rpc_function.is_empty() {
            println!("{}{}", prefix, suffix_str);
            return;
        }

        let thread_id_string = format!("{:?}", std::thread::current().id());
        let version_number = self.version.as_ref().map(|v| v.version_number).unwrap_or(0);

        println!(
            "{} {} [{}] [{}] [{}] [{}] [{}] {}",
            prefix, timestamp, thread_id_string, self.client_ip, self.request_id, self.rpc_function,
            version.unwrap_or(&version_number.to_string()), suffix_str
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

        if self.client_ip.is_empty() && self.request_id.is_null() && self.rpc_function.is_empty() {
            return;
        }
        let call_duration = self.start_time.elapsed().as_millis();
        self.log("<-", Some(&format!(" (took {} ms)", call_duration)), Some(&version_number.to_string()));
    }
}

impl Display for CallContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] [{}]", self.client_ip, self.request_id)
    }
}
