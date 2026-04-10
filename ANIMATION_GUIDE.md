# GLB Animation Guide for Slide Authors

This guide explains how to include animations from Blender (glTF 2.0 / GLB files) in your VZGLYD slides.

## Overview

VZGLYD supports animated 3D scenes where mesh transforms (translation, rotation, scale) are driven by keyframe data embedded in GLB files. Animations are authored in Blender, exported as glTF 2.0, and automatically imported by the VZGLYD engine at slide compile time.

## Prerequisites

- Blender 3.0+ with the built-in glTF 2.0 exporter
- Your slide targets ABI version 3 or later
- `VRX-64-slide = "0.3"` in your `Cargo.toml`

## Authoring Animations in Blender

### 1. Animate Your Scene

Create animations using standard Blender transform keyframes:

- **Location** → maps to `AnimationPath::Translation`
- **Rotation** → maps to `AnimationPath::Rotation` (exported as quaternions)
- **Scale** → maps to `AnimationPath::Scale`

Each animated object in your scene becomes a **channel** in the exported animation.

### 2. Name Your Objects

The object names in Blender become the `node_label` values used at runtime. These must match the mesh labels in your scene if you want to reference them programmatically.

Example:
- Object named `spinner` → `node_label: "spinner"`
- Object named `balloon` → `node_label: "balloon"`

### 3. Set Up Actions

Blender "Actions" become `AnimationClip` instances:

- Each Action exports as one clip
- The Action name becomes the clip's `name` field
- The Action's frame range determines the clip's `duration` (in seconds, at 24 fps by default)

### 4. Looping

All exported clips default to `looped: true`. If you need non-looping animations, you can control playback at runtime by clamping the elapsed time manually.

## Exporting from Blender

1. Select **File → Export → glTF 2.0 (.glb/.gltf)**
2. Use these settings:
   - **Format**: GLB Embedded (`.glb`)
   - **Include**: Selected Objects (or Scene, depending on your setup)
   - **Animation**: ✓ Enabled
     - ✓ Animation
     - ✓ Keyframes (baked animation)
     - **Frame Range**: Manual (set your start/end frames)
3. Click **Export GLTF**

Place the exported `.glb` file in your slide's `assets/` directory.

### 4. Export Settings for Correct Interpolation

**Important:** VZGLYD only supports **LINEAR** keyframe interpolation. Blender animations
that use bezier-curve handles or the Graph Editor will export with **CUBICSPLINE** samplers
by default, which the engine cannot evaluate correctly (tangent data is read as flat values
and interpolated linearly, producing distorted motion).

To guarantee LINEAR output, enable **Baked Animation** in the glTF export dialog:

- **Animation → ✓ Baked Animation** — forces all curves to LINEAR keyframes

Without this option, if the engine detects a non-LINEAR sampler in your GLB it will log
a warning similar to:

```
animation 'MyAction': node '3' uses CubicSpline interpolation; only LINEAR is supported
— re-export with baked keyframes to avoid incorrect animation curves
```

If you see this warning, re-export with Baked Animation enabled.

## Including Animations in Your Slide

### manifest.json

Add your GLB file to the `scenes` array in `manifest.json`:

```json
{
  "abi_version": 3,
  "slide_module": "my_slide.wasm",
  "assets": {
    "scenes": [
      {
        "path": "assets/animated_scene.glb"
      }
    ]
  }
}
```

### SlideSpec

The `SlideSpec` automatically includes an `animations` field populated during scene compilation. You don't need to manually construct animation data — the engine extracts it from the GLB.

```rust
// The spec is auto-populated by compile_scene_animations()
// animations: Vec<AnimationClip> appears after sounds in the spec
```

## Animation Data Structure

### AnimationClip

A clip groups multiple animation channels that share a timeline:

```rust
pub struct AnimationClip {
    /// Human-readable name (from Blender Action name)
    pub name: String,
    /// Duration in seconds
    pub duration: f32,
    /// Whether the clip repeats
    pub looped: bool,
    /// Per-node animation channels
    pub channels: Vec<AnimationChannel>,
}
```

### AnimationChannel

Each channel animates one transform property on one scene node:

```rust
pub struct AnimationChannel {
    /// Name of the mesh/node to animate
    pub node_label: String,
    /// Which property: Translation, Rotation, or Scale
    pub path: AnimationPath,
    /// Keyframe timestamps in seconds
    pub keyframe_times: Vec<f32>,
    /// Keyframe values. For translation/scale: [x, y, z, 0].
    /// For rotation: quaternion [x, y, z, w].
    pub keyframe_values: Vec<[f32; 4]>,
}
```

### AnimationPath

```rust
pub enum AnimationPath {
    Translation,  // XYZ position
    Rotation,     // Quaternion (XYZW)
    Scale,        // XYZ scale factors
}
```

## Runtime Playback

### Native Host (Rust/wgpu)

The `WorldSlideRenderer` automatically advances animation time each frame. You can query the elapsed time via `animation_elapsed()`:

```rust
let renderer = WorldSlideRenderer::new(&gpu_context, spec)?;
let elapsed = renderer.animation_elapsed(); // seconds
```

Animation matrices are sampled using `sample_animation_matrices()`, which interpolates keyframes at the current elapsed time:

- **Translation/Scale**: Linear interpolation (lerp)
- **Rotation**: Spherical linear interpolation (slerp) for smooth quaternion blending

### Web Host (JavaScript/WebGPU)

The JS renderer advances `_animationElapsed` each frame and samples animation matrices in `_renderDrawList()`. Animation matrices are pushed to the GPU as push constants before each draw call.

## Shader Contract

World3D shaders receive the model matrix via push constants:

```wgsl
struct VzglydPushConstants {
    model_matrix: mat4x4<f32>,
};

@group(0) @binding(0) var<push_constant> vzglyd_push: VzglydPushConstants;

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    let model_pos = vzglyd_push.model_matrix * vec4<f32>(position, 1.0);
    // ... rest of vertex shader
}
```

For **unanimated meshes**, the model matrix is the identity matrix, so there is zero overhead.

## Example: Spinning Cube

Here's what a simple spinning cube looks like in the exported GLB:

```
AnimationClip {
    name: "SpinAction",
    duration: 2.0,
    looped: true,
    channels: [
        AnimationChannel {
            node_label: "Cube",
            path: Rotation,
            keyframe_times: [0.0, 2.0],
            keyframe_values: [
                [0.0, 0.0, 0.0, 1.0],  // Identity quaternion (0°)
                [0.0, 1.0, 0.0, 0.0],  // 180° around Y axis
            ],
        },
    ],
}
```

At runtime:
- `t = 0.0` → cube at 0° rotation
- `t = 1.0` → cube at 90° rotation (slerp midpoint)
- `t = 2.0` → cube at 180° rotation, then loops back to 0°

## Multi-Channel Animations

You can animate multiple properties on multiple objects simultaneously:

```
AnimationClip {
    name: "SceneAnimation",
    duration: 3.0,
    looped: true,
    channels: [
        // Cube moves up and down
        AnimationChannel {
            node_label: "Cube",
            path: Translation,
            keyframe_times: [0.0, 1.5, 3.0],
            keyframe_values: [
                [0.0, 0.0, 0.0, 0.0],
                [0.0, 5.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
            ],
        },
        // Sphere scales up
        AnimationChannel {
            node_label: "Sphere",
            path: Scale,
            keyframe_times: [0.0, 3.0],
            keyframe_values: [
                [1.0, 1.0, 1.0, 0.0],
                [3.0, 3.0, 3.0, 0.0],
            ],
        },
    ],
}
```

## Limitations

- **Morph target weights** are parsed from GLB but not yet rendered at runtime
- **Multiple animation clips** exported from a single GLB are all composited simultaneously at runtime. There is no API to play only a subset of clips. If you need mutually exclusive animations, export them into separate GLB files.
- Keyframe interpolation is **linear** (no easing curves)
- Animation data is **baked** at compile time — no runtime animation editing
- Only **LINEAR** sampler interpolation is supported. GLB files that use **STEP** or **CUBICSPLINE** interpolation (e.g., Blender bezier-curve animations) will produce incorrect results — the engine logs a warning but does not error. See [Blender export notes](#4-export-settings-for-correct-interpolation) below.

## Troubleshooting

### Animation doesn't play

- Check that your GLB file is listed in `manifest.json` under `assets.scenes`
- Verify `abi_version` is `3` or higher in `manifest.json`
- Ensure the GLB contains animation data (open in a glTF viewer to verify)

### Mesh doesn't move

- The `node_label` in the animation channel must exactly match the mesh label in the scene
- Check that the mesh is a **static mesh** in the `SlideSpec` (dynamic meshes use a different path)
- Verify the keyframe times and values are correct (use a glTF inspector tool)

### Animation looks wrong

- Rotation quaternions must be normalized (Blender does this automatically)
- Scale factors should be positive (negative scales can cause mirroring issues)
- Check the animation duration matches the keyframe time range

## References

- [glTF 2.0 Specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)
- [Blender glTF Export Documentation](https://docs.blender.org/manual/en/latest/addons/import_export/scene_gltf2.html)
- [ABI_POLICY.md](ABI_POLICY.md) — ABI version 3 details
- [Quaternion Slerp](https://en.wikipedia.org/wiki/Slerp) — smooth rotation interpolation
