#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const tokensCss = fs.readFileSync(path.join(root, "ui/styles/tokens.css"), "utf8");
const appCss = fs.readFileSync(path.join(root, "ui/styles/app.css"), "utf8");
const iconsJs = fs.readFileSync(path.join(root, "ui/scripts/icons.js"), "utf8");
const presetsJs = fs.readFileSync(path.join(root, "ui/scripts/presets.js"), "utf8");
const diffJs = fs.readFileSync(path.join(root, "ui/scripts/diff.js"), "utf8");
const appJs = fs.readFileSync(path.join(root, "ui/scripts/app.js"), "utf8");

const html = `<!DOCTYPE html>
<html lang="zh-CN" data-theme="dark">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Claude Code Switcher v2.0 — Interactive Design Prototype</title>
  <style>
${tokensCss}
${appCss}
  </style>
</head>
<body>
  <div class="app-shell" id="app-shell">
    <aside class="rail" id="rail-root" aria-label="环境配置列表"></aside>
    <main class="workspace" id="workspace-root" aria-label="配置工作台"></main>
    <footer class="statusbar" id="statusbar-root" aria-label="状态栏"></footer>
  </div>
  <div id="modal-root"></div>
  <div class="toast-stack" id="toast-stack" aria-live="polite"></div>

  <script>
${iconsJs}
${presetsJs}
${diffJs}
${appJs}
  </script>
</body>
</html>
`;

const outDir = path.join(root, "designs/claude-code-switcher");
fs.mkdirSync(outDir, { recursive: true });
const outPath = path.join(outDir, "preview.html");
fs.writeFileSync(outPath, html, "utf8");
console.log("Generated self-contained design prototype:", outPath);
