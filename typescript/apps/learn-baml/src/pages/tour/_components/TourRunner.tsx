import React, { useState, useCallback } from 'react';

// Golden examples for each tour module - following the TOUR_SYLLABUS_PLAN.md
const goldenExamples: Record<string, { code: string; prompt: string; output: string; tryIt?: string }> = {
  // Module 1: Your First BAML Function
  'hello-baml': {
    code: `enum Sentiment {
  POSITIVE
  NEGATIVE
  NEUTRAL
}

function ClassifySentiment(text: string) -> Sentiment {
  client "openai/gpt-4o-mini"
  prompt #"
    Classify the sentiment of: {{ text }}
    {{ ctx.output_format }}
  "#
}`,
    prompt: `Classify the sentiment of: I absolutely love this product! Best purchase ever!

Answer with one of these values:
- POSITIVE
- NEGATIVE
- NEUTRAL`,
    output: `POSITIVE`,
    tryIt: `Try it: Change POSITIVE to HAPPY and add SAD. Run again to see the output format update automatically.`,
  },

  // Module 2: See What the Model Sees
  'prompt-transparency': {
    code: `class Email {
  subject string
  body string
  priority "urgent" | "normal" | "low"
}

function DraftReply(original: Email, tone: "formal" | "casual") -> string {
  client "openai/gpt-4o-mini"
  prompt #"
    You received this email:
    Subject: {{ original.subject }}
    Body: {{ original.body }}
    Priority: {{ original.priority }}

    Write a {{ tone }} reply.
  "#
}`,
    prompt: `You received this email:
Subject: Q3 Budget Review Meeting
Body: Hi team, we need to discuss the Q3 budget allocations. Several departments have exceeded their limits.
Priority: urgent

Write a formal reply.`,
    output: `Dear Team,

Thank you for bringing this to our attention. I understand the urgency of reviewing our Q3 budget allocations.

I am available to meet at your earliest convenience to discuss the departments that have exceeded their limits and explore potential solutions.

Please let me know your preferred time slots, and I will adjust my schedule accordingly.

Best regards`,
    tryIt: `Try it: Add conditional logic like {% if original.priority == "urgent" %}Respond within 1 hour.{% endif %}`,
  },

  // Module 3: Types That Do Work
  'types-at-work': {
    code: `class ContactInfo {
  name string
  email string?
  phone string?
  company string?
}

function ExtractContact(text: string) -> ContactInfo {
  client "openai/gpt-4o-mini"
  prompt #"
    Extract contact information from:
    {{ text }}

    {{ ctx.output_format }}
  "#
}`,
    prompt: `Extract contact information from:
Call John Smith at john@acme.com or 555-1234

Answer in JSON using this schema:
{
  "name": string,
  "email": string or null,
  "phone": string or null,
  "company": string or null
}`,
    output: `{
  "name": "John Smith",
  "email": "john@acme.com",
  "phone": "555-1234",
  "company": "Acme"
}`,
    tryIt: `Try it: Add a new optional field like "company string?" and see how the output format updates.`,
  },

  // Module 4: Unions for Messy Reality
  'union-types': {
    code: `class ContactInfo {
  name string
  email string?
  phone string?
}

class SuccessfulExtraction {
  data ContactInfo
  confidence float @description("0.0 to 1.0")
}

class NeedsMoreInfo {
  question string @description("What clarification is needed")
}

class CannotProcess {
  reason string
}

function SmartExtract(text: string) -> SuccessfulExtraction | NeedsMoreInfo | CannotProcess {
  client "openai/gpt-4o-mini"
  prompt #"
    Extract contact info from: {{ text }}

    If the text is clear, return the data with confidence score.
    If you need clarification, ask a specific question.
    If you cannot process it, explain why.

    {{ ctx.output_format }}
  "#
}`,
    prompt: `Extract contact info from: Call my friend tomorrow

If the text is clear, return the data with confidence score.
If you need clarification, ask a specific question.
If you cannot process it, explain why.

Answer with one of these JSON schemas:
SuccessfulExtraction: { "data": ContactInfo, "confidence": float }
NeedsMoreInfo: { "question": string }
CannotProcess: { "reason": string }`,
    output: `{
  "__type": "NeedsMoreInfo",
  "question": "Who is your friend? What is their contact information?"
}`,
    tryIt: `Try it: Run with clear input like "John at john@test.com" to see SuccessfulExtraction, or with emojis to see CannotProcess.`,
  },

  // Module 5: Attributes (@description, @alias)
  'attributes': {
    code: `class Person {
  name string @description("Full legal name")
  email string? @description("Primary email address")
  dob string @alias("date_of_birth") @description("Format: YYYY-MM-DD")
  ssn string @alias("social_security") @skip
}

enum Priority {
  Urgent @alias("P0") @description("Immediate attention required")
  High @alias("P1")
  Normal @alias("P2")
  Low @alias("P3") @description("Can wait until next sprint")
}

function ExtractPerson(text: string) -> Person {
  client "openai/gpt-4o-mini"
  prompt #"
    Extract person information from:
    {{ text }}

    {{ ctx.output_format }}
  "#
}`,
    prompt: `Extract person information from:
Contact Jane Doe (born March 15, 1990) at jane.doe@company.com. Her SSN is 123-45-6789.

Answer in JSON using this schema:
{
  "name": string (Full legal name),
  "email": string or null (Primary email address),
  "date_of_birth": string (Format: YYYY-MM-DD)
}

Note: "social_security" field is not included in the output.`,
    output: `{
  "name": "Jane Doe",
  "email": "jane.doe@company.com",
  "date_of_birth": "1990-03-15"
}

// Note: SSN was NOT extracted because @skip excludes it from the schema.
// The model never sees that field exists.`,
    tryIt: `Try it: Remove @skip from ssn. See how the prompt changes to include it. Then add it back for security.`,
  },

  // Module 6: Tests That Run
  'testing-loop': {
    code: `function ClassifyIntent(message: string) -> "question" | "complaint" | "feedback" | "other" {
  client "openai/gpt-4o-mini"
  prompt #"
    Classify the customer intent:
    {{ message }}
    {{ ctx.output_format }}
  "#
}

test ClassifyIntentTests {
  functions [ClassifyIntent]

  args {
    message "How do I reset my password?"
  }
  @@assert {{ result == "question" }}
}

test ComplaintTest {
  functions [ClassifyIntent]

  args {
    message "This product broke after one day!"
  }
  @@assert {{ result == "complaint" }}
}`,
    prompt: `Classify the customer intent:
How do I reset my password?

Answer with one of these values:
- question
- complaint
- feedback
- other`,
    output: `Test Results:
✓ ClassifyIntentTests - PASSED
  Input: "How do I reset my password?"
  Expected: question
  Got: question

✓ ComplaintTest - PASSED
  Input: "This product broke after one day!"
  Expected: complaint
  Got: complaint

2/2 tests passed`,
    tryIt: `Try it: Add a test case that you think might fail. Run it. Iterate on the prompt until it passes.`,
  },

  // Module 6: Match for Control Flow
  'match-expressions': {
    code: `// Using the SmartExtract function from Module 4
// which returns: SuccessfulExtraction | NeedsMoreInfo | CannotProcess

function HandleMessage(msg: string) -> string {
  let result = SmartExtract(msg)

  match result {
    SuccessfulExtraction(s) => "Thanks! I found: " + s.data.name
    NeedsMoreInfo(n) => "I need more details: " + n.question
    CannotProcess(c) => "Sorry, I couldn't process that: " + c.reason
  }
}`,
    prompt: `// This shows how match expressions handle union types

Input: "Call my friend"

SmartExtract returns NeedsMoreInfo:
{
  "question": "Who is your friend? What is their contact information?"
}

The match expression then handles this case:`,
    output: `I need more details: Who is your friend? What is their contact information?`,
    tryIt: `Try it: Add a new variant to the union. Watch the compiler tell you to handle it in the match.`,
  },

  // Module 7: Streaming with Types
  'streaming': {
    code: `class Article {
  title string
  summary string @description("2-3 sentence summary")
  keyPoints string[] @description("3-5 bullet points")
  sentiment Sentiment
}

enum Sentiment {
  POSITIVE
  NEGATIVE
  NEUTRAL
}

function SummarizeArticle(url: string) -> Article {
  client "openai/gpt-4o-mini"
  prompt #"
    Summarize the article at: {{ url }}
    {{ ctx.output_format }}
  "#
}`,
    prompt: `Summarize the article at: https://example.com/tech-news

Answer in JSON using this schema:
{
  "title": string,
  "summary": string (2-3 sentence summary),
  "keyPoints": string[] (3-5 bullet points),
  "sentiment": "POSITIVE" | "NEGATIVE" | "NEUTRAL"
}`,
    output: `// Streaming output - fields arrive as they're ready:

partial.title → "AI Advances in 2024"
partial.summary → "The article discusses..." (builds character by character)
partial.keyPoints → ["Point 1", "Point 2", ...] (array grows as items are parsed)
partial.sentiment → POSITIVE (resolves last)

Final output:
{
  "title": "AI Advances in 2024",
  "summary": "Major breakthroughs in AI have transformed the tech landscape this year. Companies are racing to implement new capabilities.",
  "keyPoints": [
    "GPT-4 class models became widely available",
    "Multi-modal AI gained mainstream adoption",
    "AI coding assistants reached 50% developer adoption"
  ],
  "sentiment": "POSITIVE"
}`,
    tryIt: `Try it: Call this from TypeScript/Python and see how streaming partial types work for real-time UIs.`,
  },

  // Module 8: Retry and Fallback
  'client-strategies': {
    code: `retry_policy Resilient {
  max_retries 3
  strategy {
    type exponential_backoff
    initial_delay_ms 500
    max_delay_ms 10000
  }
}

client<llm> ReliableGPT {
  provider openai
  options {
    model "gpt-4o-mini"
    api_key env.OPENAI_API_KEY
  }
  retry_policy Resilient
}

client<llm> FallbackChain {
  provider "baml-fallback"
  options {
    strategy [
      ReliableGPT,
      { provider anthropic, options { model "claude-3-haiku-20240307" } }
    ]
  }
}`,
    prompt: `// Client configuration - no prompt sent to model
// This shows the reliability infrastructure

Request flow:
1. Try ReliableGPT (OpenAI gpt-4o-mini)
2. If fails, retry up to 3 times with exponential backoff
3. If still failing, fallback to Claude Haiku`,
    output: `// Simulated execution trace:

[0ms] Request to ReliableGPT...
[50ms] ❌ Rate limit error (429)
[550ms] Retry 1 with 500ms backoff...
[600ms] ❌ Rate limit error (429)
[1600ms] Retry 2 with 1000ms backoff...
[1650ms] ❌ Rate limit error (429)
[3650ms] Retry 3 with 2000ms backoff...
[3700ms] ❌ Rate limit error (429)
[3700ms] Falling back to Claude Haiku...
[3900ms] ✓ Success! Response received.

Total time: 3.9s
Provider used: anthropic/claude-3-haiku`,
    tryIt: `Try it: Add a round-robin strategy for load balancing across providers.`,
  },

  // Module 9: Call from Your Language
  'polyglot-integration': {
    code: `// BAML Definition
function AnalyzeSentiment(reviews: string[]) -> { positive: int, negative: int, neutral: int } {
  client "openai/gpt-4o-mini"
  prompt #"
    Analyze these reviews:
    {% for review in reviews %}
    - {{ review }}
    {% endfor %}

    Count how many are positive, negative, and neutral.
    {{ ctx.output_format }}
  "#
}`,
    prompt: `// TypeScript Usage
import { b } from './baml_client';

const result = await b.AnalyzeSentiment([
  "Great product!",
  "Terrible service",
  "It's okay I guess"
]);

console.log(result.positive);  // TypeScript knows this is a number

// Python Usage
from baml_client import b

result = b.AnalyzeSentiment([
    "Great product!",
    "Terrible service",
    "It's okay I guess"
])

print(result.positive)  # Python type hints work`,
    output: `// Generated TypeScript types:
interface AnalyzeSentimentOutput {
  positive: number;
  negative: number;
  neutral: number;
}

// Result:
{
  "positive": 1,
  "negative": 1,
  "neutral": 1
}

// IDE autocomplete works in both languages!
// Types flow from BAML → Generated code → Your app`,
    tryIt: `Try it: See the generated TypeScript types. See how they match the BAML definition exactly.`,
  },

  // Module 11: Dynamic Types (TypeBuilder)
  'dynamic-types': {
    code: `// BAML Definition - mark class as @@dynamic
class FormField @@dynamic {
  label string
  value string
}

function ExtractForm(image: image) -> FormField[] {
  client "openai/gpt-4o-mini"
  prompt #"
    Extract all form fields from this image:
    {{ image }}

    {{ ctx.output_format }}
  "#
}`,
    prompt: `// TypeScript - Add fields at runtime!
import { b, TypeBuilder } from './baml_client';

// Create a TypeBuilder to add fields dynamically
const tb = new TypeBuilder();

// Get the dynamic class and add new properties
const formField = tb.FormField;
formField.addProperty('fieldType', tb.string());
formField.addProperty('required', tb.bool());
formField.addProperty('validation', tb.string().optional());

// Call with dynamic schema
const result = await b.ExtractForm(imageData, { tb });

// Result now has the dynamically added fields!
console.log(result[0].fieldType);   // "text"
console.log(result[0].required);    // true
console.log(result[0].validation);  // "email" or null`,
    output: `// The prompt sent to the model includes the dynamic fields:

Extract all form fields from this image:
[image data]

Answer as a JSON array with this schema:
{
  "label": string,
  "value": string,
  "fieldType": string,
  "required": boolean,
  "validation": string or null
}[]

// Result:
[
  {
    "label": "Email Address",
    "value": "user@example.com",
    "fieldType": "text",
    "required": true,
    "validation": "email"
  },
  {
    "label": "Subscribe to newsletter",
    "value": "checked",
    "fieldType": "checkbox",
    "required": false,
    "validation": null
  }
]`,
    tryIt: `Try it: Use TypeBuilder when your schema depends on user input, database config, or other runtime data.`,
  },

  // Module 12: Build Something Real
  'putting-it-together': {
    code: `// Customer Support Triage Bot - combining everything

class CustomerMessage {
  content string
  customerTier "enterprise" | "pro" | "free"
  previousTickets int
}

class DraftResponse {
  response string
  suggestedActions string[]
  confidence float
}

class EscalateToHuman {
  reason string
  urgency "immediate" | "high" | "normal"
  suggestedTeam "billing" | "technical" | "legal" | "general"
}

class NeedMoreContext {
  questions string[]
}

function TriageMessage(msg: CustomerMessage) -> DraftResponse | EscalateToHuman | NeedMoreContext {
  client FallbackChain
  prompt #"
    You are a customer support assistant.

    Customer tier: {{ msg.customerTier }}
    Previous tickets: {{ msg.previousTickets }}
    Message: {{ msg.content }}

    {% if msg.customerTier == "enterprise" %}
    Note: Enterprise customers should be escalated if issue seems complex.
    {% endif %}

    {{ ctx.output_format }}
  "#
}

test EnterprisePriority {
  functions [TriageMessage]
  args {
    msg {
      content "Our API is down and we're losing revenue"
      customerTier "enterprise"
      previousTickets 0
    }
  }
  @@assert {{ result is EscalateToHuman }}
  @@assert {{ result.urgency == "immediate" }}
}`,
    prompt: `You are a customer support assistant.

Customer tier: enterprise
Previous tickets: 0
Message: Our API is down and we're losing revenue

Note: Enterprise customers should be escalated if issue seems complex.

Answer with one of these JSON schemas:
DraftResponse: { response, suggestedActions[], confidence }
EscalateToHuman: { reason, urgency, suggestedTeam }
NeedMoreContext: { questions[] }`,
    output: `{
  "__type": "EscalateToHuman",
  "reason": "Critical production issue affecting enterprise customer revenue",
  "urgency": "immediate",
  "suggestedTeam": "technical"
}

Test Results:
✓ EnterprisePriority - PASSED
  ✓ result is EscalateToHuman
  ✓ result.urgency == "immediate"`,
    tryIt: `Try it: Fork this and adapt it to your use case. Add a new customer tier. Add a test for your edge case.`,
  },
};

interface TourRunnerProps {
  exampleKey: string;
}

export default function TourRunner({ exampleKey }: TourRunnerProps) {
  const example = goldenExamples[exampleKey];
  const [activeTab, setActiveTab] = useState<'prompt' | 'output'>('prompt');
  const [hasRun, setHasRun] = useState(false);
  const [isRunning, setIsRunning] = useState(false);
  const [copied, setCopied] = useState(false);

  const handleRun = useCallback(() => {
    setIsRunning(true);
    setTimeout(() => {
      setHasRun(true);
      setIsRunning(false);
    }, 500);
  }, []);

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(example?.code || '');
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }, [example]);

  const handleReset = useCallback(() => {
    setHasRun(false);
    setActiveTab('prompt');
  }, []);

  if (!example) {
    return <div className="tour-placeholder">Example not found: {exampleKey}</div>;
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', flex: 1 }}>
      <div className="tour-runner">
        {/* Left panel: Code */}
        <div className="tour-panel">
          <div className="tour-panel-header" style={{ display: 'flex', justifyContent: 'space-between' }}>
            <span>main.baml</span>
            <button
              onClick={handleCopy}
              style={{
                background: 'none',
                border: 'none',
                cursor: 'pointer',
                color: copied ? 'green' : 'inherit'
              }}
            >
              {copied ? '✓ Copied' : 'Copy'}
            </button>
          </div>
          <div className="tour-panel-content">
            <pre className="tour-code">{example.code}</pre>
          </div>
        </div>

        <div className="tour-divider" />

        {/* Right panel: Output */}
        <div className="tour-panel">
          <div className="tour-panel-header" style={{ display: 'flex', justifyContent: 'space-between' }}>
            <div style={{ display: 'flex', gap: '1rem' }}>
              <button
                onClick={() => setActiveTab('prompt')}
                style={{
                  background: 'none',
                  border: 'none',
                  cursor: 'pointer',
                  fontWeight: activeTab === 'prompt' ? 600 : 400,
                  borderBottom: activeTab === 'prompt' ? '2px solid var(--ifm-color-primary)' : 'none',
                }}
              >
                Prompt Preview
              </button>
              <button
                onClick={() => setActiveTab('output')}
                style={{
                  background: 'none',
                  border: 'none',
                  cursor: 'pointer',
                  fontWeight: activeTab === 'output' ? 600 : 400,
                  borderBottom: activeTab === 'output' ? '2px solid var(--ifm-color-primary)' : 'none',
                }}
              >
                Output
              </button>
            </div>
            <div style={{ display: 'flex', gap: '0.5rem' }}>
              <button onClick={handleReset} style={{ cursor: 'pointer' }}>↺ Reset</button>
              <button
                onClick={handleRun}
                disabled={isRunning}
                className="button button--primary button--sm"
              >
                {isRunning ? 'Running...' : '▶ Run'}
              </button>
            </div>
          </div>
          <div className="tour-panel-content">
            {activeTab === 'prompt' ? (
              hasRun ? (
                <pre className="tour-code">{example.prompt}</pre>
              ) : (
                <div className="tour-placeholder">
                  Click <strong>Run</strong> to see the rendered prompt
                </div>
              )
            ) : (
              hasRun ? (
                <pre className="tour-code" style={{ color: 'var(--ifm-color-success)' }}>
                  {example.output}
                </pre>
              ) : (
                <div className="tour-placeholder">
                  Click <strong>Run</strong> to see the output
                </div>
              )
            )}
          </div>
        </div>
      </div>

      {/* Try It section */}
      {example.tryIt && hasRun && (
        <div style={{
          margin: '0 1rem 1rem',
          padding: '0.75rem 1rem',
          background: 'var(--ifm-color-primary-lightest)',
          borderRadius: '8px',
          fontSize: '0.875rem',
          color: 'var(--ifm-color-primary-darkest)',
          borderLeft: '3px solid var(--ifm-color-primary)',
        }}>
          {example.tryIt}
        </div>
      )}
    </div>
  );
}
