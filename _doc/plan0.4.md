# 搜尋過濾

1. - [x] 搜尋：關鍵字 + 分類 + 地區 （含中文分詞簡易版）
    - 後端 `src/search.rs`：正規化（小寫＋全形轉半形＋去標點空白）→ ASCII 切詞＋CJK 逐字 token → 標題＋描述 AND 匹配；分類 `fresh|food|daily|service|other`（`items.category`，舊庫 ALTER 補欄預設 other）
    - 前端 Home 分類下拉＋`?cat=` 進 URL；Seller 上架選分類