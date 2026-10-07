# areashop — 區域商店買賣系統

一個鄉鎮／市鎮級的本地買賣 + 預約平台。預設視角是「我的地區」，不是全站。
規劃文件在 `_doc/`（`plan0.x.md` 總綱，`plan0.1.md`～`plan0.4.md` 分版）。

- 前端：React 18 + Vite + TypeScript + React Router（`frontend/`）
- 後端：Rust Axum + SeaORM（`backend/`）
- 資料庫：**SQLite（本地測試）／PostgreSQL（擴大規模），同一份程式碼，改 `DATABASE_URL` 即切換**

## 快速開始（SQLite，零依賴）

```bash
cp .env.example .env
cd backend && cargo run        # :8080，自動建表 + 種子資料
cd ../frontend && npm install && npm run dev   # :5173，/api 代理到 :8080
```

開瀏覽器 `http://localhost:5173`，用種子帳號登入：

| 角色 | 手機 | 密碼 |
|------|------|------|
| 管理員 | 0900000001 | password123 |
| 賣家 | 0900000002 | password123 |
| 買家 | 0900000003 | password123 |

## 切換 PostgreSQL（擴大規模）

```bash
docker compose up -d db          # 或連現有 PG
DATABASE_URL=postgres://areashop:areashop@localhost:5432/areashop cargo run
```

同一支 binary，不需改程式、不需改 SQL（SeaORM 依連線自動產生對應方言）。
完整三件套（含前端 nginx）：`docker compose up --build` → web `:3000`、api `:8080`。

## API（統一模式：賣東西＋賣服務同一種）

| 方法 | 路徑 | 說明 |
|------|------|------|
| POST | /api/v1/auth/register, /login | 註冊／登入（手機+密碼） |
| GET | /api/v1/auth/me | 本人（Bearer JWT） |
| GET | /api/v1/areas, /counties, /townships?county=, /search?q= | 地區 |
| POST/GET | /api/v1/shops | 開店／我的店 |
| GET/PATCH | /api/v1/shops/:id | 店鋪 |
| POST/GET | /api/v1/items?area_id=&q=&shop_id=&bookable=&category= | 上架／列表（`stock` 空=不限量，`bookable`=可選日期；`q` 中文分詞 AND 配對標題＋描述，`category` 限 fresh/food/daily/service/other） |
| GET/PATCH | /api/v1/items/:id | 項目（`?month=` 附可接案月曆） |
| PUT/POST/DELETE | /api/v1/items/:id/rules, /exceptions[/:date] | 週範本／單日開關（額滿・不營業） |
| POST/GET | /api/v1/orders | 下單（同店、有庫存扣庫存；`date` 有值=預約）／訂單列表 |
| GET/PATCH | /api/v1/orders/:id | 詳情（含買家＋品名）／confirm→ready→complete／cancel／noshow |
| GET | /api/v1/provider/calendar?shop_id=&month= | 店家月曆（有日期的單） |

訂單狀態機：`pending → confirmed → ready → completed ｜ cancelled ｜ noshow`（confirmed 可跳過 ready；noshow 限店主），見
`backend/src/entities/order.rs`（含單元測試）。

⚠️ 統一模式是 breaking change：舊 `products`/`services`/`appointments` 表不再使用，現有開發 DB 請 `./run.sh --reseed`。

## 目錄

```text
areashop/
  frontend/   # React SPA（Home/商品/訂單/賣家中心）
  backend/    # Axum API（src/routes/*, entities/*, schema.rs 自動建表, seed.rs）
  _doc/       # 規劃文件
  docker-compose.yml
```
