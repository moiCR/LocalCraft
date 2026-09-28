import { readFile, writeFile, mkdir } from "node:fs/promises";
import path from "node:path";

const [platform, output] = process.argv.slice(2);
if (!platform || !output || !["windows", "linux"].includes(platform)) {
  throw new Error("Usage: node create-packager-config.mjs <windows|linux> <output.json>");
}

const cargoToml = await readFile("crates/app/Cargo.toml", "utf8");
const versionMatch = cargoToml.match(/^version\s*=\s*"([^"]+)"/m);
if (!versionMatch) {
  throw new Error("Could not read app version from crates/app/Cargo.toml.");
}

const windows = platform === "windows";
const config = {
  name: "LocalCraft",
  productName: "LocalCraft",
  version: versionMatch[1],
  identifier: "com.moiCR.LocalCraft",
  description: "A native desktop manager for local Minecraft servers.",
  homepage: "https://github.com/moiCR/LocalCraft",
  authors: ["moiCR"],
  outDir: "target/packager",
  binaries: [{ path: "../release/LocalCraft", main: true }],
  formats: windows ? ["nsis"] : ["appimage", "deb"],
  icons: windows
    ? ["target/packager/windows-assets/localcraft.ico"]
    : ["target/packager/linux-assets/localcraft.png"],
};

if (windows) {
  config.resources = [{
    src: "target/packager/windows-assets/vcruntime140.dll",
    target: "vcruntime140.dll",
  }];
  config.nsis = {
    installerIcon: "target/packager/windows-assets/localcraft.ico",
    installMode: "currentUser",
    compression: "lzma",
  };
} else {
  config.linux = { generateDesktopEntry: true };
  config.deb = {
    packageName: "localcraft",
    depends: [
      "libasound2",
      "libdbus-1-3",
      "libfontconfig1",
      "libfreetype6",
      "libglib2.0-0",
      "libvulkan1",
      "libwayland-client0",
      "libx11-6",
      "libx11-xcb1",
      "libxcb1",
      "libxcb-icccm4",
      "libxcb-image0",
      "libxcb-keysyms1",
      "libxcb-randr0",
      "libxcb-render0",
      "libxcb-shape0",
      "libxcb-xfixes0",
      "libxcb-xkb1",
      "libxcursor1",
      "libxext6",
      "libxfixes3",
      "libxi6",
      "libxinerama1",
      "libxrandr2",
      "libxrender1",
      "libxkbcommon0",
      "libxkbcommon-x11-0",
    ],
  };
}

await mkdir(path.dirname(output), { recursive: true });
await writeFile(output, `${JSON.stringify(config, null, 2)}\n`, "utf8");
