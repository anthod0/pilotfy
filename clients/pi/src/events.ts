import { randomUUID } from "node:crypto";
import type { TurnContext } from "./context.js";
import type { SessionContext } from "./session.js";

export type InternalEventType = "session.ready" | "session.message_updated" | "turn.created" | "turn.started" | "turn.output" | "turn.completed" | "turn.failed";

export type SessionMessageUpdatedReason = "append" | "update" | "final";

interface BaseInternalEvent {
  event_id: string;
  session_id: string;
  client_type: "pi";
  type: InternalEventType;
  time: string;
  seq: null;
  payload: Record<string, unknown>;
}

export type InternalEvent =
  | (BaseInternalEvent & {
      turn_id: null;
      source: "agent_client";
      type: "session.ready";
    })
  | (BaseInternalEvent & {
      turn_id: null;
      source: "agent_client";
      type: "session.message_updated";
    })
  | (BaseInternalEvent & {
      turn_id: string;
      source: "agent_client";
      type: "turn.created";
    })
  | (BaseInternalEvent & {
      turn_id: string;
      source: "agent_adapter";
      type: "turn.started" | "turn.output" | "turn.completed" | "turn.failed";
    });

type AdapterTurnInternalEvent = Extract<InternalEvent, { source: "agent_adapter" }>;

function baseAdapterTurnEvent(context: TurnContext, type: AdapterTurnInternalEvent["type"]): Omit<AdapterTurnInternalEvent, "payload"> {
  return {
    event_id: `evt_${randomUUID()}`,
    session_id: context.sessionId,
    turn_id: context.turnId,
    source: "agent_adapter",
    client_type: "pi",
    type,
    time: new Date().toISOString(),
    seq: null,
  };
}

export function buildTurnCreatedEvent(context: TurnContext): InternalEvent {
  return {
    event_id: `evt_${randomUUID()}`,
    session_id: context.sessionId,
    turn_id: context.turnId,
    source: "agent_client",
    client_type: "pi",
    type: "turn.created",
    time: new Date().toISOString(),
    seq: null,
    payload: {
      runtime_instance_id: context.runtimeInstanceId,
      input: context.input ? { summary: context.input } : {},
      metadata: { source: "pi_tui" },
    },
  };
}

export function buildSessionMessageUpdatedEvent(context: TurnContext, reason: SessionMessageUpdatedReason): InternalEvent {
  return {
    event_id: `evt_${randomUUID()}`,
    session_id: context.sessionId,
    turn_id: null,
    source: "agent_client",
    client_type: "pi",
    type: "session.message_updated",
    time: new Date().toISOString(),
    seq: null,
    payload: { reason },
  };
}

export function buildTurnStartedEvent(context: TurnContext): InternalEvent {
  return {
    ...baseAdapterTurnEvent(context, "turn.started"),
    payload: {
      runtime_instance_id: context.runtimeInstanceId,
      input: context.input ? { summary: context.input } : {},
    },
  };
}

export function buildTurnOutputEvent(context: TurnContext, output: string): InternalEvent {
  return {
    ...baseAdapterTurnEvent(context, "turn.output"),
    payload: { output: { summary: output } },
  };
}

export function buildTurnCompletedEvent(context: TurnContext): InternalEvent {
  return {
    ...baseAdapterTurnEvent(context, "turn.completed"),
    payload: {},
  };
}

export function buildTurnFailedEvent(context: TurnContext, message: string): InternalEvent {
  return {
    ...baseAdapterTurnEvent(context, "turn.failed"),
    payload: { failure: { message } },
  };
}

export function buildSessionReadyEvent(context: SessionContext): InternalEvent {
  const payload: Record<string, unknown> = {
    runtime_instance_id: context.runtimeInstanceId,
  };
  if (context.clientSessionKey) payload.client_session_key = context.clientSessionKey;
  if (context.clientSessionFile) payload.client_session_file = context.clientSessionFile;
  if (context.clientSessionDir) payload.client_session_dir = context.clientSessionDir;
  if (context.clientCwd) payload.client_cwd = context.clientCwd;

  return {
    event_id: `evt_${randomUUID()}`,
    session_id: context.sessionId,
    turn_id: null,
    source: "agent_client",
    client_type: "pi",
    type: "session.ready",
    time: new Date().toISOString(),
    seq: null,
    payload,
  };
}
