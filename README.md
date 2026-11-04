# 🍲 家味 · 家庭点餐系统

一个为家庭场景打造的点餐系统：**记录菜谱 → 手机点餐 → 自动生成本餐菜单与采购清单**。
后端 Rust（actix-web + sqlx），数据库 MySQL 8.0，前端 Vue 3（移动端 / 平板优先）。

> 没有 MySQL 也能先看效果：后端不可达时，前端会自动切换到**离线演示模式**，
> 内置 30 道示例菜谱与订单，所有页面与交互都能体验（数据存内存，刷新即还原）。

---

## ✨ 功能一览

| 模块 | 能力 |
|------|------|
| 🏠 首页 | 今日三餐概览、「猜你想吃」智能推荐（避开最近吃过的）、家常榜单、快速入口 |
| 📖 菜谱库 | 搜索、分类/菜系/难度/标签筛选、多种排序、无限滚动、收藏 |
| 🍽️ 点餐 | 购物车式选菜、逐道调份数/备注/点餐人、设置用餐日期餐次人数 |
| ✍️ 顺手建档 | **点餐时菜谱库里没有的菜，可现场新建并直接加入本单**（同一事务，不丢数据） |
| 🧾 订单 | 草稿/确认/烹饪中/已开饭全流程、出菜打勾、餐后评分 |
| 🧺 采购清单 | **按份数换算、跨菜合并去重**的食材清单，分「需采购 / 家中常备」，可勾选已买 |
| 👨‍🍳 这一餐怎么做 | 把本单所有菜的步骤串成烹饪流程单 |
| 📊 家的味道 | 分类分布、人气菜品、每位成员的口味画像 |
| 👨‍👩‍👧 家庭成员 | 轻量昵称制、免密码，点头像切换「我是谁」 |

---

## 🗂️ 项目结构

```
family-dining/
├─ database/
│  ├─ 01_schema.sql      # 建表脚本：11 表 + 5 视图 + 2 存储过程
│  ├─ 02_seed_data.sql   # 示例数据：5 成员 / 77 食材 / 30 菜谱 / 4 订单
│  ├─ backup/            # 标准 SQL 备份（可直接 mysql < 恢复）
│  │  ├─ family_dining_backup_latest.sql        # 最新备份
│  │  └─ family_dining_backup_<时间戳>.sql       # 带时间戳的存档
│  └─ tools/             # 数据库运维工具（Node，无需安装 mysql 客户端）
│     ├─ sql-runner.mjs      # SQL 解析器：自行处理 DELIMITER 与注释
│     ├─ preflight.mjs       # 只读预检：实例现状、目标库是否存在
│     ├─ apply-schema.mjs    # 建库建表 + 导入数据 + 核验
│     ├─ dump-database.mjs   # 导出标准备份
│     ├─ verify-dump.mjs     # 备份回环验证（导入临时库逐表比对后自动清理）
│     ├─ check-data.mjs      # 终态核验：结构/数据/业务链路
│     └─ test-parser.mjs     # SQL 解析器自检（离线）
├─ backend/              # Rust + actix-web + sqlx(MySQL)
│  ├─ Cargo.toml
│  ├─ .env               # 环境变量（可选，不建则用内置默认值）
│  └─ src/
│     ├─ main.rs         # 入口、CORS、静态托管
│     ├─ config.rs       # 配置读取
│     ├─ db.rs           # 连接池、连通性检查、订单号生成
│     ├─ error.rs        # 统一错误 → HTTP 映射
│     ├─ models.rs       # 数据库行模型 / 响应结构
│     ├─ dto.rs          # 请求体与参数校验
│     ├─ util.rs         # 统一响应、分页、枚举字典
│     └─ routes/
│        ├─ mod.rs       # 路由注册 + /health + /meta
│        ├─ recipes.rs   # 菜谱 CRUD、收藏、随机
│        ├─ orders.rs    # 下单、顺手建档、采购清单、出菜评分
│        ├─ basics.rs    # 成员 / 食材 / 标签
│        └─ stats.rs     # 统计、排行、口味画像、智能推荐
├─ frontend/             # Vue 3 + Vite + Pinia + Vue Router
│  ├─ package.json
│  ├─ vite.config.js
│  └─ src/
│     ├─ main.js  App.vue  router/  stores/
│     ├─ api/            # http 封装、接口定义、Mock 降级
│     ├─ components/     # DishCard / DishCover / RatingStars / Stepper / TabBar …
│     ├─ composables/    # useToast 全局提示
│     ├─ styles/         # 设计变量 + 全局样式
│     ├─ utils/format.js # 日期 / 时长 / 状态格式化
│     └─ views/          # 10 个页面
├─ start.bat             # Windows 一键启动
└─ README.md
```

---

## 🚀 快速开始

### 方式 A：一键启动（Windows，最简单）

先装好下面三样，然后双击 `start.bat`，脚本会自动构建前端并启动后端。

1. **MySQL 8.0**（见下方「准备数据库」）
2. **Rust**：https://rustup.rs
3. **Node.js 18+**：https://nodejs.org

### 方式 B：手动分步

**1) 准备数据库**

> ✅ **本项目的数据库已经建好并导入完成**，位于 `169.254.104.35:3306/family_dining`（MySQL 8.0.35）：
> 11 张表、5 个视图、2 个存储过程、14 个外键、15 个 CHECK 约束、29 个索引，
> 含 30 道菜谱 / 77 种食材 / 4 笔示例订单 / 60 项采购清单。**无需再执行下面两步即可直接启动。**
>
> 以下步骤仅在你要**换库、重建或迁移到别的机器**时才需要。

```bash
# 方式一：从标准备份一次性恢复（推荐，含结构+数据+视图+存储过程）
mysql -u root -p < database/backup/family_dining_backup_latest.sql

# 方式二：从源码脚本重建
mysql -u root -p < database/01_schema.sql
mysql -u root -p < database/02_seed_data.sql
```

> `01_schema.sql` 用了 `DELIMITER`，请用 **mysql 命令行客户端**执行，
> 不要粘贴到不支持 DELIMITER 的简易客户端（如某些网页版 SQL 窗口）。
>
> 本机没有 mysql 客户端时，可用 `database/tools/` 下的 Node 脚本（见文末「数据库运维工具」），
> 它自带 `DELIMITER` 解析，无需安装 MySQL 客户端。

**2) 配置后端连接**

后端所有配置都有内置默认值，**不建 `.env` 也能启动**（默认连 `root:root@169.254.104.35:3306/family_dining`）。
若你的数据库地址或账号密码不同，在 `backend/` 下新建一个 `.env` 文件覆盖即可：

```
DATABASE_URL=mysql://你的用户:你的密码@你的地址:3306/family_dining
PORT=8080
STATIC_DIR=../frontend/dist
```

> 密码含 `@ : / # ?` 等特殊字符时需 URL 编码，例如 `p@ss` 写成 `p%40ss`。
> 完整可选变量见文末「环境变量」表。

**3) 启动后端**

```bash
cd backend
cargo run --release
# 首次编译较慢（要下载并编译依赖），看到「监听地址」即成功
```

**4) 启动前端（开发模式，热更新）**

```bash
cd frontend
npm install
npm run dev
# 浏览器打开 http://localhost:5173
```

**5) 生产部署（前端交给后端托管，单端口访问）**

```bash
cd frontend && npm run build     # 产物在 frontend/dist
cd ../backend && cargo run --release
# 浏览器打开 http://localhost:8080
```

---

## 📱 手机 / 平板访问

前后端都监听 `0.0.0.0`，同一 WiFi 下，手机浏览器直接访问电脑的局域网 IP：

- 开发模式：`http://电脑IP:5173`
- 生产模式：`http://电脑IP:8080`

查电脑 IP：Windows 执行 `ipconfig`，看「无线局域网适配器」的 IPv4 地址。

界面已针对手机（单列 + 底部导航）和平板（多列网格 + 居中限宽）分别适配，
支持 iPhone 刘海屏安全区、支持「添加到主屏幕」当 App 用。

---

## ⚙️ 环境变量（backend/.env）

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `DATABASE_URL` | `mysql://root:root@169.254.104.35:3306/family_dining` | MySQL 连接串，密码含 `@:/#?` 需 URL 编码 |
| `HOST` | `0.0.0.0` | 监听地址 |
| `PORT` | `8080` | 监听端口 |
| `CORS_ORIGIN` | `*` | 跨域来源，多个用逗号分隔 |
| `STATIC_DIR` | `../frontend/dist` | 前端产物目录；设为 `none` 关闭托管 |
| `DB_MAX_CONNECTIONS` | `10` | 连接池大小 |
| `DB_CONNECT_RETRIES` | `3` | 启动时连接重试次数 |
| `RUST_LOG` | `family_dining_api=info` | 日志级别 |

---

## 🗄️ 数据库设计要点

- **字符集** `utf8mb4` + `utf8mb4_0900_ai_ci`，完整支持中文与 emoji（封面用 emoji）。
- **菜谱** 拆成 `recipe`（主表）+ `recipe_step`（步骤）+ `recipe_ingredient`（用量）。
- **食材** 用全局字典 `ingredient`，`is_staple` 标记家中常备，采购清单里自动折叠。
- **订单** `meal_order` + `order_item`，明细存 `recipe_snapshot` 菜名快照，
  即使菜谱日后改名或删除，历史订单依然可追溯。
- **采购清单聚合**（视图 `v_order_shopping_preview` + 存储过程 `sp_generate_shopping_list`）：
  `合计用量 = Σ(菜谱用量 × 点单份数 ÷ 菜谱默认份数)`，同名同单位跨菜合并，
  **单位不同则分开统计**（如「土豆 300克」与「土豆 2个」不会错误相加）。
- **免图床封面**：`cover_emoji` + `cover_gradient` 渲染好看封面，`cover_url` 可选放真实图片。
- 关键约束：外键 + `CHECK`（难度、餐次、状态、评分区间）+ 唯一键（菜名、昵称、标签名、订单号）。

---

## 🔌 主要 API

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/health` | 健康检查（含数据库连通状态） |
| GET | `/api/meta` | 前端下拉字典（分类/餐次/难度/封面预设） |
| GET | `/api/recipes` | 菜谱分页列表（关键词/分类/排序/筛选） |
| GET | `/api/recipes/{id}` | 菜谱详情（步骤+食材分组+标签） |
| POST | `/api/recipes` | 新建菜谱 |
| PUT | `/api/recipes/{id}` | 更新菜谱 |
| DELETE | `/api/recipes/{id}?permanent=` | 下架 / 彻底删除 |
| POST | `/api/recipes/{id}/favorite/{memberId}` | 切换收藏 |
| GET | `/api/recipes/random` | 随机推荐 |
| POST | `/api/orders` | **下单**（明细支持 `recipe_id` 或 `new_recipe` 顺手建档） |
| GET | `/api/orders/{id}` | 订单详情（含采购清单） |
| POST | `/api/orders/{id}/confirm` | 确认订单并生成采购清单 |
| GET | `/api/orders/{id}/shopping-list` | 采购清单 |
| PUT | `/api/orders/shopping/{itemId}` | 勾选食材已买 |
| GET | `/api/orders/{id}/menu` | 本餐菜单卡片 |
| GET | `/api/orders/{id}/recipes` | 本餐全部做法 |
| PUT | `/api/orders/items/{itemId}` | 改份数/出菜状态/评分 |
| GET | `/api/stats/overview` | 首页概览 |
| GET | `/api/stats/popular` | 人气排行 |
| GET | `/api/stats/suggestion` | 智能推荐（避开最近吃过的） |
| GET | `/api/stats/members/{id}/taste` | 成员口味画像 |

统一响应结构：`{ "code": 0, "message": "ok", "data": ... }`，`code != 0` 表示业务错误。

---

## 🛠️ 常见问题

**Q：打开页面顶部有黄色「离线演示模式」提示？**
说明前端没连上后端或数据库。检查：① 后端是否 `cargo run` 启动；② MySQL 服务是否在跑；
③ `backend/.env` 的 `DATABASE_URL` 账号密码是否正确；④ 是否已导入两个 SQL 脚本。
离线模式下界面完整可用，但数据是内存示例，不会入库。

**Q：后端日志报「数据表或视图不存在」？**
说明连到了空的库或连错了库。本项目数据库已建好，检查 `backend/.env` 的 `DATABASE_URL`
是否指向 `169.254.104.35:3306/family_dining`；换机器时用 `database/backup/family_dining_backup_latest.sql` 恢复。

**Q：`cargo run` 很慢？**
首次要编译全部依赖（几百个 crate），属正常现象，之后增量编译很快。
可用 `cargo run`（debug）代替 `cargo run --release` 加快首次编译。

**Q：端口 8080 被占用？**
改 `backend/.env` 里的 `PORT`，前端开发模式的代理目标在 `frontend/vite.config.js` 的 `VITE_API_TARGET`。

**Q：npm 报执行策略错误（禁止运行脚本）？**
用 `npm.cmd` 代替 `npm`，或让管理员放开 PowerShell 执行策略。

---

## 🗄️ 数据库运维工具

`database/tools/` 提供一套基于 Node 的数据库工具，**本机没有 mysql / mysqldump 客户端也能用**。
其中 `sql-runner.mjs` 自带 `DELIMITER` 解析，因此能正确执行含存储过程的脚本
（这一点驱动本身做不到，需要自行切分语句）。

```bash
cd database/tools
npm install          # 首次：安装 mysql2 驱动
```

| 命令 | 作用 | 是否写库 |
|------|------|---------|
| `npm run test-parser` | SQL 解析器自检（离线，不连库） | 否 |
| `npm run preflight` | 预检：服务器版本、实例上有哪些库、目标库是否存在 | 否（只读） |
| `npm run apply` | 建库建表 + 导入示例数据 + 核验行数 | **是**（会 DROP 重建目标库） |
| `npm run dump` | 导出标准 SQL 备份到 `database/backup/` | 否（只读） |
| `npm run verify-dump` | 备份回环验证：导入临时库、逐表比对校验和、验证约束后自动删除临时库 | 是（仅临时库） |
| `node check-data.mjs` | 终态核验：结构 / 数据行数 / 约束 / 业务链路 | 否（只读） |

连接信息默认读环境变量，未设置时用内置默认值：

```
DB_HOST / DB_PORT / DB_USER / DB_PASSWORD / DB_NAME
```

### 关于备份文件

`family_dining_backup_latest.sql` 是标准备份，格式对齐 mysqldump 惯例，
包含建库、真实 `SHOW CREATE TABLE` 结构、批量 INSERT、视图与存储过程（`DELIMITER $$` 包裹），
可被 mysql 客户端、Navicat、DBeaver 等直接导入。

两点实现细节值得留意：

- **生成列不参与 INSERT**：`recipe.total_minutes` 是 `GENERATED ALWAYS AS ... STORED`，
  若写入会报 `ER_NON_DEFAULT_VALUE_FOR_GENERATED_COLUMN` 导致整份备份无法还原，因此按 mysqldump 惯例排除。
- **判定生成列用 `GENERATION_EXPRESSION` 而非 `EXTRA LIKE '%GENERATED%'`**：
  后者会把 `created_at` / `updated_at`（EXTRA 为 `DEFAULT_GENERATED`）误判成生成列，导致时间戳数据丢失。

> ⚠️ `npm run apply` 会先 DROP 再重建 `family_dining` 库，仅用于重建/迁移场景。

---

## 🧰 技术栈

- **后端**：Rust 1.75+ · actix-web 4 · sqlx 0.8（编译期无关的运行时查询）· serde · chrono · rust_decimal
- **数据库**：MySQL 8.0.16+（用到 CHECK 约束、生成列、窗口函数、CTE、ngram 全文索引）
- **前端**：Vue 3.5 · Vite 6 · Pinia · Vue Router 4 · Axios（无 UI 框架，样式手写）
- **运维工具**：Node.js · mysql2

---

## ✅ 当前部署状态

数据库已在 `169.254.104.35:3306`（MySQL 8.0.35）建好并验证通过：

| 项目 | 结果 |
|------|------|
| 结构 | 11 表 / 5 视图 / 2 存储过程 / 14 外键 / 15 CHECK 约束 / 29 索引 |
| 字符集 | `utf8mb4` + `utf8mb4_0900_ai_ci`（中文与 emoji 往返无损） |
| 数据 | 596 行，11 张表行数全部符合预期 |
| 备份验证 | 回环导入 73/73 语句成功，11 张表行数与内容校验和完全一致 |
| 约束验证 | 非法主题色、越界评分/份数、非法角色枚举均被正确拦截 |
| 业务链路 | 份数换算（600g×2÷3=400g）、生成列、视图、窗口函数、全文索引、存储过程调用均正常 |
| 后端连通 | 已实测 `MySQL 连接成功`、`数据库状态：连通正常` |

---

祝一家人吃得开心 🍚
