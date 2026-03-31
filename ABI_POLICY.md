# `vzglyd-slide` ABI Policy

`vzglyd-slide` defines the binary contract between the VZGLYD engine and every slide compiled for it. Once a slide is shipped as `slide.wasm`, the engine must be able to tell whether that package is safe to load and what compatibility guarantees apply. This document is that contract.

## Scope

This policy covers:

- the public Rust API exposed by `vzglyd-slide`
- the serialized `SlideSpec` wire format
- the exported guest functions the engine expects from a slide
- the `abi_version` recorded in slide manifests

It does not cover `vzglyd-sidecar`, which is versioned independently.

## Versioning Model

`vzglyd-slide` follows semantic versioning. ABI impact maps to versions as follows.

| Change | Version bump | ABI impact |
| --- | --- | --- |
| Remove, rename, or change a required exported symbol | MAJOR | Breaking |
| Change `SlideSpec` serialization or layout in a non-backward-compatible way | MAJOR | Breaking |
| Change a public type or trait in a way that breaks existing slide code | MAJOR | Breaking |
| Add new optional capabilities that preserve existing behavior | MINOR | Non-breaking |
| Clarify docs, add tests, or add helpers that do not affect compatibility | PATCH | Non-breaking |

## Engine Compatibility Window

The engine validates slide compatibility at load time using the manifest's `abi_version` and the slide module's exported `vzglyd_abi_version()` symbol.

Current ABI version: `1`

Compatibility guarantees:

- an engine release must reject slides that declare an unknown ABI version
- an engine release may support multiple ABI versions during a transition window
- the default policy is to support the current ABI version and, when a breaking ABI ships, the previous ABI version for one compatibility window
- a slide compiled against ABI version 1 remains compatible with any engine release that still accepts ABI version 1

## What Counts As Breaking

Breaking changes include, but are not limited to:

- changing the signature of `vzglyd_update(dt: f32) -> i32`
- adding a new required export that old slides do not provide
- removing or renaming any public `vzglyd-slide` type used by slides
- changing the postcard representation of `SlideSpec`
- changing trait bounds in a way that invalidates existing slide vertex types

Non-breaking changes include, but are not limited to:

- adding helper constructors or helper types
- adding optional fields that preserve existing behavior when omitted
- improving validation messages or documentation

## Dependency Guidance For Slide Authors

Until `vzglyd-slide` reaches `1.0.0`, follow Cargo's pre-1.0 convention and depend on the current minor line:

```toml
vzglyd-slide = "0.1"
```

That allows patch updates but avoids silently picking up a new pre-1.0 minor release that may contain breaking changes.

After `1.0.0`, depend on the current major:

```toml
vzglyd-slide = "1"
```

## ABI Version Signaling

Slides must keep these two values aligned:

- `vzglyd_abi_version() -> u32` exported by the slide module
- `abi_version` in `manifest.json`

The engine checks both at load time. If either value is unsupported, the package is rejected with a clear error instead of failing later during execution.

## Release Discipline

- breaking ABI changes require a major version bump
- every release must update `CHANGELOG.md`
- every release that affects compatibility must explicitly call out ABI impact in release notes
- silent ABI changes are not allowed
