# `vzglyd-slide`

`vzglyd-slide` is the ABI contract crate for [VZGLYD](https://github.com/vzglyd/vzglyd), a Raspberry Pi display engine for ambient slides compiled to WebAssembly.

Add it to your slide crate:

```toml
[dependencies]
vzglyd-slide = "0.1"
```

Slides export `vzglyd_update` so the engine can step the slide every frame:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn vzglyd_update(_dt: f32) -> i32 {
    0
}
```

Further reading:

- [ABI policy](./ABI_POLICY.md)
- [Slide authoring guide](https://github.com/vzglyd/vzglyd/blob/main/docs/authoring-guide.md)
