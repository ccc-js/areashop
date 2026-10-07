# v0.6 規劃 — 上線缺口：圖片、通知、店鋪頁、買家明細

> 目標：補上沒它很難上線的四項（評價先不做）。設計原則跟以前一樣：不引入重依賴、SQLite/PG 通用、舊庫能開機。

## 1. 圖片上傳（`uploads/` 本機目錄）

- 後端：`POST /api/v1/uploads`（multipart，需登入），存 `./uploads/`，檔名 `{uuid}.{ext}`，只收 jpg/png/webp/gif、單檔 5MB；回 `[{url}]`，url 形如 `/uploads/<file>`，由 `ServeDir` 直接 servir（跟 `/api` 同源，無跨域問題）。
- `items.images` 沿用既有 JSON 陣列欄位，存 url 字串陣列；上架／更新時一起送（前端先上傳圖再送單）。
- 上限：單項目最多 5 張（前端擋，後端也擋）。
- 依賴：`axum` 加 `multipart` feature；不加圖片處理 crate（原圖直存，CSS 縮圖顯示）。
- `.gitignore` 加 `uploads/`；`docker-compose` 如有 web 服務需掛 volume（待確認 compose 內容再補）。

## 2. 站內通知鈴鐺（輪詢版，不做 push）

- 新表 `notifications`：`id, user_id, kind, order_id, actor_nickname, read, created_at`。
- 寫入點（下單流程本來就有 transaction，順手在裡面記）：
  - 下單成功 → 通知店主（`order_created`）
  - confirm/ready/complete/noshow → 通知買家；cancel → 通知對方（誰按的就通知另一邊）
- API：`GET /notifications`（本人倒序 50 筆）、`GET /notifications/unread-count`、`POST /notifications/read`（`{ids?}`，空＝全讀）。
- 前端：nav 鈴鐺＋未讀數（mount 抓一次＋30s 輪詢），`/notifications` 列表頁＋全部已讀鈕；文案走 i18n（kind＋店名／項目名組裝，存參數不存句子）。
- 通知是否已讀不影響訂單邏輯；舊庫建表自動長出來，無遷移問題。

## 3. 店鋪頁（`/shop/:id`，純前端，後端免動）

- 內容：店名、店址、營業時間、店主（暱稱）、該店全部上架中項目（沿用 Home 卡片）。
- 用到全是現成 API：`GET /shops/:id`＋`GET /items?shop_id=`。項目詳情頁的店名改連到店鋪頁。

## 4. 買家訂單明細

- 後端 `GET /orders/:id` 本來就允許買家看（含品名），只改前端：Orders 買家頁籤也抓明細，顯示品項 × 數量。
- 賣家視角不變。

## 驗收

- `./test.sh` 加：上傳爛檔（txt→400、超大→413/400）、上架帶圖＋詳情回圖、下單→賣家未讀+1、確認→買家未讀+1、讀完歸零、未登入讀通知→401、買家看明細 200。
- `cargo test` 加通知寫入點單測（如有純函數可抽）。
- 前端 `npm run build`＋`lint` 乾淨；三語字串補齊（`notif.*`、`shop.*`）。
