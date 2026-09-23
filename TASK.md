# TASK：实例资源管理（第二批：服务器 / 光影包 / 结构文件 / 数据包）

## 背景

第一批（模组 / 材质包 / 存档 / 截图）已完成。本批把资源管理窗口剩下三类接上真实数据，
并补上存档下的「数据包」子页，管理粒度与第一批一致（完整管理、删除进回收站）：

| 类型 | 操作 |
| --- | --- |
| 服务器 | 列表 / 添加 / 编辑（含 acceptTextures）/ 删除（servers.dat NBT 读写） |
| 光影包 | 列表 / 启用停用（写 options.txt 的 `shaderPack=`）/ 删除 |
| 结构文件 | 列表（类型 / 尺寸 / 方块数 / 作者）/ 删除 |
| 数据包（存档子页） | 按存档选择后 列表 / 启用禁用切换 / 删除（level.dat NBT + 文件） |

## 方案要点

- **内核修正（mml-core，2 个真 bug + 1 对方法）**：
  - `game_server.rs` `remove_server`：retain 逻辑反了（保留匹配项而非删除）→ 取反
  - `game_saves.rs` `save_nbt`：用 `open_read`（只读句柄）写 level.dat 必失败 → 改 `open_write`
  - `game_options.rs` 补 vanilla options.txt 读写：`get_minecraft_options` / `save_minecraft_options`
    （照 `get_options`/`save_options`，`=` 分隔，文件走 `get_option_file()`）
- **复用 mml-game 现有 API**：`get_server_infos`/`add_server`/`save_servers`、
  `get_shaderpacks`（zip 语言文件解析）、`get_schematics`（按扩展名解析 NBT，
  Minecraft/Litematic/WorldEdit/Create 四类）、`SaveObj::get_datapacks` / `change_data_pack`
  （对传入包做状态翻转，传单个即 toggle）/ `delete_datapack`（只清 NBT 引用，文件另行回收站）
- **后端**：`resource.rs` 新增 `resource_*` 命令（list/add/update/delete/toggle/set/open_folder）；
  服务器按 (name, ip) 定位；光影启用写 options.txt（停用写 `OFF`）；删除按「目录 + 纯文件名」
  直操作（防路径穿越）；数据包命令带存档目录参数；`resource_open_folder` 加 `parent` 参数
  （datapacks 类别 = saves/<存档>/datapacks），新增 kinds：shaderpacks / schematics / servers（.minecraft 根）
- **前端**：ResourceWindow.vue 三类新列表 + 数据包子页（存档下拉选择 + 三态启用徽标）+
  服务器添加/编辑表单弹窗（BaseModal）；操作后重拉、删除先确认
- **i18n**：`resource.serverAdd/serverEdit/serverName/serverIp/acceptTextures/save/shaderOn/
  dims/blockCount/author/selectSave/dpOn/dpOff/dpNone` + `err.nameIp`（zh + en）

## 验收

- 服务器增删改查正确，servers.dat 可被游戏正常读取；图标 / acceptTextures 徽标显示
- 光影包列表显示名称与描述；启用后 options.txt 的 `shaderPack=` 指向该文件，停用变 `OFF`；
  启用中的包带「启用中」徽标
- 结构文件列表显示类型徽标、尺寸 W×H×L、方块数、作者；解析失败的照常显示可删
- 数据包子页：存档下拉选中后列出数据包；toggle 后 level.dat 的 DataPacks 列表变化
  且游戏内可见；删除 = 清 NBT 引用 + 文件进回收站
- 删除均进回收站；打开文件夹定位正确；空目录 / 未选存档显示空态
- `cargo check -p mml-game`（内核）+ `cargo check`（mml-gui/src-tauri）+
  `vue-tsc --noEmit` + `vite build` 全绿
