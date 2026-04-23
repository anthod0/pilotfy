# Control Plane Internal Event API v1

- Status: Draft
- Date: 2026-04-23
- Scope: MVP
- Target: 单机 Control Plane（Rust Backend）

## 1. 目标

本文档定义 Control Plane 与各类 agent client hook/helper 之间的内部事件上报接口。

该接口用于：

- 接收 client hook 上报的事实事件
- 在 Control Plane 内部统一维护 session / turn 状态机
- 为后续外部查询 API、Web UI、历史存储提供一致的数据基础

本文档不覆盖：

- Web UI 设计
- 对外 REST API 细节
- gRPC / WebSocket 协议
- client 内部 hook 实现方式

---

## 2. 设计原则

### 2.1 Control Plane 是唯一状态源

插件或 hook 不直接维护平台状态，只上报事件。

- 插件负责：上报事实
- Control Plane 负责：聚合、推导、存储、查询

即：

- 插件不上报 “最终状态” 作为真相
- 插件上报 “发生了什么事件”
- Control Plane 根据事件更新统一状态机

### 2.2 事件优于状态覆盖

内部接口采用 **event ingest**，而不是 `update_state`。

原因：

- 事件可审计
- 事件可重放
- 事件可做幂等去重
- 事件可关联具体 turn / approval
- 事件更适合跨 client 统一抽象

### 2.3 单机 MVP，统一使用 HTTP

MVP 阶段内部接口采用 HTTP/JSON。

- 不使用 gRPC
- 不使用 WebSocket
- CLI 若存在，也只是 HTTP 的 wrapper

### 2.4 插件为 hook-helper，而不是常驻状态服务

每个 client 的插件仅依赖其 hook 机制，在 hook 触发时发起一次 HTTP 请求上报事件。

### 2.5 session_id 由 Control Plane 分配

Control Plane 创建 session 时生成：

- `session_id`
- `plugin_token`
- 内部事件接口地址

这些信息通过环境变量或启动参数注入给 client hook/helper。

---

## 3. 总体架构

```text
client hook/helper
    ↓ HTTP POST
Internal Event API
    ↓
Event Ingest Pipeline
    ├── Auth
    ├── Validation
    ├── Idempotency / Seq Check
    ├── Event Store
    ├── Session State Reducer
    ├── Turn State Reducer
    └── Side Effects (history / approval / metrics)
    ↓
SQLite
```

### 3.1 边界划分

#### 插件 / hook 负责

- 从 client hook 中拿到事件上下文
- 读取 `session_id` / `token` / endpoint
- 发送 HTTP 请求

#### Control Plane 负责

- 鉴权
- 事件校验
- 顺序性与幂等处理
- session 状态机维护
- turn 状态机维护
- 历史持久化
- 对外查询

---

## 4. 协议概览

MVP 内部接口建议分为两类：

1. **事件上报**
2. **心跳上报**

### 4.1 事件上报

```http
POST /internal/v1/events
Authorization: Bearer <plugin_token>
Content-Type: application/json
```

### 4.2 心跳上报

```http
POST /internal/v1/heartbeat
Authorization: Bearer <plugin_token>
Content-Type: application/json
```

---

## 5. 认证模型

### 5.1 session-scoped token

MVP 不单独设计插件注册协议。

创建 session 时，Control Plane 为该 session 生成独立 token。

示例：

```json
{
  "session_id": "sess_001",
  "hook": {
    "endpoint": "http://127.0.0.1:8080/internal/v1/events",
    "heartbeat_endpoint": "http://127.0.0.1:8080/internal/v1/heartbeat",
    "token": "cp_sess_token_xxx"
  }
}
```

### 5.2 校验要求

Control Plane 在处理内部请求时至少校验：

- token 是否有效
- token 是否属于该 `session_id`
- token 是否允许对应 `client_type`

---

## 6. 事件上报接口

## 6.1 Request

```http
POST /internal/v1/events
Authorization: Bearer cp_sess_token_xxx
Content-Type: application/json
```

```json
{
  "event_id": "evt_000001",
  "seq": 1,
  "time": "2026-04-23T12:00:00Z",
  "session_id": "sess_001",
  "turn_id": "turn_001",
  "client_type": "claude_code",
  "type": "turn.started",
  "payload": {
    "source": "user_input"
  }
}
```

## 6.2 Response

```json
{
  "ok": true,
  "accepted": true,
  "state_version": 12,
  "last_event_seq": 1
}
```

## 6.3 字段定义

### `event_id`

- 类型：string
- 要求：全局唯一
- 用途：幂等去重

### `seq`

- 类型：integer
- 要求：在同一 `session_id` 内单调递增
- 用途：检测乱序、重复、丢失

### `time`

- 类型：RFC3339 timestamp
- 含义：hook 触发时间，而不是服务端接收时间

### `session_id`

- 类型：string
- 含义：Control Plane 分配的 session 标识

### `turn_id`

- 类型：string | null
- 含义：事件所属的一轮交互
- 说明：session 级事件可为空

### `client_type`

- 类型：enum string
- MVP 建议值：
  - `pi`
  - `claude_code`
  - `codex`

### `type`

- 类型：enum string
- 含义：事件类型

### `payload`

- 类型：object
- 含义：事件附加信息
- 说明：允许不同 client 扩展，但字段语义需保持稳定

---

## 7. 心跳接口

## 7.1 Request

```http
POST /internal/v1/heartbeat
Authorization: Bearer cp_sess_token_xxx
Content-Type: application/json
```

```json
{
  "session_id": "sess_001",
  "client_type": "claude_code",
  "time": "2026-04-23T12:00:05Z",
  "meta": {
    "pid": 12345
  }
}
```

## 7.2 Response

```json
{
  "ok": true
}
```

## 7.3 作用

心跳不直接改变 turn 状态，但可用于：

- 判断 session 是否仍存活
- 判断 hook/helper 是否失联
- 辅助 session 健康检查
- 辅助超时与恢复逻辑

---

## 8. 事件类型定义

MVP 建议统一事件类型如下。

## 8.1 Session 级事件

### `session.started`

含义：session 对应的 client 已启动。

示例 payload：

```json
{
  "pid": 12345
}
```

### `session.ready`

含义：client 已进入可交互状态。

### `session.idle`

含义：client 当前空闲，正在等待下一轮输入。

### `session.exited`

含义：client 进程已退出。

示例 payload：

```json
{
  "exit_code": 0
}
```

### `session.error`

含义：session 级异常。

示例 payload：

```json
{
  "code": "startup_failed",
  "message": "failed to initialize client"
}
```

## 8.2 Turn 级事件

### `turn.started`

含义：某轮交互开始处理。

示例 payload：

```json
{
  "source": "user_input"
}
```

### `turn.output`

含义：该轮交互产生输出。

示例 payload：

```json
{
  "chunk": "Planning implementation..."
}
```

说明：MVP 阶段 `turn.output` 可只作为历史和活跃性的辅助信号。

### `turn.completed`

含义：该轮交互完成，client 回到稳定态。

### `turn.failed`

含义：该轮交互失败。

示例 payload：

```json
{
  "code": "tool_error",
  "message": "command execution failed"
}
```

### `turn.interrupted`

含义：该轮交互被中断。

## 8.3 Approval 级事件

### `approval.requested`

含义：当前 turn 进入等待权限确认状态。

示例 payload：

```json
{
  "kind": "command_execution",
  "message": "Run git status?"
}
```

### `approval.resolved`

含义：该审批已被处理。

示例 payload：

```json
{
  "decision": "approved"
}
```

允许值建议：

- `approved`
- `denied`
- `cancelled`

## 8.4 系统级事件

### `hook.error`

含义：hook/helper 在上报链路或解析链路中遇到异常。

---

## 9. 统一状态机

Control Plane 内部维护两套状态：

1. `SessionState`
2. `TurnState`

---

## 9.1 SessionState

### 枚举

- `starting`
- `idle`
- `busy`
- `waiting_approval`
- `interrupted`
- `exited`
- `error`

### 状态转换建议

| 事件 | 状态变化 |
|---|---|
| `session.started` | `starting` |
| `session.ready` | `idle` |
| `turn.started` | `busy` |
| `approval.requested` | `waiting_approval` |
| `approval.resolved` | `busy` |
| `turn.completed` | `idle` |
| `turn.interrupted` | `interrupted` |
| `session.exited` | `exited` |
| `session.error` | `error` |

说明：

- `session.idle` 可作为显式纠偏事件，将 session 置为 `idle`
- Control Plane 也可结合自己的 interrupt 操作记录，将某些 turn/session 映射为 `interrupted`

---

## 9.2 TurnState

### 枚举

- `queued`
- `started`
- `streaming`
- `waiting_approval`
- `completed`
- `failed`
- `interrupted`

### 状态转换建议

| 事件 | 状态变化 |
|---|---|
| 外部 enqueue | `queued` |
| `turn.started` | `started` |
| `turn.output` | `streaming` |
| `approval.requested` | `waiting_approval` |
| `approval.resolved` | `started` 或 `streaming` |
| `turn.completed` | `completed` |
| `turn.failed` | `failed` |
| `turn.interrupted` | `interrupted` |

说明：

- `approval.resolved` 后恢复到 `started` 还是 `streaming`，可以由 reducer 根据此前状态决定
- 若某 client 不上报 `turn.output`，仍可仅依赖 `turn.started` / `turn.completed` 工作

---

## 10. 幂等与顺序性

## 10.1 幂等

以 `event_id` 作为幂等键。

规则：

- 若 `event_id` 已存在，则不重复应用 reducer
- 可以返回 `accepted: true` 或 `accepted: false`，但需明确表示该请求已被识别为重复

建议返回：

```json
{
  "ok": true,
  "accepted": false,
  "reason": "duplicate_event",
  "state_version": 12,
  "last_event_seq": 1
}
```

## 10.2 顺序性

以 `(session_id, seq)` 作为顺序约束。

规则建议：

- `seq == last_seq + 1`：正常接收
- `seq <= last_seq`：视为重复或乱序旧事件
- `seq > last_seq + 1`：记录 gap，并返回可观测错误或 warning

MVP 阶段建议：

- 先接收事件
- 标记 `seq_gap`
- 不立即实现复杂补偿机制

---

## 11. 错误响应建议

### 401 Unauthorized

- token 不存在
- token 无效

### 403 Forbidden

- token 与 `session_id` 不匹配

### 400 Bad Request

- JSON 不合法
- 缺少必要字段
- 枚举值非法

### 409 Conflict

- `seq` 明显冲突且不允许接受

MVP 也可以简化为：

- 认证错误：401/403
- 校验错误：400
- 其余服务端异常：500

---

## 12. SQLite 存储建议

MVP 至少维护三张核心表。

## 12.1 `sessions`

用于存当前 session 聚合状态。

建议字段：

- `session_id` TEXT PRIMARY KEY
- `client_type` TEXT NOT NULL
- `state` TEXT NOT NULL
- `current_turn_id` TEXT NULL
- `last_event_seq` INTEGER NOT NULL DEFAULT 0
- `last_event_at` TEXT NULL
- `last_heartbeat_at` TEXT NULL
- `pid` INTEGER NULL
- `created_at` TEXT NOT NULL
- `updated_at` TEXT NOT NULL

## 12.2 `turns`

用于存某一轮交互的聚合状态。

建议字段：

- `turn_id` TEXT PRIMARY KEY
- `session_id` TEXT NOT NULL
- `state` TEXT NOT NULL
- `started_at` TEXT NULL
- `completed_at` TEXT NULL
- `failed_at` TEXT NULL
- `interrupted_at` TEXT NULL
- `created_at` TEXT NOT NULL
- `updated_at` TEXT NOT NULL

索引建议：

- `(session_id, created_at)`

## 12.3 `events`

用于存原始事件流。

建议字段：

- `event_id` TEXT PRIMARY KEY
- `session_id` TEXT NOT NULL
- `turn_id` TEXT NULL
- `seq` INTEGER NOT NULL
- `client_type` TEXT NOT NULL
- `type` TEXT NOT NULL
- `time` TEXT NOT NULL
- `payload_json` TEXT NOT NULL
- `received_at` TEXT NOT NULL

索引建议：

- `(session_id, seq)` UNIQUE
- `(session_id, time)`
- `(turn_id)`

---

## 13. API 示例

## 13.1 turn started

```http
POST /internal/v1/events
Authorization: Bearer cp_sess_token_xxx
```

```json
{
  "event_id": "evt_1001",
  "seq": 10,
  "time": "2026-04-23T12:00:00Z",
  "session_id": "sess_001",
  "turn_id": "turn_010",
  "client_type": "pi",
  "type": "turn.started",
  "payload": {
    "source": "user_input"
  }
}
```

## 13.2 approval requested

```json
{
  "event_id": "evt_1002",
  "seq": 11,
  "time": "2026-04-23T12:00:02Z",
  "session_id": "sess_001",
  "turn_id": "turn_010",
  "client_type": "pi",
  "type": "approval.requested",
  "payload": {
    "kind": "command_execution",
    "message": "Run git status?"
  }
}
```

## 13.3 approval resolved

```json
{
  "event_id": "evt_1003",
  "seq": 12,
  "time": "2026-04-23T12:00:05Z",
  "session_id": "sess_001",
  "turn_id": "turn_010",
  "client_type": "pi",
  "type": "approval.resolved",
  "payload": {
    "decision": "approved"
  }
}
```

## 13.4 turn completed

```json
{
  "event_id": "evt_1004",
  "seq": 13,
  "time": "2026-04-23T12:00:08Z",
  "session_id": "sess_001",
  "turn_id": "turn_010",
  "client_type": "pi",
  "type": "turn.completed",
  "payload": {}
}
```

---

## 14. MVP 范围

MVP 阶段建议只实现以下能力：

### 必做

- `POST /internal/v1/events`
- `POST /internal/v1/heartbeat`
- Bearer token 校验
- `events` / `sessions` / `turns` 三张表
- `SessionState` reducer
- `TurnState` reducer
- 幂等去重（基于 `event_id`）
- 基础顺序检查（基于 `seq`）

### 可延后

- 批量事件上报
- 插件注册协议
- snapshot 同步接口
- 复杂乱序修复
- protocol version 协商
- UDS 替代 localhost HTTP

---

## 15. 后续演进方向

### 15.1 批量 ingest

后续可支持：

```http
POST /internal/v1/events/batch
```

用于减少高频 hook 带来的请求开销。

### 15.2 协议版本化

可加入：

- header version
- body `schema_version`

### 15.3 本地 Unix Socket

若后续希望进一步强化本机安全边界，可将内部 HTTP 从 `127.0.0.1:port` 迁移到 Unix Domain Socket。

### 15.4 richer approval model

后续可细化审批：

- command execution
- file write
- network access
- destructive action

---

## 16. 最终结论

Control Plane Internal Event API v1 的 MVP 设计如下：

- 使用 **HTTP/JSON** 作为插件 hook 上报协议
- 使用 **event ingest** 作为核心语义，而不是直接状态覆盖
- 使用 **session-scoped token** 做认证
- 由 Control Plane 统一维护 `SessionState` 与 `TurnState`
- 用 SQLite 保存原始事件与聚合状态

这套设计适合当前单机 MVP，复杂度低，同时为后续扩展到更多 client 和更丰富的状态机留出了空间。
