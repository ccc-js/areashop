import { useEffect, useState } from "react";
import { api, nt, type Order, type OrderDetail } from "../lib/api";
import { useAuth } from "../lib/auth";

// 狀態機：pending → confirmed → ready → completed；confirmed 可跳過 ready 直接完成；
// confirmed/ready 可記 noshow（店主）。後端擋權限，前端照狀態給按鈕。
const NEXT: Record<string, { action: string; label: string }[]> = {
  pending: [{ action: "confirm", label: "確認接單" }],
  confirmed: [
    { action: "ready", label: "可面交" },
    { action: "complete", label: "完成" },
    { action: "noshow", label: "記爽約" },
  ],
  ready: [
    { action: "complete", label: "完成" },
    { action: "noshow", label: "記爽約" },
  ],
};

const CANCELABLE = ["pending", "confirmed", "ready"];

export default function Orders() {
  const { user } = useAuth();
  const [role, setRole] = useState<"buyer" | "seller">("buyer");
  const [orders, setOrders] = useState<Order[]>([]);
  // 賣家視角才抓明細（含買家＋品項）
  const [details, setDetails] = useState<Record<number, OrderDetail>>({});
  const [err, setErr] = useState("");

  const load = () =>
    api<Order[]>(`/orders?role=${role}`)
      .then(async (o) => {
        setOrders(o);
        setErr("");
        if (role === "seller") {
          const ds = await Promise.all(
            o.map((x) => api<OrderDetail>(`/orders/${x.id}`).catch(() => null))
          );
          const m: Record<number, OrderDetail> = {};
          ds.forEach((d) => {
            if (d) m[d.id] = d;
          });
          setDetails(m);
        } else {
          setDetails({});
        }
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

  if (!user) return <p>請先登入。</p>;
  return (
    <div>
      <h2>我的訂單</h2>
      <div className="tabs">
        <button className={role === "buyer" ? "on" : ""} onClick={() => setRole("buyer")}>我是買家</button>
        <button className={role === "seller" ? "on" : ""} onClick={() => setRole("seller")}>我是賣家</button>
      </div>
      {err && <p className="error">{err}</p>}
      {orders.map((o) => (
        <div key={o.id} className="order">
          <div>
            #{o.id} · {o.status} · {nt(o.total_cents)}
            {o.date && ` · ${o.date}${o.window ? ` ${o.window}` : ""}`}
          </div>
          {o.remark && <div className="hint">備註：{o.remark}</div>}
          {role === "seller" && details[o.id] && (
            <>
              <div className="hint">
                買家：{details[o.id].buyer.nickname}（{details[o.id].buyer.phone}）
              </div>
              {details[o.id].items.map((it) => (
                <div key={it.id}>
                  {it.title} × {it.qty} · {nt(it.price_cents * it.qty)}
                </div>
              ))}
            </>
          )}
          <div className="row">
            {(NEXT[o.status] ?? []).map((n) => (
              <button key={n.action} onClick={() => act(o.id, n.action)}>{n.label}</button>
            ))}
            {CANCELABLE.includes(o.status) && (
              <button onClick={() => act(o.id, "cancel")}>取消</button>
            )}
          </div>
        </div>
      ))}
      {orders.length === 0 && <p className="empty">還沒有訂單。</p>}
    </div>
  );
}
