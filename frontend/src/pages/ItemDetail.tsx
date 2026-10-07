import { useEffect, useState } from "react";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import {
  api,
  nt,
  todayStr,
  windowLabel,
  type ItemDetail as ID,
  type Shop,
} from "../lib/api";
import { useAuth } from "../lib/auth";
import { useLang } from "../lib/i18n";

// /i/:id：統一項目詳情。賣東西（選數量）和賣服務（選日期）同一頁、同一種下單。
export default function ItemDetail() {
  const { id } = useParams();
  const nav = useNavigate();
  const { user } = useAuth();
  const { t, wdName } = useLang();
  const [sp, setSp] = useSearchParams();
  const [d, setD] = useState<ID | null>(null);
  const [shop, setShop] = useState<Shop | null>(null);
  const [qty, setQty] = useState(1);
  const [note, setNote] = useState("");
  const [msg, setMsg] = useState("");
  const month = sp.get("month") ?? todayStr().slice(0, 7);
  const [date, setDate] = useState("");
  const [window, setWindow] = useState("");

  useEffect(() => {
    api<ID>(`/items/${id}?month=${month}`)
      .then((v) => {
        setD(v);
        setMsg("");
        return api<Shop>(`/shops/${v.shop_id}`);
      })
      .then(setShop)
      .catch((e) => setMsg(String(e)));
  }, [id, month]);

  if (!d) return <p>{msg || t("common.loading")}</p>;
  const sel = d.days.find((x) => x.date === date);
  const canBuy = d.stock == null || d.stock > 0;
  const dateOk = !d.bookable || (sel && sel.open && !sel.full && date >= todayStr());
  const shift = (n: number) => {
    const [y, m] = month.split("-").map(Number);
    const t = new Date(y, m - 1 + n, 1);
    setSp({ month: `${t.getFullYear()}-${String(t.getMonth() + 1).padStart(2, "0")}` });
  };

  const order = async () => {
    if (!user) {
      nav("/login");
      return;
    }
    if (d.bookable && !date) {
      setMsg(t("item.needDate"));
      return;
    }
    try {
      const o = await api<{ id: number }>(`/orders`, {
        method: "POST",
        body: JSON.stringify({
          items: [{ item_id: Number(id), qty }],
          date: d.bookable ? date : undefined,
          window: d.bookable && window ? window : undefined,
          remark: note || undefined,
        }),
      });
      nav(`/orders?hl=${o.id}`);
    } catch (e) {
      setMsg(String(e));
    }
  };

  return (
    <div className="detail">
      <button className="back" onClick={() => nav(-1)}>{t("item.back")}</button>
      <h2>{d.title}</h2>
      <p className="price">{nt(d.price_cents)} / {d.unit}</p>
      {d.description && <p>{d.description}</p>}
      {d.notice && <p className="hint">※ {d.notice}</p>}
      {d.stock != null && <p>{t("item.stock")}{d.stock}</p>}
      {d.bookable && <p className="hint">{t("item.freeCancel", { n: d.cancel_hours })}</p>}
      {shop && (
        <div className="order">
          <div>{t("item.seller")}{shop.name}</div>
          {shop.address && <div>{t("item.address")}{shop.address}</div>}
          {shop.opening_hours && <div>{t("item.hours")}{shop.opening_hours}</div>}
          {!shop.address && <div className="hint">{t("item.noStorefront")}</div>}
        </div>
      )}

      {d.bookable && (
        <>
          <h3>
            {t("item.pickDate", { m: month })}
            <button className="mini" onClick={() => shift(-1)}>‹</button>
            <button className="mini" onClick={() => shift(1)}>›</button>
          </h3>
          <div className="cal">
            {d.days.map((x) => {
              const wd = new Date(Number(x.date.slice(0, 4)), Number(x.date.slice(5, 7)) - 1, Number(x.date.slice(8, 10))).getDay();
              const off = !x.open || x.full || x.date < todayStr();
              return (
                <button
                  key={x.date}
                  disabled={off}
                  className={`day${x.date === date ? " sel" : ""}${!x.open ? " off" : ""}${x.full ? " full" : ""}`}
                  onClick={() => {
                    setDate(x.date);
                    setWindow("");
                  }}
                >
                  <div>{Number(x.date.slice(8, 10))}</div>
                  <div className="wd">{t("item.wd", { wd: wdName(wd) })}</div>
                  <div className="wd">{!x.open ? t("item.closed") : x.full ? t("item.full") : t("item.available")}</div>
                </button>
              );
            })}
          </div>
          {sel && sel.open && !sel.full && sel.windows.length > 0 && (
            <label>
              {t("item.slot")}
              <select value={window} onChange={(e) => setWindow(e.target.value)}>
                <option value="">{t("item.anytime")}</option>
                {sel.windows.map((w) => {
                  const label = windowLabel(w);
                  return <option key={label} value={label}>{label}</option>;
                })}
              </select>
            </label>
          )}
        </>
      )}

      <div className="buy-box">
        <label>{t("item.qty")} <input type="number" min={1} max={d.stock ?? undefined} value={qty} onChange={(e) => setQty(Number(e.target.value))} /></label>
        {d.bookable && (
          <label>{t("item.note")} <input value={note} onChange={(e) => setNote(e.target.value)} placeholder={t("item.notePh")} /></label>
        )}
        <button onClick={order} disabled={!canBuy || !dateOk}>
          {d.bookable ? t("item.book") : t("item.order")}{t("item.payMeetup")}
        </button>
      </div>
      {msg && <p className="error">{msg}</p>}
    </div>
  );
}
