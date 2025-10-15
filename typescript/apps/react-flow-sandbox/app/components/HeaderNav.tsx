'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';

const links = [
  { href: '/compound-parent-nodes', label: 'Compound Parent Nodes' },
  { href: '/hierarchical-elk', label: 'Hierarchical ELK' },
  { href: '/custom-graph-2', label: 'Custom Graph 2' },
  { href: '/placeholder-demo', label: 'Placeholder Demo' },
  { href: '/custom-group-node', label: 'Custom Group Node' },
  { href: '/metadata-overlay', label: 'Metadata Overlay' },
];

export function HeaderNav() {
  const pathname = usePathname();

  return (
    <header
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '12px 24px',
        borderBottom: '1px solid rgba(148, 163, 184, 0.4)',
        background: 'rgba(15, 23, 42, 0.04)',
        backdropFilter: 'blur(4px)',
      }}
    >
      <span style={{ fontSize: '18px', fontWeight: 600 }}>React Flow Subgraph Demos</span>
      <nav style={{ display: 'flex', gap: '16px' }}>
        {links.map((link) => {
          const isActive = pathname === link.href;

          return (
            <Link
              key={link.href}
              href={link.href}
              style={{
                padding: '6px 12px',
                borderRadius: '9999px',
                border: isActive ? '1px solid #2563eb' : '1px solid transparent',
                background: isActive ? 'rgba(37, 99, 235, 0.12)' : 'transparent',
                color: isActive ? '#1d4ed8' : 'inherit',
                textDecoration: 'none',
                fontWeight: isActive ? 600 : 500,
                transition: 'all 0.15s ease-in-out',
              }}
            >
              {link.label}
            </Link>
          );
        })}
      </nav>
    </header>
  );
}
