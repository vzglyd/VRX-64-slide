# `VRX-64-slide`

`VRX-64-slide` is the ABI contract crate for [VZGLYD](https://github.com/vzglyd/vzglyd), a Raspberry Pi display engine for ambient slides compiled to WebAssembly.

Add it to your slide crate:

```toml
[dependencies]
vzglyd_slide = { package = "VRX-64-slide", path = "../VRX-64-slide" }
```

Slides export `vzglyd_update` so the engine can step the slide every frame:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn vzglyd_update(_dt: f32) -> i32 {
    0
}
```

## Tracing

The slide ABI now includes additive trace helpers for guest code:

```rust
use vzglyd_slide::{trace_event, trace_scope};

let mut scope = trace_scope("vzglyd_update");
trace_event("channel_poll");
scope.set_status("ok");
```

These helpers compile to no-ops on non-wasm targets and use optional host imports on wasm, so older hosts keep working.

Slides do not own a browser player shell. Web playback and profiling live in `VRX-64-web`; slide repos should only ship bundles and optional preview helpers that redirect into the canonical viewer.

Further reading:

- [ABI policy](./ABI_POLICY.md)
- [Slide authoring guide](https://github.com/vzglyd/vzglyd/blob/main/docs/authoring-guide.md)
