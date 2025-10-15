/*
Placeholder demo page surfaced in the nav so new experiments can start with
an empty canvas and consistent chrome. Swap the placeholder copy with your
actual demo when you are ready.
*/
'use client';

import { Background, Controls, MiniMap, ReactFlow } from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import { HeaderNav } from '../components/HeaderNav';

const nodes = [];
const edges = [];

export default function PlaceholderDemo() {
  return (
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      <HeaderNav />
      <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
        <aside
          style={{
            width: '28%',
            minWidth: 240,
            maxWidth: 400,
            borderRight: '1px solid rgba(148, 163, 184, 0.35)',
            padding: '18px',
            background: 'rgba(15, 23, 42, 0.03)',
            fontFamily: 'var(--font-geist-sans, system-ui)',
            fontSize: 14,
            lineHeight: 1.5,
          }}
        >
          <h2 style={{ margin: '0 0 12px', fontSize: 16, fontWeight: 600 }}>Placeholder Demo</h2>
          <p style={{ margin: '0 0 12px' }}>
            This route gives you a ready-to-wire React Flow canvas. Drop in nodes, update the sidebar copy,
            and add any layout logic you need.
          </p>
          <p style={{ margin: 0, color: 'rgba(15, 23, 42, 0.7)', fontSize: 13 }}>
            Remove the placeholder text and export real data when you turn this into a full example.
          </p>
        </aside>
        <div style={{ flex: 1 }}>
          <ReactFlow nodes={nodes} edges={edges} fitView minZoom={0.2}>
            <MiniMap />
            <Controls />
            <Background gap={24} size={1} color="rgba(148, 163, 184, 0.3)" />
            <div
              style={{
                position: 'absolute',
                inset: '24px',
                borderRadius: 12,
                border: '1px dashed rgba(37, 99, 235, 0.35)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                color: 'rgba(15, 23, 42, 0.6)',
                fontFamily: 'var(--font-geist-sans, system-ui)',
                fontSize: 16,
              }}
            >
              Drop your nodes here
            </div>
          </ReactFlow>
        </div>
      </div>
    </div>
  );
}
