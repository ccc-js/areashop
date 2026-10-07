import { useEffect, useState } from "react";
import { Link, useLocation, useSearchParams } from "react-router-dom";
import { api, nt, type Item } from "../lib/api";
import { useArea } from "../lib/area";

// 首頁狀態全部收進 URL（?county=&town=&q=）：上一頁返回、重新整理、分享連結
// 都會回到同一個篩選狀態；捲動位置用 sessionStorage 按 URL 記憶。
// 賣東西和賣服務同一種：一個列表，徽章區分（可預約 / 剩件數 / 不限量）。
export default function Home() {
  const { areas, current, setCurrentId } = useArea();
  const [sp, setSp] = useSearchParams();
  const location = useLocation();
  const [items, setItems] = useState<Item[]>([]);
  const [err, setErr] = useState("");

  const counties = [...new Set(areas.map((a) => a.county))];
  const county = sp.get("county") ?? current?.county ?? counties[0] ?? "";
  const town = sp.get("town") ?? (current ? String(current.id) : "all");
  const q = sp.get("q") ?? "";
  const [draft, setDraft] = useState(q);
  // 上一頁/下一頁切換 URL 時，輸入框跟著還原（打字中不會觸發，因打字不改 URL）
  useEffect(() => {
    setDraft(q);
  }, [q]);
  const townships = areas.filter((a) => a.county === county);

  // URL county 若不在已知清單（或尚未載入地區），等 areas 就緒後校正
  useEffect(() => {
    if (areas.length === 0) return;
    if (!areas.some((a) => a.county === county)) {
      const fallback = current?.county ?? counties[0];
      if (fallback) update({ county: fallback, town: "all" });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [areas]);

  const update = (patch: Record<string, string>) => {
    const next = new URLSearchParams(sp);
    for (const [k, v] of Object.entries(patch)) {
      if (v) next.set(k, v);
      else next.delete(k);
    }
    setSp(next, { replace: false });
  };

  useEffect(() => {
    if (!county) return;
    const params = new URLSearchParams();
    if (town === "all") params.set("county", county);
    else params.set("area_id", town);
    const kw = sp.get("q");
    if (kw) params.set("q", kw);
    let alive = true;
    api<{ items: Item[]; total: number }>(`/items?${params}`)
      .then((d) => {
        if (!alive) return;
        setItems(d.items);
        setErr("");
      })
      .catch((e) => alive && setErr(String(e)));
    return () => {
      alive = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sp, areas]);

  // 捲動位置：離開記住，回來還原（含瀏覽器上一頁）
  useEffect(() => {
    const key = `scroll:${location.pathname}${location.search}`;
    const y = Number(sessionStorage.getItem(key) ?? 0);
    if (y > 0) requestAnimationFrame(() => window.scrollTo(0, y));
    return () => {
      sessionStorage.setItem(key, String(window.scrollY));
    };
  }, [location.pathname, location.search]);

  const setCounty = (c: string) => {
    const first = areas.find((a) => a.county === c);
    if (first) setCurrentId(first.id);
    update({ county: c, town: "all" });
  };
  const setTown = (t: string) => {
    if (t !== "all") setCurrentId(Number(t));
    update({ town: t });
  };
  const search = () => update({ q: draft.trim() });

  return (
    <div>
      <div className="area-bar">
        <label>
          縣市
          <select value={county} onChange={(e) => setCounty(e.target.value)}>
            {counties.map((c) => (
              <option key={c} value={c}>{c}</option>
            ))}
          </select>
        </label>
        <label>
          鄉鎮
          <select value={town} onChange={(e) => setTown(e.target.value)}>
            <option value="all">不分區（全{county}）</option>
            {townships.map((a) => (
              <option key={a.id} value={a.id}>
                {a.township}
              </option>
            ))}
          </select>
        </label>
        <span className="hint">只看附近，不看全站</span>
      </div>

      <div className="search-bar">
        <input
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && search()}
          placeholder="搜尋：水餃、放山雞、寄養…"
        />
        <button onClick={search}>搜尋</button>
      </div>

      {err && <p className="error">{err}</p>}
      {items.length === 0 && !err && <p className="empty">這個地區還沒有東西，去隔壁鄉鎮看看吧。</p>}

      <div className="grid">
        {items.map((p) => (
          <Link key={p.id} className="card" to={`/i/${p.id}`}>
            <div className="card-title">{p.title}</div>
            <div className="card-meta">
              {nt(p.price_cents)}/{p.unit}
              {p.bookable && " · 可預約"}
            </div>
            <div className="card-stock">
              {p.stock != null && (p.stock > 0 ? `剩 ${p.stock}` : "售完")}
            </div>
          </Link>
        ))}
      </div>
    </div>
  );
}
