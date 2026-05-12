import { readFileSync } from 'node:fs';
import test from 'node:test';
import assert from 'node:assert/strict';

const createTaskForm = readFileSync(new URL('./CreateTaskForm.svelte', import.meta.url), 'utf8');
const apiClient = readFileSync(new URL('../../api/client.ts', import.meta.url), 'utf8');
const taskStore = readFileSync(new URL('../../stores/tasks.ts', import.meta.url), 'utf8');
const apiTypes = readFileSync(new URL('../../api/types.ts', import.meta.url), 'utf8');

test('create task form exposes a DAG task mode that defaults to pi and requires workspace', () => {
  assert.match(createTaskForm, /taskMode\s*=\s*'normal'/);
  assert.match(createTaskForm, /value="dag"/);
  assert.match(createTaskForm, /DAG task/);
  assert.match(createTaskForm, /clientType\s*=\s*'pi'/);
  assert.match(createTaskForm, /taskMode\s*===\s*'dag'[\s\S]*!workspacePath/);
  assert.match(createTaskForm, /createDagTask/);
});

test('web api and task store call the external DAG task endpoint and expose planning turn result', () => {
  assert.match(apiTypes, /interface CreateDagTaskResult[\s\S]*planning_turn/);
  assert.match(apiClient, /createDagTask\(input: CreateTaskInput\): Promise<CreateDagTaskResult>/);
  assert.match(apiClient, /'\/dag-tasks'/);
  assert.match(taskStore, /createDagTask\(input: CreateTaskInput\): Promise<CreateDagTaskResult>/);
  assert.match(taskStore, /apiCreateDagTask/);
});
