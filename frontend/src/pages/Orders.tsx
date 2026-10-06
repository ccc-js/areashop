import { useEffect, useState } from "react";
import { api, nt, type Order } from "../lib/api";
import { useAuth } from "../lib/auth";

const NEXT: Record<string, { action: string; label: string; by: string }[]> = {
  pending: [{ action: "confirm", label: "確認接單", by: "seller" }],
  confirmed: [{ action: "ready", label: "可面交", by: "seller" }],
  ready: [{ action: "complete", label: "完成", by: "any" }],
};

export default function Orders() {
  const { user } = useAuth();
  const [role, setRole] = useState<"buyer" | "seller">("buyer");
  const [orders, setOrders] = useState<Order[]>([]);
  const [err, setErr] = useState("");

  const load = () =>
    api<Order[]>(`/orders?role=${role}`)
      .then((o) => {
        setOrders(o);
        setErr("");
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
          <div>#{o.id} · {o.status} · {nt(o.total_cents)} · 面交：{o.pickup_place}</div>
          <div className="row">
            {(NEXT[o.status] ?? []).map((n) => (
              <button key={n.action} onClick={() => act(o.id, n.action)}>{n.label}</button>
            ))}
            {["pending", "confirmed", "ready"].includes(o.status) && (
              <button onClick={() => act(o.id, "cancel")}>取消</button>
            )}
          </div>
        </div>
      ))}
      {orders.length === 0 && <p className="empty">還沒有訂單。</p>}
    </div>
  );
}
