import React, { useMemo, useState } from 'react';
import '@xyflow/react/dist/style.css';

import WorkflowReplayerTab from './tabs/workflow-replayer';
import AIContentPipelineTab from './tabs/ai-content-pipeline';
import AiContentPipeline3Tab from './tabs/ai-content-pipeline3';
import AgentBuilderDemoTab from './tabs/agent-builder-demo';
import N8nItOpsTab from './tabs/n8n-it-ops';
import N8nDevopsProxmoxTab from './tabs/n8n-devops-proxmox';

interface FlowTab {
  id: string;
  label: string;
  description: string;
  component: React.ComponentType;
}

const FLOW_TABS: FlowTab[] = [
  {
    id: 'workflow',
    label: 'Workflow Replayer',
    description: 'Simulate reruns and divergence handling for a complex BAML workflow trace.',
    component: WorkflowReplayerTab,
  },
  {
    id: 'ai-content2',
    label: 'AI Content Pipeline 2',
    description: 'Map the AIContentPipeline BAML function into a staged flow.',
    component: AIContentPipelineTab,
  },
  {
    id: 'ai-content3',
    label: 'AI Content Pipeline 3',
    description: 'Map the AIContentPipeline BAML function into a staged flow.',
    component: AiContentPipeline3Tab,
  },
  {
    id: 'agent-builder',
    label: 'OpenAI Agent Builder Demo',
    description: 'Start → classify → route pipeline inspired by the agent builder UI.',
    component: AgentBuilderDemoTab,
  },
  {
    id: 'n8n-it-ops',
    label: 'n8n IT Ops onboard',
    description: 'Get Entra user → create Jira → conditional Slack actions for onboarding.',
    component: N8nItOpsTab,
  },
  {
    id: 'n8n-devops-proxmox',
    label: 'n8n devops - proxmox command translator',
    description: 'Gather Proxmox docs, build HTTP requests, and branch for GET/POST/DELETE operations.',
    component: N8nDevopsProxmoxTab,
  },
];

export default function App() {
  const [activeTab, setActiveTab] = useState<string>(FLOW_TABS[0].id);
  const activeTabConfig = useMemo(
    () => FLOW_TABS.find((tab) => tab.id === activeTab) ?? FLOW_TABS[0],
    [activeTab]
  );
  const ActiveComponent = activeTabConfig.component;

  return (
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column', background: '#f8fafc' }}>
      <div style={{ borderBottom: '1px solid #e5e7eb', background: '#ffffff', padding: '12px 16px 0' }}>
        <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
          {FLOW_TABS.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              style={{
                padding: '6px 12px',
                borderRadius: '999px',
                fontSize: '12px',
                fontWeight: 600,
                border: '1px solid',
                borderColor: activeTab === tab.id ? '#3b82f6' : 'transparent',
                background: activeTab === tab.id ? '#eff6ff' : '#f8fafc',
                color: activeTab === tab.id ? '#1d4ed8' : '#1f2937',
                cursor: 'pointer',
                transition: 'all 0.2s',
              }}
            >
              {tab.label}
            </button>
          ))}
        </div>
        {activeTabConfig.description && (
          <p style={{ marginTop: '8px', marginBottom: '12px', fontSize: '12px', color: '#6b7280' }}>
            {activeTabConfig.description}
          </p>
        )}
      </div>
      <div style={{ flex: 1, minHeight: 0, padding: '16px' }}>
        <div
          style={{
            height: '100%',
            borderRadius: '12px',
            overflow: 'hidden',
            background: '#ffffff',
            border: '1px solid #e2e8f0',
            boxShadow: '0 18px 28px rgba(15, 23, 42, 0.08)',
          }}
        >
          <ActiveComponent />
        </div>
      </div>
    </div>
  );
}
