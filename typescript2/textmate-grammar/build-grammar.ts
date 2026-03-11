#!/usr/bin/env -S npx tsx
import fs from "fs";
import path from "path";
import yaml from "js-yaml";

const __dirname = path.dirname(new URL(import.meta.url).pathname);

const src = path.resolve(__dirname, "baml.tmLanguage.yaml");
const jinjaSrc = path.resolve(__dirname, "jinja.tmLanguage.json");

const doc = yaml.load(fs.readFileSync(src, "utf8")) as Record<string, unknown>;

const variables = (doc.variables ?? {}) as Record<string, string>;
delete doc.variables;

function substitute(obj: unknown): unknown {
  if (typeof obj === "string") {
    return obj.replace(/\{\{(\w+)\}\}/g, (_, name: string) => {
      if (!(name in variables)) {
        throw new Error(`Unknown variable: {{${name}}}`);
      }
      return variables[name];
    });
  }
  if (Array.isArray(obj)) {
    return obj.map(substitute);
  }
  if (obj && typeof obj === "object") {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(obj)) {
      out[k] = substitute(v);
    }
    return out;
  }
  return obj;
}

const result = substitute(doc);
const bamlJson = JSON.stringify(result, null, 2) + "\n";

// All destinations that need the generated grammar files.
const destinations = [
  path.resolve(__dirname, "../app-vscode-ext/syntaxes"),
  path.resolve(__dirname, "../app-promptfiddle/syntaxes"),
];

for (const dir of destinations) {
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, "baml.tmLanguage.json"), bamlJson);
  fs.copyFileSync(jinjaSrc, path.join(dir, "jinja.tmLanguage.json"));
}
