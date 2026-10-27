// 本地 demo：npm run demo → http://localhost:8000/demo.html
// 加载 wasm 绑定（dist 是构建产物，需先 npm run build）
import init, { toSvg, toPng, toJpg, toGif, toFaviconZip } from "../dist/multiavatar.js";

const $ = (id) => document.getElementById(id);

function blobOf(bytes, type) {
  return URL.createObjectURL(new Blob([bytes], { type }));
}

async function render(input) {
  const png = toPng(input, 256);
  $("png").src = blobOf(png, "image/png");

  const jpg = toJpg(input, 256);
  $("jpg").src = blobOf(jpg, "image/jpeg");

  const gif = toGif(input, 96);
  $("gif").src = blobOf(gif, "image/gif");

  // favicon ZIP：解出第一张 16×16 展示
  const zip = toFaviconZip(input);
  const view = new DataView(zip.buffer, zip.byteOffset, zip.byteLength);
  const nameLen = view.getUint16(26, true);
  const extraLen = view.getUint16(28, true);
  const offset = 30 + nameLen + extraLen;
  $("fav").src = blobOf(zip.slice(offset), "image/png");

  const svg = toSvg(input);
  $("svg-info").textContent = `SVG ${svg.length} 字符 · PNG ${png.length} B · JPEG ${jpg.length} B · GIF ${gif.length} B · ZIP ${zip.length} B`;
}

await init();
$("input").addEventListener("input", (e) => render(e.target.value.trim() || " "));
render($("input").value);
