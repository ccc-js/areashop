# plan0.1 — 本地市集 MVP（商品 + 面交訂單）

> 目標：一個鄉鎮的人可以開店、上架雞/菜/水餃/牛肉，並完成面交訂單。
> 覆蓋場景：1 養雞、2 農產品、4 水餃、5 牛肉 ｜ 工期參考：3–4 週

## 1. 本版目標與驗收

- [ ] 使用者可選地區（縣市→鄉鎮），首頁只看該地區商品
- [ ] 手機號+密碼註冊登入，記住我的地區
- [ ] 個人 10 分鐘開店上架（含圖片、庫存、面交地點/時段）
- [ ] 買家下單 → 賣家確認 → 約面交 → 完成，全程狀態可追蹤
- [ ] Demo：單一示範鄉鎮 20 件商品、5 家店，全流程無 500 錯誤

**不做**：預約（v0.2）、評價/聊天（v0.3）、金流（面交現金）、Admin 後台（用 SQL + 簡易下架 API 代替）。

## 2. 前端交付（React）

- 路由：`/`首頁、` /p/:id` 商品詳情、` /shop/:id` 店鋪頁、` /checkout`、` /orders`、` /orders/:id`、` /seller/products|orders|shop`、` /me`、` /login|register`。
- 元件：`AreaPicker`（縣市→鄉鎮兩層 select，存 localStorage + user.home_area_id）、`ProductCard`、`StockStepper`、`OrderStatusBadge`、`ImageUploader`（<5MB，client 壓縮）。
- 狀態：TanStack Query 快取 `["products", areaId]`；Zustand `useAreaStore` 存 currentArea。
- 表單：React Hook Form + Zod（價格>0、庫存>=0、面交地必填）。
- 空狀態/錯誤：無商品插圖 + 「去隔壁鄉鎮看看」按鈕；API 錯誤 toast。

## 3. 後端交付（Axum）

- routers：`auth`、`areas`、`shops`、`products`、`orders`、`healthz`。全部 `/api/v1/*`。
- 資料庫彈性：SeaORM 同一份 entity 跑 SQLite（`sqlite://./areashop.db?mode=rwc`，本地測試預設）與 PostgreSQL（擴大規模時改 `DATABASE_URL` 即可，已實測兩邊建表+下單皆通）；建表見 `backend/src/schema.rs`（SchemaManager `if_not_exists`），種子見 `backend/src/seed.rs`。
- migrations（順序）：
  1. `areas`（id, county, township, village, center_lat/lng, radius_km）
  2. `users`（id uuid, phone unique, password_hash argon2, nickname, home_area_id FK, role[user|admin], created_at）
  3. `shops`（id, owner_id FK users, area_id FK, name, type[personal|store], desc, address_text, phone, status[open|closed|banned], created_at）
  4. `products`（id, shop_id FK, title, category[agri|poultry|beef|dumpling|other], price_cents int, stock int, unit, pickup_places JSONB[{label,detail}], pickup_note, images text[], status[on|off|soldout], created_at）+ index `(shop_id,status)`、`(status, id)`
  5. `orders` + `order_items`（見總綱狀態機 `pending→confirmed→ready→completed|cancelled`，只能本人/店主操作對應轉換，寫 `check_transition()` 單元測試）
- Auth：JWT access(2h)+refresh(30d, httpOnly cookie)，`AuthUser` extractor，refresh rotation，login rate limit 10/min/IP（tower-governor 或 redis）。
- 圖片：先存 `backend/uploads/` + 靜態掛載 `/static`，後綴白名單 jpg/png/webp，後端再壓一次（image crate max 1600px）。
- 种子：`seed.sql` 灌一個示範鄉鎮 + 7 場景各 2 商品，共 ~20 筆。

## 4. API 契約（精簡）

```text
POST /auth/register {phone,password,nickname,home_area_id} → {access_token,user}
POST /auth/login {phone,password} → {access_token,user}
GET  /areas/counties → ["屏東縣",...]   GET /areas/townships?county=屏東縣
GET  /areas/search?q=美濃
POST /shops {name,area_id,type,address_text,phone} → shop (一人限 1 店，MVP自動 open)
GET  /shops/:id  PATCH /shops/:id
POST /products {shop_id,title,category,price_cents,stock,unit,pickup_places[],images[]} 
GET  /products?area_id=&q=&category=&page=&page_size=20
GET  /products/:id  PATCH /products/:id  DELETE /products/:id(軟刪 off)
POST /orders {items:[{product_id,qty}], pickup_place, pickup_at, remark} → 扣庫存(事務, SELECT FOR UPDATE)
PATCH /orders/:id {action: confirm|ready|complete|cancel} (權限: confirm/ready=店主, complete=買家確認或店主完成, cancel=雙方在 pending/confirmed 可取消)
GET  /orders?role=buyer|seller  GET /orders/:id
```

扣庫存規則：下單即預扣（stock -= qty，check >=0），取消則回補，全包在同一個 DB transaction。

## 5. 測試與 DoD

- 後端 `cargo test`：狀態機非法轉換拒絕、下單超賣拒絕、未授權操作 403。
- 整合測試：register→開店→上架→下單→確認→完成 全鏈路（SeaORM + SQLite 測試 DB，PG 用同一套程式 nightly 跑一次）。
- 前端 Playwright 1 條：登入→切地區→下單→訂單頁看到 pending。
- `docker compose up` 一鍵起 pg+redis+api+web；README 有步驟；CI 跑 fmt+clippy+test+build。

## 6. 風險

- 面交時間喬不攏 → `pickup_at` 讓買家填 3 個志願時段（text），賣家確認時選定一個。
- 農產品單位混亂 → `unit` 用枚舉（斤/公斤/顆/箱/包/隻/台斤）+ 規格欄。
