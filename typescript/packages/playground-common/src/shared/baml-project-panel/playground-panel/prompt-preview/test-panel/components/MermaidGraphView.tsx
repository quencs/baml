import { useAtomValue } from 'jotai';
import { useEffect, useRef, useCallback } from 'react';
import mermaid from 'mermaid';
import { Button } from '@baml/ui/button';
import { ZoomIn, ZoomOut, Maximize2 } from 'lucide-react';
import svgPanZoom from 'svg-pan-zoom';
import { functionGraphAtom } from '../../../atoms-orch-graph';
import { vscode } from '../../../../vscode';

const MermaidHeader: React.FC = () => {
  return (
    <div className="pt-4">
      <div className="text-sm font-bold">Function Flow Diagram</div>
      <div className="flex flex-col-reverse items-start gap-0.5">
        <span className="pl-2 text-xs text-muted-foreground flex flex-row flex-wrap items-center gap-0.5">
          Mermaid diagram visualization
        </span>
      </div>
    </div>
  );
};

export const MermaidGraphView: React.FC = () => {
  const { graph } = useAtomValue(functionGraphAtom);
  const mermaidRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const svgRef = useRef<SVGSVGElement | null>(null);
  const panZoomRef = useRef<ReturnType<typeof svgPanZoom> | null>(null);

  const zoomIn = useCallback(() => {
    panZoomRef.current?.zoomBy(1.2);
  }, []);
  const zoomOut = useCallback(() => {
    panZoomRef.current?.zoomBy(1 / 1.2);
  }, []);
  const resetView = useCallback(() => {
    if (!panZoomRef.current) return;
    panZoomRef.current.resetZoom();
    panZoomRef.current.resize();
    panZoomRef.current.fit();
    panZoomRef.current.center();
  }, []);

  useEffect(() => {
    if (!graph || !mermaidRef.current) return;

    let isCancelled = false;
    let resizeObserver: ResizeObserver | null = null;

    // Raw graph string for debugging
    try {
      console.log('[MermaidGraphView] raw graph string:', graph);
    } catch {}

    const onResize = () => {
      if (!panZoomRef.current) return;
      panZoomRef.current.resize();
      panZoomRef.current.fit();
      panZoomRef.current.center();
    };
    window.addEventListener('resize', onResize);

    (async () => {
      try {
        mermaid.initialize({
          startOnLoad: false,
          theme: 'dark',
          themeCSS: '.mermaid svg { max-width: none !important; }',
          // flowchart: {
          //   nodeSpacing: 50,
          //   rankSpacing: 50,
          //   curve: 'basis',
          // },
          securityLevel: 'loose',
        });

        // Render via mermaid.render to get SVG string with id
        // Helper to trigger span navigation/flash by nodeId
        const triggerSpan = (nodeId?: string) => {
          try {
            const map = (window as any).__bamlSpanMap as Record<string, any> | undefined;
            if (!map) {
              console.warn('[MermaidGraphView] triggerSpan: no span map present');
              return;
            }
            const span = nodeId ? map[nodeId] : undefined;
            if (!span) {
              console.warn('[MermaidGraphView] triggerSpan: unknown nodeId', nodeId);
              return;
            }
            console.log('[MermaidGraphView] triggerSpan', { nodeId, span });
            vscode.postMessage?.({
              command: 'jumpToFile',
              span: {
                start: span.start,
                end: span.end,
                source_file: span.file_path,
                value: `${(span.file_path.split('/').pop() || '<file>.baml')}:${span.start_line + 1}`,
              },
            });
            window.postMessage(
              {
                command: 'set_flashing_regions',
                content: {
                  spans: [
                    {
                      file_path: span.file_path,
                      start_line: span.start_line,
                      start: span.start,
                      end_line: span.end_line,
                      end: span.end,
                    },
                  ],
                },
              },
              '*',
            );
          } catch (err) {
            console.error('[MermaidGraphView] error in triggerSpan', err);
          }
        };

        // Define the global callback expected by the generated Mermaid graph
        (window as any).bamlMermaidNodeClick = (nodeId?: string) => {
          console.log('[MermaidGraphView] global callback fired', nodeId);
          triggerSpan(nodeId);
        };

        const { svg } = await mermaid.render('bamlMermaidSvg', graph);
        if (isCancelled || !mermaidRef.current) return;

        const normalizedSvg = svg
          .replace(/[ ]*max-width:[ 0-9\.]*px;/i, '')
          .replace(/width="[0-9\.]+px"/i, '')
          .replace(/height="[0-9\.]+px"/i, '');
        mermaidRef.current.innerHTML = normalizedSvg;

        const svgEl = mermaidRef.current.querySelector('#bamlMermaidSvg') as SVGSVGElement | null;
        if (!svgEl) return;
        svgRef.current = svgEl;
        // Make the SVG fill the container
        svgEl.setAttribute('width', '100%');
        svgEl.setAttribute('height', '100%');
        svgEl.setAttribute('preserveAspectRatio', 'xMidYMid meet');
        svgEl.style.width = '100%';
        svgEl.style.height = '100%';
        svgEl.style.display = 'block';

        // Initialize svg-pan-zoom on the generated SVG
        panZoomRef.current = svgPanZoom(svgEl as unknown as SVGElement, {
          panEnabled: true,
          zoomEnabled: true,
          controlIconsEnabled: false,
          fit: true,
          center: true,
          minZoom: 0.25,
          maxZoom: 4,
          zoomScaleSensitivity: 0.3,
          preventMouseEventsDefault: false,
        });

        // After init, ensure proper fit/center
        panZoomRef.current.resize();
        panZoomRef.current.fit();
        panZoomRef.current.center();

        // Prevent wheel from scrolling parent while zooming/panning
        const wheelPreventer = (e: WheelEvent) => {
          e.preventDefault();
        };
        // Attach to the container hosting the SVG
        svgEl.addEventListener('wheel', wheelPreventer, { passive: false });

        // Parse span map from a special mermaid comment emitted at the end
        // Look for a comment node like %%__BAML_SPANMAP__={...}
        const raw = graph as string;
        const spanMapMatch = raw.match(/%%__BAML_SPANMAP__=(\{[\s\S]*\})/);
        let spanMap: Record<string, {
          file_path: string;
          start_line: number;
          start: number;
          end_line: number;
          end: number;
        }> | null = null;
        if (spanMapMatch && spanMapMatch[1]) {
          try {
            spanMap = JSON.parse(spanMapMatch[1]);
          } catch (_e) {
            spanMap = null;
          }
        }

        // Expose the span map globally for the callback to use
        if (spanMap) {
          (window as any).__bamlSpanMap = spanMap;
          console.log('[MermaidGraphView] span map ready', Object.keys(spanMap));
          // Also attach direct click handlers to avoid interference from pan/zoom
          const manualListeners: Array<{ el: Element; fn: (ev: Event) => void }> = [];
          Object.entries(spanMap).forEach(([nodeId, span]) => {
            const candidates = [
              `#${nodeId}`,
              `#flowchart-${nodeId}`,
              `g[id^="${nodeId}"]`,
              `g[id*="-${nodeId}"]`,
              `g[id^="flowchart-${nodeId}"]`,
            ];
            const target = candidates
              .map((sel) => svgEl.querySelector(sel) as SVGElement | null)
              .find((el) => !!el);
            if (!target) return;
            target.style.cursor = 'pointer';
            const onClick = (ev: Event) => {
              ev.stopPropagation();
              console.log('[MermaidGraphView] node click (manual handler)', { nodeId, span, target: (ev.target as Element)?.tagName });
              triggerSpan(nodeId);
            };
            target.addEventListener('click', onClick);
            manualListeners.push({ el: target, fn: onClick });
          });
          // Ensure all visible mermaid nodes show pointer cursor for UX/debug
          svgEl.querySelectorAll('.node').forEach((el) => {
            (el as SVGGraphicsElement).style.cursor = 'pointer';
            (el as SVGGraphicsElement).style.pointerEvents = 'all';
          });
          // Attach generic listener to any node group; derive nodeId from its id content
          const generic = (ev: Event) => {
            const el = ev.target as Element | null;
            if (!el) return;
            const g = el.closest('g[id]');
            const id = g?.getAttribute('id') || '';
            const key = Object.keys(spanMap!).find((k) => id.includes(k));
            if (key) {
              console.log('[MermaidGraphView] node click (generic)', { rawId: id, key });
              triggerSpan(key);
            }
          };
          svgEl.querySelectorAll('g.node').forEach((g) => {
            g.addEventListener('click', generic);
            manualListeners.push({ el: g, fn: generic });
          });

          // Cleanup manual listeners on unmount
          (window as any).__bamlCleanupListeners = () => {
            manualListeners.forEach(({ el, fn }) => {
              el.removeEventListener('click', fn);
            });
          };
        }

        // Observe container size changes (ResizablePanel drags)
        if (containerRef.current && typeof ResizeObserver !== 'undefined') {
          resizeObserver = new ResizeObserver(() => {
            if (!panZoomRef.current) return;
            panZoomRef.current.resize();
            panZoomRef.current.fit();
            panZoomRef.current.center();
          });
          resizeObserver.observe(containerRef.current);
        }
      } catch (_err) {
        // swallow render errors; UI may show diagnostics elsewhere
      }
    })();

    return () => {
      isCancelled = true;
      window.removeEventListener('resize', onResize);
      if (resizeObserver) resizeObserver.disconnect();
      panZoomRef.current?.destroy();
      panZoomRef.current = null;
      try {
        const svgEl = svgRef.current;
        if (svgEl) {
          svgEl.removeEventListener('wheel', (e: any) => e.preventDefault());
        }
        if ((window as any).__bamlCleanupListeners) {
          (window as any).__bamlCleanupListeners();
          delete (window as any).__bamlCleanupListeners;
        }
      } catch {}
    };
  }, [graph]);

  return (
    <div className="flex flex-col w-full h-full min-h-0">
      <MermaidHeader />
      <div
        ref={containerRef}
        className="relative flex-1 min-h-0 overflow-hidden"
        style={{ backgroundColor: '#0f172a' }}
      >
        <div
          ref={mermaidRef}
          className="absolute inset-0 overflow-hidden p-2"
        />

        {/* Controls overlay */}
        <div className="absolute top-2 right-2 z-10">
          <div className="flex flex-col items-center gap-1 p-2 rounded border bg-[var(--vscode-editor-background)] border-[var(--vscode-panel-border)] shadow">
            <Button size="icon" variant="outline" onClick={zoomIn} className="h-8 w-8 p-0" title="Zoom In">
              <ZoomIn className="h-4 w-4" />
            </Button>
            <Button size="icon" variant="outline" onClick={zoomOut} className="h-8 w-8 p-0" title="Zoom Out">
              <ZoomOut className="h-4 w-4" />
            </Button>
            <Button size="icon" variant="outline" onClick={() => resetView()} className="h-8 w-8 p-0" title="Reset View">
              <Maximize2 className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
};


