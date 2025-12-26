# Notan ↔ wgpu テクスチャ共有実装プラン (wgpu-hal patch)

## 概要
Notanで描画したテクスチャをwgpuでRGB反転処理し、結果をNotanで表示する。
wgpu-halをパッチしてglow::Textureハンドルを公開し、CPUコピーなしでテクスチャ共有。

## アーキテクチャ
```
ferris.png → [Notan] texture1 → [wgpu] invert RGB → texture2 → [Notan] display
                 ↓                    ↑                 ↓
            glow::Texture ──────────────────────→ glow::Texture
                      (wgpu-hal patch で共有)
```

---

## 実装ステップ

### 1. wgpuをローカルにclone/patch
```bash
git clone https://github.com/gfx-rs/wgpu.git crates/wgpu-patched
cd crates/wgpu-patched
git checkout v23.0.0  # 使用中のバージョン
```

### 2. wgpu-hal/src/gles/mod.rs にテクスチャハンドル公開

```rust
// wgpu-hal/src/gles/mod.rs の Texture struct に追加
impl Texture {
    /// Get raw glow texture handle
    pub fn raw(&self) -> glow::Texture {
        self.inner.raw
    }
}

impl Device {
    /// Create wgpu texture from existing glow texture (external ownership)
    pub unsafe fn texture_from_raw(
        &self,
        raw: glow::Texture,
        desc: &wgpu_types::TextureDescriptor<()>,
        format_desc: super::TextureFormatDesc,
    ) -> Texture {
        Texture {
            inner: TextureInner::Texture {
                raw,
                target: glow::TEXTURE_2D,
            },
            // ... fill other fields from desc
        }
    }
}
```

### 3. Cargo.toml でパッチ設定
```toml
[patch.crates-io]
wgpu = { path = "crates/wgpu-patched/wgpu" }
wgpu-hal = { path = "crates/wgpu-patched/wgpu-hal" }
wgpu-core = { path = "crates/wgpu-patched/wgpu-core" }
wgpu-types = { path = "crates/wgpu-patched/wgpu-types" }
```

### 4. notan_glow にテクスチャアクセサ追加
**ファイル:** `crates/notan_glow/src/lib.rs`

```rust
pub use texture::TextureKey;

impl GlowBackend {
    pub fn get_texture_handle(&self, texture_id: u64) -> Option<TextureKey> {
        self.textures.get(&texture_id).map(|t| t.texture)
    }

    pub fn register_external_texture(&mut self, handle: TextureKey, info: &TextureInfo) -> u64 {
        // ...
    }
}
```

### 5. wgpu処理モジュール更新
**ファイル:** `crates/main/src/wgpu.rs`

```rust
pub struct WgpuProcessor {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    invert_pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl WgpuProcessor {
    /// Process input texture with RGB inversion, output to target texture
    pub fn invert(
        &self,
        input_handle: glow::Texture,
        output_handle: glow::Texture,
        width: u32,
        height: u32,
    ) {
        // 1. Wrap glow handles as wgpu textures
        // 2. Create bind group with input texture
        // 3. Render to output texture with invert shader
    }
}
```

### 6. RGB反転シェーダー (WGSL)
**ファイル:** `crates/main/src/invert.wgsl`

```wgsl
@group(0) @binding(0) var t_input: texture_2d<f32>;
@group(0) @binding(1) var s_input: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@location(0) pos: vec2<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = pos * 0.5 + 0.5;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_input, s_input, in.uv);
    return vec4<f32>(1.0 - color.rgb, color.a);
}
```

### 7. メイン描画フロー
**ファイル:** `crates/main/src/lib.rs`

```rust
struct State {
    ferris: Texture,
    texture1: RenderTexture,
    texture2: RenderTexture,
    wgpu_processor: WgpuProcessor,
    // notan描画用pipeline
}

fn draw(gfx: &mut Graphics, state: &mut State) {
    // 1. Notan: ferrisをtexture1に描画
    gfx.render_to(&state.texture1, &renderer);

    // 2. wgpu: texture1を反転してtexture2へ
    let backend = get_glow_backend(gfx);
    let input = backend.get_texture_handle(state.texture1.texture().id()).unwrap();
    let output = backend.get_texture_handle(state.texture2.texture().id()).unwrap();
    state.wgpu_processor.invert(input, output, W, H);

    // 3. Notan: texture2を画面に描画
    gfx.render(&renderer);
}
```

---

## 修正ファイル一覧

1. `crates/wgpu-patched/` - wgpuローカルパッチ（新規）
2. `Cargo.toml` - [patch.crates-io] 追加
3. `crates/notan_glow/src/lib.rs` - テクスチャアクセサ追加
4. `crates/notan_glow/src/texture.rs` - TextureKey公開
5. `crates/main/src/lib.rs` - メイン描画ロジック
6. `crates/main/src/wgpu.rs` - WgpuProcessor実装
7. `crates/main/src/invert.wgsl` - 反転シェーダー

---

## 注意点
- wgpu v23.0.0 に対するパッチ
- wgpuバージョン更新時はパッチの再適用が必要
- glow::Textureのライフタイム管理に注意（Notanが所有権を持つ）
