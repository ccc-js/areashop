import { useEffect, useState } from "react";
import { api, nt, type Order, type OrderDetail } from "../lib/api";
import { useAuth } from "../lib/auth";
import { useLang } from "../lib/i18n";

export default function Orders() {
  const { user } = useAuth();
  const { t, statusLabel } = useLang();
  const [role, setRole] = useState<"buyer" | "seller">("buyer");
  const [orders, setOrders] = useState<Order[]>([]);
  // 賣家視角才抓明細（含買家＋品項）
  const [details, setDetails] = useState<Record<number, OrderDetail>>({});
  const [err, setErr] = useState("");

  const NEXT: Record<string, { action: string; key: "orders.confirm" | "orders.ready" | "orders.complete" | "orders.noshow" }[]> = {
    pending: [{ action: "confirm", key: "orders.confirm" }],
    confirmed: [
      { action: "ready", key: "orders.ready" },
      { action: "complete", key: "orders.complete" },
      { action: "noshow", key: "orders.noshow" },
    ],
    ready: [
      { action: "complete", key: "orders.complete" },
      { action: "noshow", key: "orders.noshow" },
    ],
  };

  const CANCELABLE = ["pending", "confirmed", "ready"];

  const load = () =>
    api<Order[]>(`/orders?role=${role}`)
      .then(async (o) => {
        setOrders(o);
        setErr("");
        // 買賣雙方都抓明細：買家看品項，賣家多看買家是誰
        const ds = await Promise.all(
          o.map((x) => api<OrderDetail>(`/orders/${x.id}`).catch(() => null))
        );
        const m: Record<number, OrderDetail> = {};
        ds.forEach((d) => {
          if (d) m[d.id] = d;
        });
        setDetails(m);
      })
      .catch((e) => setErr(String(e)));

  useEffect(() => {
    load();
  }, [role]); // eslint-disable-line react-hooks/exhaustive-deps

  const act = async (id: number, action: string) => {
    try {
      await api(`/orders/${id}`, { method: "PATCH", body: JSON.stringify({ action }) });
      load();
    } catch (e) {
      setErr(String(e));
    }
  };

  if (!user) return <p>{t("common.loginFirst")}</p>;
  return (
    <div>
      <h2>{t("orders.title")}</h2>
      <div className="tabs">
        <button className={role === "buyer" ? "on" : ""} onClick={() => setRole("buyer")}>{t("orders.buyer")}</button>
        <button className={role === "seller" ? "on" : ""} onClick={() => setRole("seller")}>{t("orders.seller")}</button>
      </div>
      {err && <p className="error">{err}</p>}
      {orders.map((o) => (
        <div key={o.id} className="order">
          <div>
            #{o.id} · {statusLabel(o.status)} · {nt(o.total_cents)}
            {o.date && ` · ${o.date}${o.window ? ` ${o.window}` : ""}`}
          </div>
          {o.remark && <div className="hint">{t("orders.remark")}{o.remark}</div>}
          {details[o.id] && (
            <>
              {role === "seller" && (
                <div className="hint">
                  {t("orders.buyerIs")}{details[o.id].buyer.nickname}（{details[o.id].buyer.phone}）
                </div>
              )}
              {details[o.id].items.map((it) => (
                <div key={it.id}>
                  {it.title} × {it.qty} · {nt(it.price_cents * it.qty)}
                </div>
              ))}
            </>
          )}
          <div className="row">
            {(NEXT[o.status] ?? []).map((n) => (
              <button key={n.action} onClick={() => act(o.id, n.action)}>{t(n.key)}</button>
            ))}
            {CANCELABLE.includes(o.status) && (
              <button onClick={() => act(o.id, "cancel")}>{t("orders.cancel")}</button>
            )}
          </div>
        </div>
      ))}
      {orders.length === 0 && <p className="empty">{t("orders.empty")}</p>}
    </div>
  );
}
