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

fn trace_status_for_code(status_code: i32) -> &'static str {
    if status_code < 0 { "error" } else { "ok" }
}

/// Run a slide `vzglyd_configure` implementation inside a standard top-level trace span.
#[doc(hidden)]
pub fn traced_configure_entrypoint<F>(len: i32, configure: F) -> i32
where
    F: FnOnce(i32) -> i32,
{
    let bytes = len.max(0);
    let bytes_str = bytes.to_string();
    let mut trace = trace_scope_with_attrs("vzglyd_configure", &[("bytes", bytes_str.as_str())]);
    let status_code = configure(len);
    trace.set_status(trace_status_for_code(status_code));
    trace.add_attr("status_code", status_code.to_string());
    status_code
}

/// Run a slide `vzglyd_init` implementation inside a standard top-level trace span.
#[doc(hidden)]
pub fn traced_init_entrypoint<F>(init: F) -> i32
where
    F: FnOnce() -> i32,
{
    let mut trace = trace_scope("vzglyd_init");
    let status_code = init();
    trace.set_status(trace_status_for_code(status_code));
    trace.add_attr("status_code", status_code.to_string());
    status_code
}

/// Run a slide `vzglyd_update` implementation inside a standard top-level trace span.
#[doc(hidden)]
pub fn traced_update_entrypoint<F>(dt: f32, update: F) -> i32
where
    F: FnOnce(f32) -> i32,
{
    let dt_ms = format!("{:.3}", dt * 1000.0);
    let mut trace = trace_scope_with_attrs("vzglyd_update", &[("dt_ms", dt_ms.as_str())]);
    let status_code = update(dt);
    trace.set_status(trace_status_for_code(status_code));
    trace.add_attr("status_code", status_code.to_string());
    status_code
}

#[cfg(test)]
mod tests {
    use super::{
        encode_end_payload, encode_payload, traced_configure_entrypoint, traced_init_entrypoint,
        traced_update_entrypoint,
    };

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

    #[test]
    fn traced_configure_returns_inner_status() {
        let mut seen = None;
        let status = traced_configure_entrypoint(42, |len| {
            seen = Some(len);
            0
        });
        assert_eq!(seen, Some(42));
        assert_eq!(status, 0);
    }

    #[test]
    fn traced_init_returns_inner_status() {
        let mut called = false;
        let status = traced_init_entrypoint(|| {
            called = true;
            0
        });
        assert!(called);
        assert_eq!(status, 0);
    }

    #[test]
    fn traced_update_returns_inner_status() {
        let mut seen = None;
        let status = traced_update_entrypoint(0.25, |dt| {
            seen = Some(dt);
            1
        });
        assert_eq!(seen, Some(0.25));
        assert_eq!(status, 1);
    }
}
