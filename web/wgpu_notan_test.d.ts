/* tslint:disable */
/* eslint-disable */

export function start(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly start: () => void;
  readonly wasm_bindgen__convert__closures_____invoke__h4e3fbc3b35c84004: (a: number, b: number) => void;
  readonly wasm_bindgen__closure__destroy__hb52523e9981ca6dc: (a: number, b: number) => void;
  readonly wasm_bindgen__convert__closures_____invoke__ha72937c1220ce112: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen__closure__destroy__h6b749a9f96a2f48e: (a: number, b: number) => void;
  readonly wasm_bindgen__convert__closures_____invoke__h99e1a9d0e3e3c2f6: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen__closure__destroy__h6c4bda48f0e68864: (a: number, b: number) => void;
  readonly wasm_bindgen__convert__closures_____invoke__hef2f0cb811ab853b: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen__closure__destroy__ha60207fe563a180a: (a: number, b: number) => void;
  readonly wasm_bindgen__convert__closures_____invoke__hf3f0fed7cce41ac1: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen__closure__destroy__h46fb1005c14de99d: (a: number, b: number) => void;
  readonly wasm_bindgen__convert__closures_____invoke__h8ae89dc7429b427f: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen__closure__destroy__h52c7c61cd0157c3d: (a: number, b: number) => void;
  readonly wasm_bindgen__convert__closures_____invoke__h4d9e259ea4df71e9: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen__closure__destroy__h4a2246b544fc80c8: (a: number, b: number) => void;
  readonly wasm_bindgen__convert__closures_____invoke__h92cd2abcf1eab3d4: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen__closure__destroy__h746476a68f9bfae9: (a: number, b: number) => void;
  readonly wasm_bindgen__convert__closures_____invoke__h54313d437db8d071: (a: number, b: number) => number;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_externrefs: WebAssembly.Table;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
