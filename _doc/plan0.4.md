# 搜尋過濾

1. - [x] 搜尋：關鍵字 + 分類 + 地區 （含中文分詞簡易版）
    - 後端 `src/search.rs`：正規化（小寫＋全形轉半形＋去標點空白）→ ASCII 切詞＋CJK 逐字 token → 標題＋描述 AND 匹配；分類 `fresh|food|daily|service|other`（`items.category`，舊庫 ALTER 補欄預設 other）
    - 前端 Home 分類下拉＋`?cat=` 進 URL；Seller 上架選分類

# 管理員介面

2. - [x] 地區設定（CRUD）
    - `POST /admin/areas`、`PUT /admin/areas/:id`、`DELETE /admin/areas/:id`（皆 admin-only；重複 409、使用中 400、缺欄 422）
    - 村里不同算不同筆；縣市＋鄉鎮＋村里（NULL 用 IS NULL 比，不可用 `= NULL`）
3. - [x] 店家管理（一覽＋停權/恢復）
    - `GET /admin/shops`（全店＋店主暱稱/手機）；停權沿用 `PATCH /shops/:id {status}`（admin 已放行）
4. - [x] 項目管理（一覽＋下架/恢復）
    - `GET /admin/items`（全部＋店名，含已下架）；沿用 `PATCH /items/:id {status}`
5. - [x] 使用者一覽（`GET /admin/users`，不含密碼雜湊）
6. - [x] 訂單一覽（`GET /admin/orders`，最新 100 筆＋買家/店名）
    - 前端 `/admin` 五頁籤＋nav 只對 admin 顯示＋中英簡三語；test.sh 有 403/401/404/409/422/400 全覆蓋
