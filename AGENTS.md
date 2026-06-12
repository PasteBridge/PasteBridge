# AGENTS.md

> 本文件适用于本项目内所有 AI 编程代理(Claude / Cursor / Copilot / Cline / Continue / Aider 等)。
> 项目已通过 **GitNexus** MCP 索引(代号 `pastebridge`),使用图谱工具可显著降低上下文 token 消耗。

## 设置

如果当前 agent 已配置 `gitnexus` MCP 服务,直接调用 `gitnexus_*` 工具即可。
未配置的 agent 可在终端运行:

```bash
npx gitnexus analyze   # 仅在 GitNexus 提示索引过期时执行
```

## 核心工具(必用 5 个)

| 工具 | 用途 | 关键参数 |
|------|------|---------|
| `gitnexus_query` | 按概念检索执行流(替代 grep) | `query`, `limit` |
| `gitnexus_context` | 获取单个符号的 360° 视图(调用方/被调方/所属流程) | `name` 或 `uid` |
| `gitnexus_impact` | 改动前评估爆炸半径(直接影响/风险等级) | `target`, `direction="upstream"` |
| `gitnexus_detect_changes` | 提交前验证改动是否只影响预期符号/流程 | `scope="unstaged"` |
| `gitnexus_rename` | 跨文件协调重命名(基于图谱,优于 sed) | `symbol_name`, `new_name`, `dry_run=true` |

## 工作规约

### 必须做

- 修改任何函数/类/方法前,**先调用 `gitnexus_impact`**,把爆炸半径(直接调用方、影响的流程、风险等级)告知用户。
- 提交前**必须调用 `gitnexus_detect_changes`** 验证改动范围。
- 探索未知代码时,优先用 `gitnexus_query` 检索执行流,而不是全文 grep。
- 需要某符号的完整上下文时,调用 `gitnexus_context`,避免读整文件。

### 严禁做

- 未经 `gitnexus_impact` 评估就修改函数/类/方法。
- 忽略 `gitnexus_impact` 返回的 **HIGH / CRITICAL** 风险警告。
- 用 find-and-replace 重命名符号 — 必须用 `gitnexus_rename`。
- 不调用 `gitnexus_detect_changes` 就提交。

## 风险阈值

- `LOW` — 可直接修改
- `MEDIUM` — 修改前简要说明影响范围
- `HIGH` — 必须列出受影响流程,等待用户确认
- `CRITICAL` — 必须拆分改动,先沟通方案

## 索引信息

- 仓库代号: `pastebridge`
- 符号数: 871
- 关系数: 1440
- 执行流: 50

> 任何 agent 都可读取此文件。修改前请保留本节以便其他 agent 识别。
