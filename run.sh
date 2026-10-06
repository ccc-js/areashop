#!/usr/bin/env bash
# areashop 一鍵啟動
#
#   ./run.sh          # 選單：1.從空開始 2.塞假資料，再啟動（SQLite :8080＋前端 :5173）
#   ./run.sh --empty # 直接從空資料庫開始（會刪掉本機 sqlite 檔 / 清空 pg schema）
#   ./run.sh --seed  # 假測試資料；DB 已有資料則沿用（不會覆蓋）
#   ./run.sh --reseed # 砍掉重練：清空後重塞假資料（種子改版時用）
#   ./run.sh --pg [--empty|--seed]  # 改連 PostgreSQL（需 docker，自動起一個 pg）
#   ./run.sh --prod   # 正式模式：docker compose 建置並啟動全部（web :3000）
#   ./run.sh --stop   # 停掉 run.sh 起的背景行程 / 容器
#
# 開發模式按 Ctrl-C 即全部停止。種子帳號：0900000003 / password123（買家）。

set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
BIN="$ROOT/backend/target/debug/areashop-api"
MODE="dev"
SEED_MODE="ask"   # ask | seed | empty

for arg in "$@"; do
  case "$arg" in
    --pg) MODE="pg" ;;
    --prod) MODE="prod" ;;
    --stop) MODE="stop" ;;
    --empty) SEED_MODE="empty" ;;
    --seed) SEED_MODE="seed" ;;
    --reseed) SEED_MODE="reseed" ;;
    *) echo "unknown arg: $arg"; exit 2 ;;
  esac
done

stop_all() {
  for pid in ${API_PID:-} ${WEB_PID:-}; do
    kill "$pid" 2>/dev/null || true
  done
  pkill -f "target/debug/areashop-api" 2>/dev/null || true
  pkill -f "vite --port 5173" 2>/dev/null || true
  docker rm -f areashop-pg areashop-test-pg 2>/dev/null || true
  echo "stopped."
}

if [ "$MODE" = "stop" ]; then stop_all; exit 0; fi
trap stop_all EXIT

if [ "$MODE" = "prod" ]; then
  docker compose -f "$ROOT/docker-compose.yml" up --build
  exit 0
fi

# 沒指定 --empty/--seed 且在終端機前：跳選單
if [ "$SEED_MODE" = "ask" ] && [ -t 0 ]; then
  echo "areashop 啟動選項："
  echo "  1) 從空的開始（清空舊資料，不塞假資料）"
  echo "  2) 塞假測試資料（美濃區＋店家＋商品＋測試帳號）"
  printf "請選擇 [1/2，預設 2]: "
  read -r choice </dev/tty || choice="2"
  case "$choice" in
    1) SEED_MODE="empty" ;;
    *) SEED_MODE="seed" ;;
  esac
elif [ "$SEED_MODE" = "ask" ]; then
  SEED_MODE="seed"   # 非互動（管線/CI）預設塞假資料
fi

command -v cargo >/dev/null || { echo "需要 cargo（rust）"; exit 1; }
command -v npm >/dev/null || { echo "需要 node/npm"; exit 1; }

if [ ! -x "$BIN" ]; then
  echo ">> 建置後端…"
  (cd "$ROOT/backend" && cargo build 2>&1 | tail -1)
fi
if [ ! -d "$ROOT/frontend/node_modules" ]; then
  echo ">> 安裝前端依賴…"
  (cd "$ROOT/frontend" && npm install 2>&1 | tail -1)
fi

if [ "$MODE" = "pg" ]; then
  command -v docker >/dev/null || { echo "--pg 需要 docker"; exit 1; }
  docker rm -f areashop-pg >/dev/null 2>&1 || true
  echo ">> 啟動 PostgreSQL…"
  docker run -d --name areashop-pg -e POSTGRES_USER=areashop \
    -e POSTGRES_PASSWORD=areashop -e POSTGRES_DB=areashop \
    -p 5432:5432 postgres:16-alpine >/dev/null
  for _ in $(seq 1 30); do
    docker exec areashop-pg pg_isready -U areashop >/dev/null 2>&1 && break
    sleep 1
  done
  export DATABASE_URL="postgres://areashop:areashop@localhost:5432/areashop"
  echo ">> 資料庫：PostgreSQL（docker areashop-pg）"
  if [ "$SEED_MODE" = "empty" ] || [ "$SEED_MODE" = "reseed" ]; then
    echo ">> 清空 pg schema…"
    docker exec areashop-pg psql -U areashop -d areashop \
      -c "DROP SCHEMA public CASCADE; CREATE SCHEMA public;" >/dev/null
  fi
else
  export DATABASE_URL="${DATABASE_URL:-sqlite://$ROOT/areashop.db?mode=rwc}"
  echo ">> 資料庫：$DATABASE_URL"
  if [ "$SEED_MODE" = "empty" ] || [ "$SEED_MODE" = "reseed" ]; then
    case "$DATABASE_URL" in
      sqlite://*)
        dbfile="${DATABASE_URL#sqlite://}"; dbfile="${dbfile%%\?*}"
        case "$dbfile" in
          ""|":memory:") ;;
          *) rm -f "$dbfile" && echo ">> 已刪除舊 sqlite 檔：$dbfile" ;;
        esac
        ;;
    esac
  fi
fi
export PORT=8080

if [ "$SEED_MODE" = "empty" ]; then
  export SEED=0
  echo ">> 模式：從空開始（不塞假資料）"
else
  export SEED=1
  if [ "$SEED_MODE" = "reseed" ]; then
    echo ">> 模式：砍掉重練（已清空＋重塞假資料）"
  else
    echo ">> 模式：含假測試資料（DB 已有資料則沿用；要重塞請用 --reseed）"
  fi
fi

echo ">> 啟動後端 :8080…"
pushd "$ROOT/backend" >/dev/null
nohup "$BIN" >/tmp/areashop_api.log 2>&1 &
API_PID=$!
popd >/dev/null
for _ in $(seq 1 30); do
  curl -s -o /dev/null http://localhost:8080/api/v1/healthz && break
  sleep 1
done
curl -s http://localhost:8080/api/v1/healthz | grep -q ok \
  && echo ">> 後端 OK" || { echo "後端啟動失敗，看 /tmp/areashop_api.log"; exit 1; }

echo ">> 資料現況：$(curl -s http://localhost:8080/api/v1/areas | python3 -c "import sys,json; print('、'.join(a['county']+a['township'] for a in json.load(sys.stdin)))")"

echo ">> 啟動前端 :5173…"
pushd "$ROOT/frontend" >/dev/null
nohup npm run dev -- --port 5173 >/tmp/areashop_web.log 2>&1 &
WEB_PID=$!
popd >/dev/null
sleep 6

echo ""
echo "  前端  http://localhost:5173"
echo "  後端  http://localhost:8080/api/v1/healthz"
echo "  測試  ./test.sh"
echo "按 Ctrl-C 停止。"
wait
