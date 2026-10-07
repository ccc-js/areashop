# plan0.2 — 可預約項目 + 預約（單一流程）

> 目標：在 v0.1 商品市集上疊加「可接案時間 + 預約」能力，只做一種通用流程。
> 覆蓋場景：3 貓狗寄養、6 理髮、7 診所（全部用同一套） ｜ 依賴：v0.1 的 users/shops/areas/Auth ｜ 工期參考：1–2 週

## 1. 本版目標與驗收

- [ ] 店家可新增可預約項目（標題、價格、須知）；寄養只是其中一個項目，沒有特殊流程
- [ ] 店家設好**每週固定可接案時間**：每天開/關＋自填時段，每個時段就是（起點，終點）一對，可任意增減
- [ ] 臨時狀況逐日處理：**當日額滿** / **當日不營業**兩個開關，不用動範本
- [ ] 居民選日期（＋時段）預約；不營業/額滿的日子選不進去
- [ ] 店家可確認/取消，居民可取消，履約後完成；爽約可記 no-show
- [ ] Demo：寄養、理髮、診所各 1 個項目，各完成 1 筆預約

**不做**：種類分支（無差別）、人數/名額計算（滿了店家自己勾）、預切時段格、線上付款、視訊看診、病歷、Redis、站內通知（輪詢列表即可，v0.4 再做）。

## 2. 核心概念

```text
可預約項目 → 週範本（每天 開/關 + 時段清單） → 單日開關（額滿/不營業） → 預約
```

- **時段＝(起點，終點)**：賣家自由填寫（如 09:00、中午、晚上都可），一天可加好幾個，也可刪；系統只存不解讀，顯示＋給買家選用。
- **預約單位**：`(項目, 日期[, 時段])`；時段有列才需選，沒列就只約日期。
- **某日可約 ⇔ 範本有開，且沒被勾額滿/不營業**。人數不進系統。
- **改時間一律寫單日例外**：正常接 / 當日額滿 / 當日不營業三選一＋備註，不動範本。
- **日期存 TEXT**（`YYYY-MM-DD`，字串可比大小，SQLite/PG 通用，不碰 timestamptz）；weekday 由程式算（0=週日..6=週六）。

## 3. 前端交付

- 新路由：`/s/:id` 項目詳情（選日期＋時段）、`/book/:serviceId` 下單確認、`/appointments`、`/appointments/:id`、`/provider`（週範本＋單日開關＋當日預約）。
- 元件：
  - `WeekTemplateEditor`：7 天，每天然後開關＋時段清單（起點/終點輸入＋新增，逐條刪除），一次存檔。不用表格。
  - `DayToggle`：月視圖小格子，點一下選中看明細，點兩下快速開/關。
  - `DayDetail`：當日預約列表（一鍵確認/取消/完成）＋三選一（正常接/當日額滿/當日不營業）＋備註。
  - `BookSheet`：選日期（不營業/額滿灰掉）→ 選時段 → 填需求說明 → 送出。
- 校驗：不可約過去日期、不營業/額滿日；需求說明純文字（寄養要寫寵物資訊就寫在這欄）。

## 4. 後端交付

- 建表（沿用 v0.1 模式：新增 entities + `schema.rs::setup_schema`，不寫 migration SQL）：
  - `services`（id, shop_id FK, title, description nullable, price_cents, cancel_hours default 24, notice nullable, status[on|off], created_at）
  - `availability_rules`（id, service_id FK, weekday 0–6, open, windows JSON[{start,end}]）+ unique(service_id, weekday)
  - `availability_exceptions`（id, service_id FK, date TEXT, open, full, note nullable）+ unique(service_id, date)
  - `appointments`（id, service_id, shop_id, buyer_id, date TEXT, window nullable, note nullable, status[pending|confirmed|completed|cancelled|noshow], created_at）
- 可接判斷寫成純函數 `resolve(rules, exceptions, date) -> {open, full, windows}`（無 DB，好單元測試）；例外優先於範本。
- 防重複送單：同一人同項目同日只留一筆有效（pending/confirmed）預約，多送回 400。
- 狀態機：`pending→confirmed→completed | cancelled | noshow`；`confirm`/`noshow` 限店主，`complete`/`cancel` 雙方；取消檢查 `now < 當日 00:00 - cancel_hours` 否則 422。
- 相容：舊庫 `availability_exceptions` 沒有 `full` 欄，啟動時 `ALTER TABLE ADD COLUMN` 補；舊格式 windows 字串（`"09:00-12:00"`）讀取時轉成對，PUT 一律寫新格式。

## 5. API 契約（新增）

```text
POST /services {shop_id,title,price_cents,cancel_hours?,notice?}
GET  /services?area_id=&q=   GET /services/:id?month=YYYY-MM（附該月每日 open/full/windows）
PATCH /services/:id（店主：改標題/價格/notice/上下架）
PUT  /services/:id/rules {weekly:[{weekday,open,windows:[{start,end}]}]}（整包覆寫，起終點自由填、不可空）
POST /services/:id/exceptions {date,open,full?,note?}（upsert）
DELETE /services/:id/exceptions/:date（刪例外，回到範本）
POST /appointments {service_id,date,window?,note?} → 201 {appointment}
PATCH /appointments/:id {action: confirm|cancel|complete|noshow}
GET  /appointments?role=buyer|seller&from=&to=   GET /appointments/:id
GET  /provider/calendar?shop_id=&month=（按日：預約列表，店主 scope）
```

## 6. 種子（Demo 用，舊庫自動補）

- 3 個項目＋週範本（週二至週六開，每天 09:00-12:00、14:00-18:00 兩段）：
  - 寄養（個人店「金城寵物寄養」）、理髮（「金城快剪」）、診所掛號（「金城安心診所」，notice 寫「僅掛號登記」）。
- 法務文案：診所項目 notice 固定寫「本平台僅提供掛號登記，不提供診斷與處方」。

## 7. 測試與 DoD

- 單元測試：狀態機非法轉換拒絕；`resolve()` 例外優先、無規則即公休、full 視為不可約；時段格式檢查；爛日期；閏月。
- `./test.sh` 加 e2e：建項目→設範本（爛 weekday/爛時段 400）→預約→同人同日重複 400→過去/亂填日期 400→亂填時段 400→confirm/noshow 權限→取消政策 422→單日關閉→刪例外恢復→勾額滿→取消額滿恢復→月曆/列表/下架。
