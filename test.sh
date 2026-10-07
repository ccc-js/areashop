#!/usr/bin/env bash
# areashop v0.1 一鍵測試
#
#   ./test.sh            # SQLite：cargo test + 建置 + API 端到端全流程
#   ./test.sh --pg       # 外加 PostgreSQL（需 docker）：同一套 API 在 PG 上重跑一次
#   ./test.sh --no-build # 跳過 cargo build（已編過時加速）
#
# 全部通過顯示 ALL TESTS PASSED；任一失敗即停並顯示是哪一步。

set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
BACKEND="$ROOT/backend"
BIN="$BACKEND/target/debug/areashop-api"
PORT=18090
BASE="http://localhost:$PORT/api/v1"
TMP="$(mktemp -d /tmp/areashop_test.XXXXXX)"
PASS=0
FAIL=0

BUILD=1
WITH_PG=0
for arg in "$@"; do
  case "$arg" in
    --no-build) BUILD=0 ;;
    --pg) WITH_PG=1 ;;
    *) echo "unknown arg: $arg"; exit 2 ;;
  esac
done

cleanup() {
  [ -n "${SRV_PID:-}" ] && { kill "$SRV_PID" 2>/dev/null || true; wait "$SRV_PID" 2>/dev/null || true; }
  pkill -f "target/debug/areashop-api" 2>/dev/null || true
  [ -n "${PG_CONTAINER:-}" ] && docker rm -f "$PG_CONTAINER" >/dev/null 2>&1 || true
  rm -rf "$TMP"
}
trap cleanup EXIT

ok()   { PASS=$((PASS+1)); echo "  PASS $1"; }
bad()  { FAIL=$((FAIL+1)); echo "  FAIL $1 (HTTP $2) $(cat "$TMP/body.json" 2>/dev/null | head -c 200)"; exit 1; }

# jget <file> <python expr>：從 JSON 取值（d 為解析結果）
jget() { python3 -c "import json,sys; d=json.load(open('$1')); print($2)"; }

# fdate <N>：N 天後的 YYYY-MM-DD（macOS BSD date / GNU date 通吃）
fdate() {
  if date -v+1d +%F >/dev/null 2>&1; then date -v+"$1"d +%F; else date -d "+$1 days" +%F; fi
}

# call <METHOD> <PATH> [TOKEN] [DATA] → 回傳 HTTP code，body 存 $TMP/body.json
call() {
  local method="$1" path="$2" token="${3:-}" data="${4:-}"
  local args=(-s -o "$TMP/body.json" -w "%{http_code}" -X "$method")
  [ -n "$token" ] && args+=(-H "Authorization: Bearer $token")
  if [ -n "$data" ]; then
    args+=(-H "Content-Type: application/json" -d "$data")
  fi
  curl "${args[@]}" "$BASE$path"
}

expect() { # expect <label> <want_code> <METHOD> <PATH> [TOKEN] [DATA]
  local label="$1" want="$2"; shift 2
  local code
  code=$(call "$@")
  if [ "$code" = "$want" ]; then ok "$label [$code]"; else bad "$label (want $want)" "$code"; fi
}

wait_for() { # 等後端 ready
  for _ in $(seq 1 30); do
    curl -s -o /dev/null "$BASE/healthz" && return 0
    sleep 1
  done
  echo "backend did not start. log:"; tail -20 "$TMP/server.log"; exit 1
}

start_server() { # start_server <DATABASE_URL>
  pkill -f "target/debug/areashop-api" 2>/dev/null || true
  # 等舊行程真的釋放 port，避免測到殭屍伺服器
  for _ in $(seq 1 15); do
    curl -s -o /dev/null --max-time 1 "http://localhost:$PORT/api/v1/healthz" 2>/dev/null && sleep 1 || break
  done
  sleep 1
  rm -f "$TMP/server.log"
  DATABASE_URL="$1" PORT="$PORT" SEED=1 UPLOAD_DIR="$TMP/up" nohup "$BIN" >"$TMP/server.log" 2>&1 &
  SRV_PID=$!
  wait_for
}

suite() { # suite <標籤>：完整 API 流程，假設是全新 DB（含種子資料）
  local tag="$1"
  echo "== $tag =="

  expect "$tag healthz" 200 GET /healthz
  expect "$tag 種子地區" 200 GET /areas
  [ "$(jget "$TMP/body.json" "len(d)")" -ge 1 ] && ok "$tag 地區筆數>=1" || bad "$tag 地區為空" "-"
  AREA="$(jget "$TMP/body.json" "d[0]['id']")"

  # --- Auth ---
  local phone="09$(date +%s | tail -c 9)"
  expect "$tag 註冊" 200 POST /auth/register "" \
    "{\"phone\":\"$phone\",\"password\":\"password123\",\"nickname\":\"測試\",\"home_area_id\":$AREA}"
  TOKEN_NEW="$(jget "$TMP/body.json" "d['access_token']")"
  expect "$tag 重複註冊→409" 409 POST /auth/register "" \
    "{\"phone\":\"$phone\",\"password\":\"password123\",\"nickname\":\"重複\",\"home_area_id\":$AREA}"
  expect "$tag 錯誤密碼→401" 401 POST /auth/login "" \
    "{\"phone\":\"$phone\",\"password\":\"wrong\"}"
  expect "$tag 登入" 200 POST /auth/login "" \
    "{\"phone\":\"$phone\",\"password\":\"password123\"}"
  expect "$tag me" 200 GET /auth/me "$TOKEN_NEW"
  expect "$tag 未帶 token→401" 401 GET /auth/me

  # 種子帳號
  TOKEN_B="$(call POST /auth/login "" '{"phone":"0900000003","password":"password123"}' >/dev/null; jget "$TMP/body.json" "d['access_token']")"
  # 上面 call 會覆寫 body；重新登入取 token（順便驗登入 200）
  expect "$tag 種子買家登入" 200 POST /auth/login "" '{"phone":"0900000003","password":"password123"}'
  TOKEN_B="$(jget "$TMP/body.json" "d['access_token']")"
  expect "$tag 種子賣家登入" 200 POST /auth/login "" '{"phone":"0900000002","password":"password123"}'
  TOKEN_S="$(jget "$TMP/body.json" "d['access_token']")"
  expect "$tag 種子管理員登入" 200 POST /auth/login "" '{"phone":"0900000001","password":"password123"}'
  TOKEN_A="$(jget "$TMP/body.json" "d['access_token']")"

  # --- 商店 ---
  expect "$tag 開店" 200 POST /shops "$TOKEN_NEW" \
    "{\"name\":\"測試店\",\"area_id\":$AREA,\"kind\":\"personal\"}"
  SHOP="$(jget "$TMP/body.json" "d['id']")"
  expect "$tag 我的店" 200 GET /shops "$TOKEN_NEW"
  expect "$tag 開店面店" 200 POST /shops "$TOKEN_NEW" \
    "{\"name\":\"測試店面\",\"area_id\":$AREA,\"kind\":\"store\",\"address\":\"金城鎮模範街1號\",\"opening_hours\":\"每日 09:00-18:00\"}"
  [ "$(jget "$TMP/body.json" "d['address']")" = "金城鎮模範街1號" ] && ok "$tag 店面有地址" || bad "$tag 店面地址" "-"
  expect "$tag 店面無地址→400" 400 POST /shops "$TOKEN_NEW" \
    "{\"name\":\"無址店\",\"area_id\":$AREA,\"kind\":\"store\"}"
  expect "$tag 種類亂填→400" 400 POST /shops "$TOKEN_NEW" \
    "{\"name\":\"亂填店\",\"area_id\":$AREA,\"kind\":\"mall\"}"

  # --- 上架（統一路徑：賣東西 / 賣服務 / 混合同一種） ---
  # 最小合法 png（1x1）＋假圖＋超大檔
  python3 -c "import base64; open('$TMP/a.png','wb').write(base64.b64decode('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=='))"
  echo "not an image" > "$TMP/a.txt"
  head -c 6291456 /dev/zero > "$TMP/big.png"
  code=$(curl -s -o "$TMP/body.json" -w "%{http_code}" -X POST -F "files=@$TMP/a.png" "$BASE/uploads"); [ "$code" = "401" ] && ok "$tag 未登入上傳→401 [$code]" || bad "$tag 未登入上傳" "$code"
  code=$(curl -s -o "$TMP/body.json" -w "%{http_code}" -X POST -H "Authorization: Bearer $TOKEN_NEW" -F "files=@$TMP/a.txt" "$BASE/uploads"); [ "$code" = "400" ] && ok "$tag 假圖→400 [$code]" || bad "$tag 假圖" "$code"
  code=$(curl -s -o "$TMP/body.json" -w "%{http_code}" -X POST -H "Authorization: Bearer $TOKEN_NEW" -F "files=@$TMP/big.png" "$BASE/uploads"); [ "$code" = "400" ] && ok "$tag 超大檔→400 [$code]" || bad "$tag 超大檔" "$code"
  code=$(curl -s -o "$TMP/body.json" -w "%{http_code}" -X POST -H "Authorization: Bearer $TOKEN_NEW" -F "files=@$TMP/a.png" "$BASE/uploads"); [ "$code" = "200" ] && ok "$tag 上傳png [$code]" || bad "$tag 上傳" "$code"
  IMGURL="$(jget "$TMP/body.json" "d[0]['url']")"
  [ -n "$IMGURL" ] && ok "$tag 有url" || bad "$tag url空" "-"
  code=$(curl -s -o "$TMP/img.bin" -w "%{http_code}" "http://localhost:$PORT$IMGURL"); [ "$code" = "200" ] && ok "$tag 圖可讀 [$code]" || bad "$tag 讀圖" "$code"
  expect "$tag 上架商品型" 200 POST /items "$TOKEN_NEW" \
    "{\"shop_id\":$SHOP,\"title\":\"測試水餃\",\"description\":\"現包冷凍\",\"price_cents\":16000,\"stock\":5,\"unit\":\"包\",\"category\":\"food\"}"
  PID="$(jget "$TMP/body.json" "d['id']")"
  expect "$tag 上架服務型" 200 POST /items "$TOKEN_NEW" \
    "{\"shop_id\":$SHOP,\"title\":\"測試快剪\",\"price_cents\":30000,\"stock\":null,\"bookable\":true,\"category\":\"service\"}"
  SID="$(jget "$TMP/body.json" "d['id']")"
  expect "$tag 上架混合型" 200 POST /items "$TOKEN_NEW" \
    "{\"shop_id\":$SHOP,\"title\":\"預訂水餃\",\"price_cents\":16000,\"stock\":3,\"unit\":\"包\",\"bookable\":true,\"category\":\"food\"}"
  MID="$(jget "$TMP/body.json" "d['id']")"
  expect "$tag 上架帶圖" 200 POST /items "$TOKEN_NEW" \
    "{\"shop_id\":$SHOP,\"title\":\"有圖水餃\",\"price_cents\":100,\"stock\":1,\"category\":\"daily\",\"images\":[\"$IMGURL\"]}"
  IMGID="$(jget "$TMP/body.json" "d['id']")"
  expect "$tag 詳情回圖" 200 GET "/items/$IMGID"
  [ "$(jget "$TMP/body.json" "d['images'][0]")" = "$IMGURL" ] && ok "$tag 圖url對" || bad "$tag 圖" "-"
  expect "$tag 上架爛圖→400" 400 POST /items "$TOKEN_NEW" \
    "{\"shop_id\":$SHOP,\"title\":\"爛圖\",\"price_cents\":100,\"stock\":1,\"images\":\"x\"}"
  expect "$tag 上架6張圖→400" 400 POST /items "$TOKEN_NEW" \
    "{\"shop_id\":$SHOP,\"title\":\"爛圖\",\"price_cents\":100,\"stock\":1,\"images\":[\"$IMGURL\",\"$IMGURL\",\"$IMGURL\",\"$IMGURL\",\"$IMGURL\",\"$IMGURL\"]}"
  expect "$tag 他人商店上架→403" 403 POST /items "$TOKEN_B" \
    "{\"shop_id\":$SHOP,\"title\":\"偷渡\",\"price_cents\":100,\"stock\":1}"
  expect "$tag 無庫存又不可約→400" 400 POST /items "$TOKEN_NEW" \
    "{\"shop_id\":$SHOP,\"title\":\"爛\",\"price_cents\":100}"
  expect "$tag 爛分類→400" 400 POST /items "$TOKEN_NEW" \
    "{\"shop_id\":$SHOP,\"title\":\"爛\",\"price_cents\":100,\"stock\":1,\"category\":\"nope\"}"
  expect "$tag 地區列表" 200 GET "/items?area_id=$AREA"
  [ "$(jget "$TMP/body.json" "d['total']")" -ge 3 ] && ok "$tag 列表有3項" || bad "$tag 列表" "-"
  expect "$tag 不分區全縣" 200 GET "/items?county=金門縣"
  [ "$(jget "$TMP/body.json" "d['total']")" -ge 1 ] && ok "$tag 全縣有項目" || bad "$tag 全縣為空" "-"
  expect "$tag 不存在的縣→空" 200 GET "/items?county=火星市"
  [ "$(jget "$TMP/body.json" "d['total']")" = "0" ] && ok "$tag 不存在的縣回空" || bad "$tag 不存在的縣應回空" "-"
  expect "$tag 只看可預約" 200 GET "/items?shop_id=$SHOP&bookable=true"
  [ "$(jget "$TMP/body.json" "d['total']")" = "2" ] && ok "$tag 2個可預約" || bad "$tag bookable篩選" "-"
  expect "$tag 分類過濾food" 200 GET "/items?shop_id=$SHOP&category=food"
  [ "$(jget "$TMP/body.json" "d['total']")" = "2" ] && ok "$tag food有2個" || bad "$tag 分類篩選" "-"
  expect "$tag 分類過濾service" 200 GET "/items?shop_id=$SHOP&category=service"
  [ "$(jget "$TMP/body.json" "d['total']")" = "1" ] && ok "$tag service有1個" || bad "$tag 分類篩選" "-"
  expect "$tag 爛分類查詢→400" 400 GET "/items?shop_id=$SHOP&category=nope"
  expect "$tag 關鍵字中文分詞" 200 GET "/items?shop_id=$SHOP&q=測試水餃"
  [ "$(jget "$TMP/body.json" "d['total']")" = "1" ] && ok "$tag 中文命中1個" || bad "$tag 中文分詞" "-"
  expect "$tag 關鍵字含空白標點" 200 GET "/items?shop_id=$SHOP&q=測試，水餃"
  [ "$(jget "$TMP/body.json" "d['total']")" = "1" ] && ok "$tag 標點容錯" || bad "$tag 標點容錯" "-"
  expect "$tag 關鍵字AND需全中" 200 GET "/items?shop_id=$SHOP&q=測試快剪水餃"
  [ "$(jget "$TMP/body.json" "d['total']")" = "0" ] && ok "$tag 全中才回" || bad "$tag AND語義" "-"
  expect "$tag 描述也命中" 200 GET "/items?shop_id=$SHOP&q=現包冷凍"
  [ "$(jget "$TMP/body.json" "d['total']")" = "1" ] && ok "$tag 描述命中" || bad "$tag 描述搜尋" "-"
  expect "$tag 改分類" 200 PATCH "/items/$PID" "$TOKEN_NEW" '{"category":"fresh"}'
  [ "$(jget "$TMP/body.json" "d['category']")" = "fresh" ] && ok "$tag 分類改了" || bad "$tag 改分類" "-"
  expect "$tag 改爛分類→400" 400 PATCH "/items/$PID" "$TOKEN_NEW" '{"category":"nope"}'
  expect "$tag 改回food" 200 PATCH "/items/$PID" "$TOKEN_NEW" '{"category":"food"}'

  # --- 訂單：直接買正常流程 ---
  expect "$tag 下單" 200 POST /orders "$TOKEN_B" \
    "{\"items\":[{\"item_id\":$PID,\"qty\":2}]}"
  OID="$(jget "$TMP/body.json" "d['id']")"
  [ "$(jget "$TMP/body.json" "d['status']")" = "pending" ] && ok "$tag 新單 pending" || bad "$tag 新單狀態" "-"
  [ "$(jget "$TMP/body.json" "d['buyer']['nickname']")" != "" ] && ok "$tag 明細有買家" || bad "$tag 買家" "-"
  expect "$tag 庫存扣了" 200 GET "/items/$PID"
  [ "$(jget "$TMP/body.json" "d['stock']")" = "3" ] && ok "$tag 庫存 5→3" || bad "$tag 庫存未扣" "-"
  expect "$tag 超賣→400" 400 POST /orders "$TOKEN_B" \
    "{\"items\":[{\"item_id\":$PID,\"qty\":99}]}"
  expect "$tag 買家confirm→403" 403 PATCH "/orders/$OID" "$TOKEN_B" '{"action":"confirm"}'
  expect "$tag 賣家confirm" 200 PATCH "/orders/$OID" "$TOKEN_NEW" '{"action":"confirm"}'
  [ "$(jget "$TMP/body.json" "d['status']")" = "confirmed" ] && ok "$tag confirmed" || bad "$tag confirm狀態" "-"
  expect "$tag 賣家ready" 200 PATCH "/orders/$OID" "$TOKEN_NEW" '{"action":"ready"}'
  expect "$tag 完成" 200 PATCH "/orders/$OID" "$TOKEN_NEW" '{"action":"complete"}'
  [ "$(jget "$TMP/body.json" "d['status']")" = "completed" ] && ok "$tag completed" || bad "$tag complete狀態" "-"
  expect "$tag 已完成再取消→400" 400 PATCH "/orders/$OID" "$TOKEN_NEW" '{"action":"cancel"}'

  # --- 訂單：取消回補 ---
  expect "$tag 再下一單" 200 POST /orders "$TOKEN_B" \
    "{\"items\":[{\"item_id\":$PID,\"qty\":1}]}"
  OID2="$(jget "$TMP/body.json" "d['id']")"
  expect "$tag 取消" 200 PATCH "/orders/$OID2" "$TOKEN_B" '{"action":"cancel"}'
  [ "$(jget "$TMP/body.json" "d['status']")" = "cancelled" ] && ok "$tag cancelled" || bad "$tag cancel狀態" "-"
  expect "$tag 庫存回補" 200 GET "/items/$PID"
  [ "$(jget "$TMP/body.json" "d['stock']")" = "3" ] && ok "$tag 庫存回到3" || bad "$tag 庫存未回補" "-"

  # --- 通知：下單→店主，確認→買家 ---
  expect "$tag 未登入看通知→401" 401 GET /notifications
  expect "$tag 賣家未讀數" 200 GET /notifications/unread-count "$TOKEN_NEW"
  N0="$(jget "$TMP/body.json" "d['count']")"
  expect "$tag 通知用下單" 200 POST /orders "$TOKEN_B" \
    "{\"items\":[{\"item_id\":$PID,\"qty\":1}]}"
  OID3="$(jget "$TMP/body.json" "d['id']")"
  expect "$tag 店主未讀+1" 200 GET /notifications/unread-count "$TOKEN_NEW"
  [ "$(jget "$TMP/body.json" "d['count']")" = "$((N0+1))" ] && ok "$tag 未讀加一" || bad "$tag 未讀數" "-"
  expect "$tag 通知列表" 200 GET /notifications "$TOKEN_NEW"
  [ "$(jget "$TMP/body.json" "[x['kind'] for x in d][0]")" = "order_created" ] && ok "$tag 最新是下單" || bad "$tag 通知kind" "-"
  expect "$tag 買家未讀數" 200 GET /notifications/unread-count "$TOKEN_B"
  M0="$(jget "$TMP/body.json" "d['count']")"
  expect "$tag 賣家確認3" 200 PATCH "/orders/$OID3" "$TOKEN_NEW" '{"action":"confirm"}'
  expect "$tag 買家未讀+1" 200 GET /notifications/unread-count "$TOKEN_B"
  [ "$(jget "$TMP/body.json" "d['count']")" = "$((M0+1))" ] && ok "$tag 買家未讀加一" || bad "$tag 買家未讀" "-"
  expect "$tag 買家明細有品名" 200 GET "/orders/$OID3" "$TOKEN_B"
  [ "$(jget "$TMP/body.json" "d['items'][0]['title']")" != "" ] && ok "$tag 品名有" || bad "$tag 品名空" "-"
  expect "$tag 全部已讀" 200 POST /notifications/read "$TOKEN_NEW" '{"ids":[]}'
  expect "$tag 賣家未讀歸零" 200 GET /notifications/unread-count "$TOKEN_NEW"
  [ "$(jget "$TMP/body.json" "d['count']")" = "0" ] && ok "$tag 歸零" || bad "$tag 未歸零" "-"

  # --- 訂單：不限量下大單不擋 ---
  expect "$tag 服務型下99件" 200 POST /orders "$TOKEN_B" \
    "{\"items\":[{\"item_id\":$SID,\"qty\":99}]}"
  [ "$(jget "$TMP/body.json" "d['total_cents']")" = "2970000" ] && ok "$tag 金額對" || bad "$tag 金額" "-"

  # --- 訂單：選日期（預約同一套） ---
  WIN='{"start":"09:00","end":"12:00"}'
  expect "$tag 設週範本全開" 200 PUT "/items/$SID/rules" "$TOKEN_NEW" \
    "{\"weekly\":[{\"weekday\":0,\"open\":true,\"windows\":[$WIN]},{\"weekday\":1,\"open\":true,\"windows\":[$WIN]},{\"weekday\":2,\"open\":true,\"windows\":[$WIN]},{\"weekday\":3,\"open\":true,\"windows\":[$WIN]},{\"weekday\":4,\"open\":true,\"windows\":[$WIN]},{\"weekday\":5,\"open\":true,\"windows\":[$WIN]},{\"weekday\":6,\"open\":true,\"windows\":[$WIN]}]}"
  expect "$tag 混合型設範本" 200 PUT "/items/$MID/rules" "$TOKEN_NEW" \
    "{\"weekly\":[{\"weekday\":0,\"open\":true,\"windows\":[$WIN]},{\"weekday\":1,\"open\":true,\"windows\":[$WIN]},{\"weekday\":2,\"open\":true,\"windows\":[$WIN]},{\"weekday\":3,\"open\":true,\"windows\":[$WIN]},{\"weekday\":4,\"open\":true,\"windows\":[$WIN]},{\"weekday\":5,\"open\":true,\"windows\":[$WIN]},{\"weekday\":6,\"open\":true,\"windows\":[$WIN]}]}"
  expect "$tag 爛weekday→400" 400 PUT "/items/$SID/rules" "$TOKEN_NEW" \
    '{"weekly":[{"weekday":9,"open":true,"windows":[]}]}'
  expect "$tag 爛時段→400" 400 PUT "/items/$SID/rules" "$TOKEN_NEW" \
    '{"weekly":[{"weekday":1,"open":true,"windows":[{"start":"","end":"09:00"}]}]}'
  expect "$tag 他人設範本→403" 403 PUT "/items/$SID/rules" "$TOKEN_B" \
    '{"weekly":[{"weekday":1,"open":true,"windows":[]}]}'

  D1="$(fdate 2)"; M1="${D1:0:7}"
  expect "$tag 選日期下單" 200 POST /orders "$TOKEN_B" \
    "{\"items\":[{\"item_id\":$SID,\"qty\":1}],\"date\":\"$D1\",\"window\":\"09:00-12:00\"}"
  [ "$(jget "$TMP/body.json" "d['status']")" = "pending" ] && ok "$tag 預約單 pending" || bad "$tag 預約單狀態" "-"
  [ "$(jget "$TMP/body.json" "d['date']")" = "$D1" ] && ok "$tag 有日期" || bad "$tag 日期" "-"
  AID="$(jget "$TMP/body.json" "d['id']")"
  expect "$tag 同人同日同項目→400" 400 POST /orders "$TOKEN_B" \
    "{\"items\":[{\"item_id\":$SID,\"qty\":1}],\"date\":\"$D1\"}"
  expect "$tag 同日多人可約" 200 POST /orders "$TOKEN_S" \
    "{\"items\":[{\"item_id\":$SID,\"qty\":1}],\"date\":\"$D1\"}"
  expect "$tag 不可約項目帶日期→400" 400 POST /orders "$TOKEN_B" \
    "{\"items\":[{\"item_id\":$PID,\"qty\":1}],\"date\":\"$D1\"}"
  expect "$tag 過去日期→400" 400 POST /orders "$TOKEN_B" \
    "{\"items\":[{\"item_id\":$SID,\"qty\":1}],\"date\":\"2000-01-01\"}"
  expect "$tag 亂填日期→400" 400 POST /orders "$TOKEN_B" \
    "{\"items\":[{\"item_id\":$SID,\"qty\":1}],\"date\":\"明天\"}"
  expect "$tag 亂填區段→400" 400 POST /orders "$TOKEN_S" \
    "{\"items\":[{\"item_id\":$SID,\"qty\":1}],\"date\":\"$(fdate 9)\",\"window\":\"半夜\"}"
  expect "$tag 訂單明細" 200 GET "/orders/$AID" "$TOKEN_B"
  [ "$(jget "$TMP/body.json" "d['items'][0]['title']")" = "測試快剪" ] && ok "$tag 品名對" || bad "$tag 品名" "-"
  expect "$tag 他人看單→403" 403 GET "/orders/$AID" "$TOKEN_S"
  expect "$tag 買家confirm→403" 403 PATCH "/orders/$AID" "$TOKEN_B" '{"action":"confirm"}'
  expect "$tag 賣家confirm" 200 PATCH "/orders/$AID" "$TOKEN_NEW" '{"action":"confirm"}'
  expect "$tag 跳過ready直接完成" 200 PATCH "/orders/$AID" "$TOKEN_NEW" '{"action":"complete"}'
  [ "$(jget "$TMP/body.json" "d['status']")" = "completed" ] && ok "$tag 預約completed" || bad "$tag complete狀態" "-"
  expect "$tag 已完成再取消→400" 400 PATCH "/orders/$AID" "$TOKEN_NEW" '{"action":"cancel"}'

  # --- 訂單：爽約＋取消期限 ---
  D2="$(fdate 3)"
  expect "$tag 再約一筆" 200 POST /orders "$TOKEN_B" \
    "{\"items\":[{\"item_id\":$SID,\"qty\":1}],\"date\":\"$D2\"}"
  AID2="$(jget "$TMP/body.json" "d['id']")"
  expect "$tag 買家noshow→403" 403 PATCH "/orders/$AID2" "$TOKEN_B" '{"action":"noshow"}'
  expect "$tag 賣家confirm2" 200 PATCH "/orders/$AID2" "$TOKEN_NEW" '{"action":"confirm"}'
  expect "$tag 記爽約" 200 PATCH "/orders/$AID2" "$TOKEN_NEW" '{"action":"noshow"}'
  [ "$(jget "$TMP/body.json" "d['status']")" = "noshow" ] && ok "$tag noshow" || bad "$tag noshow狀態" "-"
  expect "$tag 建高門檻項目" 200 POST /items "$TOKEN_NEW" \
    "{\"shop_id\":$SHOP,\"title\":\"高門檻\",\"price_cents\":100,\"bookable\":true,\"cancel_hours\":720}"
  HID="$(jget "$TMP/body.json" "d['id']")"
  expect "$tag 高門檻設範本" 200 PUT "/items/$HID/rules" "$TOKEN_NEW" \
    '{"weekly":[{"weekday":0,"open":true,"windows":[]},{"weekday":1,"open":true,"windows":[]},{"weekday":2,"open":true,"windows":[]},{"weekday":3,"open":true,"windows":[]},{"weekday":4,"open":true,"windows":[]},{"weekday":5,"open":true,"windows":[]},{"weekday":6,"open":true,"windows":[]}]}'
  D3="$(fdate 4)"
  expect "$tag 政策內下單" 200 POST /orders "$TOKEN_B" \
    "{\"items\":[{\"item_id\":$HID,\"qty\":1}],\"date\":\"$D3\"}"
  AID3="$(jget "$TMP/body.json" "d['id']")"
  expect "$tag 太晚取消→422" 422 PATCH "/orders/$AID3" "$TOKEN_B" '{"action":"cancel"}'
  D4="$(fdate 5)"
  expect "$tag 正常預約" 200 POST /orders "$TOKEN_B" \
    "{\"items\":[{\"item_id\":$SID,\"qty\":1}],\"date\":\"$D4\"}"
  AID4="$(jget "$TMP/body.json" "d['id']")"
  expect "$tag 買家取消" 200 PATCH "/orders/$AID4" "$TOKEN_B" '{"action":"cancel"}'
  [ "$(jget "$TMP/body.json" "d['status']")" = "cancelled" ] && ok "$tag 預約cancelled" || bad "$tag cancel狀態" "-"
  expect "$tag 取消後可重約" 200 POST /orders "$TOKEN_B" \
    "{\"items\":[{\"item_id\":$SID,\"qty\":1}],\"date\":\"$D4\"}"

  # --- 訂單：混合型（有庫存＋有日期一起扣） ---
  D5="$(fdate 6)"
  expect "$tag 混合下單" 200 POST /orders "$TOKEN_B" \
    "{\"items\":[{\"item_id\":$MID,\"qty\":2}],\"date\":\"$D5\"}"
  expect "$tag 混合庫存扣了" 200 GET "/items/$MID"
  [ "$(jget "$TMP/body.json" "d['stock']")" = "1" ] && ok "$tag 混合庫存 3→1" || bad "$tag 混合庫存" "-"

  # --- 單日例外 ---
  D6="$(fdate 7)"
  expect "$tag 單日關閉" 200 POST "/items/$SID/exceptions" "$TOKEN_NEW" \
    "{\"date\":\"$D6\",\"open\":false}"
  expect "$tag 公休日→400" 400 POST /orders "$TOKEN_B" \
    "{\"items\":[{\"item_id\":$SID,\"qty\":1}],\"date\":\"$D6\"}"
  expect "$tag 刪例外回範本" 200 DELETE "/items/$SID/exceptions/$D6" "$TOKEN_NEW"
  expect "$tag 恢復可約" 200 POST /orders "$TOKEN_B" \
    "{\"items\":[{\"item_id\":$SID,\"qty\":1}],\"date\":\"$D6\"}"
  D7="$(fdate 8)"
  expect "$tag 單日勾額滿" 200 POST "/items/$SID/exceptions" "$TOKEN_NEW" \
    "{\"date\":\"$D7\",\"open\":true,\"full\":true,\"note\":\"人手不足\"}"
  expect "$tag 額滿日→400" 400 POST /orders "$TOKEN_B" \
    "{\"items\":[{\"item_id\":$SID,\"qty\":1}],\"date\":\"$D7\"}"
  expect "$tag 取消額滿" 200 POST "/items/$SID/exceptions" "$TOKEN_NEW" \
    "{\"date\":\"$D7\",\"open\":true,\"full\":false}"
  expect "$tag 恢復可約2" 200 POST /orders "$TOKEN_B" \
    "{\"items\":[{\"item_id\":$SID,\"qty\":1}],\"date\":\"$D7\"}"

  # --- 列表＋月曆 ---
  expect "$tag 本店項目" 200 GET "/items?shop_id=$SHOP"
  [ "$(jget "$TMP/body.json" "d['total']")" -ge 4 ] && ok "$tag 有4個項目" || bad "$tag 項目列表" "-"
  expect "$tag 項目月曆" 200 GET "/items/$SID?month=$M1"
  [ "$(jget "$TMP/body.json" "len(d['days'])")" -ge 28 ] && ok "$tag 月曆天數" || bad "$tag 月曆" "-"
  expect "$tag 非預約無月曆" 200 GET "/items/$PID?month=$M1"
  [ "$(jget "$TMP/body.json" "len(d['days'])")" = "0" ] && ok "$tag 無days" || bad "$tag days應空" "-"
  expect "$tag 爛月份→400" 400 GET "/items/$SID?month=2026-13"
  expect "$tag 賣家訂單列表" 200 GET "/orders?role=seller" "$TOKEN_NEW"
  expect "$tag 有日期篩選" 200 GET "/orders?role=seller&from=$D1&to=$D1" "$TOKEN_NEW"
  [ "$(jget "$TMP/body.json" "len(d)")" -ge 2 ] && ok "$tag 當日有單" || bad "$tag 日期篩選" "-"
  expect "$tag 店家月曆" 200 GET "/provider/calendar?shop_id=$SHOP&month=$M1" "$TOKEN_NEW"
  expect "$tag 他人看月曆→403" 403 GET "/provider/calendar?shop_id=$SHOP&month=$M1" "$TOKEN_B"
  expect "$tag 下架項目" 200 PATCH "/items/$SID" "$TOKEN_NEW" '{"status":"off"}'
  expect "$tag 下架不可買→400" 400 POST /orders "$TOKEN_S" \
    "{\"items\":[{\"item_id\":$SID,\"qty\":1}]}"
  expect "$tag 下架不可約→400" 400 POST /orders "$TOKEN_S" \
    "{\"items\":[{\"item_id\":$SID,\"qty\":1}],\"date\":\"$(fdate 10)\"}"

  # --- 管理員：地區 CRUD ---
  expect "$tag 非管理員建區→403" 403 POST /admin/areas "$TOKEN_NEW" \
    '{"county":"測試縣","township":"測試鎮"}'
  expect "$tag 未登入建區→401" 401 POST /admin/areas "" \
    '{"county":"測試縣","township":"測試鎮"}'
  expect "$tag 建區" 200 POST /admin/areas "$TOKEN_A" \
    '{"county":"測試縣","township":"測試鎮"}'
  NAID="$(jget "$TMP/body.json" "d['id']")"
  expect "$tag 重複建區→409" 409 POST /admin/areas "$TOKEN_A" \
    '{"county":"測試縣","township":"測試鎮"}'
  expect "$tag 建區缺欄→422" 422 POST /admin/areas "$TOKEN_A" \
    '{"county":"測試縣"}'
  expect "$tag 改區名" 200 PUT "/admin/areas/$NAID" "$TOKEN_A" \
    '{"township":"測試鎮改"}'
  [ "$(jget "$TMP/body.json" "d['township']")" = "測試鎮改" ] && ok "$tag 區名改了" || bad "$tag 改區" "-"
  expect "$tag 建區自動trim" 200 POST /admin/areas "$TOKEN_A" \
    '{"county":" 測試縣 ","township":" 空白鎮 "}'
  [ "$(jget "$TMP/body.json" "d['township']")" = "空白鎮" ] && ok "$tag 空白 trimmed" || bad "$tag trim" "-"
  TID="$(jget "$TMP/body.json" "d['id']")"
  expect "$tag 刪trim區" 200 DELETE "/admin/areas/$TID" "$TOKEN_A"
  expect "$tag 非管理員改區→403" 403 PUT "/admin/areas/$NAID" "$TOKEN_NEW" \
    '{"township":"壞"}'
  expect "$tag 改不存在區→404" 404 PUT "/admin/areas/999999" "$TOKEN_A" \
    '{"township":"壞"}'
  expect "$tag 使用中刪區→400" 400 DELETE "/admin/areas/$AREA" "$TOKEN_A"
  expect "$tag 非管理員刪區→403" 403 DELETE "/admin/areas/$NAID" "$TOKEN_NEW"
  expect "$tag 刪空區" 200 DELETE "/admin/areas/$NAID" "$TOKEN_A"

  # --- 管理員：一覽＋停權 ---
  expect "$tag 非管理員看店→403" 403 GET /admin/shops "$TOKEN_NEW"
  expect "$tag 店家一覽" 200 GET /admin/shops "$TOKEN_A"
  [ "$(jget "$TMP/body.json" "len(d)")" -ge 1 ] && ok "$tag 有店" || bad "$tag 店一覽" "-"
  expect "$tag 店家停權" 200 PATCH "/shops/$SHOP" "$TOKEN_A" '{"status":"closed"}'
  expect "$tag 項目一覽" 200 GET /admin/items "$TOKEN_A"
  [ "$(jget "$TMP/body.json" "len(d)")" -ge 1 ] && ok "$tag 有項目" || bad "$tag 項目一覽" "-"
  expect "$tag 項目下架" 200 PATCH "/items/$PID" "$TOKEN_A" '{"status":"off"}'
  [ "$(jget "$TMP/body.json" "d['status']")" = "off" ] && ok "$tag 下架了" || bad "$tag 下架" "-"
  expect "$tag 項目恢復" 200 PATCH "/items/$PID" "$TOKEN_A" '{"status":"on"}'
  expect "$tag 店家恢復" 200 PATCH "/shops/$SHOP" "$TOKEN_A" '{"status":"open"}'
  expect "$tag 使用者一覽" 200 GET /admin/users "$TOKEN_A"
  [ "$(jget "$TMP/body.json" "len(d)")" -ge 3 ] && ok "$tag 有使用者" || bad "$tag 使用者一覽" "-"
  expect "$tag 非管理員看人→403" 403 GET /admin/users "$TOKEN_B"
  expect "$tag 訂單一覽" 200 GET /admin/orders "$TOKEN_A"
  [ "$(jget "$TMP/body.json" "len(d)")" -ge 1 ] && ok "$tag 有訂單" || bad "$tag 訂單一覽" "-"
  expect "$tag 非管理員看單→403" 403 GET /admin/orders "$TOKEN_B"

  echo "== $tag 全過 =="
}

echo "## 1/3 後端單元測試"
(cd "$BACKEND" && cargo test 2>&1 | tail -3)

if [ "$BUILD" = 1 ]; then
  echo "## 2/3 建置"
  (cd "$BACKEND" && cargo build 2>&1 | tail -2)
else
  echo "## 2/3 建置（跳過）"
fi
[ -x "$BIN" ] || { echo "找不到 $BIN，先跑 cargo build"; exit 1; }

echo "## 3/3 API 端到端（SQLite）"
start_server "sqlite://$TMP/t.db?mode=rwc"
suite "sqlite"

if [ "$WITH_PG" = 1 ]; then
  echo "## 4/3 API 端到端（PostgreSQL）"
  command -v docker >/dev/null || { echo "需要 docker 才能跑 --pg"; exit 1; }
  PG_CONTAINER="areashop-test-pg"
  docker rm -f "$PG_CONTAINER" >/dev/null 2>&1 || true
  docker run -d --name "$PG_CONTAINER" -e POSTGRES_USER=areashop \
    -e POSTGRES_PASSWORD=areashop -e POSTGRES_DB=areashop \
    -p 15433:5432 postgres:16-alpine >/dev/null
  for _ in $(seq 1 30); do
    docker exec "$PG_CONTAINER" pg_isready -U areashop >/dev/null 2>&1 && break
    sleep 1
  done
  start_server "postgres://areashop:areashop@localhost:15433/areashop"
  suite "postgres"
fi

echo ""
echo "ALL TESTS PASSED (unit: $PASS checks above, suites done)"
