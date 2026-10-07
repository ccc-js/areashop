# plan0.4 — 營運與上線（Admin 後台 / 通知 / PWA / 硬化）

> 目標：可交給真實鄉鎮試營運。依賴：v0.1–v0.3 ｜ 工期參考：2 週 + 1 週試營運

## 1. 本版目標與驗收

- [ ] Admin 可審核店家、處理檢舉、開關商品、管理地區/分類、看報表
- [ ] 訂單/預約關鍵節點推播 + 簡訊（至少站內+Email，簡訊接三竹/薦 LINE 推播擇一）
- [ ] PWA 可安裝，弱網可看訂單；前端錯誤上報
- [ ] 上線硬化：HTTPS、備份、監控、壓測、RateLimit、稽核日誌
- [ ] 試營運：在 1 個真實鄉鎮跑 2 週，收集 20+ 回饋並修 top 10 bug

**不做**：金流（v1.0 專題）、多語系（先繁中）、WebSocket 即時聊天（v1.0）。

## 2. 前端交付

- `/admin` 後台（Admin role 才可進）：儀表板（今日訂單/預約/GMV 面交額）、`shops待審核`、`reports檢舉`、`products下架`、`areas管理`、`users查詢`。表格用 shadcn Table + 分頁 + 搜尋。
- 通知中心：右上 bell（未讀數輪詢 `/notifications/unread-count`），訂單/預約/評價/檢舉結果都進來。
- PWA：`vite-plugin-pwa`（manifest 中文名 areashop、icons、offline 頁、訂單詳情 stale-while-revalidate）。
- 可觀測：Sentry FE（env 開關）、全域 ErrorBoundary、空/錯/載入三態統一。

## 3. 後端交付

- `admin` routers（`RequireAdmin` extractor）：`GET /admin/overview`、`GET|PATCH /admin/shops?status=pending`、`GET|PATCH /admin/reports`、`PATCH /admin/products/:id/status`、`CRUD /admin/areas`、`GET /admin/users?q=`、`GET /admin/orders?`。
- `audit_logs`(id, actor_id, action, target, meta JSONB, created_at)：所有 admin 寫操作記一筆。
- 通知發送器 `notify(user_id, kind, title, link)`：寫 `notifications` 表 + 可選通道：
  - Email（SMTP/Resend，二選一，先做 Email 最便宜）
  - LINE Notify / 推播（擇一試點，env 開關）
  - 簡訊 OTP 預留介面（`SmsSender` trait + Mock 實作，正式再接三竹）
- 排程（tokio-cron 或獨立 worker）：預約前 2h 提醒、待確認訂單 24h 未處理提醒店家、no-show 統計。
- 硬化：CORS 白名單、helmet headers、request-id、 structured tracing 日誌、` /healthz|/readyz`、pg 每日備份（pg_dump + cron + 異地 S3）、`.env.example` 完整化。

## 4. 部署與測試

- `docker-compose.yml`：`web(frontend dist由nginx serve)+api+db+redis+minio`；另給 `compose.prod.yml`（api 多副本 2 + restart always）。
- CI（GitHub Actions）：`fmt → clippy -D warnings → cargo test → docker build → playwright smoke`。
- 壓測門檻：k6 `GET /products?area_id=` 100 RPS p95<300ms；預約併發見 v0.2。
- 備份演練：做一次 restore 到空 DB 確認 seed 可重建。
- 試營運清單：示範鄉鎮種子資料、店家操作手冊（1 頁 PDF）、居民傳單 QR（連 PWA）、回饋表單。

## 5. v1.0 展望（本版不做，先記錄）

金流（綠界/藍新/LINE Pay 面交訂金）、WebSocket 即時聊天、PostGIS 半徑多鄉鎮、Meilisearch、店家 POS 小票列印、多語（客語/台語語音？）、評價圖片審核。
