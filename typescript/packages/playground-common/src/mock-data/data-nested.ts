import type { Graph } from './types';

/**
 * E-commerce order processing system with doubly nested subgraphs
 *
 * This realistic example demonstrates:
 * - Payment Processing with nested Fraud Detection
 * - Inventory Management with nested Warehouse Operations
 * - Multiple cross-boundary edges at different nesting levels
 *
 * Tests edge routing with:
 * - Root → Outer edges
 * - Outer → Inner edges (2 levels deep)
 * - Inner → Root edges (crossing 2 levels)
 * - Cross-subgraph edges
 */
export const nestedData: Graph = {
  nodes: [
    // Main order processing flow
    { id: 'ORDER_RECEIVED', label: 'Order Received', kind: 'item' },
    { id: 'VALIDATE_ORDER', label: 'Validate Order', kind: 'item' },
    {
      id: 'SHIPPING_DECISION',
      label: 'Ready to Ship?',
      kind: 'item',
      shape: 'diamond',
    },
    { id: 'SHIP_ORDER', label: 'Ship Order', kind: 'item' },
    { id: 'NOTIFY_CUSTOMER', label: 'Send Confirmation', kind: 'item' },
    { id: 'ORDER_COMPLETE', label: 'Order Complete', kind: 'item' },

    // Payment Processing subgraph (outer)
    { id: 'PAYMENT', label: 'Payment Processing', kind: 'group' },
    {
      id: 'VALIDATE_PAYMENT',
      label: 'Validate Payment Info',
      kind: 'item',
      parent: 'PAYMENT',
    },
    {
      id: 'CHARGE_CARD',
      label: 'Charge Credit Card',
      kind: 'item',
      parent: 'PAYMENT',
    },

    // Fraud Detection subgraph (nested inside PAYMENT)
    {
      id: 'FRAUD_DETECTION',
      label: 'Fraud Detection',
      kind: 'group',
      parent: 'PAYMENT',
    },
    {
      id: 'CHECK_CARD',
      label: 'Verify Card',
      kind: 'item',
      parent: 'FRAUD_DETECTION',
    },
    {
      id: 'CHECK_ADDRESS',
      label: 'Verify Billing Address',
      kind: 'item',
      parent: 'FRAUD_DETECTION',
    },
    {
      id: 'RISK_SCORE',
      label: 'Calculate Risk Score',
      kind: 'item',
      parent: 'FRAUD_DETECTION',
    },

    // Inventory Management subgraph (outer)
    { id: 'INVENTORY', label: 'Inventory Management', kind: 'group' },
    {
      id: 'CHECK_STOCK',
      label: 'Check Stock Levels',
      kind: 'item',
      parent: 'INVENTORY',
    },
    {
      id: 'RESERVE_ITEMS',
      label: 'Reserve Items',
      kind: 'item',
      parent: 'INVENTORY',
    },

    // Warehouse Operations subgraph (nested inside INVENTORY)
    {
      id: 'WAREHOUSE',
      label: 'Warehouse Operations',
      kind: 'group',
      parent: 'INVENTORY',
    },
    {
      id: 'PICK_ITEMS',
      label: 'Pick Items from Shelves',
      kind: 'item',
      parent: 'WAREHOUSE',
    },
    {
      id: 'PACK_ITEMS',
      label: 'Pack Items',
      kind: 'item',
      parent: 'WAREHOUSE',
    },
    {
      id: 'GENERATE_LABEL',
      label: 'Generate Shipping Label',
      kind: 'item',
      parent: 'WAREHOUSE',
    },
  ],
  edges: [
    // Main flow
    {
      id: 'e_order_validate',
      from: 'ORDER_RECEIVED',
      to: 'VALIDATE_ORDER',
      style: 'solid',
    },

    // Validate triggers parallel processing
    {
      id: 'e_validate_payment',
      from: 'VALIDATE_ORDER',
      to: 'PAYMENT',
      style: 'solid',
    },
    {
      id: 'e_validate_inventory',
      from: 'VALIDATE_ORDER',
      to: 'INVENTORY',
      style: 'solid',
    },

    // Payment processing flow (inside PAYMENT group)
    {
      id: 'e_payment_fraud',
      from: 'VALIDATE_PAYMENT',
      to: 'FRAUD_DETECTION',
      style: 'solid',
    },
    {
      id: 'e_fraud_card_address',
      from: 'CHECK_CARD',
      to: 'CHECK_ADDRESS',
      style: 'solid',
    },
    {
      id: 'e_fraud_address_risk',
      from: 'CHECK_ADDRESS',
      to: 'RISK_SCORE',
      style: 'solid',
    },
    {
      id: 'e_fraud_charge',
      from: 'FRAUD_DETECTION',
      to: 'CHARGE_CARD',
      style: 'solid',
    },

    // Inventory processing flow (inside INVENTORY group)
    {
      id: 'e_stock_reserve',
      from: 'CHECK_STOCK',
      to: 'RESERVE_ITEMS',
      style: 'solid',
    },
    {
      id: 'e_reserve_warehouse',
      from: 'RESERVE_ITEMS',
      to: 'WAREHOUSE',
      style: 'solid',
    },
    {
      id: 'e_pick_pack',
      from: 'PICK_ITEMS',
      to: 'PACK_ITEMS',
      style: 'solid',
    },
    {
      id: 'e_pack_label',
      from: 'PACK_ITEMS',
      to: 'GENERATE_LABEL',
      style: 'solid',
    },

    // Convergence to shipping decision
    {
      id: 'e_payment_decision',
      from: 'PAYMENT',
      to: 'SHIPPING_DECISION',
      style: 'solid',
    },
    {
      id: 'e_inventory_decision',
      from: 'INVENTORY',
      to: 'SHIPPING_DECISION',
      style: 'solid',
    },

    // Final flow
    {
      id: 'e_decision_ship',
      from: 'SHIPPING_DECISION',
      to: 'SHIP_ORDER',
      style: 'solid',
    },
    {
      id: 'e_ship_notify',
      from: 'SHIP_ORDER',
      to: 'NOTIFY_CUSTOMER',
      style: 'solid',
    },
    {
      id: 'e_notify_complete',
      from: 'NOTIFY_CUSTOMER',
      to: 'ORDER_COMPLETE',
      style: 'solid',
    },

    // Cross-boundary edge: Fraud detection directly affects shipping (2 levels deep → root)
    {
      id: 'e_fraud_fail',
      from: 'RISK_SCORE',
      to: 'ORDER_COMPLETE',
      style: 'dashed',
    },
  ],
};
