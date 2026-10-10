# 服务器包待办

## 1. 服务器包生成（未完成）

本仓库目前只有**消费侧**：`server_pack_update` 从 `<server_url>/server.json` 取远端清单，
`upgrade_serverpack` 按它增删文件。**生成侧没有做** —— 全仓没有任何地方写出
`server.json` / `server.json.sha1`（`SERVER_FILE` 只被读，没有写入点）。

要做：

- 从实例（游戏目录 + 版本 / 加载器 / 启动参数 / 窗口设置）生成 `ServerPackObj`
  （`serverpack/serverpack_obj.rs`）。
- 落盘 `server.json` + `server.json.sha1`（`names::SERVER_FILE` / `names::SHA1_DOT_EXT`）。
- 映射规则：客户端文件 → `online_list`（`file` / `pid` / `fid` / `url` / `sha1` / `sha256`），
  配置目录 → `archive_list`（`file` / `dir` / `delete_old` / `url` / `sha1` / `sha256`）。
- 生成结果要能被现有升级链路直接吃下（往返用例：生成 → 获取 → 升级）。

## 2. HMCL 服务器网址 index 兼容（未做）

- HMCL 服务端包清单是 `server-manifest.json`（`HMCLServerObj`：`name` / `author` / `version` /
  `description` / `file_api` / `files[path, hash]` / `addons`）。
- 目前只有**安装**路径（`add_game::hmcl_server_archive`），**服务器网址 index 的兼容没有做**。
- 要做：按 HMCL 服务器网址 index 取清单与文件（`file_api` + `path` / `hash` 校验），
  与现有 `PackType::HMCLServer` 安装 / 同步链路打通。

## 3. 已发现的缺陷（待修）

- **服务器包第一次同步必定失败**：`server_pack_update` 在本地还没有 `server.json` 时
  （`FileHash::None`）会走到 `move_serverpack_to_old()`，而该函数在源文件不存在时返回 `Err`
  （`path_helper::move_file` 没有"缺失就跳过"分支）。`upgrade_serverpack` 本身能处理"没有旧清单"，
  这次移动既没必要又致命。已由 `tests/serverpack.rs::move_to_old_without_file_is_error` 钉住现状，
  修法：`move_serverpack_to_old` 源文件不存在时返回 `Ok(())`，或调用方先判存在。

## 相关位置（供下轮定位）

- 消费侧：`mml-core/mml-game/src/serverpack/mod.rs`（升级）、`serverpack/plan.rs`（纯判定，已抽）、
  `game_launch.rs::server_pack_update`（获取 + 更新判定，私有方法）
- 远端协议：`<server_url>/server.json` + `<server_url>/server.json.sha1`
- 格式定义：`serverpack/serverpack_obj.rs`；HMCL：`other_launcher/hmcl_obj.rs`
- 测试入口：`mml-core/mml-game/tests/serverpack.rs`
