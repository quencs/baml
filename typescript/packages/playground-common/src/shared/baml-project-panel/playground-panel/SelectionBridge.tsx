import { useAtom } from 'jotai';
import { useEffect, useRef } from 'react';

import { selectedNodeIdAtom } from '../../../sdk/atoms/core.atoms';
import { unifiedSelectionAtom } from './unified-atoms';

/**
 * Keeps legacy selectedNodeIdAtom and the unified selection atom in sync.
 * This lets us treat unifiedSelectionAtom as the single source of truth while
 * still supporting code that reads selectedNodeIdAtom via SDK hooks.
 */
export function SelectionBridge() {
  const [selectedNodeId, setSelectedNodeId] = useAtom(selectedNodeIdAtom);
  const [unifiedSelection, setUnifiedSelection] = useAtom(unifiedSelectionAtom);

  const syncingFromUnified = useRef(false);
  const syncingFromLegacy = useRef(false);

  // unifiedSelection -> selectedNodeIdAtom
  useEffect(() => {
    if (syncingFromLegacy.current) return;
    if (selectedNodeId === unifiedSelection.selectedNodeId) return;

    syncingFromUnified.current = true;
    setSelectedNodeId(unifiedSelection.selectedNodeId);
    syncingFromUnified.current = false;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [unifiedSelection.selectedNodeId]);

  // selectedNodeIdAtom -> unifiedSelection (handles legacy updates from SDK)
  useEffect(() => {
    if (syncingFromUnified.current) return;
    if (selectedNodeId === unifiedSelection.selectedNodeId) return;

    syncingFromLegacy.current = true;
    setUnifiedSelection((prev) => ({
      ...prev,
      selectedNodeId,
    }));
    syncingFromLegacy.current = false;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedNodeId]);

  return null;
}
