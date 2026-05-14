import assert from 'node:assert/strict';
import { test } from 'node:test';
import { selectCurrentTurnOutput } from './currentTurnOutput.ts';

const turn = (overrides) => ({
  turn_id: 'turn-old',
  session_id: 'session-1',
  state: 'completed',
  input: { summary: 'old input' },
  output: { summary: 'old output' },
  failure: null,
  created_at: '2026-01-01T00:00:00Z',
  started_at: '2026-01-01T00:00:01Z',
  completed_at: '2026-01-01T00:00:02Z',
  metadata: {},
  ...overrides,
});

test('selects the session current turn even when it is not the newest turn', () => {
  const selected = selectCurrentTurnOutput(
    { current_turn_id: 'turn-current' },
    [
      turn({ turn_id: 'turn-current', state: 'running', output: { summary: 'current output' }, created_at: '2026-01-01T00:00:00Z' }),
      turn({ turn_id: 'turn-newer', output: { summary: 'newer completed output' }, created_at: '2026-01-01T00:05:00Z' }),
    ],
  );

  assert.equal(selected?.turn.turn_id, 'turn-current');
  assert.equal(selected?.title, 'Current turn output');
  assert.equal(selected?.outputSummary, 'current output');
});

test('falls back to the newest turn when the session has no current turn', () => {
  const selected = selectCurrentTurnOutput(
    { current_turn_id: null },
    [
      turn({ turn_id: 'turn-a', created_at: '2026-01-01T00:00:00Z' }),
      turn({ turn_id: 'turn-b', created_at: '2026-01-01T00:10:00Z', output: { summary: 'latest output' } }),
    ],
  );

  assert.equal(selected?.turn.turn_id, 'turn-b');
  assert.equal(selected?.title, 'Latest turn output');
  assert.equal(selected?.outputSummary, 'latest output');
});
