# Control Plane 整体设计

- Status: Draft
- Date: 2026-04-23
- Scope: MVP
- Target: 单机 Control Plane（Rust Backend）

## 1. 文档目标

本文档用于定义 Control Plane 在 MVP 阶段的整体设计。

重点回答以下问题：

- Control Plane 的职责边界是什么
- 它与 agent client、hook、runtime、外部调用方之间的关系是什么
- 它内部有哪些核心子系统
- 它围绕哪些核心领域对象组织
- 它的主要数据流和状态流是什么

本文档聚焦系统整体，不深入模块级 API 细节。

---

## 2. 产品定位

Control Plane 是一个**单机运行的后端控制层**，负责统一管理多个长期驻留的 agent TUI 会话，并对外暴露标准化控制与查询能力。

它不是 agent 本体，也不是上层业务 orchestrator，而是位于两者之间的基础控制层。

其核心价值是：

- 把不同 agent client 的差异收敛到统一控制面
- 以统一的 session / turn / approval 模型对外提供能力
- 为后续的 UI、自动化调用、人工介入提供稳定基础

---

## 3. 系统边界

## 3.1 Control Plane 负责什么

### 3.1.1 会话管理

- 创建 session
- 维护 session 生命周期
- 记录 session 与 client 类型、工作目录、runtime 的绑定关系
- 为 session 分配内部身份（如 `session_id`）

### 3.1.2 运行时控制

- 通过 tmux 托管 agent client TUI 进程
- 启动、停止、定位 session 对应的 runtime
- 承接 interrupt 等控制动作

### 3.1.3 事件接入与状态聚合

- 接收来自各类 client hook/helper 的事件上报
- 将事件归一化为统一模型
- 维护 session 状态机、turn 状态机、approval 状态

### 3.1.4 持久化与查询

- 保存原始事件流
- 保存当前聚合状态
- 提供统一查询接口

### 3.1.5 外部控制接口

- 对外暴露 HTTP API
- 支持 session 管理、状态查询、审批操作、运行时控制

---

## 3.2 Control Plane 不负责什么

### 3.2.1 不负责 agent 内部逻辑

例如：

- prompt 组织
- 模型调用
- tool 执行编排
- client 内部 UI 渲染

这些都属于各自 agent client 的职责。

### 3.2.2 不负责复杂业务编排

Control Plane 不是上层 orchestrator。

它可以被 orchestrator 调用，但不应承载：

- 多步骤业务编排
- 跨系统工作流引擎
- 高层策略决策

### 3.2.3 不负责重型插件平台

MVP 阶段只需要 hook/helper 通过 HTTP 上报事件，不需要构建通用插件生态或插件运行时。

---

## 4. 设计原则

## 4.1 单机优先

MVP 阶段，以下组件都部署在同一台机器上：

- Control Plane
- tmux
- agent client
- hook/helper
- SQLite

不考虑分布式部署、跨机调度和多节点一致性。

## 4.2 Session 是一等公民

这里管理的是**长期驻留的 TUI 会话**，而不是“一次性执行完即退出的 CLI 命令”。

因此系统的核心对象不是 job，而是：

- Session
- Turn
- Event
- Approval

## 4.3 Control Plane 是唯一状态源

各 client hook 只上报事件，不维护平台状态。

统一状态由 Control Plane 内部维护：

- SessionState
- TurnState
- ApprovalState

## 4.4 事件驱动，状态派生

Control Plane 以事件作为事实输入，以状态作为派生结果。

即：

- Event = fact
- State = projection

内部所有聚合状态都应可由事件流和 reducer 推导出来。

## 4.5 客户端差异前置收敛

不同 client 的差异，原则上收敛在以下边界：

- 启动方式
- hook 机制
- 事件映射
- 可选输出来源

Control Plane 内部尽量只面对统一事件模型，而不是散落的 client 特殊逻辑。

---

## 5. 总体架构

```text
┌────────────────────────────────────────────┐
│               External Callers             │
│  - future web ui                           │
│  - orchestrator                            │
│  - cli / automation                        │
└────────────────────┬───────────────────────┘
                     │ HTTP
                     ▼
┌────────────────────────────────────────────┐
│               Control Plane                │
│--------------------------------------------│
│ API Layer                                  │
│ Session Manager                            │
│ Runtime Manager                            │
│ Internal Event Ingest                      │
│ State Reducer                              │
│ Approval Manager                           │
│ History / Event Store                      │
│ Query Layer                                │
└───────────────┬────────────────────────────┘
                │
    ┌───────────┴────────────┬───────────────────┐
    │                        │                   │
    ▼                        ▼                   ▼
 tmux session A         tmux session B      tmux session C
    │                        │                   │
    ▼                        ▼                   ▼
   pi                  claude code            codex
    │                        │                   │
    └──────── hook/helper -> HTTP events -----┘

Persistence:
- SQLite
```

---

## 6. 关键参与方与关系

## 6.1 External Callers

外部调用方包括：

- 未来的 Web UI
- 上层 orchestrator
- 自动化脚本或 CLI

它们通过外部 HTTP API 使用 Control Plane 的能力。

## 6.2 Agent Client

Agent client 是长期驻留的 TUI 程序，例如：

- pi
- claude code
- codex

它们运行在 tmux 会话中，由 Control Plane 托管，但业务行为仍由 client 自身决定。

## 6.3 Hook / Helper

每个 client 通过自身 hook 机制，在关键节点触发轻量 helper，向 Control Plane 上报事件。

hook/helper 的职责是：

- 读取 hook 上下文
- 读取 Control Plane 注入的 session 信息
- 发 HTTP 请求上报事件

它不负责平台状态管理。

## 6.4 Runtime

MVP 阶段 runtime 统一采用 tmux。

它负责承载长期 TUI 进程，并提供：

- 会话创建
- 会话定位
- 中断控制
- 基础生命周期观察

---

## 7. 核心子系统设计

## 7.1 API Layer

职责：

- 暴露 HTTP API
- 请求校验
- 鉴权
- 将请求路由到应用服务

从职责上分两类入口：

### External API

面向外部系统，提供：

- session 创建
- session 查询
- interrupt
- approval 操作
- 历史查询

### Internal Event API

面向 hook/helper，提供：

- 事件上报
- 心跳上报

---

## 7.2 Session Manager

职责：

- 创建和初始化 session
- 分配 `session_id`
- 管理 session 元数据
- 关联 client 类型、workspace、runtime、token
- 维护 session 生命周期主线

Session Manager 是系统的主入口之一，因为所有 turn、approval、event 都依附于 session。

---

## 7.3 Runtime Manager

职责：

- 启动和停止 tmux session
- 记录 session 与 tmux runtime 的绑定关系
- 承接中断等运行时控制动作
- 提供最基础的 runtime 观察能力

边界要求：

- Runtime Manager 只关心运行时资源
- 不直接决定业务状态
- 不负责 turn 完成与否的语义判断

---

## 7.4 Internal Event Ingest

职责：

- 接收 client hook 上报事件
- 校验 token
- 校验 schema
- 幂等去重
- 顺序检查
- 写入事件存储
- 驱动状态 reducer

这是系统最重要的写入入口之一。

---

## 7.5 State Reducer

职责：

- 消费统一事件模型
- 更新 SessionState
- 更新 TurnState
- 派生 approval 等待态
- 维护当前聚合视图

建议内部拆分为：

### Session Reducer

管理：

- `starting`
- `idle`
- `busy`
- `waiting_approval`
- `interrupted`
- `exited`
- `error`

### Turn Reducer

管理：

- `queued`
- `started`
- `streaming`
- `waiting_approval`
- `completed`
- `failed`
- `interrupted`

---

## 7.6 Approval Manager

职责：

- 管理当前 pending approval
- 将 approval 与 session / turn 关联
- 处理 approve / deny
- 记录 approval 生命周期

Approval 是 Control Plane 中非常关键的人机协作入口，后续可能扩展为更丰富的审批模型，因此在整体设计上单独成子系统是合理的。

---

## 7.7 History / Event Store

职责：

- 保存原始事件流
- 保存审计与调试所需的事实数据
- 为状态重建提供基础

设计原则：

- 原始事件必须保留
- 聚合状态不能替代事件流

---

## 7.8 Query Layer

职责：

- 为外部 API 提供聚合视图
- 查询 session 当前状态
- 查询 turn 历史
- 查询 approval 列表
- 查询事件流

该层的意义是将外部读模型与底层存储结构隔离，避免外部接口直接耦合数据库表结构。

---

## 8. 核心领域模型

## 8.1 Session

表示一个长期存活的 agent TUI 会话。

关键属性：

- `session_id`
- `client_type`
- `runtime_type`
- `workspace_path`
- `worktree_path`
- `state`
- `current_turn_id`
- `last_event_seq`
- `last_heartbeat_at`

## 8.2 Turn

表示 session 内的一轮交互。

关键属性：

- `turn_id`
- `session_id`
- `state`
- `source`
- `started_at`
- `completed_at`
- `failed_at`
- `interrupted_at`

## 8.3 Event

表示由 hook/helper 上报的事实事件。

关键属性：

- `event_id`
- `session_id`
- `turn_id`
- `seq`
- `type`
- `payload`
- `occurred_at`
- `received_at`

## 8.4 Approval

表示一次等待人工决策的审批请求。

关键属性：

- `approval_id`
- `session_id`
- `turn_id`
- `kind`
- `status`
- `message`
- `created_at`
- `resolved_at`
- `decision`

## 8.5 RuntimeBinding

表示 session 与底层 tmux runtime 的绑定关系。

关键属性：

- `session_id`
- `tmux_session_name`
- `pid`
- `started_at`
- `exited_at`

---

## 9. 关键数据流

## 9.1 创建会话

```text
External API -> Session Manager
             -> Runtime Manager 启动 tmux/client
             -> 生成 session_id / token
             -> 写入 sessions
             -> 返回 hook 配置
```

结果：

- 新 session 创建成功
- client 被托管到 runtime 中
- hook 有了可上报的 endpoint / token / session_id

## 9.2 事件上报

```text
hook/helper -> Internal Event API
            -> Auth / Validation
            -> Event Store
            -> State Reducer
            -> 更新 sessions / turns / approvals
```

结果：

- 原始事件被保存
- 聚合状态被推进

## 9.3 人工审批

```text
client hook -> approval.requested
            -> Approval Manager 创建 pending approval
            -> session state => waiting_approval

External API -> approve/deny
             -> Approval Manager 更新 approval
             -> Runtime Manager / client 输入通道承接后续动作
```

结果：

- 系统能可靠识别等待审批状态
- 外部系统可以对审批做显式操作

## 9.4 查询状态

```text
External API -> Query Layer
             -> 读取 sessions / turns / approvals / events
             -> 返回聚合视图
```

结果：

- 外部调用方获取统一读模型

---

## 10. 状态模型

在整体设计层面，先固定三类状态概念。

## 10.1 SessionState

- `starting`
- `idle`
- `busy`
- `waiting_approval`
- `interrupted`
- `exited`
- `error`

## 10.2 TurnState

- `queued`
- `started`
- `streaming`
- `waiting_approval`
- `completed`
- `failed`
- `interrupted`

## 10.3 ApprovalState

- `pending`
- `approved`
- `denied`
- `cancelled`

这些状态的精确转换规则由后续模块级文档定义。

---

## 11. 存储设计原则

## 11.1 SQLite 作为主存储

MVP 阶段统一使用 SQLite 作为主存储。

主要保存：

- session 当前状态
- turn 当前状态
- approval 当前状态
- 原始事件
- runtime 绑定关系

## 11.2 原始事件必须保留

保留事件流的价值包括：

- 审计
- 调试
- 故障排查
- 状态重建
- reducer 升级后的重新投影

## 11.3 聚合状态单独存储

为了保证查询效率和 API 简洁性，聚合状态应单独保存，而不是每次查询时重放全部事件。

---

## 12. 对外能力边界

整体上，Control Plane 应对外提供四类能力。

## 12.1 Session 管理

- 创建 session
- 查询 session
- 列出 session
- 终止 session

## 12.2 Turn / History 查询

- 查询当前 turn
- 查询历史 turns
- 查询事件流

## 12.3 Approval 管理

- 查询 pending approval
- approve
- deny

## 12.4 Runtime 控制

- interrupt
- attach（后续）
- resume（后续）

---

## 13. MVP 范围

为了避免整体设计过重，MVP 阶段建议只做以下范围。

## 13.1 必做

- 单机 Control Plane
- tmux runtime 管理
- session 创建与管理
- internal event ingest
- session / turn 状态机
- approval pending 管理
- SQLite 持久化
- 基础外部 HTTP 查询/控制 API

## 13.2 延后项

- 多机分布式架构
- WebSocket
- gRPC
- 通用插件注册中心
- 重型插件生态
- 多租户权限体系
- 复杂事件补偿与重排机制

---

## 14. 代码组织建议

如果按 Rust 后端组织，建议先按以下模块边界思考：

```text
control-plane/
  api/
    external/
    internal/
  application/
    session_service/
    runtime_service/
    ingest_service/
    approval_service/
    query_service/
  domain/
    session/
    turn/
    event/
    approval/
    runtime/
  reducers/
    session_reducer/
    turn_reducer/
  infrastructure/
    tmux/
    sqlite/
    auth/
    clock/
    id/
```

设计意图：

- `domain` 放核心模型和状态规则
- `application` 放用例编排
- `reducers` 放状态推进逻辑
- `infrastructure` 放外部依赖实现细节
- `api` 放 HTTP 接口适配层

---

## 15. 与模块级文档的关系

本文档属于整体设计文档。

后续可基于它继续拆分出更细的模块级文档，例如：

- Internal Event API v1
- Session / Turn / Approval 领域模型
- Runtime Manager 设计
- External API v1
- SQLite Schema 设计

这些文档都应服从本文定义的整体边界和设计原则。

---

## 16. 最终结论

Control Plane 在 MVP 阶段应被定义为：

> 一个单机、事件驱动、以 session 为中心的后端控制层；
> 它通过 tmux 管理长期驻留的 agent TUI，通过内部 HTTP 接口接收 client hook 事件，
> 并在自身内部统一维护 session、turn、approval 三类状态，对外提供控制与查询能力。

这一定义明确了它与 agent client、hook、runtime、外部调用方之间的边界，也为后续模块设计和实现提供了统一基线。
