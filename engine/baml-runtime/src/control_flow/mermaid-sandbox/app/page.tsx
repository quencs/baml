import path from "node:path";
import fs from "node:fs/promises";
import { spawnSync } from "node:child_process";

import HeaderFileWatcher from "./components/HeaderFileWatcher";
import MermaidDiagram from "./components/MermaidDiagram";

type ExampleRow = {
  baseName: string;
  baml?: {
    fileName: string;
    contents: string;
    isJson: boolean;
  };
  graph?: {
    fileName: string;
    json: unknown;
  };
  mermaid?: {
    fileName: string;
    contents: string;
  };
  error?: string;
};

const SANDBOX_ROOT = process.cwd();
const CONTROL_FLOW_DIR = path.resolve(SANDBOX_ROOT, "..");
const SNAPSHOT_DIR = path.join(CONTROL_FLOW_DIR, "snapshots");
const TESTDATA_ROOT = path.join(SANDBOX_ROOT, "app/testdata");
const BAML_DIR = path.join(TESTDATA_ROOT, "baml");
const GRAPH_DIR = path.join(TESTDATA_ROOT, "graph");
const MERMAID_DIR = path.join(TESTDATA_ROOT, "mermaid");

function runDataGenerator() {
  const generatorPath = path.join(SANDBOX_ROOT, "scripts/generate-data.mjs");
  try {
    const result = spawnSync("node", [generatorPath], {
      stdio: "inherit",
      cwd: SANDBOX_ROOT,
    });

    if (result.error) {
      console.error("Failed to run data generator", result.error);
    }
  } catch (error) {
    console.error("Error spawning data generator", error);
  }
}

async function loadExamples(): Promise<ExampleRow[]> {
  runDataGenerator();

  await Promise.all([
    fs.mkdir(BAML_DIR, { recursive: true }),
    fs.mkdir(GRAPH_DIR, { recursive: true }),
    fs.mkdir(MERMAID_DIR, { recursive: true }),
  ]);

  const bamlFiles = new Map<string, { fileName: string; contents: string; isJson: boolean }>();
  const bamlEntries = await fs.readdir(BAML_DIR).catch(() => []);
  for (const file of bamlEntries) {
    if (!file.endsWith(".baml") && !file.endsWith(".json")) {
      continue;
    }
    const base = file.replace(/\.(baml|json)$/, "");
    const contents = await fs.readFile(path.join(BAML_DIR, file), "utf8");
    bamlFiles.set(base, {
      fileName: file,
      contents,
      isJson: file.endsWith(".json"),
    });
  }

  const graphFiles = new Map<string, { fileName: string; json: unknown }>();
  const graphEntries = await fs.readdir(GRAPH_DIR).catch(() => []);
  for (const file of graphEntries) {
    if (!file.endsWith(".json")) {
      continue;
    }
    const base = file.replace(/\.json$/, "");
    const contents = await fs.readFile(path.join(GRAPH_DIR, file), "utf8");
    try {
      graphFiles.set(base, {
        fileName: file,
        json: JSON.parse(contents),
      });
    } catch (error) {
      graphFiles.set(base, {
        fileName: file,
        json: { __error: `Failed to parse graph JSON: ${error}` },
      });
    }
  }

  const mermaidFiles = new Map<string, { fileName: string; contents: string }>();
  const mermaidEntries = await fs.readdir(MERMAID_DIR).catch(() => []);
  for (const file of mermaidEntries) {
    if (!file.endsWith(".mmd")) {
      continue;
    }
    const base = file.replace(/\.mmd$/, "");
    const contents = await fs.readFile(path.join(MERMAID_DIR, file), "utf8");
    mermaidFiles.set(base, {
      fileName: file,
      contents,
    });
  }

  const allBases = new Set<string>();
  for (const key of bamlFiles.keys()) {
    allBases.add(key);
  }
  for (const key of graphFiles.keys()) {
    allBases.add(key);
  }
  for (const key of mermaidFiles.keys()) {
    allBases.add(key);
  }

  const cases: ExampleRow[] = Array.from(allBases)
    .sort((a, b) => a.localeCompare(b))
    .map((baseName) => ({ baseName }));

  for (const testCase of cases) {
    const baml = bamlFiles.get(testCase.baseName);
    if (baml) {
      testCase.baml = baml;
    }

    const graph = graphFiles.get(testCase.baseName);
    if (graph) {
      testCase.graph = graph;
      if (
        graph.json &&
        typeof graph.json === "object" &&
        graph.json !== null &&
        "__error" in graph.json &&
        typeof (graph.json as { __error?: unknown }).__error === "string"
      ) {
        testCase.error = (graph.json as { __error: string }).__error;
      }
    }

    const mermaid = mermaidFiles.get(testCase.baseName);
    if (mermaid) {
      testCase.mermaid = mermaid;
    }
  }

  return cases;
}

function formatBaml({ contents, isJson }: { contents: string; isJson: boolean }) {
  if (!isJson) {
    return contents;
  }
  try {
    return JSON.stringify(JSON.parse(contents), null, 2);
  } catch {
    return contents;
  }
}

export default async function Home() {
  const examples = await loadExamples();

  return (
    <main className="min-h-screen bg-white text-black px-6 py-10">
      <HeaderFileWatcher />
      <h1 className="text-3xl font-semibold mb-6">Mermaid Headers Playground</h1>

      {examples.length === 0 ? (
        <p className="text-gray-600">No Mermaid files found in the headers directory.</p>
      ) : (
        <div className="space-y-10">
          {examples.map((example) => (
            <article key={example.baseName} className="border border-gray-200 rounded-md shadow-sm overflow-hidden">
              <header className="border-b border-gray-200 bg-gray-50 px-4 py-3">
                <h2 className="text-xl font-medium">{example.baseName}</h2>
                <p className="mt-1 text-sm text-gray-600">
                  {example.baml?.fileName ?? "(no source)"}
                </p>
              </header>
              <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)_minmax(0,1fr)] p-4">
                <section className="space-y-2">
                  <h3 className="font-semibold text-sm text-gray-700 uppercase tracking-wide">BAML Source</h3>
                  <pre className="overflow-auto rounded-md border border-gray-200 bg-white p-4 text-sm text-gray-800">
                    <code>
                      {example.baml
                        ? formatBaml(example.baml)
                        : "// No matching .baml file found for this diagram."}
                    </code>
                  </pre>
                </section>

                <section className="space-y-4">
                  <h3 className="font-semibold text-sm text-gray-700 uppercase tracking-wide">Mermaid Diagram</h3>
                  {example.error ? (
                    <p className="text-sm text-red-600">{example.error}</p>
                  ) : example.mermaid ? (
                    <figure className="border border-gray-200 rounded-md overflow-hidden">
                      <figcaption className="px-3 py-2 text-sm font-medium bg-gray-50 border-b border-gray-200">
                        {example.mermaid.fileName}
                      </figcaption>
                      <div className="p-4 bg-white">
                        <MermaidDiagram chart={example.mermaid.contents} className="w-full overflow-auto" />
                      </div>
                    </figure>
                  ) : (
                    <p className="text-sm text-gray-600">No mermaid diagram available.</p>
                  )}
                </section>

                <section className="space-y-2">
                  <h3 className="font-semibold text-sm text-gray-700 uppercase tracking-wide">Graph Snapshot</h3>
                  <pre className="overflow-auto rounded-md border border-gray-200 bg-white p-4 text-sm text-gray-800">
                    <code>
                      {example.graph
                        ? JSON.stringify(example.graph.json, null, 2)
                        : "// No graph snapshot available."}
                    </code>
                  </pre>
                </section>
              </div>
            </article>
          ))}
        </div>
      )}
    </main>
  );
}
