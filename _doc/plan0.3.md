# plan0.3 — 信任與找到（評價 / 訊息 / 搜尋 / 圖片 / 登入）

> 目標：讓人「找得到、敢買、敢約」。依賴：v0.1 + v0.2 ｜ 工期參考：2–3 週

## 1. 本版目標與驗收

- [ ] 訂單/預約完成後可互評（買家評店家為主），商品/服務頁顯示星等
- [ ] 買賣雙方可站內私訊約面交（先輪詢，WebSocket v1.0）
- [ ] 搜尋：關鍵字 + 分類 + 地區 + 附近半徑（含中文分詞簡易版）
- [ ] 圖片上 S3（R2/MinIO），列表縮圖加速
- [ ] LINE Login 可一鍵登入（台灣居民主流）
- [ ] 檢舉下架流程上線（食品安全/詐騙第一道防線）

## 2. 前端交付

- `ReviewStars`、`ReviewList`（店鋪/商品/服務共用）、訂單完成頁引導評價。
- `ChatBox`（訂單/預約詳情內嵌「聯絡對方」，30s 輪詢 + 已讀標記簡易版）。
- `SearchBar`（聯想詞 + 歷史紀錄 localStorage）+ `NearbyToggle`（本鄉鎮 / 含隔壁 / 5km 內，需授權定位）。
- 圖片：上傳前 client 壓縮（<1920px, webp），列表用 `?w=400` 縮圖 URL。
- LINE 登入按鈕（`/login` 新增綠色按鈕，走 OAuth code → 後端換 token → 綁定 phone）。

## 3. 後端交付

- migrations：`reviews`(id, order_id/appointment_id 二選一, from_id, shop_id, target_type[product|service|shop], stars 1-5, text, images, created_at; 同一訂單限一評 unique)、`conversations`(id, order_id|appointment_id unique, buyer_id, shop_id)、`messages`(id, conversation_id FK, sender_id, body, is_read, created_at; index conversation_id,id)、`reports`(id, reporter_id, target_type, target_id, reason, status[open|accepted|rejected], created_at)、`users` 加 `line_sub, avatar_url, no_show_count`。
- 搜尋：Postgres `pg_trgm + tsvector`（title+desc 中文先用 trigram 相似度，v1.0 再評估 Meilisearch）；附近：啟用 PostGIS `shops.geom`，`GET /products/nearby?lat=&lng=&km=5` 用 `ST_DWithin`。
- 上傳：`POST /uploads` 轉 S3（presigned PUT，前端直傳，後端只存 key），保留本地 fallback（env `STORAGE=local|s3`）。
- LINE Login：`GET /auth/line/url` → `GET /auth/line/callback?code=` 換 token、取 profile、upsert user（line_sub 唯一），若無 phone 則導向補填頁。
- 檢舉：`POST /reports` → admin 待審（v0.4 後台前先用 `GET /admin/reports` JSON API + SQL），接受後自動 `products.status=off / shops.status=banned`。
- 評分聚合：`shops.rating_avg, rating_count` 用 trigger 或 service 層更新，防刷（僅 completed 訂單可評）。

## 4. API 契約（新增）

```text
POST /reviews {order_id|appointment_id, stars, text}  GET /shops/:id/reviews
POST /conversations {order_id|appointment_id}  GET /conversations/mine
GET  /conversations/:id/messages?after_id=  POST /conversations/:id/messages {body}
GET  /products/search?q=&area_id=&category=&near_lat=&near_lng=&km=
POST /uploads/request {ext} → {put_url, key}   POST /reports {target_type,target_id,reason}
GET  /auth/line/url  GET /auth/line/callback?code=
```

訊息輪詢 15–30s 一次；body 上限 2000 字，敏感詞先做簡易黑名單（v1.0 再做審核）。

## 5. 測試與 DoD

- 未完成訂單不可評價（403）；同一訂單重複評價（409）。
- 陌生人不可讀別人的 conversation（403 測試）。
- 搜尋：「水餃」「放牧雞」「剪髮」在示範資料中召回正確。
- S3 直傳 e2e（MinIO local 測）；LINE callback 用 mock server 測。
- Lighthouse mobile 首頁 >80（縮圖+分頁後）。
