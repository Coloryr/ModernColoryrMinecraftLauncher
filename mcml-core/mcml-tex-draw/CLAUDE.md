# mcml-tex-draw 包开发须知

26.2 方块/物品图标渲染器：wgpu GPU 3D 渲染（回退链 DX12→VK→GL，逐图标回退 CPU skia），1:1 还原游戏内 GUI 物品渲染。

## 渲染关键不变量（排遮挡/透明问题先查这两条）

1. **深度方向**：屏幕像素空间里离观察者越近 z 越大，MVP 的 z 必须反向映射（`z_ndc = 0.5 - z_pix/2000`，见 gpu.rs 投影矩阵第三行取负），否则 `LessEqual` 永远保留最远的面，症状像"没开深度测试"。CPU 侧同理：远→近排序 = 小 z 先画。
2. **透明度判定**（照反编译 `NativeImage.computeTransparency`）：`a==0` 只是镂空，走 cutout 管线（深度写入）；仅 alpha 在 `1..254` 之间才走半透明管线（无深度写入）。判成 `a != 255` 会把二值 alpha 模型（大叶子、玻璃等）整只送进半透明管线，quad 按贴图组顺序覆盖绘制。

## 诊断工具

- 单方块对照渲染：`cargo test -p mcml-tex-draw --test render_one -- --ignored --nocapture`
  环境变量 `MCML_TEST_JAR` 指客户端 jar，`MCML_TEST_BLOCK` 指模型（如 `block/big_dripleaf`），GPU/CPU 各出一张图到 `tests/out/`
- 全量重渲染前先删 `tests/out/block.json`（版本号一致会短路跳过）
- 反编译参考源码：`C:\Users\40206\AppData\Local\Temp\mcml-262-ref\src-java2`；缺的类用 `C:\Program Files\Java\jdk-21\bin\javap.exe` 反汇编 client.jar 里的 class

## 待办任务：物品绘制

方块图标已完成（752 个图标、18 个 APNG，全部测试通过）。下一步做**物品绘制**，包含：

- [ ] 2D 精灵渲染（items/*.json 直接贴图）
- [ ] 动画物品的 APNG
- [ ] extrude：builtin/generated 平面物品的挤出模型（ItemModelGenerator：front/back 全幅面 + 不透明像素边界裙边，gui_light=front，ITEMS_FLAT 光照）
- [ ] glint 附魔光效（enchanted_glint_item.png，`uv' = T(-off0,off1)·RotZ(π/18)·S(8)`，加色混合，depth EQUAL，静态图取 t=0）
- [ ] special 类型（实体渲染：箱子/头颅/旗帜等）→ 跳过

glint/extrude/tint 的精确数学推导见 `C:\Users\40206\.claude\plans\steady-prancing-boole.md` 对应小节。
