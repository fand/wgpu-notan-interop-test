# notan-wgpu間テクスチャ共有

## 問題

notanで作成したテクスチャをwgpuで処理しようとすると、slotmapアサーションエラーが発生:

```
panicked at slotmap-1.1.1/src/basic.rs:683:9:
assertion failed: self.contains_key(key)
```

原因: notanとwgpuが別々のglow::Contextを持っており、それぞれ独自のslotmapでテクスチャを管理。notanのTextureKey(slotmapキー)をwgpuに渡しても、wgpuのslotmapには存在しない。

## 解決策

raw WebGlTextureを直接共有する方式に変更。

## 修正箇所

### 1. glow (web_sys.rs) - raw texture API追加

```rust
impl Context {
    /// Get raw WebGlTexture from a texture key.
    pub fn get_raw_texture(&self, texture: WebTextureKey) -> Option<WebGlTexture> {
        self.textures.borrow().get(texture).cloned()
    }

    /// Register an external WebGlTexture and return a texture key.
    pub fn texture_from_raw(&self, raw: WebGlTexture) -> WebTextureKey {
        self.textures.borrow_mut().insert(raw)
    }
}
```

### 2. notan_glow (lib.rs) - raw texture取得メソッド追加

```rust
impl GlowBackend {
    #[cfg(target_arch = "wasm32")]
    pub fn get_raw_texture(&self, texture_id: u64) -> Option<web_sys::WebGlTexture> {
        let texture_key = self.textures.get(&texture_id)?.texture;
        self.gl.get_raw_texture(texture_key)
    }
}
```

### 3. wgpu-hal (gles/mod.rs) - from_raw_webgl追加

```rust
impl Texture {
    #[cfg(webgl)]
    pub unsafe fn from_raw_webgl(
        gl: &glow::Context,
        raw_webgl: web_sys::WebGlTexture,
        desc: &crate::TextureDescriptor,
        format_desc: TextureFormatDesc,
    ) -> Self {
        let raw = gl.texture_from_raw(raw_webgl);
        Self::from_raw(raw, desc, format_desc)
    }
}
```

### 4. wgpu-hal (gles/device.rs) - glow context公開

```rust
impl Device {
    pub fn glow_context(&self) -> &glow::Context {
        self.shared.context.lock()
    }
}
```

### 5. wgpu.rs - WebGlTextureを使用

```rust
pub fn invert(
    &self,
    input_raw: web_sys::WebGlTexture,
    output_raw: web_sys::WebGlTexture,
    width: u32,
    height: u32,
) {
    let input_texture = self.wrap_raw_texture(input_raw, width, height, false);
    // ...
}

fn wrap_raw_texture(&self, raw_texture: web_sys::WebGlTexture, ...) -> wgpu::Texture {
    unsafe {
        self.device.as_hal::<hal::api::Gles, _, _>(|hal_device| {
            let hal_device = hal_device.expect("Failed to get hal device");
            let gl = hal_device.glow_context();
            let hal_texture = hal::gles::Texture::from_raw_webgl(gl, raw_texture, &hal_desc, format_desc);
            self.device.create_texture_from_hal::<hal::api::Gles>(hal_texture, &desc)
        }).expect("Failed to access hal device")
    }
}
```

### 6. lib.rs - get_raw_texture使用

```rust
let input_raw = backend
    .get_raw_texture(state.texture1.texture().id())
    .expect("Failed to get texture1 raw handle");
```

## フロー図

```
notan texture ID
    ↓
GlowBackend::get_raw_texture()
    ↓
glow::Context::get_raw_texture(TextureKey) → WebGlTexture
    ↓
WgpuProcessor::invert(WebGlTexture, ...)
    ↓
device.as_hal() → hal::gles::Device
    ↓
hal_device.glow_context() → &glow::Context (wgpu's)
    ↓
glow::Context::texture_from_raw(WebGlTexture) → glow::Texture (wgpu's key)
    ↓
hal::gles::Texture::from_raw()
    ↓
device.create_texture_from_hal() → wgpu::Texture
```

## 重要ポイント

- 同じWebGlTextureを両方のglow contextが参照
- 各contextは自分のslotmapにそのWebGlTextureを登録
- slotmapキーは異なるが、実際のGPUリソースは同一
