import fs from "node:fs";
import init, { toSvg, toPng, toJpg, toGif, toFaviconZip, toDataUri } from "./dist/multiavatar.js";

const bytes = fs.readFileSync("./dist/multiavatar_bg.wasm");
await init({ module_or_path: bytes });

const svg = toSvg("demon1110");
console.log("toSvg len=" + svg.length, "head=" + svg.slice(0, 40));
const png = toPng("demon1110", 64);
console.log("toPng magic=" + [...png.slice(0, 8)].join(","), "bytes=" + png.length);
const jpg = toJpg("demon1110", 64);
console.log("toJpg magic=" + [...jpg.slice(0, 3)].join(","), "bytes=" + jpg.length);
const gif = toGif("demon1110", 32);
console.log("toGif magic=" + [...gif.slice(0, 6)].join(","), "bytes=" + gif.length);
const uri = toDataUri("demon1110", 32);
console.log("toDataUri head=" + uri.slice(0, 30), "len=" + uri.length);
const zip = toFaviconZip("demon1110");
console.log("toFaviconZip PK=" + [...zip.slice(0, 4)].join(","), "bytes=" + zip.length);

if (svg.length > 0 && png.length > 8 && jpg.length > 10 && gif.length > 20 && zip.length > 100 && uri.startsWith("data:image/png;base64,")) {
  console.log("SMOKE PASS");
} else {
  console.log("SMOKE FAIL");
  process.exit(1);
}
