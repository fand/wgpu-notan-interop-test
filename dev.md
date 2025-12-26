# wgpu-notan-test 開発ログ

## 目的

wgpuとnotanで同じWebGL2コンテキストを共有して動作させるテスト。

## 実装の流れ

### 1. notan cratesのコピー

```bash
git clone --depth 1 https://github.com/Nazariglez/notan.git /tmp/notan
cp -r /tmp/notan/crates/* ./crates/
```

### 2. notan修正: 外部WebGL2コンテキスト対応

**crates/notan_glow/src/lib.rs**
```rust
#[cfg(target_arch = "wasm32")]
pub fn from_webgl2_context(
    webgl2_ctx: web_sys::WebGl2RenderingContext,
) -> Result<Self, String> {
    let gl = glow::Context::from_webgl2_context(webgl2_ctx);
    Self::from(gl, "webgl2")
}
```

**crates/notan_web/src/backend.rs**
```rust
pub fn with_webgl2_context(
    webgl2_ctx: web_sys::WebGl2RenderingContext,
) -> Result<Self, String>
```

### 3. メインアプリ (crates/main)

wgpuとnotanの両方で描画:
1. wgpuがcanvasからsurfaceを作成（内部でWebGL2コンテキスト作成）
2. 同じcanvasから`getContext("webgl2")`で同一コンテキストを取得
3. そのコンテキストをnotanに渡す
4. drawループ内で:
   - notan: カスタムシェーダーで背景描画
   - wgpu: `LoadOp::Load`で背景保持しつつ赤い矩形描画

### 4. ビルド

```bash
CMAKE_POLICY_VERSION_MINIMUM=3.5 trunk build
trunk serve
```

## 解決した問題

- **Canvas重複**: `WindowConfig::default().set_app_id("notan")`で既存canvasを使用
- **wgpu limits**: `Limits::downlevel_webgl2_defaults()`でWebGL2制限に対応
- **Viewport問題**: DPIスケーリングを考慮してcanvasサイズを設定
- **シェーダーコンパイル**: `notan_macro`に`glsl-to-spirv` featureを有効化

## 構成

```
crates/
├── main/           # テストアプリ
│   ├── src/
│   │   ├── lib.rs      # wgpu + notan統合
│   │   └── shader.wgsl # wgpu用シェーダー
│   └── Cargo.toml
├── notan_glow/     # 修正: from_webgl2_context追加
├── notan_web/      # 修正: with_webgl2_context追加
└── ...             # その他notanクレート
```

## 依存関係（主要）

- wgpu 23 (webgl feature)
- notan crates (ローカル)
- glsl-to-spirv (シェーダーコンパイル)
