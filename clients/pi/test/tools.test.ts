import { describe, expect, test, vi } from "vitest";
import { createLlmpartyPiExtension } from "../src/index.js";

interface RegisteredTool {
  name: string;
}

function install(env: Record<string, string | undefined>) {
  const tools: RegisteredTool[] = [];
  const pi = {
    on: vi.fn(),
    registerTool: vi.fn((tool: RegisteredTool) => tools.push(tool)),
  };

  createLlmpartyPiExtension(pi as any, {
    env,
    loadContext: vi.fn(),
    makeReporter: vi.fn(),
    logDiagnostic: vi.fn(),
  });

  return { pi, tools };
}

describe("llmparty pi external API tools", () => {
  test("does not register agent-visible tools from the pi extension", () => {
    const { pi, tools } = install({});

    expect(tools).toEqual([]);
    expect(pi.registerTool).not.toHaveBeenCalled();
  });
});
