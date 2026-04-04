#![allow(dead_code)]

//! Guest tracing helpers for slide WASM modules.

use std::collections::BTreeMap;

use serde::Serialize;

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "vzglyd_host")]
unsafe extern "C" {
    #[link_name = "trace_span_start"]
    fn host_trace_span_start(ptr: *const u8, len: i32) -> i32;
    #[link_name = "trace_span_end"]
    fn host_trace_span_end(span_id: i32, ptr: *const u8, len: i32) -> i32;
    #[link_name = "trace_event"]
    fn host_trace_event(ptr: *const u8, len: i32) -> i32;
}

#[derive(Serialize)]
struct TracePayload<'a> {
    name: &'a str,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    attrs: BTreeMap<&'a str, &'a str>,
}

#[derive(Serialize)]
struct TraceEndPayload<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<&'a str>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    attrs: BTreeMap<&'a str, &'a str>,
}

fn encode_payload(name: &str, attrs: &[(&str, &str)]) -> Option<Vec<u8>> {
    let payload = TracePayload {
        name,
        attrs: attrs.iter().copied().collect(),
    };
    serde_json::to_vec(&payload).ok()
}

fn encode_end_payload<'a>(
    status: Option<&'a str>,
    attrs: &[(&'a str, &'a str)],
) -> Option<Vec<u8>> {
    let payload = TraceEndPayload {
        status,
        attrs: attrs.iter().copied().collect(),
    };
    serde_json::to_vec(&payload).ok()
}

/// RAII trace span for guest slide code.
///
/// Construct values with [`trace_scope`] or [`trace_scope_with_attrs`]. The span ends when
/// dropped, or earlier via [`TraceScope::end`].
pub struct TraceScope {
    span_id: i32,
    finished: bool,
    status: Option<String>,
    attrs: Vec<(String, String)>,
}

impl TraceScope {
    /// Set the final status attached when the span ends.
    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = Some(status.into());
    }

    /// Add a final attribute attached when the span ends.
    pub fn add_attr(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.attrs.push((key.into(), value.into()));
    }

    /// End the span immediately.
    pub fn end(mut self) {
        self.finish();
    }

    fn finish(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;

        #[cfg(target_arch = "wasm32")]
        unsafe {
            if self.span_id <= 0 {
                return;
            }

            let attr_pairs = self
                .attrs
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str()))
                .collect::<Vec<_>>();
            if let Some(bytes) = encode_end_payload(self.status.as_deref(), &attr_pairs) {
                let _ = host_trace_span_end(self.span_id, bytes.as_ptr(), bytes.len() as i32);
            } else {
                let _ = host_trace_span_end(self.span_id, std::ptr::null(), 0);
            }
        }
    }
}

impl Drop for TraceScope {
    fn drop(&mut self) {
        self.finish();
    }
}

/// Start a named trace span with no attributes.
pub fn trace_scope(name: &str) -> TraceScope {
    trace_scope_with_attrs(name, &[])
}

/// Start a named trace span with string attributes.
pub fn trace_scope_with_attrs(name: &str, attrs: &[(&str, &str)]) -> TraceScope {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let span_id = encode_payload(name, attrs)
            .map(|bytes| host_trace_span_start(bytes.as_ptr(), bytes.len() as i32))
            .unwrap_or(0);
        return TraceScope {
            span_id,
            finished: false,
            status: None,
            attrs: Vec::new(),
        };
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = name;
        let _ = attrs;
        TraceScope {
            span_id: 0,
            finished: false,
            status: None,
            attrs: Vec::new(),
        }
    }
}

/// Emit an instant trace event with no attributes.
pub fn trace_event(name: &str) {
    trace_event_with_attrs(name, &[]);
}

/// Emit an instant trace event with string attributes.
pub fn trace_event_with_attrs(name: &str, attrs: &[(&str, &str)]) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        if let Some(bytes) = encode_payload(name, attrs) {
            let _ = host_trace_event(bytes.as_ptr(), bytes.len() as i32);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = name;
        let _ = attrs;
    }
}

#[cfg(test)]
mod tests {
    use super::{encode_end_payload, encode_payload};

    #[test]
    fn encodes_trace_payload_with_attrs() {
        let payload = encode_payload("vzglyd_update", &[("kind", "slide")]).expect("payload");
        let json = String::from_utf8(payload).expect("utf8");
        assert!(json.contains("\"name\":\"vzglyd_update\""));
        assert!(json.contains("\"kind\":\"slide\""));
    }

    #[test]
    fn encodes_trace_end_payload_with_status() {
        let payload = encode_end_payload(Some("ok"), &[("changed", "true")]).expect("payload");
        let json = String::from_utf8(payload).expect("utf8");
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"changed\":\"true\""));
    }
}
