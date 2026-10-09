# TASK：整合包升级（upgrade_modpack）测试

> 临时任务清单，做完即删（见 AGENTS.md §6.1）。约定不要写在这里，长期约定进 AGENTS.md。

## 为什么提这个

内核**已经实现了**整合包升级，但既没有测试、也没接到界面：

| 东西 | 位置 |
| --- | --- |
| CurseForge 升级入口 | `mml-core/mml-game/src/curseforge/mod.rs:619` |
| Modrinth 升级入口 | `mml-core/mml-game/src/modrinth/mod.rs:489` |
| 比对逻辑（trait） | `mml-core/mml-game/src/modpack/mod.rs:85` |
| CF 有旧 manifest 的比对 | `mml-core/mml-game/src/modpack/curseforge_worker.rs:338` |
| CF 无旧 manifest 的比对 | `mml-core/mml-game/src/modpack/curseforge_worker.rs:497` |
| Modrinth 的比对 | `mml-core/mml-game/src/modpack/modrinth_worker.rs:248` |

- 全仓测试文件里 **0 处** `upgrade_modpack` / `check_upgrade`（只出现在实现里）。
- `mml-gui/src-tauri/src/windows/*.rs` 里 **0 处** `upgrade` → 界面没接（本轮不做，见文末）。

风险不在"能不能装上新版"，而在**差异算错**：该删的没删（残留旧模组）、不该下的重复下、
manifest 没写回导致下次升级又走全量。这些用"装一遍看能不能启动"是验不出来的。

## 三条比对路径（要覆盖的目标）

manifest 文件名：`names::MANIFEST_FILE` = `manifest.json`，`names::MODRINTH_FILE` = `modrinth.index.json`。

| 路径 | 触发条件 | 判定依据 |
| --- | --- | --- |
| CF 有旧 manifest | base 目录存在 `manifest.json` | 按 `project_id` 配对，`file_id` 不同 = 变更 |
| CF 无旧 manifest | 不存在 / 解析失败 | 调 API 取文件信息，按 SHA1 |
| Modrinth | 存在 / 不存在 `modrinth.index.json` | SHA1；无旧清单时按 `mod_id` |

同 manifest 的两条规则（CF，`curseforge_worker.rs:318` 起）：只在新清单 = 新增；
只在旧清单 = 删除；`project_id` 与 `file_id` 都不变 = 不动。

## 用例清单

### 纯差异计算（离线，优先做）

- [ ] 同 `project_id`、`file_id` 变 → 新版进"要下载"、旧版进"要删除"
- [ ] 只在新清单 → 新增
- [ ] 只在旧清单 → 删除
- [ ] `project_id` 与 `file_id` 都不变 → 两个列表都不进（不重复下载）
- [ ] 新清单为空 / 旧清单为空（边界，不应 panic）
- [ ] 同一 `project_id` 在新清单里出现两次（旧项只该被配对一次）

> **第一步是重构，不是写测试**：现在这段差异计算**内联在 async 函数里**，中途还要调
> API 解析文件（`check_upgrade_with_old_manifest` 第 5 步），所以离线测不了。
> 先把它抽成一个纯函数 —— 输入新旧清单、输出 `(新增, 变更, 删除)`，无 IO、无网络 ——
> 再对纯函数写单测。抽的时候只搬逻辑，不要顺手改判定规则。

### manifest 写回（离线）

- [ ] 升级成功后 `manifest.json` / `modrinth.index.json` 被新清单覆盖（下次升级据此比对）
- [ ] 取消（cancel 令牌触发）或比对失败时，**旧 manifest 不被写坏**
- [ ] 旧 manifest 损坏 / 不是合法 JSON → 落到"无旧 manifest"分支，不 panic

### 端到端（在线，必须走门控）

- [ ] 装旧版 → 升级到新版 → 实例磁盘上的模组集合 == 新版清单
- [ ] 升级**不重建实例**：图标 / 名称 / 分组保持原样
      （`upgrade_modpack` 里传 `None` 的那条路径，见 cf `mod.rs:653`、mr `mod.rs:521`）

## 可直接复用的现成材料

`mml-core/mml-game/tests/modpack_install.rs` 里已经有：

- `make_curseforge_pack(name)` / `make_modrinth_pack(name)` —— 合成整合包（zip + 目录）
- mock 的 `AddModPackGui` 实现（`name_replace` / `overwrite` / `reached` / `set_state`…）
- `network_available()` —— 在线用例的门控
- `TEST_LOCK` + `ensure_init()` 的写法（内核系统是进程级单例，见 AGENTS.md §6）

⚠️ 这两个构造器目前清单是 `"files":[]`，**造不出"版本差异"**：要测差异就得给它们加一个
"文件清单"参数（`project_id` / `file_id` / 路径 / hash），照着真实 manifest 的结构造。

## 怎么跑

```bash
# 仓库根目录；只跑本任务相关的，不要 --workspace（AGENTS.md §3）
cargo test -p mml-game --test modpack_upgrade
cargo check -p mml-game
```

- 临时目录走 `mml_testutil`（落在根目录 `target/temp`），不要用系统 `%TEMP%`；
  每个用例各用各的子目录（落盘是异步的）。
- 每个新用例都要能被 `network_available()` 跳过在线部分，保证**无网也能跑离线用例**。

## 验收

- 离线用例全绿且完成时不触网。
- `cargo test -p mml-game --test modpack_upgrade` 与 `cargo check -p mml-game` 通过。
- 没有为了过测试而放宽判定逻辑（改了就要在用例里写明理由）。

## 不在本轮范围

- **接界面**：`mml-gui` 加"检查更新 / 升级整合包"命令 + 前端入口（另有任务）。
- **serverpack 升级**是另一条线（`mml-game/src/serverpack/mod.rs:83`
  `upgrade_serverpack`，由 `game_launch.rs:507` 在启动时调用）——也要测的话另开一轮。
