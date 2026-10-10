# Scene Blocking with Kitbashing 使用指南

“Scene Blocking with Kitbashing”是 ArtCraft 的 **3D 场景编辑器**（代码库中叫 `pagescene` / Stage3D / PageScene）。
核心思路和电影、游戏里的「blocking」一样：先用**一整套现成的 3D 资产 + 基础几何体**（kit）把场景的**机位、物体摆放、前后景深度**粗略搭出来，再交给模型生成最终画面——用构图去控制结果，而不是只靠提示词。

## 1. 进入 3D 场景

- 从「Image to 3D」入口，或新建 / 打开一个 scene（路由为 `/scene/:sceneToken`）进入 3D 编辑器。
- 顶部中央是工具栏 `Controls3D`：最左边是 **`+`（添加资产）**，中间是 Move / Rotate / Scale，右边是 World / Local。

## 2. 用 kit 搭建场景（blocking 的主体）

点 **`+`** 弹出「Add an asset to scene」菜单（代码在 `Controls3D.tsx`）：

| 菜单项                          | 内容                                                        |
| ------------------------------- | ----------------------------------------------------------- |
| ArtCraft Presets (B)            | 预设资产库，blocking 主要靠这个（见下）                      |
| Animations                      | Mixamo 骨骼动画片段                                         |
| My Library                      | 用户自己的素材库                                            |
| Upload 3D Model / Image / Splat | 上传 GLB / FBX、图片、高斯泼溅 `.spz`                        |

**ArtCraft Presets** 打开 `AssetModal`，左侧分类标签（来自 `AssetModal.tsx` + `demoAssets.ts`）：

- **Objects（基础几何体）**——这才是真正的 kitbash 积木：
  `Cube / Cylinder / Donut / Sphere / Water / Point Light`（立方体、圆柱、圆环、球、水面、点光源）。
  这些 `SHAPE` 由 `addShape.ts` 走 `"Parim"` 通道创建，专门用来快速摆 blockout。
- **Characters**（角色）、**Sets**（场景 / 地点套装）、**Creatures**（生物）、**Memes**、**Skybox**（天空盒：Day / Night / Gray / Black / Empty）、**Animations**。

资产有两种加入方式：**点一下即添加**，或**拖拽到画布**（拖出时面板变透明让位给画布，
`Reopen after adding` 勾选框控制拖完是否自动再开面板）。

> 典型 blocking 流程：先放一个 **Skybox** 或 **Set** 定环境 → 用 **Cube / Sphere / Cylinder** 摆出主体和前景遮挡物
> → 放 **Point Light** 定光影 → 再放 **Character / Object** 精修。

## 3. 摆放、旋转、缩放（控制 object placement）

- 顶部切换模式：**Move (T)** / **Rotate (R)** / **Scale (G)**。
- **World / Local** 切换：点按钮或按 **X**（Gamer 方案）。
- 吸附 **N**、网格显示 **H**、复制 **Ctrl+D**、聚焦选中物体 **F**、删除 **Del / Backspace**、撤销 **Ctrl+Z** / 重做 **Ctrl+Shift+Z**。
- 3D 深度就是靠把物体在 Z 轴前后错开、缩放大小，制造前景 / 中景 / 背景层次。

## 4. 控制机位与景深（camera angles / depth）

- **自由飞相机**（Gamer 默认）：`WASD` 平移、`Q/E` 下降 / 上升、方向键俯仰 / 偏航。
- **渲染相机**：场景里有一个 `::CAM::` 相机占位体（线框视锥），选中它拖动 / 旋转就定最终出图的机位；
  **双击相机**（或 Outliner 里相机行右侧的进入图标）进入相机视角，**Space** 切换 / 退出相机视角。
- **焦距**：相机视角下有 `35mm` 焦距标签，可拖滑杆改。
- 习惯 Blender 的话，可在 **Settings → Keybinds** 把方案从 Gamer 换成 **Blender-like**（G/R/S + 数字小键盘导航），
  见 `keybinds/presets.ts`。

## 5. 输出

- 搭好场景后进入 **Record（Tab）** 模式：只读回放，只有播放 / 拖动进度条。
  - **Capture (C)**：把当前机位帧导出为静帧。
  - **Record (R)**：沿时间轴渲染成视频（时间轴可给物体加关键帧做动画）。

## 一句话总结

进 3D 场景 → `+` 打开 Presets 用几何体和套装快速 blockout → Move / Rotate / Scale 摆位
→ 拖 `::CAM::` 相机定机位 → Capture / Record 出图 / 出片。
