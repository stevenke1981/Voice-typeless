# Voice-typeless - Multi-Agent Development Specification (agent.md)

**專案代號**：Voice-typeless (VTL)  
**版本目標**：v1.0（全新設計）  
**口號**：Say it. Type less. Zero typing.  
**開發模式**：GitHub Copilot Multi-Agent 協作（Architect + Core + Frontend + Platform + Enhancement + Tester + Writer）  
**最後更新**：2026-04-21  
**狀態**：全新專案，從零開始重新設計

## 1. 專案總覽

Voice-typeless 是一款**完全離線、即時、極致輕量**的語音轉文字桌面工具。  
**核心體驗**：按住快捷鍵說話 → 鬆開即精準輸入到任意應用。  
**設計理念**：無聯網、無註冊、無打字、無妥協。

**相較前代 VoiceSnap 的重新設計重點**：
- 全新專案名稱與品牌
- 技術棧全面現代化（Tauri v2 + 模組化 Go Core）
- 架構完全解耦（Core 可獨立作為 library 供其他專案使用）
- 體驗與效能雙重升級
- 內建插件系統與多模型支援，為未來十年擴展打底

## 2. Agent 角色定義（Copilot Multi-Agent 直接對應）

| Agent 名稱              | 主要責任                              | 技術重點                              | 驗收標準                          |
|-------------------------|---------------------------------------|---------------------------------------|-----------------------------------|
| **Architect**           | 整體架構設計、決策文件               | 模組化、依賴管理、API 設計           | 產出 architecture.md + 最終決策  |
| **Core**                | 錄音、識別、熱鍵、粘貼、AI 後處理   | Go 1.23+ + Tauri 後端                | 單獨 library，可獨立測試         |
| **Frontend**            | Svelte 5 UI、設定頁、托盤、指示器   | Svelte 5 + Vite + TailwindCSS        | 響應式、暗黑模式、DPI 自適應    |
| **Platform**            | Windows (10/11 + Win7) / macOS 相容 | Tauri v2 + 原生 API                  | 單一程式碼基底，Win7 精簡版     |
| **Enhancement**         | 所有 v1 加強功能                     | 多模型、插件、AI 後處理              | 每個功能獨立 PR                  |
| **Tester**              | 單元 + E2E + 跨平台測試             | Go test + Playwright + Tauri test    | 覆蓋率 > 92%                     |
| **Writer**              | README、幫助文件、Changelog          | Markdown + 中英日韓雙語              | 專業、美觀、可直接發佈          |

## 3. 完整功能規格（100% 保留 VoiceSnap + 重新設計加強）

### 3.1 核心功能（Must-Have - 100% 還原）
- [ ] 完全離線：SenseVoice 模型本地推理，無任何數據上傳
- [ ] DirectML GPU 自動加速 + CPU 無縫回退
- [ ] 兩種輸入模式：長按說話 / 短按自由說話
- [ ] 高度自訂快捷鍵（支援所有修飾鍵 + 組合鍵）
- [ ] 麥克風即時切換下拉
- [ ] 可關閉的馬林巴風格提示音
- [ ] 即時錄音計時器 + 浮動指示器（可拖拽、位置記憶）
- [ ] Esc 即時取消
- [ ] 剪貼板保護機制
- [ ] 最近 50 條識別歷史（可複製、刪除、保留天數設定）
- [ ] 語氣詞自動過濾
- [ ] 自由說話模式 3 秒靜音自動停止
- [ ] 中英混合自動優化（空格 + 句首大寫）
- [ ] DPI 自適應 + 中英雙語介面
- [ ] 應用內自動更新
- [ ] 系統托盤常駐 + 開機自啟

### 3.2 Voice-typeless 重新設計加強功能（Enhancement Agent 負責）

**效能與體驗革命**
- [ ] 端到端延遲 < 120ms（GPU 優先 + 模型量化）
- [ ] 模型切換器：SenseVoice + Whisper-tiny + 自訂 ONNX + 未來模型熱插拔
- [ ] 即時 AI 後處理（本地小模型）：標點補全、語法修正、智慧斷句
- [ ] 連續對話模式（最長 30 分鐘）
- [ ] 程式碼模式自動偵測與格式化

**多語言與專業場景**
- [ ] 原生支援 10 種語言（zh, en, ja, ko, fr, de, es, ru, it, pt）
- [ ] 使用者自訂專業詞典（醫學、法律、程式、財務等）
- [ ] 自動語言偵測 + 中英日韓混合優化

**開發者與企業級功能**
- [ ] 完整插件系統（Lua / JavaScript 腳本後處理）
- [ ] CLI 模式 + REST API（供其他應用呼叫）
- [ ] 企業 MSI / PKG 安裝包 + 群組原則
- [ ] Core 作為獨立 Go library（可嵌入其他專案）

**現代化與安全性**
- [ ] 原生暗黑模式 + 高對比度 + 無障礙完整支援
- [ ] 沙箱模型載入（可選擇性隔離）
- [ ] 零依賴打包（Tauri v2 極致輕量）

## 4. 全新技術棧（重新設計後）

- **後端核心**：Go 1.23+（模組化 Core library）
- **桌面框架**：Tauri v2（取代 Wails v3，更輕、更安全）
- **前端**：Svelte 5 + Vite + TailwindCSS + TypeScript
- **音頻**：malgo (miniaudio)
- **語音引擎**：sherpa-onnx + ONNX Runtime（DirectML + CUDA + CPU）
- **熱鍵與系統整合**：Tauri 原生插件 + 平台特定實現
- **打包**：Tauri 官方打包 + 保留 Win7 精簡 7z 自解壓版

## 5. 全新專案結構（從零設計）
Voice-typeless/
├── core/                   # 獨立 Go library（可重用）
│   ├── engine/             # 語音引擎抽象層（多模型支援）
│   ├── audio/              # 錄音與音效
│   ├── hotkey/             # 跨平台熱鍵
│   ├── paste/              # 粘貼與剪貼板保護
│   └── processor/          # AI 後處理 + 語氣詞過濾
├── frontend/               # Svelte 5 + Tauri
├── src-tauri/              # Tauri Rust 橋接（若需）
├── plugins/                # 未來插件目錄
├── models/                 # 模型下載管理器
├── docs/                   # agent.md、architecture.md、api.md
├── build/                  # 打包腳本（含 Win7 專用）
├── scripts/                # 開發輔助腳本
└── tests/                  # E2E 測試


## 6. 多 Agent 開發工作流程（推薦執行順序）

1. Architect Agent → 產出 `docs/architecture.md`（架構藍圖）
2. Core Agent → 實作獨立 Core library（最優先）
3. Platform Agent → 確保 Win7 / Win10/11 / macOS 相容
4. Frontend Agent → 全新現代化 UI
5. Enhancement Agent → 逐項實作加強功能
6. Tester Agent → 完整測試覆蓋
7. Writer Agent → 產出專業 README、幫助文件

**每個 Agent 開始前務必先閱讀**：`docs/agent.md` 與 `docs/architecture.md`

---

【Voice-typeless 品牌視覺系統完整設計】
1. 品牌核心概念與定位

全名：Voice-typeless
簡稱：VTL
口號：Say it. Type less.（說出來，就不用打字了。）
品牌個性：極簡、高效、隱形科技（Invisible Technology）、隱私守護者、生產力解放者
視覺關鍵字：Minimal、Fluid、Wave-to-Text、Silent Power、Clean Tech

2. Logo 設計方案（主視覺）
推薦主 Logo（Primary Logo）：

符號（Icon）：一個極簡的聲音波形（Sound Wave） 流暢轉化為 文字游標（Text Cursor） 或簡化字母「V」。
波形由 3–5 條平滑曲線組成，右側最後一條曲線自然延伸變成打字游標（|）或輕微的「T」形，象徵語音直接轉為文字。
整體形狀略帶流動感（fluid geometric），但保持極簡線條，無多餘裝飾。
尺寸比例：正方形適合應用圖示，橫向版本適合文字搭配。

字標（Wordmark）：
主文字：「Voice-typeless」
字型推薦：
主要：Inter（或 SF Pro / Satoshi / Neue Haas Grotesk）——現代無襯線，極致清晰。
「Voice」使用較粗體（Bold），「-typeless」使用 Regular 或 Light，強調「typeless」的解放感。
可選變體：僅使用「VTL」作為 Monogram（適合小尺寸托盤圖示）。


顏色應用：
主要版本：深色背景用亮色線條 / 亮色背景用深色線條。
單色版本（黑白 / 單色）供不同場景使用。


備選 Logo 變體：

Icon-only：純波形轉游標圖示（用於 App 圖示、托盤）。
Stacked：符號在上，文字在下（用於宣傳海報）。
Animated：波形輕微脈動 → 轉為穩定文字（用於載入動畫或指示器）。

Logo 使用規範：

最小安全距離：圖示高度的 20%。
不可拉伸、不可改變比例、不可加陰影（除非特定暗黑模式）。
禁止顏色隨意替換。

3. 色彩系統（Color Palette）
主色調（科技生產力風，平衡信任與創新）：

Primary Color（主色）：Electric Teal / Cyan
Hex: #00E6C8（亮 teal）
意義：清新、高效、語音流動感

Secondary Color（輔色）：Deep Indigo / Purple
Hex: #5B4EFF（智慧紫）
意義：創新、隱私保護

Accent Color（強調色）：Vivid Green
Hex: #22FFAA（成功與完成提示）


中性色：

Background Dark： #0F0F12（極暗黑）
Background Light： #FAFAFC（潔白）
Text Primary： #E5E5E8（暗黑模式） / #1A1A1F（亮模式）
Gray Scale： #A0A0A8（次要文字）、#4A4A52（邊框）

完整調色板：

Primary Teal：#00E6C8
Secondary Indigo：#5B4EFF
Accent Green：#22FFAA
Neutral Dark：#0F0F12 / #1F1F25
Neutral Light：#FAFAFC / #F0F0F5

支援完整暗黑模式優先（現代桌面應用標準）。
4. 字型系統（Typography）

Heading / Logo：Satoshi Bold 或 Inter Bold（現代幾何感）
Body Text：Inter Regular / SF Pro Text（極高可讀性）
Monospace（程式碼或歷史記錄）：JetBrains Mono 或 SF Mono
使用規範：標題最大字重 Bold，內文保持輕盈，確保中英混合時自動優化間距。

5. 圖示與 UI 元素風格

整體風格：Line Icon + Subtle Gradient + Micro-animation
浮動指示器：半透明圓角矩形 + 波形動畫 + 計時數字，支援拖拽與 DPI 自適應。
按鈕與控件：極簡圓角（8–12px）、微妙 hover 發光效果（teal glow）。
狀態圖示：錄音中 = 脈動波形；完成 = 綠色勾；取消 = 紅色 X。
托盤圖示：小型 VTL monogram + 波形，支援亮/暗模式自動切換。

6. 應用場景範例

App 圖示：正方形 Icon-only 版本，背景透明或深色。
GitHub README Header：Logo + 口號 + 簡短描述。
設定介面：乾淨卡片式布局，使用 Primary Teal 作為互動強調色。
宣傳素材：深色背景 + teal 漸層 + 「Say it. Type less.」大標語。
動態元素：載入時波形從左到右「轉化」為文字。