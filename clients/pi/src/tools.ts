import toolContract from "../../tools/llmparty-tools.v1.json" with { type: "json" };
import type { EnvLike, LoadTurnContextResult } from "./context.js";
import { loadTurnContext } from "./context.js";
import { appendDiagnostic, type DiagnosticEntry } from "./diagnostics.js";

type FetchLike = typeof fetch;

type ToolContract = typeof toolContract;
type ToolSpec = ToolContract["tools"][number];

type AgentToolResult = {
  content: Array<{ type: "text"; text: string }>;
  details: unknown;
};

export interface LlmpartyAgentToolDependencies {
  env?: EnvLike;
  loadContext?: (env: EnvLike) => Promise<LoadTurnContextResult>;
  logDiagnostic?: (logFile: string, entry: DiagnosticEntry) => Promise<void>;
  fetch?: FetchLike;
}

export interface PiToolDefinitionLike {
  name: string;
  label: string;
  description: string;
  promptSnippet: string;
  promptGuidelines: string[];
  parameters: unknown;
  execute(
    toolCallId: string,
    params: unknown,
    signal?: AbortSignal,
    onUpdate?: unknown,
    ctx?: unknown,
  ): Promise<AgentToolResult>;
}

const TOOL_NAMES = ["getContext", "submitPlan", "submitResult", "raiseSignal"] as const;
type LlmpartyToolName = (typeof TOOL_NAMES)[number];

function isLlmpartyToolName(name: string): name is LlmpartyToolName {
  return (TOOL_NAMES as readonly string[]).includes(name);
}

function piToolName(toolName: LlmpartyToolName): string {
  return `llmparty_${toolName}`;
}

function responseText(toolName: string, payload: unknown): string {
  if (toolName === "llmparty_getContext") {
    const text = agentContextText(payload);
    if (text) return text;
  }
  return `${toolName} response:\n${JSON.stringify(payload, null, 2)}`;
}

function agentContextText(payload: unknown): string | undefined {
  if (typeof payload === "string") return payload;
  if (!payload || typeof payload !== "object") return undefined;
  const record = payload as Record<string, unknown>;
  const result = record.result;
  if (typeof result === "string") return result;
  if (result && typeof result === "object") {
    const text = (result as Record<string, unknown>).text;
    if (typeof text === "string") return text;
  }
  const text = record.text;
  return typeof text === "string" ? text : undefined;
}

function failureResult(message: string, details: unknown): AgentToolResult {
  return {
    content: [{ type: "text", text: message }],
    details,
  };
}

export function internalAgentToolUrl(internalEventUrl: string, toolName: string): string {
  const trimmed = internalEventUrl.replace(/\/+$/, "");
  if (trimmed.endsWith("/internal/v1/events")) {
    return `${trimmed.slice(0, -"/events".length)}/agent-tools/${encodeURIComponent(toolName)}`;
  }
  return `${trimmed}/agent-tools/${encodeURIComponent(toolName)}`;
}

async function parseResponseBody(response: Response): Promise<unknown> {
  const text = await response.text().catch(() => "");
  if (text.length === 0) return null;
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

function buildTool(spec: ToolSpec, dependencies: Required<LlmpartyAgentToolDependencies>): PiToolDefinitionLike | undefined {
  if (!isLlmpartyToolName(spec.name)) return undefined;

  const piName = piToolName(spec.name);
  return {
    name: piName,
    label: spec.title,
    description: spec.description,
    promptSnippet: spec.title,
    promptGuidelines: [
      `Use ${piName} for llmparty DAG-managed ${spec.name} operations. Do not include task_id, work_item_id, or run_id; llmparty authorizes the call from the current turn context.`,
    ],
    parameters: spec.inputSchema,
    async execute(_toolCallId, params, signal) {
      const loaded = await dependencies.loadContext(dependencies.env);
      const logFile = loaded.logFile;
      if (!loaded.ok) {
        await dependencies.logDiagnostic(logFile, {
          level: "warn",
          code: "agent_tool_context_unavailable",
          message: `${piName} unavailable: ${loaded.reason}`,
          details: { contextFile: loaded.contextFile },
        });
        return failureResult(`${piName} unavailable: ${loaded.reason}`, { ok: false, reason: loaded.reason });
      }

      const request = {
        session_id: loaded.context.sessionId,
        turn_id: loaded.context.turnId,
        runtime_instance_id: loaded.context.runtimeInstanceId,
        input: params && typeof params === "object" ? params : {},
      };
      const url = internalAgentToolUrl(loaded.context.internalEventUrl, spec.name);

      try {
        const response = await dependencies.fetch(url, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(request),
          signal,
        });
        const body = await parseResponseBody(response);

        if (!response.ok) {
          await dependencies.logDiagnostic(logFile, {
            level: "error",
            code: "agent_tool_post_failed",
            message: `${piName} failed: ${response.status} ${response.statusText}`,
            details: { tool: spec.name, status: response.status, body },
          });
          return failureResult(`${piName} failed: ${response.status} ${response.statusText}\n${JSON.stringify(body)}`, {
            ok: false,
            status: response.status,
            body,
          });
        }

        return {
          content: [{ type: "text", text: responseText(piName, body) }],
          details: body,
        };
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        await dependencies.logDiagnostic(logFile, {
          level: "error",
          code: "agent_tool_post_exception",
          message: `${piName} Internal Agent Tool API POST failed`,
          details: { tool: spec.name, error: message },
        });
        return failureResult(`${piName} failed: ${message}`, { ok: false, error: message });
      }
    },
  };
}

export function buildLlmpartyTools(dependencies: LlmpartyAgentToolDependencies = {}): PiToolDefinitionLike[] {
  const resolved: Required<LlmpartyAgentToolDependencies> = {
    env: dependencies.env ?? process.env,
    loadContext: dependencies.loadContext ?? loadTurnContext,
    logDiagnostic: dependencies.logDiagnostic ?? appendDiagnostic,
    fetch: dependencies.fetch ?? fetch,
  };

  return toolContract.tools
    .map((spec) => buildTool(spec, resolved))
    .filter((tool): tool is PiToolDefinitionLike => Boolean(tool));
}
