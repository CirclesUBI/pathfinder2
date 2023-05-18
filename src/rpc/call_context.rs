use std::fmt;
use std::fmt::{Display, Formatter};
use std::time::SystemTime;
use json::JsonValue;

pub struct CallContext {
    client_ip: String,
    request_id: JsonValue,
    rpc_function: String,
    start_time: std::time::Instant,
}

impl CallContext {
    pub fn default() -> CallContext {
        CallContext {
            client_ip: "".to_string(),
            request_id: JsonValue::Null,
            rpc_function: "".to_string(),
            start_time: std::time::Instant::now(),
        }
    }
}

impl CallContext {
    pub fn new(client_ip: &str, request_id: &JsonValue, rpc_function: &str) -> Self {
        let context = CallContext {
            client_ip: client_ip.to_string(),
            request_id: request_id.clone(),
            rpc_function: rpc_function.to_string(),
            start_time: std::time::Instant::now(),
        };

        context.log("->", None);
        context
    }

    pub fn log(&self, prefix: &str, suffix: Option<&str>) {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let suffix_str = match suffix {
            Some(s) => s,
            None => "",
        };

        if self.client_ip.is_empty() && self.request_id.is_null() && self.rpc_function.is_empty() {
            println!(
                "{}",
                suffix_str
            );
            return;
        }

        let thread_id_string = format!("{:?}", std::thread::current().id());

        println!(
            "{} {} [{}] [{}] [{}] [{}]{}",
            prefix, timestamp, thread_id_string, self.client_ip, self.request_id, self.rpc_function, suffix_str
        );
    }

    pub fn log_message(&self, message: &str) {
        self.log("  ", Some(&format!(" {}", message)));
    }
}

impl Drop for CallContext {
    fn drop(&mut self) {
        if self.client_ip.is_empty() && self.request_id.is_null() && self.rpc_function.is_empty() {
            return;
        }
        let call_duration = self.start_time.elapsed().as_millis();
        self.log("<-", Some(&format!(" (took {} ms)", call_duration)));
    }
}

impl Display for CallContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] [{}]", self.client_ip, self.request_id)
    }
}
