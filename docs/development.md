# rsTerminal 开发规范

本文约定分支模型、日常开发流程与 Commit 格式，并与 `.github/workflows` 中的 CI（Develop → RC → Master → Release）对齐。

## 1. 分支模型

| 分支 | 用途 | 如何进入 |
|------|------|----------|
| `develop` | 日常集成、功能开发合入点 | Feature 分支经 PR 合入；Owner 可直接推送 |
| `RC` | 发布候选（Release Candidate） | **仅允许 PR** 从 `develop`（或修复分支）合入 |
| `master` | 可发布主干，始终保持可发版状态 | **仅允许 PR** 从 `RC` 合入 |
| `feature/*`、`fix/*` 等 | 个人/任务分支 | 从 `develop` 拉出，开发完成后 PR 回 `develop` |
| `v*` Tag | 正式版本标记 | 仅在已验证的 `master` 上由 Owner 创建 |

晋升关系：

```text
feature/* ──PR──► develop ──PR──► RC ──PR──► master ──Tag v*──► Release
                      │              │            │
                 Develop CI       RC CI      Master CI
                                                   │
                                              Security（并行）
```

不要把未经验证的改动直接推到 `RC` / `master`。不要在 `master` 上做功能开发。

## 2. 开发流程

### 2.1 日常功能 / 修复

1. 从最新 `develop` 拉出分支，命名建议：
   - 功能：`feature/<简短描述>`，如 `feature/ble-multi-uart`
   - 修复：`fix/<简短描述>`，如 `fix/ssh-resize-prompt`
   - 重构 / 杂项：`refactor/...`、`chore/...`
2. 本地完成改动后自检（须与 CI 一致，全部通过后再推送）：
   ```bash
   cargo fmt --all
   cargo clippy --all-targets --all-features -- -D warnings
   cargo check --all-targets
   cargo test
   ```
   Clippy 使用 `-D warnings`：**警告视为错误**，与 Develop / RC / Master 的 quality job 相同。若有大量可自动修复的告警，可先：
   ```bash
   cargo clippy --all-targets --all-features --fix -- -D warnings
   ```
   再对 `--fix` 无法处理的项按提示手改，直到无 error。不要为过 CI 随意 `#[allow(...)]` 大面积压制；确有合理例外时在最小范围内注明原因。
3. 向 `develop` 开 PR（或 Owner 在权限允许时直接推送 `develop`）。
4. 等待 **Develop CI** 通过后再合并：格式、Clippy、check、测试，以及 Linux / Windows / Android 基础构建。

小改动可走短分支 + 单 commit；大改动拆成语义清晰的多个 commit，便于 review 与回滚。

### 2.2 进入发布候选（RC）

1. 确认 `develop` 上目标功能已稳定，Develop CI 为绿。
2. 从 `develop` 向 `RC` 开 PR（禁止直推 `RC`）。
3. 等待 **RC CI** 通过：在 Develop 基础上增加 Debian amd64 / arm64 / i386 / armhf 等完整构建。
4. 通过后合并；可下载 CI artifact 做人工冒烟（安装 deb、跑 APK 等）。

### 2.3 合入 master

1. 从 `RC` 向 `master` 开 PR。
2. 等待 **Master CI** 通过（完整矩阵 + 打包校验），并将仓库 Required Status Check 指向 **Master CI / gate**（若已配置）。
3. 合并后 `master` 即视为可打 Tag 的状态。

### 2.4 正式发版

1. 确认 `Cargo.toml` 中 `version` 已更新，且与即将打的 Tag 一致（例如 `0.7.2` ↔ `v0.7.2`）。
2. 在 `master` 上创建并推送 Tag：`git tag vX.Y.Z && git push origin vX.Y.Z`。
3. **Release** 流水线会校验版本号、全平台打包、生成 `SHA256SUMS`（若配置了 GPG Secret 则签名），并创建 GitHub Release。

发版相关版本号变更应单独 commit（见下文 `chore`），不要与大功能混在同一 commit。

### 2.5 安全扫描

**Security** 流水线（`cargo audit`、CodeQL）在 `develop` / `RC` / `master` 的 PR 与推送以及定期任务上运行。发现漏洞时应优先修复或评估豁免，不要长期忽略红色 audit。

## 3. Commit 格式

采用 [Conventional Commits](https://www.conventionalcommits.org/) 风格，与仓库现有历史一致。

### 3.1 结构

```text
<type>(<scope>): <subject>

[optional body]

[optional footer]
```

- **type**（必填）：见下表。
- **scope**（可选）：影响模块，如 `ssh`、`ble`、`ui`、`packaging`、`ci`。
- **subject**（必填）：简短说明改动意图；使用中文或英文均可，但同一 PR 内风格尽量统一；**不加句号**；不超过约 72 个字符。
- **body**（可选）：说明动机、方案取舍、破坏性变更细节；与 subject 空一行。
- **footer**（可选）：`BREAKING CHANGE:`、`Fixes #123` 等。

### 3.2 type 一览

| type | 含义 | 示例 |
|------|------|------|
| `feat` | 新功能 | `feat(ble): 支持多特征 UART 配置` |
| `fix` | 缺陷修复 | `fix(ssh): 修复 resize 后多余 prompt 行` |
| `refactor` | 重构（行为不变） | `refactor: 清理 session 模块引用` |
| `perf` | 性能优化 | `perf(ui): 减少终端重绘次数` |
| `docs` | 仅文档 | `docs: 补充开发规范与分支流程` |
| `test` | 仅测试 | `test(ssh): 增加认证失败用例` |
| `chore` | 构建、依赖、版本号、杂项 | `chore: 更新版本号至 0.7.2` |
| `ci` | CI/CD 配置 | `ci: 拆分 develop/RC/master 流水线` |
| `style` | 格式或无逻辑影响的排版 | `style: 统一 rustfmt 结果` |

不要用含糊的 subject，例如「更新」「改一下」「fix bug」。应写清改了什么、解决什么问题。

### 3.3 示例

```text
feat(ui): 添加全屏模式与快捷键

通过菜单与 F11 切换全屏；退出时恢复窗口几何信息。
```

```text
fix(serial): 处理设备拔出后的阻塞读

在 read 返回错误时关闭会话并提示用户重新连接。

Fixes #42
```

```text
chore: 更新版本号至 0.7.2
```

```text
refactor(ssh)!: 认证参数结构重命名

BREAKING CHANGE: `AuthParams` 重命名为 `SshAuthParams`，调用方需同步修改。
```

### 3.4 其他约定

- **一个 commit 只做一类事**：功能、修复、版本号、纯格式化尽量分开。
- **禁止** 把密钥、token、私钥提交进仓库。
- 合并 PR 时优先 **squash** 或保持线性、可读的 commit 历史；若保留多个 commit，每个都应独立符合本规范。
- 回滚类改动使用 `revert:` 或说明被回滚的 commit 哈希。

## 4. Pull Request 建议

- 标题可用与 commit 相同的 `type(scope): subject` 形式。
- 描述中写清：改动目的、测试方式、是否涉及打包/发版。
- CI 未通过不要强行合并；若为已知环境问题，在 PR 中说明原因与跟进方式。
- 涉及 UI / 协议行为变化时，尽量附截图或复现步骤。

## 5. 代码质量门禁（fmt / clippy / test）

晋升 CI 的 quality job（见 `reusable-quality.yml`）强制执行：

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo check --all-targets
cargo test
```

本地开发应对齐上述标准；提交前建议完整跑一遍。Clippy 告警处理顺序：

1. `cargo clippy --all-targets --all-features --fix -- -D warnings`（自动修复）
2. 手改剩余报错，直至 `cargo clippy --all-targets --all-features -- -D warnings` 退出码为 0
3. 再跑 `cargo fmt --all` 与 `cargo test`，避免 fix 引入格式或测试回归

## 6. 本地与 CI 对照

| 阶段 | 本地建议 | CI |
|------|----------|-----|
| 日常开发 | `fmt` / `clippy --all-targets --all-features -- -D warnings` / `test` | Develop CI |
| 进 RC | 同上 + 必要时本地打 deb/APK | RC CI（含 Debian 矩阵） |
| 进 master | 确认 RC 已验证 | Master CI + 打包校验 |
| 发版 | 对齐 `Cargo.toml` 与 Tag | Release（校验、打包、SHA256、Release） |

更细的工作流定义见 `.github/workflows/` 下的 `develop-ci.yml`、`rc-ci.yml`、`master-ci.yml`、`release.yml`、`security.yml`。
