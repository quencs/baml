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
  const caseName = snapshotId.replace(/^headers__/, "");

  const yamlDocs = parseAllDocuments(await fs.readFile(snapshotPath, "utf8"));
  const dataDoc = yamlDocs.at(-1)?.toJSON();

  let mermaidContents = null;
  let graphData = null;
  let errorMessage = null;

  if (dataDoc && typeof dataDoc === "object") {
    if (typeof dataDoc.__error === "string") {
      errorMessage = dataDoc.__error;
    } else {
      if (typeof dataDoc.mermaid === "string") {
        mermaidContents = dataDoc.mermaid;
      }
      if (typeof dataDoc.expr === "object" && dataDoc.expr !== null) {
        graphData = dataDoc.expr;
      }
    }
  }

  const baseName = toKebab(caseName);

  const candidateFiles = [];
  if (sourceHint) {
    candidateFiles.push(sourceHint);
  }
  candidateFiles.push(`${caseName}.baml`);

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

  if (mermaidContents) {
    await fs.writeFile(path.join(outMermaidDir, `${baseName}.mmd`), mermaidContents, "utf8");
  }

  const graphFile = path.join(outGraphDir, `${baseName}.json`);
  const graphPayload = errorMessage
    ? { __error: errorMessage }
    : { expr: graphData ?? null };
  await fs.writeFile(graphFile, JSON.stringify(graphPayload, null, 2), "utf8");
}

console.log("Generated data for", snapshotFiles.length, "snapshots");
