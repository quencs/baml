import { useEffect, useMemo, useState } from "react";
import type { CombinedRow, SnapshotEntry, EventRecord, StateUpdate } from "./types";

function readJsonl<T>(input: string): T[] {
  return input
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => JSON.parse(line) as T);
}

const snapshotModules = import.meta.glob("../../snapshots/**/*", {
  eager: true,
  as: "raw",
});

function discoverSnapshots(): SnapshotEntry[] {
  const entries: SnapshotEntry[] = [];

  Object.entries(snapshotModules)
    .filter(([path]) => path.endsWith(".events.jsonl"))
    .forEach(([path, eventsText]) => {
      const fixture = path.split("/").pop()?.replace(".events.jsonl", "");
      if (!fixture || typeof eventsText !== "string") return;

      const stackKey = path.replace(".events.jsonl", ".stack.jsonl");
      const updatesKey = path.replace(".events.jsonl", ".updates.jsonl");

      const stackText = snapshotModules[stackKey];
      const updatesText = snapshotModules[updatesKey];
      if (typeof stackText !== "string" || typeof updatesText !== "string") return;

      entries.push({
        fixture,
        eventsText,
        stackText,
        updatesText,
      });
    });

  return entries.sort((a, b) => a.fixture.localeCompare(b.fixture));
}

async function loadSnapshot(entry: SnapshotEntry): Promise<CombinedRow[]> {
  const events = readJsonl<EventRecord>(entry.eventsText);
  const stacks = readJsonl<string[]>(entry.stackText);
  const updates = readJsonl<StateUpdate>(entry.updatesText);

  const len = Math.min(events.length, stacks.length, updates.length);
  return Array.from({ length: len }, (_, idx) => ({
    index: idx,
    event: events[idx],
    stack: stacks[idx],
    update: updates[idx],
  }));
}

function toLabel(entry: SnapshotEntry): string {
  return entry.fixture.replaceAll("_", " ");
}

export default function App() {
  const [manifest, setManifest] = useState<SnapshotEntry[]>([]);
  const [selected, setSelected] = useState<string>();
  const [rows, setRows] = useState<CombinedRow[]>([]);
  const current = useMemo(
    () => manifest.find((item) => item.fixture === selected),
    [manifest, selected],
  );

  useEffect(() => {
    const entries = discoverSnapshots();
    setManifest(entries);
    if (entries.length > 0) {
      setSelected(entries[0].fixture);
    }
  }, []);

  useEffect(() => {
    if (!current) {
      setRows([]);
      return;
    }

    loadSnapshot(current)
      .then(setRows)
      .catch((err) => {
        console.error("Failed to load snapshot", err);
        setRows([]);
      });
  }, [current]);

  return (
    <div style={styles.page}>
      <header style={styles.header}>
        <h1>Viz Runtime Snapshots</h1>
        <p>Watching {manifest.length} fixture(s)</p>
      </header>
      <section style={styles.sidebar}>
        <h2>Fixtures</h2>
        <div style={styles.list}>
          {manifest.map((entry) => (
            <button
              key={entry.fixture}
              onClick={() => setSelected(entry.fixture)}
              style={{
                ...styles.listItem,
                ...(selected === entry.fixture ? styles.listItemActive : {}),
              }}
            >
              {toLabel(entry)}
            </button>
          ))}
        </div>
      </section>
      <main style={styles.main}>
        {!current ? (
          <p>No snapshots found yet. Run the Rust snapshot tests to generate them.</p>
        ) : (
          <table style={styles.table}>
            <thead>
              <tr>
                <th style={styles.th}>#</th>
                <th style={styles.th}>Event</th>
                <th style={styles.th}>Value</th>
                <th style={styles.th}>Stack</th>
                <th style={styles.th}>State</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <tr key={row.index} style={styles.tr}>
                  <td style={styles.td}>{row.index}</td>
                  <td style={styles.td}>
                    <div>{row.event.kind}</div>
                    <div style={styles.subtext}>{row.event.function}</div>
                    {row.event.header ? (
                      <div style={styles.badge}>hdr {row.event.header.level}</div>
                    ) : null}
                    {row.event.variable ? (
                      <div style={styles.badge}>var {row.event.variable}</div>
                    ) : null}
                  </td>
                  <td style={styles.td}>
                    <pre style={styles.pre}>
                      {row.event.value ? JSON.stringify(row.event.value, null, 2) : "—"}
                    </pre>
                  </td>
                  <td style={styles.td}>
                    <pre style={styles.pre}>{JSON.stringify(row.stack, null, 2)}</pre>
                  </td>
                  <td style={styles.td}>
                    <div style={styles.badge}>{row.update.new_state}</div>
                    <div style={styles.subtext}>{row.update.lexical_id}</div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </main>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  page: {
    display: "grid",
    gridTemplateColumns: "280px 1fr",
    gridTemplateRows: "auto 1fr",
    height: "100vh",
    fontFamily: "Inter, system-ui, sans-serif",
  },
  header: {
    gridColumn: "1 / span 2",
    padding: "16px 20px",
    borderBottom: "1px solid #e6e6e6",
    background: "#fafafa",
  },
  sidebar: {
    borderRight: "1px solid #e6e6e6",
    padding: "12px",
    overflowY: "auto",
  },
  list: {
    display: "flex",
    flexDirection: "column",
    gap: "8px",
  },
  listItem: {
    textAlign: "left",
    padding: "8px 10px",
    borderRadius: "8px",
    border: "1px solid #d8d8d8",
    background: "#fff",
    cursor: "pointer",
  },
  listItemActive: {
    borderColor: "#6366f1",
    boxShadow: "0 0 0 2px rgba(99, 102, 241, 0.2)",
  },
  main: {
    overflow: "auto",
  },
  table: {
    width: "100%",
    borderCollapse: "collapse",
  },
  th: {
    textAlign: "left",
    padding: "8px 10px",
    borderBottom: "1px solid #e6e6e6",
    background: "#f7f7f7",
    position: "sticky",
    top: 0,
    zIndex: 1,
  },
  td: {
    padding: "8px 10px",
    verticalAlign: "top",
    borderBottom: "1px solid #f0f0f0",
  },
  tr: {
    background: "#fff",
  },
  pre: {
    margin: 0,
    background: "#f8f8f8",
    padding: "8px",
    borderRadius: "6px",
    fontSize: "12px",
  },
  badge: {
    display: "inline-block",
    padding: "2px 6px",
    borderRadius: "6px",
    background: "#eef2ff",
    color: "#3730a3",
    fontSize: "12px",
    marginTop: "4px",
  },
  subtext: {
    color: "#6b7280",
    fontSize: "12px",
  },
};
