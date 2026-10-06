# areashop 規劃總綱 — plan0.x

> 區域商店電子商務系統：一個鄉鎮 / 市鎮級的本地買賣 + 預約平台
> 前端：React.js（Vite + TypeScript）｜ 後端：Rust Axum ｜ 文件位置：`_doc/`
> 本文件為總綱，細部拆解見 `plan0.1.md` ~ `plan0.4.md`。

---

## 1. 產品理念（一句話）

全國性電商解決「買得到」，areashop 解決「附近買得到、約得到、面交得到」。

預設視角永遠是「我的地區」，而非「全站」。使用者先選地區（例如：美濃區、鹿港鎮、池上鄉），看到的商品、店家、服務都以該地區為主。

## 2. 七個典型場景 → 兩種交易模型

| # | 場景 | 模型 | 關鍵欄位 |
|---|------|------|----------|
| 1 | 自己養雞 → 賣雞 | 商品 Product | 庫存、面交地點、保存方式 |
| 2 | 種植農產品 → 賣菜 | 商品 Product | 產季、單位（斤/箱）、面交時間 |
| 4 | 自製水餃 → 賣鄰居 | 商品 Product | 口味規格、冷凍/現做、取貨時段 |
| 5 | 養牛 → 賣牛肉 | 商品 Product | 部位、重量、冷鏈/面交 |
| 3 | 幫人養貓狗 → 寄養 | 服務預約 Service+Appointment | 可寄養時段、體型限制、價格/天 |
| 6 | 理髮店 → 預約理髮 | 服務預約 Service+Appointment | 設計師、時段格（30min）、價格表 |
| 7 | 診所 → 預約看診 | 服務預約 Service+Appointment | 門診時段、號次上限、初複診 |

結論：系統只需要做好兩條主流程：

- **A. 商品流程**：上架 → 下單 → 賣家確認 → 約面交 → 完成 → 評價
- **B. 預約流程**：定義服務 → 開放時段 → 預約 → 店家確認/取消 → 到店履約 → 完成 → 評價

v0.1 先做 A，v0.2 做 B，v0.3 做信任（評價/訊息/搜尋），v0.4 做營運（後台/通知/上線）。

## 3. 使用者角色

- **買家/居民 Buyer**：瀏覽、購買、預約、評價。預設有 `home_area_id`。
- **賣家 Seller（個人）**：養雞戶、農戶、水餃姨。一人一店，手機即可開店。
- **店家 Provider（店鋪）**：理髮店、診所、寄養家庭。可多員工、多服務。
- **地區營運 Admin**：審核店家、處理檢舉、管理地區/分類、看報表。
- **系統維運**：部署、監控、備份（v0.4）。

## 4. 地區模型（核心差異化）

```text
County(縣市) → Township(鄉鎮市區) → Village(村里, optional)
每個 Township 有 center_lat/lng + radius_km（預設 5~10km）
User.home_area_id, Shop.area_id, Product/Service 繼承 Shop.area_id
搜尋預設 scope=home_area，可切換隔壁鄉鎮，不做全站混合排序
```

MVP 不做複雜 GIS，`areas` 表先用三級行政區 + 中心點，查詢用 `area_id = X` 精確過濾；v0.3 再加 `PostGIS < radius` 附近搜尋。

## 5. 系統架構

```text
┌─────────────┐      HTTPS/JSON      ┌──────────────┐
│ React SPA   │ ◄──────────────────► │ Axum API     │
│ Vite + TS   │   /api/v1/* + JWT    │ :8080        │
│ TanStack    │                      │ routers/     │
│ Query       │                      │ services/    │
│ Zustand     │                      └──────┬───────┘
└─────────────┘                             │
                                     ┌──────▼───────┐
                                     │ Postgres 16  │
                                     │ (+PostGIS v0.3)│
                                     │ Redis (sess/  │
                                     │  slot lock)  │
                                     │ S3相容儲存   │
                                     └──────────────┘
```

### 5.1 前端（`frontend/`）

- **基座**：React 18 + Vite 5 + TypeScript strict + React Router 6 + TanStack Query 5 + Zustand + React Hook Form + Zod。
- **UI**：TailwindCSS + shadcn/ui（Button/Input/Dialog/Sheet/Calendar），行動優先 RWD，一個鄉鎮阿姨也看得懂的大按鈕、大字體。
- **地圖（v0.3+）**：Leaflet + OpenStreetMap，只顯示面交點，不做導航。
- **PWA（v0.4）**：vite-plugin-pwa，可安裝、離線看訂單。
- **目錄**：`src/pages/ buyer|seller|provider|admin, components/, features/{product,order,booking,area}/, stores/, lib/api.ts`

關鍵頁面：`/`首頁（地區切換器+分類+附近）→ `/p/:id` 商品詳情 → `/s/:id` 服務詳情+行事曆 → `/shop/:id` 店鋪 → `/checkout` → `/orders` `/appointments` → `/seller/*` → `/admin/*`。

### 5.2 後端（`backend/` — Rust workspace）

- **執行**：Axum 0.7 + Tokio + Tower（CORS/Trace/RateLimit）+ `utoipa` 自動 OpenAPI/Swagger。
- **資料**：SeaORM 1.x（`sqlx-sqlite` + `sqlx-postgres` 雙 feature）+ SQLite（本地測試）/ PostgreSQL 16（擴大規模），同一份 entity 程式碼，改 `DATABASE_URL` 即切換；建表用 SeaORM SchemaManager 自動建表（v0.1），不寫方言 SQL。Redis 7（預約時段鎖、OTP、rate limit，v0.2 起）。
- **安全**：argon2 密碼雜湊、JWT（access 2h + refresh 30d）、validator 輸入校驗、審計欄位 `created_at/updated_at`。
- **結構**：`crates/{api,core,db,auth,common}`，router 按領域切：`auth, areas, shops, products, orders, services, appointments, uploads, reviews, messages, admin`。
- **上傳**：本地 `uploads/`（v0.1）→ S3 相容（MinIO/R2，v0.3），圖片經 `image` crate 壓縮 + 限制 5MB。

### 5.3 Monorepo 目錄（目標態）

```text
areashop/
  frontend/          # React SPA
  backend/           # Rust workspace
    migrations/     # sqlx migrate
    crates/api|core|db|auth|common
  _doc/              # plan0.x.md, plan0.1~0.4.md
  docker-compose.yml # pg + redis + api + web
  .github/workflows/ci.yml
```

### 5.4 REST API 概覽（v1）

```text
POST /api/v1/auth/register|login|refresh  GET /me
GET  /api/v1/areas/counties|townships?village=
CRUD /api/v1/shops (POST 需審核, v0.1 先自動通過+admin可下架)
CRUD /api/v1/products  GET /api/v1/products?area_id=&q=&category=
POST /api/v1/orders  PATCH /orders/:id/confirm|cancel|complete  (狀態機)
CRUD /api/v1/services  GET /services?area_id=&type=pet|hair|clinic
POST /api/v1/appointments  PATCH /appointments/:id/confirm|cancel|complete
POST /api/v1/uploads (v0.3 S3)  POST /reviews  WS或輪詢 /messages (v0.3)
GET  /api/v1/admin/* (v0.4)
```

訂單狀態機：`pending → confirmed → ready_for_pickup → completed | cancelled`。
預約狀態機：`pending → confirmed → completed | cancelled(no-show記號)`，同一 slot 用 Redis 分散式鎖 + DB unique constraint 防超賣。

## 6. 資料模型（精簡版，詳見各 plan）

`users(id, phone unique, password_hash, nickname, home_area_id, role)`
`areas(id, county, township, village nullable, center_lat/lng, radius_km, parent_id)`
`shops(id, owner_id, area_id, name, type[personal|store], status, address_text, lat/lng)`
`products(id, shop_id, title, category, price_cents, stock, unit, pickup_places JSONB, status)`
`orders(id, buyer_id, shop_id, status, pickup_at, pickup_place, total_cents, remark)`
`order_items(order_id, product_id, qty, price_cents)`
`services(id, shop_id, kind[pet|hair|clinic|other], title, duration_min, price_cents, buffer_min)`
`slots/appointments(id, service_id, buyer_id, start_at, end_at, status)` + unique(service_id, start_at)
`reviews(id, order_id|appointment_id, from_id, to_shop_id, stars, text)`
`conversations/messages` (v0.3), `favorites`, `audit_logs` (v0.4)

金額一律 `price_cents` 整數；時間一律 UTC，顯示轉 `Asia/Taipei`。

## 7. 版本路線圖（總覽）

| 版本 | 主題 | 核心交付 | 對應場景 |
|------|------|----------|----------|
| **v0.1** | 本地市集 MVP | 地區+ Auth + 開店 + 商品CRUD + 下單面交 + 買家訂單頁 | 1,2,4,5 |
| **v0.2** | 服務預約 | 服務定義 + 時段模板 + 預約/確認/取消 + 店家行事曆 | 3,6,7 |
| **v0.3** | 信任與找到 | 評價 + 站內訊息 + 全文/附近搜尋 + S3圖片 + LINE登入 | 全部強化回購 |
| **v0.4** | 營運與上線 | Admin後台 + 推播/簡訊通知 + PWA + 監控備份 + 壓測 | 正式試營運 |

- 詳情：`plan0.1.md` / `plan0.2.md` / `plan0.3.md` / `plan0.4.md`
- 原則：每版都可獨立部署演示；不做金流（面交現金/轉帳，v1.0 再評估綠界/LINE Pay）；不做全國物流。
- 時程參考（1 全端 + AI 協作）：v0.1 約 3–4 週，v0.2 約 2–3 週，v0.3 約 2–3 週，v0.4 約 2 週。

## 8. 非功能需求（貫穿各版）

- **效能**：列表 API p95 < 300ms（分頁 20筆，有 `area_id+status` 複合索引）；前端首屏 Lighthouse mobile > 80。
- **安全**：OTP/登入 rate limit、XSS（React 預設跳脫 + DOMPurify for 富文本）、上傳白名單、JWT refresh rotation。
- **可觀測**：tracing 日誌 + `/healthz`，v0.4 接 OpenTelemetry + Sentry + pg backup。
- **測試門檻**：後端 domain 單元測試 + API 整合測試（sqlx testcontainers），前端關鍵流程 Playwright（下單/預約各一條）。

## 9. 風險與對策

1. **診所涉及醫療法規** → v0.2 診所只做「門診時段查詢+預約登記」，不寫病歷、不做線上診斷，文案加免責聲明。
2. **食品安全（水餃/牛肉）** → 商品頁強制標示「保存方式/有效期限/自製聲明」，檢舉下架流程 v0.3 上線。
3. **預約超賣/爽約** → Redis 鎖 + 取消政策（開診前 N 小時不可取消）+ no-show 計數。
4. **冷啟動沒供給** → v0.1 Admin 可匯入種子店家 + 單一示範鄉鎮先行（選一個真實鄉鎮做 UAT）。

## 10. 下一步（執行順序）

1. 照 `plan0.1.md` 起 monorepo + docker-compose + migrate(areas/users/shops/products/orders) + Auth。
2. 每版結束做一次「鄉鎮演示」：用 7 個場景的假資料走一次買賣/預約。
3. v0.4 結束後再決定 v1.0：金流、即時聊天 WebSocket、PostGIS 半徑、多鄉鎮營運。

---

*變更紀錄：2026-10-06 初版總綱建立。*
