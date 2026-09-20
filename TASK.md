# TASK：优化整合包搜索用时

## 背景

下载整合包窗口搜一页（Modrinth 源）要 **30~60 秒**才出结果，界面上就是一直「搜索中…」。
CurseForge 源没这个问题（一次 `get_modpack_list` 就够）。

## 实测（2026-09-20，本机）

| 路径 | 单次请求 | 20 条结果合计 |
| --- | --- | --- |
| 直连 | project 0.82s / team 0.91s | ≈ 35s |
| 走 7890 代理 | project 1.03s / team 1.45s | ≈ 50s+ |

限流不是原因：`LIMITE_PER_MIN` 是 300，40 个请求碰不到。

## 根因

`mcml-gui/src-tauri/src/windows/add_modpack.rs:226` 的 Modrinth 分支逐条构造列表项：

```rust
for item in list.hits {                                  // 一页 20 条
    let temp = ProjectItemDto::new_modrinth(&item, ...).await?;
}
```

而 `ProjectItemDto::new_modrinth`（`dtos/add_resource_dto.rs:217`）**每条都要发两次网络请求，且串行**：

```rust
let mut project = modrinth_api::get_project(&data.project_id).await?;   // project 详情
let team = modrinth_api::get_team(&data.project_id).await?;            // 成员（作者）
```

于是**一页 = 40 次串行往返**。

`get_categories_icon`（`mcml-game/src/modrinth/mod.rs:100`）只在首次拉一次 `/tag/category`，
之后走 `OnceLock` 缓存，不是瓶颈。

## 参照物：旧 C# 不这么干

`e:/code/ColorMC/src/ColorMC.Gui/UIBinding/WebBinding.cs` 的 `GetModPackListAsync`（:139 起）
Modrinth 分支是**直接从 `HitObj` 构造列表项**的，没有任何 project / team 请求：

```csharp
// 循环之前只做一次批量请求，取 mcmod 百科翻译（不是 per-hit 的两次）
var list2 = await ColorMCAPI.GetMcModFromMOAsync(modlist, 1);
foreach (var item in list.Hits)
{
    list1.Add(new FileItemModel(item, FileType.Modpack,
        list2?.TryGetValue(item.ProjectId, out var data1) == true ? data1 : null));
}
```

项目详情（`get_project` + `get_team` + 截图）在 C# 里是**单独一条路** —— `GetFileItemAsync`（:181），
只在用户点开某个项目时才调（对应 `AddModPackControlModel.GoFile`）。

移植时把这条「详情」路径塞进了列表构造，于是每搜一页多打 40 个请求。

## 方案

**A（推荐，对齐 C#）**：列表项只从 `HitObj` 构造 —— 作者名取 `HitObj.author`、分类取 `categories`、
图标 `icon_url`、描述 `description`、时间 `date_modified`、下载数 `downloads`；
`authors`（作者头像）/ `tag`（带 svg 的分类图标）/ `screenshots` 在列表里留空，
等以后做「打开项目详情」（对齐 `GetFileItemAsync`）时再补。

- 预期：**50s → ~1s**。
- 代价：列表卡片少作者头像和分类图标（作者名、分类名仍在）。

**B（保留现在的富列表）**：把每条的 project / team 请求并行化（限并发 8 左右），
并按 `project_id` 缓存 project / team 结果。首次约 5~8s，翻页与二次搜索命中缓存。

- 偏离 C# 行为，且首次仍明显慢于 A。

## 影响面

- `ProjectItemDto::new_modrinth` 是**列表与资源窗口共用**的：`windows/add_resource.rs:392`
  也调它，改了对两边都生效。
- 前端 `mcml-vue/src/windows/add_modpack/ModpackMode.vue` 现在会渲染 `authors` / `tag` / `screenshots`，
  走 A 之后这几项为空 —— 不显示即可，不用改结构。
- CurseForge 分支不受影响。

## 验收

- 出一页结果的端到端耗时降到 ~1s 量级；Modrinth / CurseForge 两种源都测，直连与走代理各测一次。
- 翻页、切筛选（游戏版本 / 排序 / 分类）不再重复发起同一批 project 请求。
- 列表卡片的信息不缺失：名字、作者名、描述、分类名、下载数、更新时间。

## 备注

- 复现时注意本机代理：`config.json` 里 `ProxyWork = 2`（走 127.0.0.1:7890），单次往返比直连慢 30% 左右。
- 诊断手段：`ProjectItemDto::new_modrinth` 里加一次性的耗时打点，或直接看服务端请求条数。
