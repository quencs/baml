#!/usr/bin/env node
import path from "node:path";
import { fileURLToPath } from "node:url";
import fs from "node:fs/promises";
import { parseAllDocuments } from "yaml";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const sandboxRoot = path.resolve(__dirname, "..");
const controlFlowDir = path.resolve(sandboxRoot, "..");
const snapshotsDir = path.join(controlFlowDir, "snapshots");
const headersDir = path.join(controlFlowDir, "testdata");

const outBamlDir = path.join(sandboxRoot, "app/testdata/baml");
const outMermaidDir = path.join(sandboxRoot, "app/testdata/mermaid");
const outGraphDir = path.join(sandboxRoot, "app/testdata/graph");

await fs.mkdir(outBamlDir, { recursive: true });
await fs.mkdir(outMermaidDir, { recursive: true });
await fs.mkdir(outGraphDir, { recursive: true });

function toKebab(name) {
  return name
    .trim()
    .replace(/[^A-Za-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .toLowerCase();
}

const snapshotFiles = (await fs.readdir(snapshotsDir))
  .filter((file) => file.startsWith("baml_runtime__control_flow__tests__headers__"))
  .sort();

// Clear output directories so stale files don't linger
for (const dir of [outBamlDir, outMermaidDir, outGraphDir]) {
  try {
    const files = await fs.readdir(dir);
    await Promise.all(files.map((file) => fs.rm(path.join(dir, file), { recursive: true, force: true })));
  } catch {
    // ignore
  }
}

for (const snapshotFile of snapshotFiles) {
  const snapshotPath = path.join(snapshotsDir, snapshotFile);
  const withoutPrefix = snapshotFile.replace(/^baml_runtime__control_flow__tests__headers__/, "");
  const withoutSuffix = withoutPrefix.replace(/\.snap(?:\.new)?$/, "");
  const [snapshotId, sourceHint] = withoutSuffix.split("@");
  const snapshotBase = snapshotId;

  const yamlDocs = parseAllDocuments(await fs.readFile(snapshotPath, "utf8"));
  const dataDoc = yamlDocs.at(-1)?.toJSON();

  const functions = [];
  const graphData = {};

  if (dataDoc && typeof dataDoc === "object") {
    for (const [key, value] of Object.entries(dataDoc)) {
      if (key.startsWith("mermaid::") && typeof value === "string") {
        const [, kind = "expr", name = "diagram"] = key.split("::");
        functions.push({
          kind,
          name,
          mermaid: value,
        });
      }

      if ((key.startsWith("expr::") || key.startsWith("llm::")) && typeof value === "object") {
        graphData[key] = value;
      }
    }
  }

  const baseName = toKebab(snapshotBase);

  const candidateFiles = [];
  if (sourceHint) {
    candidateFiles.push(sourceHint);
  }
  candidateFiles.push(`${snapshotBase}.baml`);
  candidateFiles.push(`${snapshotBase.replace(/__/g, "/")}.baml`);

  let bamlPath = null;
  for (const candidate of candidateFiles) {
    const candidatePath = path.join(headersDir, candidate);
    try {
      const stat = await fs.stat(candidatePath);
      if (stat.isFile()) {
        bamlPath = candidatePath;
        break;
      }
    } catch {
      // ignore missing
    }
  }

  if (!bamlPath) {
    const bamlContents = JSON.stringify({ warning: "No matching .baml file" }, null, 2);
    await fs.writeFile(path.join(outBamlDir, `${baseName}.json`), bamlContents);
  } else {
    const bamlContents = await fs.readFile(bamlPath, "utf8");
    await fs.writeFile(path.join(outBamlDir, `${baseName}.baml`), bamlContents, "utf8");
  }

  for (const fn of functions) {
    const mermaidFile = path.join(outMermaidDir, `${baseName}__${fn.kind}_${fn.name}.mmd`);
    await fs.writeFile(mermaidFile, fn.mermaid, "utf8");
  }

  const graphFile = path.join(outGraphDir, `${baseName}.json`);
  await fs.writeFile(graphFile, JSON.stringify(graphData, null, 2), "utf8");
}

console.log("Generated data for", snapshotFiles.length, "snapshots");
