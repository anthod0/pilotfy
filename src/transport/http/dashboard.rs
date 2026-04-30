use axum::response::Html;

pub async fn dashboard() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

const DASHBOARD_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>llmparty Dashboard</title>
  <style>
    :root { color-scheme: light dark; --accent: #4f46e5; --muted: #6b7280; --border: #d1d5db; }
    * { box-sizing: border-box; }
    body { margin: 0; font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; background: #f8fafc; color: #111827; }
    header { padding: 1rem 1.5rem; background: #111827; color: white; display: flex; align-items: center; justify-content: space-between; gap: 1rem; }
    header h1 { margin: 0; font-size: 1.25rem; }
    main { display: grid; grid-template-columns: 22rem minmax(0, 1fr); gap: 1rem; padding: 1rem; }
    section, aside, .card { background: white; border: 1px solid var(--border); border-radius: .75rem; padding: 1rem; box-shadow: 0 1px 2px rgb(0 0 0 / .04); }
    aside { align-self: start; position: sticky; top: 1rem; }
    h2, h3 { margin-top: 0; }
    label { display: block; margin: .75rem 0 .25rem; font-weight: 600; }
    input, textarea, select { width: 100%; padding: .55rem; border: 1px solid var(--border); border-radius: .5rem; font: inherit; background: white; color: inherit; }
    textarea { min-height: 5rem; resize: vertical; }
    button { border: 0; border-radius: .5rem; padding: .55rem .8rem; background: var(--accent); color: white; font-weight: 700; cursor: pointer; margin: .25rem .25rem .25rem 0; }
    button.secondary { background: #64748b; }
    button.danger { background: #dc2626; }
    button:disabled { opacity: .5; cursor: not-allowed; }
    .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr)); gap: 1rem; }
    .list { display: grid; gap: .5rem; }
    .item { border: 1px solid var(--border); border-radius: .5rem; padding: .65rem; cursor: pointer; }
    .item:hover, .item.active { border-color: var(--accent); background: #eef2ff; }
    .muted { color: var(--muted); }
    .row { display: flex; gap: .5rem; align-items: center; flex-wrap: wrap; }
    pre { white-space: pre-wrap; word-break: break-word; background: #0f172a; color: #e2e8f0; padding: .75rem; border-radius: .5rem; max-height: 24rem; overflow: auto; }
    .status { min-height: 1.5rem; font-weight: 600; }
    @media (prefers-color-scheme: dark) {
      body { background: #020617; color: #e5e7eb; }
      section, aside, .card { background: #111827; }
      input, textarea, select { background: #020617; }
      .item:hover, .item.active { background: #1e1b4b; }
    }
    @media (max-width: 800px) { main { grid-template-columns: 1fr; } aside { position: static; } }
  </style>
</head>
<body>
  <header>
    <h1>llmparty Dashboard</h1>
    <span class="muted">Minimal Web Control Panel</span>
  </header>
  <main>
    <aside>
      <h2>Connection</h2>
      <label for="token">API token</label>
      <input id="token" type="password" autocomplete="off" placeholder="Bearer token for /external/v1">
      <button id="save-token">Save token</button>
      <button id="refresh" class="secondary">Refresh sessions</button>
      <p id="status" class="status"></p>

      <h2>Sessions</h2>
      <div id="sessions" class="list muted">No sessions loaded.</div>
    </aside>

    <section>
      <div class="grid">
        <div class="card">
          <h2>Create session</h2>
          <label for="client-type">Client type</label>
          <select id="client-type"><option value="generic">generic</option><option value="pi">pi</option></select>
          <label for="workspace">Workspace</label>
          <input id="workspace" placeholder="/path/to/workspace">
          <label for="initial-task">Initial task input</label>
          <textarea id="initial-task" placeholder="Optional initial task"></textarea>
          <button id="create-session">Create session</button>
        </div>

        <div class="card">
          <h2>Selected session</h2>
          <pre id="session-detail">Select a session.</pre>
          <div class="row">
            <button id="interrupt-session" class="secondary">Interrupt</button>
            <button id="restart-session" class="secondary">Restart</button>
            <button id="terminate-session" class="danger">Terminate</button>
          </div>
        </div>
      </div>

      <div class="grid">
        <div class="card">
          <h2>Submit turn</h2>
          <label for="turn-input">Turn input</label>
          <textarea id="turn-input" placeholder="Task for the selected session"></textarea>
          <p id="active-turn" class="muted">Active turn: none</p>
          <button id="submit-turn">Submit turn</button>
          <h3>Latest reply</h3>
          <pre id="turn-output">No turn output yet.</pre>
          <h3>Turn history</h3>
          <div id="turns" class="list muted">Select a session.</div>
        </div>

        <div class="card">
          <h2>Event timeline</h2>
          <button id="load-events" class="secondary">Load events</button>
          <div id="events" class="list muted">Select a session.</div>
        </div>
      </div>

      <div class="card">
        <h2>Artifact browser</h2>
        <button id="discover-artifacts" class="secondary">Discover artifacts</button>
        <button id="load-artifacts" class="secondary">Load artifacts</button>
        <div id="artifacts" class="list muted">Select a session.</div>
        <h3>Artifact content</h3>
        <pre id="artifact-content">Select an artifact.</pre>
      </div>
    </section>
  </main>

  <script>
    const $ = (id) => document.getElementById(id);
    let selectedSessionId = null;
    let selectedSession = null;
    let eventAbort = null;
    let lastEventId = null;

    $('token').value = localStorage.getItem('llmparty.externalApiToken') || '';
    $('save-token').onclick = () => {
      localStorage.setItem('llmparty.externalApiToken', $('token').value.trim());
      setStatus('API token saved.');
    };
    $('refresh').onclick = loadSessions;
    $('create-session').onclick = createSession;
    $('submit-turn').onclick = submitTurn;
    $('load-events').onclick = loadEvents;
    $('load-artifacts').onclick = loadArtifacts;
    $('discover-artifacts').onclick = discoverArtifacts;
    $('interrupt-session').onclick = () => sessionAction('interrupt', 'POST');
    $('restart-session').onclick = () => sessionAction('restart', 'POST');
    $('terminate-session').onclick = () => sessionAction('', 'DELETE');

    function token() { return $('token').value.trim() || localStorage.getItem('llmparty.externalApiToken') || ''; }
    function headers(json = false) {
      const result = { 'Authorization': `Bearer ${token()}`, 'Idempotency-Key': crypto.randomUUID() };
      if (json) result['Content-Type'] = 'application/json';
      return result;
    }
    function setStatus(message, error = false) {
      $('status').textContent = message;
      $('status').style.color = error ? '#dc2626' : '#16a34a';
    }
    function showJson(node, value) { node.textContent = JSON.stringify(value, null, 2); }
    async function request(path, options = {}) {
      const response = await fetch(path, options);
      const text = await response.text();
      const body = text ? JSON.parse(text) : null;
      if (!response.ok || (body && body.error)) {
        throw new Error(body?.error?.message || `${response.status} ${response.statusText}`);
      }
      return body;
    }

    async function loadSessions() {
      try {
        const body = await request('/external/v1/sessions', { headers: headers() });
        renderSessions(body.data.sessions || []);
        setStatus('Sessions loaded.');
      } catch (error) { setStatus(error.message, true); }
    }
    function renderSessions(sessions) {
      const root = $('sessions');
      root.className = 'list';
      root.innerHTML = '';
      if (!sessions.length) { root.className = 'list muted'; root.textContent = 'No sessions.'; return; }
      for (const session of sessions) {
        const item = document.createElement('div');
        item.className = `item ${session.session_id === selectedSessionId ? 'active' : ''}`;
        item.innerHTML = `<strong>${session.session_id}</strong><br><span class="muted">${session.client_type} · ${session.state}</span>`;
        item.onclick = () => selectSession(session.session_id);
        root.appendChild(item);
      }
    }
    async function selectSession(sessionId) {
      selectedSessionId = sessionId;
      try {
        const body = await request(`/external/v1/sessions/${sessionId}`, { headers: headers() });
        selectedSession = body.data.session;
        showJson($('session-detail'), selectedSession);
        updateBusyState();
        await Promise.all([loadTurns(), loadEvents(), loadArtifacts()]);
        openEventStream();
      } catch (error) { setStatus(error.message, true); }
    }
    async function createSession() {
      try {
        const payload = { client_type: $('client-type').value, workspace: $('workspace').value || '.', initial_task: $('initial-task').value ? { input: $('initial-task').value } : null };
        const body = await request('/external/v1/sessions', { method: 'POST', headers: headers(true), body: JSON.stringify(payload) });
        selectedSessionId = body.data.session.session_id;
        await loadSessions();
        await selectSession(selectedSessionId);
        setStatus('Session created.');
      } catch (error) { setStatus(error.message, true); }
    }
    async function sessionAction(action, method) {
      if (!selectedSessionId) return setStatus('Select a session first.', true);
      try {
        const suffix = action ? `/${action}` : '';
        await request(`/external/v1/sessions/${selectedSessionId}${suffix}`, { method, headers: headers() });
        await selectSession(selectedSessionId);
        setStatus(`Session ${action || 'terminated'}.`);
      } catch (error) { setStatus(error.message, true); }
    }
    async function submitTurn() {
      if (!selectedSessionId) return setStatus('Select a session first.', true);
      try {
        await request(`/external/v1/sessions/${selectedSessionId}/turns`, { method: 'POST', headers: headers(true), body: JSON.stringify({ input: $('turn-input').value }) });
        $('turn-input').value = '';
        await selectSession(selectedSessionId);
        setStatus('Turn submitted. Waiting for pi hook events.');
      } catch (error) { setStatus(error.message, true); }
    }
    async function loadTurns() {
      if (!selectedSessionId) return;
      const body = await request(`/external/v1/sessions/${selectedSessionId}/turns`, { headers: headers() });
      const turns = body.data.turns || [];
      renderList($('turns'), turns, (turn) => `<strong>${turn.turn_id}</strong><br><span class="muted">${turn.state}</span><br>${escapeHtml(turn.output?.summary || '')}`);
      const latestOutput = [...turns].reverse().find((turn) => turn.output?.summary);
      if (latestOutput) $('turn-output').textContent = latestOutput.output.summary;
    }
    async function loadEvents() {
      if (!selectedSessionId) return;
      const body = await request(`/external/v1/sessions/${selectedSessionId}/events`, { headers: headers() });
      renderList($('events'), body.data.events || [], (event) => `<strong>${event.type}</strong><br><span class="muted">${event.event_id} · ${event.time}</span>`);
    }
    async function discoverArtifacts() {
      if (!selectedSessionId) return setStatus('Select a session first.', true);
      try {
        await request(`/external/v1/sessions/${selectedSessionId}/artifacts/discover`, { method: 'POST', headers: headers() });
        await loadArtifacts();
        setStatus('Artifacts discovered.');
      } catch (error) { setStatus(error.message, true); }
    }
    async function loadArtifacts() {
      if (!selectedSessionId) return;
      const body = await request(`/external/v1/sessions/${selectedSessionId}/artifacts`, { headers: headers() });
      renderList($('artifacts'), body.data.artifacts || [], (artifact) => `<strong>${artifact.name || artifact.artifact_id}</strong><br><span class="muted">${artifact.kind || 'file'} · ${artifact.artifact_id}</span>`, loadArtifactContent);
    }
    async function loadArtifactContent(artifact) {
      try {
        const body = await request(`/external/v1/artifacts/${artifact.artifact_id}/content`, { headers: headers() });
        showJson($('artifact-content'), body.data);
      } catch (error) { setStatus(error.message, true); }
    }
    function updateBusyState() {
      const activeTurnId = selectedSession?.current_turn_id;
      const busy = Boolean(activeTurnId);
      $('active-turn').textContent = busy
        ? `Active turn: ${activeTurnId}. This session is busy; wait for turn.completed or turn.failed from the pi hook before submitting the next turn.`
        : 'Active turn: none';
      $('submit-turn').disabled = busy;
      if (busy) setStatus('session is busy; if it stays running, check .llmparty/pi-hook.log and Internal Event API reporting.', true);
    }

    function openEventStream() {
      if (!selectedSessionId) return;
      if (eventAbort) eventAbort.abort();
      eventAbort = new AbortController();
      lastEventId = null;
      consumeEventStream(eventAbort.signal).catch((error) => {
        if (error.name !== 'AbortError') setStatus(`Event stream stopped: ${error.message}`, true);
      });
    }

    async function consumeEventStream(signal) {
      const url = `/external/v1/sessions/${selectedSessionId}/events/stream${lastEventId ? `?after=${encodeURIComponent(lastEventId)}` : ''}`;
      const response = await fetch(url, { headers: headers(), signal });
      if (!response.ok) throw new Error(`${response.status} ${response.statusText}`);
      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let buffer = '';
      while (true) {
        const { value, done } = await reader.read();
        if (done) break;
        buffer += decoder.decode(value, { stream: true });
        const frames = buffer.split('\n\n');
        buffer = frames.pop() || '';
        for (const frame of frames) handleSseFrame(frame);
      }
    }

    function handleSseFrame(frame) {
      let id = null;
      let data = '';
      for (const line of frame.split('\n')) {
        if (line.startsWith('id:')) id = line.slice(3).trim();
        if (line.startsWith('data:')) data += line.slice(5).trim();
      }
      if (id) lastEventId = id;
      if (!data) return;
      const event = JSON.parse(data);
      appendEvent(event);
      if (event.type === 'turn.output') $('turn-output').textContent = event.payload?.output?.summary || JSON.stringify(event.payload);
      if (event.type === 'turn.completed' || event.type === 'turn.failed') selectSession(selectedSessionId);
    }

    function appendEvent(event) {
      const root = $('events');
      if (root.classList.contains('muted')) { root.className = 'list'; root.innerHTML = ''; }
      const el = document.createElement('div');
      el.className = 'item';
      el.innerHTML = `<strong>${escapeHtml(event.type)}</strong><br><span class="muted">${escapeHtml(event.event_id)} · ${escapeHtml(event.time)}</span>`;
      root.prepend(el);
    }

    function escapeHtml(value) {
      return String(value).replace(/[&<>"']/g, (ch) => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[ch]));
    }

    function renderList(root, items, html, onClick) {
      root.className = 'list';
      root.innerHTML = '';
      if (!items.length) { root.className = 'list muted'; root.textContent = 'No records.'; return; }
      for (const item of items) {
        const el = document.createElement('div');
        el.className = 'item';
        el.innerHTML = html(item);
        if (onClick) el.onclick = () => onClick(item);
        root.appendChild(el);
      }
    }

    loadSessions();
  </script>
</body>
</html>
"#;
