# 📥 学习板块 · JSON 数据格式说明

「常用功能 → 学习」支持用**一份固定格式的 JSON** 批量导入题库、单词与知识点。
导出的文件结构与导入格式完全一致，可以反复导入、跨设备迁移。

- 当前格式版本：`version: 1`
- 类型标识：`type: "family-dining.study"`
- 现成模板：`frontend/public/study-template.json`（页面内可一键下载）
- 数据存储：浏览器 `localStorage`（键名 `fd_study`），不写数据库、不上传服务器

---

## 1. 顶层结构

```json
{
  "version": 1,
  "type": "family-dining.study",
  "exported_at": "2026-10-19 10:00:00",
  "data": {
    "subjects":  [],
    "questions": [],
    "english":   [],
    "notes":     [],
    "progress":  { "questions": {}, "words": {}, "notes": {} }
  }
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `version` | number | 建议填 | 格式版本。高于程序支持版本时导入会被拒绝并提示升级 |
| `type` | string | 否 | 固定 `family-dining.study`，便于识别文件用途 |
| `exported_at` | string | 否 | 导出时间，仅作记录 |
| `data.subjects` | array | 否 | 学科字典。不填也能导入，题目/知识点里的 `subject` 会自动兜底显示 |
| `data.banks` | array | 否 | 题库元数据（名称/展示图片/说明）。名称唯一，刷题页轮盘按名称排序展示 |
| `data.questions` | array | 三选一 | 题库 |
| `data.english` | array | 三选一 | 英语单词 |
| `data.notes` | array | 三选一 | 知识点 |
| `data.progress` | object | 否 | 练习进度。手工准备数据时留空对象即可 |

> `questions` / `english` / `notes` 至少要有一项非空，否则提示「文件里没有可导入的数据」。

### 兼容的简化写法

为了降低手写成本，导入器还接受这两种写法：

```json
// 写法 A：省略外层，直接给 data 内容
{ "questions": [ ... ], "english": [ ... ], "notes": [ ... ] }

// 写法 B：只给一个数组，按字段特征自动归类
[ { "stem": "...", "answer": ["A"] }, { "word": "apple", "meaning": "苹果" } ]
```

归类规则：有 `word`/`en` → 单词；有 `stem`/`question` → 题目；有 `title`/`content` → 知识点。

别名也做了兼容：`english` 可写成 `words`，`notes` 可写成 `knowledge`。

---

## 2. subjects · 学科字典

```json
{ "value": "math", "name": "数学", "emoji": "🔢" }
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `value` | string | 是* | 学科标识，题目与知识点通过它关联。缺省时用 `name` 顶替 |
| `name` | string | 是* | 显示名称。缺省时用 `value` 顶替 |
| `emoji` | string | 否 | 图标，默认 `📘` |

\* `value` 与 `name` 不能同时为空，否则该条被跳过。

内置预设（不写 `subjects` 也能直接用）：
`chinese 语文` `math 数学` `english 英语` `physics 物理` `chemistry 化学`
`biology 生物` `history 历史` `geography 地理` `politics 道德与法治` `other 其他`

---

## 2.5 banks · 题库元数据

题库（试卷）级别的元数据：名称唯一，附带展示图片与说明，刷题页以「试卷轮盘」按名称排序呈现。

```json
{ "name": "三年级数学上册", "cover": "/api/uploads/xxx.jpg", "desc": "人教版三年级上册期末复习卷" }
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `name` | string | **是** | 题库唯一名称，与题目 `bank` 字段对应；重名会被拒绝 |
| `cover` | string | 否 | 展示图片地址（`/api/uploads/xxx`）。无图时轮盘用渐变占位 |
| `desc` | string | 否 | 说明/简介，轮盘信息卡展示 |

> 界面导入时会自动生成 `banks` 元数据（名称必填、图片可选上传、说明可选）；
> 手工准备 JSON 时可不写 `banks`，题目仍会按 `bank` 字段归类，只是轮盘上无图片与说明。

---

## 3. questions · 题库

```json
{
  "id": "q-001",
  "subject": "math",
  "type": "single",
  "difficulty": "easy",
  "stem": "一元二次方程 x² - 5x + 6 = 0 的两个根是？",
  "options": [
    { "key": "A", "text": "1 和 6" },
    { "key": "B", "text": "2 和 3" }
  ],
  "answer": ["B"],
  "analysis": "因式分解得 (x-2)(x-3)=0。",
  "tags": ["一元二次方程", "必考"],
  "source": "课本例题 3-2",
  "score": 3
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | 否 | 题目唯一标识，**合并导入靠它判断是新增还是覆盖**。不填自动生成 |
| `subject` | string | 否 | 学科 `value` |
| `type` | string | 否 | 题型，见下表。非法值回落为 `single` |
| `difficulty` | string | 否 | `easy` 基础 / `medium` 提高 / `hard` 拔高。默认 `easy` |
| `stem` | string | **是** | 题干。缺失该条被跳过 |
| `options` | array | 选择题必填 | 选项，选择题至少 2 个 |
| `answer` | array | **是** | 正确答案，见各题型写法 |
| `analysis` | string | 否 | 解析，作答后展示 |
| `tags` | array/string | 否 | 标签。字符串可用逗号、顿号、分号分隔 |
| `source` | string | 否 | 出处 |
| `score` | number | 否 | 分值，默认 1 |

### 题型与 answer 写法

| `type` | 名称 | `options` | `answer` 写法 | 判分方式 |
|--------|------|-----------|---------------|----------|
| `single` | 单选题 | 必填 ≥2 | `["B"]`，取第一个 | 自动判分 |
| `multiple` | 多选题 | 必填 ≥2 | `["A","C","D"]`，**至少 2 个**，顺序无关 | 自动判分（须全对） |
| `judge` | 判断题 | 可省略 | `["T"]` 正确 / `["F"]` 错误 | 自动判分 |
| `fill` | 填空题 | 忽略 | `["一览众山小"]` | 自动判分，忽略大小写与空格 |
| `answer` | 问答题 | 忽略 | 参考答案文本 | 不自动判分，作答后翻答案自评 |

**判断题的 `options` 可以完全不写**，程序会自动补「正确 / 错误」两项。
答案也兼容多种写法：`true/false`、`对/错`、`正确/错误`、`√/×`、`是/否`、`1/0`，统一归一为 `T`/`F`。

**填空题多个可接受答案**用竖线分隔在一个字符串里：

```json
"answer": ["一览众山小|一覽眾山小"]
```

**选项的简写**：`options` 里的元素可以直接写字符串，程序按顺序自动分配 A/B/C/D：

```json
"options": ["1 和 6", "2 和 3", "-2 和 -3"]
```

**字段别名**：`stem` ← `question`/`title`；`analysis` ← `explanation`/`parse`；
`answer` ← `answers`/`correct`/`correct_answer`；选项内 `text` ← `label`/`content`。

---

## 4. english · 单词

```json
{
  "id": "w-001",
  "word": "abandon",
  "phonetic": "/əˈbæn.dən/",
  "meaning": "v. 放弃；抛弃",
  "example": "He had to abandon the plan.",
  "example_cn": "他不得不放弃这个计划。",
  "level": "初中",
  "unit": "Unit 3",
  "tags": ["高频词"]
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | 否 | 唯一标识，合并导入靠它去重 |
| `word` | string | **是** | 单词。缺失该条被跳过 |
| `phonetic` | string | 否 | 音标 |
| `meaning` | string | 否 | 释义 |
| `example` | string | 否 | 英文例句 |
| `example_cn` | string | 否 | 例句翻译 |
| `level` | string | 否 | 等级/学段，如「小学」「初中」「高中」「CET4」，用于筛选 |
| `unit` | string | 否 | 单元/课次，如「Unit 3」，用于筛选 |
| `tags` | array/string | 否 | 标签 |

字段别名：`word` ← `en`/`english`；`phonetic` ← `pronunciation`；
`meaning` ← `cn`/`translation`；`example` ← `sentence`；`example_cn` ← `sentence_cn`；`unit` ← `lesson`。

三种练习模式都基于这些字段：
- **看英文选中文**：用 `word` + `phonetic` 出题，`meaning` 做选项
- **看中文选英文**：用 `meaning` 出题，`word` 做选项
- **拼写默写**：给 `meaning`、音标与首字母提示，要求拼出 `word`

朗读功能使用浏览器内置语音合成（`speechSynthesis`），不支持时会提示，不影响其他功能。

---

## 5. notes · 知识点

```json
{
  "id": "n-001",
  "subject": "math",
  "title": "一元二次方程求根公式",
  "content": "对于 ax² + bx + c = 0（a ≠ 0）：\nx = [-b ± √(b² - 4ac)] / 2a",
  "key_points": [
    "判别式 Δ = b² - 4ac",
    "两根之和 = -b/a，两根之积 = c/a"
  ],
  "tags": ["代数", "必考"],
  "is_exam_point": true
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | 否 | 唯一标识 |
| `subject` | string | 否 | 学科 `value` |
| `title` | string | 是* | 标题。缺失时自动取 `content` 前 20 字 |
| `content` | string | 是* | 正文，`\n` 换行会原样保留 |
| `key_points` | array/string | 否 | 要点列表，页面里单独高亮成一块 |
| `tags` | array/string | 否 | 标签，可筛选 |
| `is_exam_point` | boolean | 否 | 考点标记，为 `true` 时列表里优先显示并带 ⭐ |
| `updated_at` | string | 否 | 更新时间，用于列表排序。不填按导入时刻计 |

\* `title` 与 `content` 不能同时为空，否则该条被跳过。

字段别名：`title` ← `name`；`content` ← `body`/`text`；`key_points` ← `points`/`keywords`；
`is_exam_point` ← `exam_point`/`important`。

---

## 6. progress · 练习进度（可选）

导出的文件会带上进度，方便换设备继续练。手工准备数据时留空对象即可。

```json
"progress": {
  "questions": {
    "q-001": { "attempts": 3, "correct": 2, "wrong": 1, "last_result": "wrong", "last_at": "2026-10-19 10:00:00" }
  },
  "words": {
    "w-001": { "status": "learning", "reviews": 5, "attempts": 4, "correct": 3, "last_at": "2026-10-19 10:00:00" }
  },
  "notes": {
    "n-001": { "views": 2, "mastered": true, "last_at": "2026-10-19 10:00:00" }
  }
}
```

- `questions[id].last_result`：`"wrong"` 时该题进入**错题本**
- `words[id].status`：`new` 未学 / `learning` 学习中 / `mastered` 已掌握
- `notes[id].mastered`：知识点是否标记为已掌握

> 导入时进度**默认不覆盖**（避免清掉本地已有练习记录）。仅在调用 `importJSON(text, { withProgress: true })` 时才合并进度。

---

## 7. 导入模式

| 模式 | 行为 | 适用场景 |
|------|------|----------|
| **增量合并**（默认） | 按 `id`（学科按 `value`）匹配：已存在则覆盖字段，不存在则新增。练习进度保留 | 日常补充题目、换一份单词表 |
| **整体替换** | 清空现有题库、单词、知识点与**全部练习进度**，只保留本次导入内容 | 换学期、换教材、彻底重置 |

界面会在结果里报告新增/覆盖条数，并列出被跳过的条目及原因（如「缺少题干」「多选题的正确答案至少 2 个」），最多显示前 8 条。

---

## 8. 校验规则速查

导入是**逐条容错**的：单条不合法只跳过该条，不影响其余数据。

会被跳过的情况：

- 题目：无 `stem`；选择题选项不足 2 个；选择题/判断题无 `answer`；多选题答案不足 2 个
- 单词：无 `word`
- 知识点：`title` 与 `content` 同时为空
- 学科：`value` 与 `name` 同时为空

会被整体拒绝的情况：

- JSON 解析失败（语法错误）
- 顶层既不是对象也不是数组
- `version` 高于程序支持的版本
- `questions` / `english` / `notes` 全为空

自动纠正（不报错）：

- 选择题答案 key 统一转大写，并过滤掉不在 `options` 里的 key
- 多选题答案去重后按字母排序
- 判断题答案归一为 `T`/`F`
- `tags` 字符串按 `, ， 、 ; ； |` 切分
- `score` 非正数时回落为 1

---

## 9. 三种使用方式

**① 页面内导入（推荐）**
常用功能 → 学习 → 📥 导入 JSON，粘贴内容或选择 `.json` 文件，选模式后导入。
弹层里有「填入模板」按钮，可直接在页面上改内容。

**② 下载模板改**
学习页 → 「下载空白模板」，得到 `study-template.json`，改完再导入。
构建后也可从站点根路径直接取：`/study-template.json`。

**③ 程序化调用**

```js
import { useStudyStore } from '@/stores/study'

const study = useStudyStore()

// 导入（文件文本）
const res = study.importJSON(text, { mode: 'merge', withProgress: false })
if (!res.ok) console.error(res.error)

// 导出（可直接再导入）
const json = study.exportJSON({ withProgress: true, pretty: true })

// 单条增改（页面表单走的就是这些）
study.saveQuestion({ subject: 'math', type: 'single', stem: '…', options: […], answer: ['A'] })
study.saveWord({ word: 'apple', meaning: '苹果' })
study.saveNote({ title: '…', content: '…', is_exam_point: true })
```

---

## 10. 相关文件

| 路径 | 作用 |
|------|------|
| `frontend/src/stores/study.js` | 数据层：字段规范化、校验、导入导出、进度记录 |
| `frontend/src/views/StudyView.vue` | 学习首页 + JSON 导入导出界面 |
| `frontend/src/views/StudyQuizView.vue` | 刷题：顺序 / 随机 / 错题本 |
| `frontend/src/views/StudyEnglishView.vue` | 英语学习：单词卡 + 三种记忆模式 |
| `frontend/src/views/StudyNotesView.vue` | 知识点：卡片浏览 + 单条增删改 |
| `frontend/public/study-template.json` | 可直接导入的完整示例模板 |

后期若要扩充格式（如新增题型），把 `study.js` 里的 `STUDY_VERSION` 加 1，
并在 `normalizeQuestion` / `normalizeWord` / `normalizeNote` 里补字段即可；
旧文件因为 `version` 较低仍可正常导入。
