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
  DATABASE_URL="$1" PORT="$PORT" SEED=1 nohup "$BIN" >"$TMP/server.log" 2>&1 &
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

  # --- 商店 ---
  expect "$tag 開店" 200 POST /shops "$TOKEN_NEW" \
    "{\"name\":\"測試店\",\"area_id\":$AREA,\"kind\":\"personal\"}"
  SHOP="$(jget "$TMP/body.json" "d['id']")"
  expect "$tag 我的店" 200 GET /shops "$TOKEN_NEW"
  expect "$tag 開店面店" 200 POST /shops "$TOKEN_NEW" \
    "{\"name\":\"測試店面\",\"area_id\":$AREA,\"kind\":\"store\",\"address\":\"金城鎮模範街1號\",\"opening_hours\":\"每日 09:00-18:00\",\"pickup_mode\":\"store\"}"
  [ "$(jget "$TMP/body.json" "d['pickup_mode']")" = "store" ] && ok "$tag 店面 pickup_mode=store" || bad "$tag pickup_mode" "-"
  expect "$tag 店面無地址→400" 400 POST /shops "$TOKEN_NEW" \
    "{\"name\":\"無址店\",\"area_id\":$AREA,\"kind\":\"store\",\"pickup_mode\":\"store\"}"
  expect "$tag 取貨方式亂填→400" 400 POST /shops "$TOKEN_NEW" \
    "{\"name\":\"亂填店\",\"area_id\":$AREA,\"kind\":\"personal\",\"pickup_mode\":\"drone\"}"

  # --- 商品 ---
  expect "$tag 上架" 200 POST /products "$TOKEN_NEW" \
    "{\"shop_id\":$SHOP,\"title\":\"測試水餃\",\"category\":\"dumpling\",\"price_cents\":16000,\"stock\":5,\"unit\":\"包\",\"pickup_places\":[],\"images\":[]}"
  PID="$(jget "$TMP/body.json" "d['id']")"
  expect "$tag 他人商店上架→403" 403 POST /products "$TOKEN_B" \
    "{\"shop_id\":$SHOP,\"title\":\"偷渡\",\"category\":\"other\",\"price_cents\":100,\"stock\":1,\"unit\":\"份\",\"pickup_places\":[],\"images\":[]}"
  expect "$tag 地區商品列表" 200 GET "/products?area_id=$AREA"
  [ "$(jget "$TMP/body.json" "d['total']")" -ge 1 ] && ok "$tag 列表有商品" || bad "$tag 列表為空" "-"
  expect "$tag 不分區全縣" 200 GET "/products?county=金門縣"
  [ "$(jget "$TMP/body.json" "d['total']")" -ge 1 ] && ok "$tag 全縣有商品" || bad "$tag 全縣為空" "-"
  expect "$tag 不存在的縣→空" 200 GET "/products?county=火星市"
  [ "$(jget "$TMP/body.json" "d['total']")" = "0" ] && ok "$tag 不存在的縣回空" || bad "$tag 不存在的縣應回空" "-"

  # --- 訂單：正常流程 ---
  expect "$tag 下單" 200 POST /orders "$TOKEN_B" \
    "{\"items\":[{\"product_id\":$PID,\"qty\":2}],\"pickup_place\":\"區公所前\"}"
  OID="$(jget "$TMP/body.json" "d['id']")"
  [ "$(jget "$TMP/body.json" "d['status']")" = "pending" ] && ok "$tag 新單 pending" || bad "$tag 新單狀態" "-"
  expect "$tag 庫存扣了" 200 GET "/products/$PID"
  [ "$(jget "$TMP/body.json" "d['stock']")" = "3" ] && ok "$tag 庫存 5→3" || bad "$tag 庫存未扣" "-"
  expect "$tag 超賣→400" 400 POST /orders "$TOKEN_B" \
    "{\"items\":[{\"product_id\":$PID,\"qty\":99}],\"pickup_place\":\"區公所前\"}"
  expect "$tag 買家confirm→403" 403 PATCH "/orders/$OID" "$TOKEN_B" '{"action":"confirm"}'
  expect "$tag 賣家confirm" 200 PATCH "/orders/$OID" "$TOKEN_NEW" '{"action":"confirm"}'
  [ "$(jget "$TMP/body.json" "d['status']")" = "confirmed" ] && ok "$tag confirmed" || bad "$tag confirm狀態" "-"
  expect "$tag 賣家ready" 200 PATCH "/orders/$OID" "$TOKEN_NEW" '{"action":"ready"}'
  expect "$tag 完成" 200 PATCH "/orders/$OID" "$TOKEN_NEW" '{"action":"complete"}'
  [ "$(jget "$TMP/body.json" "d['status']")" = "completed" ] && ok "$tag completed" || bad "$tag complete狀態" "-"
  expect "$tag 已完成再取消→400" 400 PATCH "/orders/$OID" "$TOKEN_NEW" '{"action":"cancel"}'

  # --- 訂單：取消回補 ---
  expect "$tag 再下一單" 200 POST /orders "$TOKEN_B" \
    "{\"items\":[{\"product_id\":$PID,\"qty\":1}],\"pickup_place\":\"區公所前\"}"
  OID2="$(jget "$TMP/body.json" "d['id']")"
  expect "$tag 取消" 200 PATCH "/orders/$OID2" "$TOKEN_B" '{"action":"cancel"}'
  [ "$(jget "$TMP/body.json" "d['status']")" = "cancelled" ] && ok "$tag cancelled" || bad "$tag cancel狀態" "-"
  expect "$tag 庫存回補" 200 GET "/products/$PID"
  [ "$(jget "$TMP/body.json" "d['stock']")" = "3" ] && ok "$tag 庫存回到3" || bad "$tag 庫存未回補" "-"

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
