# Timeline 刷新与 Cursor 设计

日期：2026-06-13

## 目标

让 Chat timeline 的加载与刷新机制变得确定、安全，并且不依赖第二个“向后/尾部 cursor”。Dashboard 应该能够：

1. 高效加载最新消息窗口；
2. 使用单一的历史 cursor 加载更早消息；
3. 使用稳定的 `item_id` 作为锚点刷新后续新增消息。

这个设计用于修复当前刷新逻辑的问题：前端重新拉取最新 3 轮消息后，直接按 `item_id` 去重 append；如果页面失焦期间新增消息超过最新窗口范围，中间消息可能被静默跳过。

## 当前问题

当前行为大致是：

```text
页面聚焦 / 收到 message_updated
  -> GET /sessions/:id/timeline?limit=3
  -> 将返回的最新窗口按 item_id 去重合并到当前 items
```

主要问题：

- 最新窗口刷新只有在“新拉取窗口”和“前端当前 timeline”有重叠时才安全。
- 如果用户离开页面期间新增超过 3 轮，前端可能无法发现中间 gap，并静默 append 后面的消息。
- 当前去重逻辑只跳过已有 `item_id`，不会替换/更新已有 item。
- API 仍暴露 `tail_cursor`，但前端没有使用它。
- 后端 pi JSONL parser 当前仍然全量读取文件来返回一个很小的最新窗口；期望行为是从 transcript 尾部反向扫描。
- `next_cursor` 字段名像是“向后/下一页”cursor，但实际语义是“加载更早历史”。

## 非目标

- Dashboard 不直接读取 raw transcript 文件。
- 不引入 forward cursor / tail cursor 来做刷新。
- 不从 tmux、runtime log 或 TUI 内容推断 timeline 状态。
- 不要求所有 client 使用同一种 `item_id` 生成规则。

## 目标 API 模型

Timeline 有两类不同的读取路径：

1. **窗口/历史加载**：加载最新窗口，或继续加载更早历史。
2. **刷新/增量加载**：基于前端已知的某个 `item_id`，加载它之后新增的 items。

这两类 API 应该分开，因为虽然它们都可能从文件尾部开始扫描，但停止条件、返回语义和前端合并策略都不同。

## Timeline 窗口 API

接口：

```http
GET /external/v1/sessions/:session_id/timeline?limit=3
GET /external/v1/sessions/:session_id/timeline?older_cursor=<older_cursor>&limit=3
```

语义：

- 不带 `older_cursor`：读取最新的 `limit` 个 user round。
- 带 `older_cursor`：从该 cursor 继续向更早历史读取。
- 对 pi JSONL 来说，后端应该反向扫描 transcript，而不是全量读取文件。
- 返回的 `items` 按时间正序排列，方便前端直接渲染。

响应结构建议：

```ts
interface TimelinePage {
  session_id: string;
  binding_id: string;
  items: TimelineItem[];
  older_cursor: string | null;
  has_more: boolean;
  source_id: string;
}
```

`older_cursor` 的语义：

```text
older_cursor 表示当前返回窗口最早 selected line 的起始位置。
下一次读取以该位置作为 upper bound，返回它之前的更早内容。
```

必须清理的旧字段：

1. 移除 `tail_cursor`。
2. 移除 `is_tail`，除非后续重新出现明确的 UI/API 需求。
3. 将 `next_cursor` 重命名为更清晰的字段名。推荐使用：`older_cursor`。

选择 `older_cursor` 的原因：

- 它描述真实方向：加载比当前最早窗口更早的消息。
- 不暗示存在第二个 forward pagination 方向。
- 与 UI 行为一致，例如“Load earlier messages”。

## Timeline 刷新 API

新增接口：

```http
GET /external/v1/sessions/:session_id/timeline/updates?after_item_id=<item_id>
```

语义：

- 前端传入当前 timeline 中最后一个已知 `item_id`。
- 后端从 transcript 尾部向前扫描。
- 扫描直到找到 `after_item_id`。
- 返回这个 anchor 之后新增的 timeline items。
- 返回的 `items` 按时间正序排列。
- 不使用也不暴露 `tail_cursor`。

响应结构建议：

```ts
interface TimelineUpdatesPage {
  session_id: string;
  binding_id: string;
  after_item_id: string;
  items: TimelineItem[];
  anchor_found: boolean;
  truncated: boolean;
  source_id: string;
}
```

字段含义：

- `anchor_found: true`：后端找到了前端传入的 anchor，前端可以安全合并返回的新增 items。
- `anchor_found: false`：后端没有找到 anchor。前端不能静默 append，因为中间可能有 gap。
- `truncated: true`：扫描达到了安全上限仍未找到 anchor。前端应该把它当作 gap 处理。

Refresh API 必须有扫描安全上限。第一版只保留一个上限即可：

```text
max_scan_bytes = 8 MiB
```

该上限只是防止异常情况下单次 refresh 扫描成本过高，不改变正常“扫到文件开头仍未找到 anchor”的语义：

- 如果在 8 MiB 内找到 anchor：`anchor_found = true`，`truncated = false`，返回 anchor 之后的新 items。
- 如果扫描到文件开头仍未找到 anchor：`anchor_found = false`，`truncated = false`。
- 如果尚未到文件开头，但已经达到 8 MiB 上限：`anchor_found = false`，`truncated = true`。

前端对 `anchor_found = false` 和 `truncated = true` 都不能静默 append；可以统一走 rebuild/gap 处理。

前端 fallback：

- 如果 `anchor_found === false` 或 `truncated === true`，不要静默 append。
- 第一版直接 rebuild 最新窗口。
- 后续如果需要更好的 UX，可以再引入“有较多新消息，点击跳到最新/重新加载”的提示。

## 不保留 tail cursor

设计上只保留一个方向的 cursor：

```text
older_cursor：用于加载更早消息
```

不再维护：

```text
tail_cursor：文件尾部位置
forward cursor：从旧 tail 继续向后读
```

刷新不依赖文件尾 cursor，而依赖前端已经显示过的 `item_id`：

```text
前端 lastKnownItemId
  -> 后端从尾部反向扫描直到找到该 item
  -> 返回该 item 之后的新 items
```

这样避免了同时维护两个方向 cursor 带来的复杂性，也更符合“默认倒序读取窗口”的模型。

## TimelineItem 身份标识

`TimelineItem.item_id` 是刷新 API 的 anchor。它必须稳定、可复现，并且由后端在普通 timeline 读取和 refresh 扫描中用同一逻辑生成。

要求：

- 同一个 raw transcript item 多次解析得到同一个 `item_id`。
- 在同一个 binding/source 内尽量唯一。
- 能作为 refresh anchor 使用。
- 由各 client 的 transcript parser 负责生成，不做成全局硬编码规则。

推荐抽象：

```text
client raw transcript parser
  -> 解析 raw entry/block
  -> 进入 item identity 解析/生成步骤
  -> 输出带稳定 item_id 的 TimelineItem
```

这个 item identity 步骤应该是 transcript parser 的明确子步骤，而不是散落在 mapping 代码中的字符串拼接。它的职责是：

- 从 client 原生 transcript 中提取可用的稳定 id；
- 在一个 raw entry 会展开为多个 timeline items 时，为每个 block 派生稳定子 id；
- 对没有原生 id 的 client，使用该 client 定义的 fallback 生成策略；
- 保证普通 timeline 读取、历史分页、refresh anchor 扫描都调用同一套逻辑。

可以理解为每个 client parser 都要实现自己的 `parse_item_identity` / `timeline_item_id` helper。对 pi 来说这个 helper 很简单，但抽象层仍然应该存在。

### pi 的 item_id 策略

pi session JSONL 当前会为 session/model/message 等 entry 提供稳定的顶层 `id`。因此 pi 的 item identity helper 不需要复杂生成逻辑；它只需要读取 JSONL entry 的原生字段并附加 block 维度。

pi 的 timeline item id 应直接基于原生 entry id：

```text
pi:entry:<entry.id>:block:<block_index>
```

其中：

- `entry.id` 来自 pi JSONL 顶层字段；
- `block_index` 用于区分同一 assistant message 中的多个 content block，例如 thinking、tool call、text；
- 同一个 entry 只有一个 timeline item 时，仍使用 `block:0` 保持格式统一。

也就是说，pi 仍然经过统一的 item identity 步骤，只是这个步骤的实现是“读取原生 `entry.id` + block index”。

如果某条 pi entry 缺少 `id`，应视为异常/不完整 transcript，而不是默认进入 hash fallback。第一版处理策略：

```text
跳过该 entry，并记录诊断；不使用 hash fallback。
```

也就是说：hash fallback 是其他无原生 id client 的通用方案；对 pi 来说，稳定原生 `id` 是 contract。

### 其他 client

没有原生 id 的 client 可以使用 canonical item representation 的哈希：

```text
<client>:item:<hash>
```

canonical 输入可以包含：

- kind；
- role；
- timestamp；
- content；
- block index；
- 必要时加入 source-local ordinal 或 byte range 来区分重复内容。

## 前端状态模型

前端 timeline state 应该明确保存：

```ts
interface TimelineState {
  sessionId: string;
  bindingId: string | null;
  sourceId: string | null;

  // 按时间正序保存，用于渲染 chat message。
  items: TimelineItem[];

  // 只用于加载更早消息。
  olderCursor: string | null;
  hasMore: boolean;

  loading: boolean;
  refreshing: boolean;
  error: string | null;
}
```

刷新所需的 anchor 可以从 `items` 派生：

```ts
const lastKnownItemId = timelineState.items.at(-1)?.item_id ?? null;
```

如果为了避免竞态或提升可读性，也可以显式保存：

```ts
lastKnownItemId: string | null;
```

状态职责：

1. 保存 chat 渲染所需的 timeline item 列表。
2. 保存 `olderCursor`，用于加载更早消息。
3. 保存或派生最新的 `item_id`，用于刷新后续消息。
4. 不再保存 `tailCursor` / `isTail`。

## 前端数据流

### 初始加载 / 进入路由

```text
GET /timeline?limit=3
  -> 用返回 items 替换 timeline state
  -> olderCursor = page.older_cursor
  -> hasMore = page.has_more
```

### 加载更早消息

```text
GET /timeline?older_cursor=olderCursor&limit=3
  -> 将返回 items prepend 到当前 items 前面
  -> 按 item_id 去重
  -> olderCursor = page.older_cursor
  -> hasMore = page.has_more
```

### 页面重新聚焦 / SSE message update

```text
lastKnownItemId = items.at(-1)?.item_id

如果 lastKnownItemId 存在：
  GET /timeline/updates?after_item_id=lastKnownItemId
否则：
  GET /timeline?limit=3
```

如果 refresh 响应安全：

```text
anchor_found && !truncated
  -> append 返回的新增 items
  -> 按 item_id 去重/更新
```

如果 refresh 响应不安全：

```text
!anchor_found || truncated
  -> 直接 rebuild 最新窗口
```

## 合并语义

当前 `appendUniqueTimelineItems` 只 append 未见过的 `item_id`，保留已有 item 的旧版本。新的 refresh 合并应该更明确：

- 保持时间正序。
- 按 `item_id` upsert。
- 如果同一个 `item_id` 再次出现，用新解析结果替换旧 item。
- 当后端报告 anchor 未找到时，绝不静默 append。

第一版可以采用最保守规则：

```text
updates API 找到 anchor -> upsert/append 返回的新 items
updates API 没找到 anchor -> rebuild 最新窗口
```

## 后端 pi JSONL 读取策略

pi parser 应该避免为了读取最近 3 轮而全量读取文件。

推荐窗口读取流程：

- 使用 file metadata 获取文件长度。
- 从 EOF 或 `older_cursor` 指定 offset 开始，按 chunk 反向读取。
- 识别行边界。
- 从新到旧解析 JSONL 行，直到收集到足够的 user rounds。
- 将选中的范围按时间正序解析为 items 返回。
- 如果更早处仍有 user round，则生成 `older_cursor`。

推荐 refresh 读取流程：

- 从 EOF 反向读取。
- 从新到旧解析 items。
- 遇到 `after_item_id` 后停止。
- 返回 anchor 之后的 items，按时间正序排列。
- 设置扫描安全上限，超过上限则返回 `truncated: true`。

## 错误与 gap 处理

- `older_cursor` 无效：返回 `cursor_invalid`；前端应 reset/rebuild timeline。
- `after_item_id` 缺失或格式无效：返回 validation error。
- 找不到 anchor：返回 `anchor_found: false`，前端不能 unsafe append。
- 扫描达到安全上限：返回 `truncated: true`，前端按 gap 处理。
- 第一版 refresh 安全上限使用单一字节限制：`max_scan_bytes = 8 MiB`。不额外引入行数上限或返回 item 数上限，避免过度设计。
- binding 尚未发现且 source 不可用：保留现有 `not_ready` 行为。
- binding 已发现、session 终态且 source 不可用：保留现有 `source_unavailable` 行为。

## 迁移计划

1. 后端 API 类型
   - 移除 timeline response 中的 `tail_cursor` 和 `is_tail`。
   - 将 `next_cursor` 重命名为 `older_cursor`。
   - 新增 `TimelineUpdatesPage` response 类型。

2. 后端 pi parser
   - 将 client-specific item id 生成逻辑抽成 parser-local helper。
   - 实现 latest/older window 的反向扫描。
   - 实现基于 `after_item_id` 的 refresh 反向扫描。

3. 前端 API 类型与 store
   - 移除 `tailCursor` 和 `isTail`。
   - 将 `nextCursor` 重命名为 `olderCursor`。
   - `items` 作为 chat 渲染消息的唯一 timeline 数据源。
   - 从 `items.at(-1)?.item_id` 派生，或显式保存 latest item id 用于 refresh。

4. 前端刷新行为
   - 将 foreground/SSE 的“重拉最新窗口 append”替换为 updates API。
   - 只有 `anchor_found && !truncated` 时才 upsert/append。
   - 否则直接 rebuild 最新窗口。

5. 测试
   - 后端 parser 测试：latest window、older paging、稳定 item_id、after anchor refresh。
   - API 测试：确认返回 `older_cursor`，不再返回 `tail_cursor` / `is_tail`。
   - 前端 store 测试：prepend older history、append refresh updates、gap fallback。

## 已定决策

- Refresh endpoint 使用 `/timeline/updates`。
- 历史分页请求参数使用 `older_cursor`，响应字段也使用 `older_cursor`。
- Refresh 安全上限第一版使用单一字节限制：`max_scan_bytes = 8 MiB`。
- Refresh 找不到 anchor 或触发 `truncated` 时，前端第一版直接 rebuild 最新窗口。
- pi entry 缺少原生 `id` 时跳过该 entry 并记录诊断，不使用 hash fallback。

## 后续可优化项

- 如果 rebuild 最新窗口带来的 UX 不够好，可以增加用户可见的 gap 提示。
- 反向读取 chunk size 属于实现细节，默认可从 64 KiB 起步，再根据性能调整。
