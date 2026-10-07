import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import {
  api,
  nt,
  todayStr,
  windowLabel,
  type Item,
  type ItemDetail as SD,
  type Order,
  type TimeWindow,
} from "../lib/api";
import { useAuth } from "../lib/auth";
import { useLang } from "../lib/i18n";
import { toast } from "../lib/toast";

interface Shop {
  id: number;
  name: string;
}

interface DayRow {
  date: string;
  booked: number;
  orders: Order[];
}

// /provider：接案管理（週範本＋單日開關＋當日預約）
export default function Provider() {
  const { user } = useAuth();
  const { t, wdName, statusLabel } = useLang();
  const [shops, setShops] = useState<Shop[]>([]);
  const [shopId, setShopId] = useState(0);
  const [items, setItems] = useState<Item[]>([]);
  const [itemId, setItemId] = useState(0);
  const [detail, setDetail] = useState<SD | null>(null);
  const [cal, setCal] = useState<DayRow[]>([]);
  const [month, setMonth] = useState(todayStr().slice(0, 7));
  const [day, setDay] = useState("");
  const [msg, setMsg] = useState("");

  // 新增項目表單
  const [title, setTitle] = useState("");
  const [price, setPrice] = useState("300");

  // 週範本編輯：每天 {open, 時段陣列}＋新增用起終點
  const [week, setWeek] = useState<{ open: boolean; wins: TimeWindow[] }[]>(
    Array.from({ length: 7 }, () => ({ open: false, wins: [] }))
  );
  const [draft, setDraft] = useState<{ s: string; e: string }[]>(
    Array.from({ length: 7 }, () => ({ s: "09:00", e: "12:00" }))
  );

  // 單日：不營業 / 額滿二選一＋備註
  const [excMode, setExcMode] = useState<"open" | "full" | "closed">("open");
  const [excNote, setExcNote] = useState("");

  useEffect(() => {
    api<Shop[]>("/shops").then(setShops).catch(() => {});
  }, []);

  useEffect(() => {
    if (!shopId) return;
    api<{ items: Item[] }>(`/items?shop_id=${shopId}`).then((d) => setItems(d.items)).catch(() => {});
    setItemId(0);
    setDetail(null);
  }, [shopId]);

  const loadDetail = (id: number, m: string) => {
    api<SD>(`/items/${id}?month=${m}`).then((d) => {
      setDetail(d);
      const byWd = new Map(d.rules.map((r) => [r.weekday, r]));
      setWeek(
        Array.from({ length: 7 }, (_, wd) => {
          const r = byWd.get(wd);
          return r ? { open: r.open, wins: r.windows } : { open: false, wins: [] };
        })
      );
      setMsg("");
    }).catch((e) => setMsg(String(e)));
    api<{ days: DayRow[] }>(`/provider/calendar?shop_id=${shopId}&month=${m}`)
      .then((c) => setCal(c.days))
      .catch(() => {});
  };

  useEffect(() => {
    if (itemId) loadDetail(itemId, month);
  }, [itemId, month]); // eslint-disable-line react-hooks/exhaustive-deps

  if (!user) return <p>{t("common.loginFirst")}</p>;

  const addItem = async () => {
    try {
      const s = await api<Item>("/items", {
        method: "POST",
        body: JSON.stringify({
          shop_id: shopId,
          title,
          price_cents: Math.round(Number(price) * 100),
          stock: null,
          bookable: true,
        }),
      });
      setTitle("");
      setMsg(t("provider.added"));
      const list = await api<{ items: Item[] }>(`/items?shop_id=${shopId}`);
      setItems(list.items);
      setItemId(s.id);
    } catch (e) {
      setMsg(String(e));
    }
  };

  const saveWeek = async () => {
    try {
      await api(`/items/${itemId}/rules`, {
        method: "PUT",
        body: JSON.stringify({
          weekly: week.map((w, weekday) => ({ weekday, open: w.open, windows: w.wins })),
        }),
      });
      setMsg(t("provider.tplSaved"));
      toast(t("common.saved"));
      loadDetail(itemId, month);
    } catch (e) {
      setMsg(String(e));
    }
  };

  const addWin = (wd: number) => {
    const s = draft[wd].s.trim();
    const e = draft[wd].e.trim();
    if (!s || !e) {
      setMsg(t("provider.needEnds"));
      return;
    }
    const n = [...week];
    n[wd] = { ...n[wd], wins: [...n[wd].wins, { start: s, end: e }] };
    setWeek(n);
    setMsg("");
  };

  const delWin = (wd: number, i: number) => {
    const n = [...week];
    n[wd] = { ...n[wd], wins: n[wd].wins.filter((_, j) => j !== i) };
    setWeek(n);
  };

  // 單日快速開關（點兩下）：直接寫 open 反轉、full=false
  const toggleDay = async (date: string, nowOpen: boolean) => {
    try {
      await api(`/items/${itemId}/exceptions`, {
        method: "POST",
        body: JSON.stringify({ date, open: !nowOpen, full: false }),
      });
      loadDetail(itemId, month);
    } catch (e) {
      setMsg(String(e));
    }
  };

  const saveExc = async () => {
    if (!day) return;
    try {
      await api(`/items/${itemId}/exceptions`, {
        method: "POST",
        body: JSON.stringify({
          date: day,
          open: excMode !== "closed",
          full: excMode === "full",
          note: excNote || undefined,
        }),
      });
      setMsg(t("provider.daySaved"));
      toast(t("common.saved"));
      setExcNote("");
      loadDetail(itemId, month);
    } catch (e) {
      setMsg(String(e));
    }
  };

  const act = async (aid: number, action: string) => {
    try {
      await api(`/orders/${aid}`, { method: "PATCH", body: JSON.stringify({ action }) });
      loadDetail(itemId, month);
    } catch (e) {
      setMsg(String(e));
    }
  };

  const selDay = detail?.days.find((x) => x.date === day);
  const selOrders = cal.find((x) => x.date === day)?.orders ?? [];

  const shiftMonth = (n: number) => {
    const [y, m] = month.split("-").map(Number);
    const t = new Date(y, m - 1 + n, 1);
    setMonth(`${t.getFullYear()}-${String(t.getMonth() + 1).padStart(2, "0")}`);
  };

  return (
    <div>
      <h2>{t("provider.title")}</h2>
      <div className="form">
        <label>{t("provider.shop")}
          <select value={shopId} onChange={(e) => setShopId(Number(e.target.value))}>
            <option value={0}>{t("provider.pickShop")}</option>
            {shops.map((s) => (
              <option key={s.id} value={s.id}>{s.name}</option>
            ))}
          </select>
        </label>
        {shopId > 0 && (
          <label>{t("provider.item")}
            <select value={itemId} onChange={(e) => setItemId(Number(e.target.value))}>
              <option value={0}>{t("provider.pickItem")}</option>
              {items.map((s) => (
                <option key={s.id} value={s.id}>{s.title}（{nt(s.price_cents)}）</option>
              ))}
            </select>
          </label>
        )}
      </div>

      {shopId > 0 && (
        <div className="form">
          <h3>{t("provider.addTitle")}</h3>
          <input value={title} onChange={(e) => setTitle(e.target.value)} placeholder={t("provider.namePh")} />
          <input value={price} onChange={(e) => setPrice(e.target.value)} placeholder={t("provider.pricePh")} />
          <button onClick={addItem}>{t("provider.add")}</button>
        </div>
      )}

      {detail && (
        <>
          <h3>{t("provider.weekTitle")}</h3>
          <div className="form">
            {week.map((w, wd) => (
              <div key={wd}>
                <label>
                  <input
                    type="checkbox"
                    checked={w.open}
                    onChange={(e) => {
                      const n = [...week];
                      n[wd] = { ...w, open: e.target.checked };
                      setWeek(n);
                    }}
                  />
                  {t("provider.wdOpen", { wd: wdName(wd) })}
                </label>
                {w.wins.map((t_, i) => (
                  <div key={i} className="area-bar">
                    <span>{windowLabel(t_)}</span>
                    <button className="mini" onClick={() => delWin(wd, i)}>{t("provider.del")}</button>
                  </div>
                ))}
                <div className="area-bar">
                  <input
                    value={draft[wd].s}
                    onChange={(e) => {
                      const n = [...draft];
                      n[wd] = { ...n[wd], s: e.target.value };
                      setDraft(n);
                    }}
                    placeholder={t("provider.startPh")}
                  />
                  <span>～</span>
                  <input
                    value={draft[wd].e}
                    onChange={(e) => {
                      const n = [...draft];
                      n[wd] = { ...n[wd], e: e.target.value };
                      setDraft(n);
                    }}
                    placeholder={t("provider.endPh")}
                  />
                  <button className="mini" onClick={() => addWin(wd)}>{t("provider.addSlot")}</button>
                </div>
              </div>
            ))}
            <button onClick={saveWeek}>{t("provider.saveTpl")}</button>
          </div>

          <h3>{t("provider.dayTitle", { m: month })}
            <button className="mini" onClick={() => shiftMonth(-1)}>‹</button>
            <button className="mini" onClick={() => shiftMonth(1)}>›</button>
          </h3>
          <div className="cal">
            {detail.days.map((x) => (
              <button
                key={x.date}
                className={`day${x.date === day ? " sel" : ""}${!x.open ? " off" : ""}${x.full ? " full" : ""}`}
                onClick={() => {
                  setDay(x.date);
                  setExcMode(!x.open ? "closed" : x.full ? "full" : "open");
                }}
                onDoubleClick={() => toggleDay(x.date, x.open)}
                title={t("provider.dblClick")}
              >
                <div>{Number(x.date.slice(8, 10))}</div>
                <div className="wd">{!x.open ? t("provider.closed") : x.full ? t("provider.full") : t("provider.available")}</div>
              </button>
            ))}
          </div>

          {selDay && (
            <div className="order">
              <h3>{day}（{!selDay.open ? t("provider.dayClosed") : selDay.full ? t("provider.dayFull") : t("provider.dayOpen")}）</h3>
              {selOrders.map((a) => (
                <div key={a.id}>
                  #{a.id} · {a.window ?? t("provider.notSpecified")} · {statusLabel(a.status)} · {a.remark ?? ""}
                  <span className="row">
                    {a.status === "pending" && <button onClick={() => act(a.id, "confirm")}>{t("provider.confirm")}</button>}
                    {a.status === "confirmed" && (
                      <>
                        <button onClick={() => act(a.id, "ready")}>{t("orders.ready")}</button>
                        <button onClick={() => act(a.id, "complete")}>{t("provider.complete")}</button>
                        <button onClick={() => act(a.id, "noshow")}>{t("provider.noshow")}</button>
                      </>
                    )}
                    {a.status === "ready" && (
                      <>
                        <button onClick={() => act(a.id, "complete")}>{t("provider.complete")}</button>
                        <button onClick={() => act(a.id, "noshow")}>{t("provider.noshow")}</button>
                      </>
                    )}
                    {["pending", "confirmed", "ready"].includes(a.status) && (
                      <button onClick={() => act(a.id, "cancel")}>{t("provider.cancel")}</button>
                    )}
                  </span>
                </div>
              ))}
              {selOrders.length === 0 && <p className="hint">{t("provider.noOrders")}</p>}
              <div className="buy-box">
                <h4>{t("provider.editDay")}</h4>
                <div className="tabs">
                  <button className={excMode === "open" ? "on" : ""} onClick={() => setExcMode("open")}>{t("provider.normal")}</button>
                  <button className={excMode === "full" ? "on" : ""} onClick={() => setExcMode("full")}>{t("provider.fullDay")}</button>
                  <button className={excMode === "closed" ? "on" : ""} onClick={() => setExcMode("closed")}>{t("provider.closedDay")}</button>
                </div>
                <input value={excNote} onChange={(e) => setExcNote(e.target.value)} placeholder={t("provider.notePh")} />
                <button onClick={saveExc}>{t("provider.saveDay")}</button>
                <Link to={`/i/${detail.id}?month=${month}`}>{t("provider.preview")}</Link>
              </div>
            </div>
          )}
        </>
      )}
      {msg && <p className="error">{msg}</p>}
    </div>
  );
}
