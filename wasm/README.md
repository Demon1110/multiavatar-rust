# multiavatar · WASM（浏览器端）

Rust 版 multiavatar 的浏览器端包。输出 SVG / PNG / JPEG / GIF / favicon，与原生库**逐字节一致**（同一套 Rust 渲染管线）。

## 目录结构

```
wasm/
├── package.json        # npm 包清单（发布内容仅 dist/；build 指向根级 scripts/build-wasm.ps1）
├── README.md
├── smoke.mjs           # Node 冒烟测试
├── src/                # 前端源码（本地 demo 页）
│   ├── demo.html
│   └── demo.js
└── dist/               # 构建产物（发布目录，不进 git）
    ├── multiavatar_bg.wasm    # ≈594 KB（gzip 后 ≈232 KB）
    ├── multiavatar.js         # JS 胶水（ESM）
    └── multiavatar.d.ts       # TS 类型

scripts/                # 根级脚本（与 wasm/ 同级）
└── build-wasm.ps1      # 打包 wasm：cargo build --target wasm32-unknown-unknown + wasm-bindgen → wasm/dist
```

## 构建

```bash
npm run build        # 等价于 powershell -File ../scripts/build-wasm.ps1
```

## 使用

```bash
npm i multiavatar
```

```js
import init, { toSvg, toPng, toJpg, toGif, toDataUri, toFaviconZip, toFaviconIco } from "multiavatar";

await init();                                    // 默认按 dist/multiavatar_bg.wasm 相对路径加载
// 或显式传入字节：await init({ module_or_path: bytes })

const svg = toSvg("demon1110");                  // string
const png = toPng("demon1110", 256);             // Uint8Array → Blob / URL.createObjectURL
```

## API

| 函数 | 参数 | 返回 |
| --- | --- | --- |
| `toSvg` | `(input: string)` | `string` |
| `toPng` | `(input: string, size: number)` | `Uint8Array` |
| `toJpg` | `(input: string, size: number)` | `Uint8Array` |
| `toGif` | `(input: string, size: number)` | `Uint8Array` |
| `toDataUri` | `(input: string, size: number)` | `string` |
| `toFaviconZip` | `(input: string)` | `Uint8Array` |
| `toFaviconIco` | `(input: string)` | `Uint8Array` |

## 本地 demo

```bash
npm run demo    # http://localhost:8000/demo.html
```

## npm 发布

```bash
npm login
cd wasm && npm publish
```
