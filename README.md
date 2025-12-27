# wgpu-notan-test

Demo of wgpu + notan interop in WebAssembly. Shares WebGL2 textures between notan (high-level 2D framework) and wgpu (low-level GPU API).

## Demo

https://fand.github.io/wgpu-notan-test/

## How It Works

The rendering pipeline:

1. **Notan**: Blit ferris.png to RenderTexture 1
2. **wgpu**: Apply hue rotation shader, output to RenderTexture 2
3. **Notan**: Apply vertex animation, render to canvas

Both libraries share the same WebGL2 context. wgpu wraps notan's raw WebGL textures using `hal::gles::Texture::from_raw_webgl()`.

## Requirements

- Rust (stable)
- [Trunk](https://trunkrs.dev/)

## Development

```bash
# Dev server with hot reload
trunk serve

# Production build
trunk build --release
```

## Project Structure

```
crates/
  main/           # Main app (lib.rs, wgpu.rs, notan_pipeline.rs)
  notan-patched/  # Patched notan (exposes raw texture handles)
  wgpu-patched/   # Patched wgpu (exposes hal::gles for texture wrapping)
  glow-patched/   # Patched glow
```

## Key Patches

- **notan**: `GlowBackend::get_raw_texture()` - exposes WebGlTexture handles
- **glow**: `get_raw_texture()` / `texture_from_raw()` - bidirectional conversion between glow texture keys and raw WebGlTexture
- **wgpu**: `Texture::from_raw_webgl()` - wraps external WebGL textures

## License

MIT OR Apache-2.0
