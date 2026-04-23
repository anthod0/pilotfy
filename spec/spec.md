**统一 Coding Agent Headless 接入层 设计文档**

**版本**：v0.2（融入 B/S 架构要求）  
**日期**：2026-04-22  
**核心一致点**（已确认）：

1. 使用 tmux 控制进程（runtime 层）
2. 实现进程池优化（session 重用 + pre-warmed pool）
3. 监听 agent client 原生 log file 读取输出（薄适配层）
4. 整个工具作为一个**外部控制层**

**新增产品形态要求**：

- **B/S 架构**（Browser/Server）
- Agent Headless 控制层与所有 agent client 安装在**同一台设备/服务器**上
- 对外暴露统一 **REST/gRPC/WebSocket API**
- 提供 **Web UI** 用于人类交互（监控、操作、human-in-the-loop）

### 1. 整体产品形态与部署

- **部署方式**：单机部署（控制层 + tmux + 所有 agent client 运行在同一服务器）
- **访问方式**：
  - Web UI（浏览器访问，支持手机/平板，移动端友好）
  - API（供上层 Orchestrator 或其他系统调用）
- **架构分层**：

```
Web UI (React/Vue/Svelte + WebSocket)
    ↓ (实时更新)
Control Plane (Backend Server)
    ├── Session & Process Pool Manager (tmux)
    ├── Thin Log Adapter Layer（发现 + 监听 client 原生 log 文件）
    ├── Message History & Inbox Queue (SQLite + JSONL)
    ├── Operation Executor (interrupt / enqueue / insert)
    └── API Layer (REST + WebSocket)
        │
        └─ Runtime: tmux sessions + git worktree 隔离
```

- **技术栈建议**（平衡开发速度与性能）：
  - Backend：Rust（推荐，性能高、并发好）或 Python FastAPI + libtmux
  - Frontend：React + Tailwind + shadcn/ui（或 Vue），支持实时 terminal preview（xterm.js）
  - 数据库：SQLite（主状态）+ JSONL（git-backed 历史，可 compaction）
  - 实时：WebSocket（状态更新、日志流、操作反馈）
  - 监控：watchdog / inotify 监听 log 文件变化

### 2. 核心功能与流程

#### 2.1 tmux Runtime + 进程池

- 每个 agent/task 一个独立 tmux session（支持多 panes）
- 进程池：hash-based 重用（work_dir + agent_type hash）、pre-warmed idle sessions
- Acquire/Release 机制 + 资源上限控制
- pipe-pane 辅助日志（ansifilter 清理），capture-pane 用于屏幕快照

#### 2.2 输出读取（薄适配层）

- 不修改 client 默认日志位置
- 按 agent_type 自动 discover log 文件：
  - Aider：`.aider.chat.history.md`
  - Claude Code：`~/.claude/projects/<hash>/*.jsonl`
  - Cursor 等：对应默认路径
  - Fallback：tmux cleaned log
- 监听：文件变更事件 → 解析 → 更新结构化历史（List[Message]）
- 解决 TUI 噪声、隐藏、截断问题，以 client 原生文件为 truth source

#### 2.3 消息历史与队列

- **History**：外部 SQLite + JSONL（支持查询、compaction）
- **Inbox Queue**：per-agent JSON 文件（enqueue、优先级、work claiming）
- 支持中断（tmux C-c + 状态更新）、插入（写 inbox）、resume

#### 2.4 Web UI 功能（人类交互）

- Agent 列表 + 状态概览（running/idle/error/heartbeat）
- 实时日志查看（cleaned 输出流）
- Terminal preview（可选 xterm.js 嵌入当前 pane）
- 操作面板：enqueue 新任务、interrupt、insert 消息、attach tmux、release to pool
- 多 agent 并行监控、git worktree 切换、历史查询
- 移动端友好（响应式设计）
- 可选：dashboard 统计（资源占用、任务完成率等）

#### 2.5 对外 API

- REST：`/agents`（CRUD）、`/agents/{id}/history`、`/agents/{id}/enqueue`、`/agents/{id}/interrupt`
- WebSocket：实时推送状态变更、新日志、idle 事件
- 认证：可选 API Key / JWT（本地部署可简化）

### 3. 关键非功能要求

- **规模**：支持几十个 agent 并发（硬件建议 32GB+ RAM、多核）
- **持久性**：tmux detach/reattach + resurrect，支持服务器重启恢复
- **可靠性**：外部状态机主导，log 文件为主通道，tmux 为辅助
- **可扩展**：薄适配层易新增 client 类型
- **安全性**：本地运行，控制对敏感操作的审批（human-in-the-loop）

### 4. 可参考的开源项目（2026 年最新，重点带 Web UI 或类似 B/S 特性）

- **AgentDock**（<https://github.com/vishalnarkhede/agentdock）：移动友好> Web dashboard，tmux + git worktree 隔离，支持并行 Claude/Cursor 等 agent，从浏览器创建 session、观看实时输出、输入、切换 agent。
- **AI Maestro**（23blocks-OS/ai-maestro）：浏览器-based dashboard，agent-agnostic，支持 OpenClaw/tmux auto-discovery，集中监控多种 agent。
- **ai-beacon**（manusa/ai-beacon）：实时 dashboard，用于监控并行 AI coding agents，解决跨设备可见性问题。
- **Agent Conductor**（gaurav-yadav/agent-conductor）：CLI-first 但带 Web dashboard（/dashboard），inbox messaging + tmux supervisor。
- **IttyBitty**（adamwulf/ittybitty）：管理多 Claude Code 实例，带 dashboard，可 spawn sub-agents in tmux。
- **Claude Code Agent Farm**（Dicklesworthstone/claude_code_agent_farm）：并行 farm + 实时监控 dashboard（tmux 内），适合大规模参考。
- **Agent Hand**（Rust TUI）：虽是 TUI，但 session 管理极强，可作为 Web UI 的后端参考。
- **其他**：Corral（agent-coral）、Tmux-Orchestrator、Gas Town（Steve Yegge，tmux 重度使用，可扩展 Web）、dmux、kage 等。

这些项目大多已验证 tmux + log 监听 + orchestrator 模式，**AgentDock** 和 **AI Maestro** 最接近你想要的 Web UI 产品形态，可直接 fork 或借鉴前端 + API 设计。

### 5. 实施建议与下一步

- **原型路径**：先用 Python + FastAPI + libtmux 快速验证（Session Pool + Log Adapter + 简单 Web UI），再考虑 Rust 重构性能。
- **Web UI 实现**：用 xterm.js 实现终端预览，WebSocket 推送日志/状态。
- **风险**：Claude Code .jsonl 文件定位需可靠 hash 匹配；大规模时监控 tmux server 资源。
