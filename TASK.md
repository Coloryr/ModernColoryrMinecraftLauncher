# 待办：滚动条出现时不要将内容往左撑开

> 临时任务文件（见 AGENTS.md §6.1）：做完即删，不作为仓库文档长期保留。

## 问题

滚动条一出现，内容就被**往左撑开**（或者说被挤窄一截）。典型现象：

- 列表内容一多、滚动条冒出来，卡片/行整体左移约 9px，布局"跳一下"；
- 同一个窗口在不同数据量下，右边缘对不齐。

根因：**滚动条占据的是布局空间**。`overflow-y: auto` 时，滚动条会从内容盒里"吃掉"它自己的宽度
（`styles/scrollbar.css` 里是 9px），于是内容可用宽度随"有没有滚动条"变化。

## 目标

**所有滚动容器，滚动条出现时内容宽度都不变** —— 有滚动条和没有滚动条两种情况下的
内容盒宽度一致，不出现横向跳动。

## 方案

统一用 `scrollbar-gutter: stable`：让浏览器**预留**滚动条的位置（只在需要滚动的容器上），
内容宽度不随滚动条的有无而变。

- 已经在用（可作参考写法）：
  - `mml-vue/src/windows/resource/resource.css` → `.item-list`、`.shot-list`
    （注意那里的 `padding-right: 26px` 是**卡片留白**，与 gutter 是两件事，别合并）
- 需要排查并补上的地方（**未逐一确认，开工时先搜**）：

  ```bash
  # 找出所有滚动容器
  rg -n "overflow-y:\s*(auto|scroll)|overflow:\s*(auto|scroll)" mml-vue/src
  ```

  逐个判断：
  1. 该容器内容**长短会变**（列表 / 日志 / 表格…）→ **加 `scrollbar-gutter: stable`**；
  2. 内容永远不滚动（固定高度的小面板）→ 不用加；
  3. 横向滚动（`overflow-x`）另算：横向没有 `scrollbar-gutter` 的等价物，
     要固定高度或改用 `scrollbar-width`，单独处理。

- 全局兜底（可选，**先别急着做**）：在 `styles/scrollbar.css` 里给
  `html` 或常用容器加 `scrollbar-gutter: stable`。全局加的影响面大
  （会让所有页面右侧常驻一条 9px 空白），**优先逐容器加**。

## 注意（踩过的坑，别再犯）

1. **不要用写死 `padding-right: 9px` 代替** —— 那个值只在"有滚动条"时成立，
   没滚动条时那 9px 会把内容顶出右边缘（本项目已经踩过两次，见
   `resource.css` 里 `.cat-content` 上方那段"改之前先读这段"）。
2. **`width: 100%` 会把 gutter 也算进去**：放在滚动容器里的子元素如果自己写
   `width: 100%`，有/无滚动条时会比其他元素宽出一条。用 `align-self: stretch`
   之类由容器决定宽度（`styles/skeleton.css` 的 `.sk-row` 就是这么改的）。
3. **滚动条的宽度只有一个来源**：`styles/scrollbar.css` 的 9px。若某处必须要让位
   （例如顶栏要让开滚动条，见 `resource.css` 的 `.content-head`），
   写成 `calc(26px + 9px)` 并在注释里点明与那边同源。

## 验收

- 数据量从"不滚动"变成"滚动"时，**内容左右边缘不动**（卡片不左移、右边缘不跳）。
- 逐窗口自查：主窗口（实例列表 / 日志）、资源窗口（七个分类 + 数据包）、
  设置窗口、账户窗口、日志窗口、下载窗口、整合包 / 添加资源窗口。
- 前端验证照最小集：`npx vue-tsc --noEmit` + `npm run build`（在 `mml-vue/`）。