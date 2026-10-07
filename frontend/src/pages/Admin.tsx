import { useEffect, useState } from "react";
import { api, nt } from "../lib/api";
import { useAuth } from "../lib/auth";
import { useLang } from "../lib/i18n";
import { toast } from "../lib/toast";

interface Area {
  id: number;
  county: string;
  township: string;
  village: string | null;
}

interface ShopRow {
  id: number;
  name: string;
  status: string;
  owner_nickname: string;
  owner_phone: string;
}

interface ItemRow {
  id: number;
  title: string;
  status: string;
  shop_name: string;
  price_cents: number;
}

interface UserRow {
  id: number;
  phone: string;
  nickname: string;
  role: string;
}

interface OrderRow {
  id: number;
  status: string;
  total_cents: number;
  date: string | null;
  buyer_nickname: string;
  shop_name: string;
}

type Tab = "areas" | "shops" | "items" | "users" | "orders";

// /admin：管理員介面（地區 CRUD＋店家停權＋項目下架＋使用者/訂單一覽）
export default function Admin() {
  const { user } = useAuth();
  const { t, statusLabel } = useLang();
  const [tab, setTab] = useState<Tab>("areas");
  const [areas, setAreas] = useState<Area[]>([]);
  const [shops, setShops] = useState<ShopRow[]>([]);
  const [items, setItems] = useState<ItemRow[]>([]);
  const [users, setUsers] = useState<UserRow[]>([]);
  const [orders, setOrders] = useState<OrderRow[]>([]);
  const [county, setCounty] = useState("");
  const [township, setTownship] = useState("");
  const [editing, setEditing] = useState<Area | null>(null);
  const [msg, setMsg] = useState("");

  const load = () => {
    api<Area[]>("/areas").then(setAreas).catch((e) => setMsg(String(e)));
    api<ShopRow[]>("/admin/shops").then(setShops).catch((e) => setMsg(String(e)));
    api<ItemRow[]>("/admin/items").then(setItems).catch((e) => setMsg(String(e)));
    api<UserRow[]>("/admin/users").then(setUsers).catch((e) => setMsg(String(e)));
    api<OrderRow[]>("/admin/orders").then(setOrders).catch((e) => setMsg(String(e)));
  };
  useEffect(load, []); // eslint-disable-line react-hooks/exhaustive-deps

  if (!user) return <p>{t("common.loginFirst")}</p>;
  if (user.role !== "admin") return <p>{t("admin.forbidden")}</p>;

  const addArea = async () => {
    try {
      await api("/admin/areas", {
        method: "POST",
        body: JSON.stringify({ county, township }),
      });
      setCounty("");
      setTownship("");
      toast(t("common.saved"));
      load();
    } catch (e) {
      setMsg(String(e));
    }
  };

  const saveArea = async () => {
    if (!editing) return;
    try {
      await api(`/admin/areas/${editing.id}`, {
        method: "PUT",
        body: JSON.stringify({ county: editing.county, township: editing.township }),
      });
      setEditing(null);
      toast(t("common.saved"));
      load();
    } catch (e) {
      setMsg(String(e));
    }
  };

  const delArea = async (id: number) => {
    try {
      await api(`/admin/areas/${id}`, { method: "DELETE" });
      toast(t("common.saved"));
      load();
    } catch (e) {
      const s = String(e);
      setMsg(s.includes("area in use") ? t("admin.inUse") : s);
    }
  };

  const flipShop = async (id: number, to: string) => {
    try {
      await api(`/shops/${id}`, { method: "PATCH", body: JSON.stringify({ status: to }) });
      load();
    } catch (e) {
      setMsg(String(e));
    }
  };

  const flipItem = async (id: number, to: string) => {
    try {
      await api(`/items/${id}`, { method: "PATCH", body: JSON.stringify({ status: to }) });
      load();
    } catch (e) {
      setMsg(String(e));
    }
  };

  const tabs: { id: Tab; label: string }[] = [
    { id: "areas", label: t("admin.tabAreas") },
    { id: "shops", label: t("admin.tabShops") },
    { id: "items", label: t("admin.tabItems") },
    { id: "users", label: t("admin.tabUsers") },
    { id: "orders", label: t("admin.tabOrders") },
  ];

  return (
    <div>
      <h2>{t("admin.title")}</h2>
      <div className="tabs">
        {tabs.map((x) => (
          <button key={x.id} className={tab === x.id ? "on" : ""} onClick={() => setTab(x.id)}>
            {x.label}
          </button>
        ))}
      </div>
      {msg && <p className="error">{msg}</p>}

      {tab === "areas" && (
        <>
          <div className="form">
            <input value={county} onChange={(e) => setCounty(e.target.value)} placeholder={t("admin.county")} />
            <input value={township} onChange={(e) => setTownship(e.target.value)} placeholder={t("admin.township")} />
            <button onClick={addArea}>{t("admin.add")}</button>
          </div>
          {editing && (
            <div className="form">
              <input value={editing.county} onChange={(e) => setEditing({ ...editing, county: e.target.value })} />
              <input value={editing.township} onChange={(e) => setEditing({ ...editing, township: e.target.value })} />
              <div className="row">
                <button onClick={saveArea}>{t("admin.save")}</button>
                <button onClick={() => setEditing(null)}>{t("admin.cancel")}</button>
              </div>
            </div>
          )}
          {areas.map((a) => (
            <div key={a.id} className="order">
              {a.county} {a.township}{a.village ? ` ${a.village}` : ""}
              <span className="row">
                <button onClick={() => setEditing(a)}>{t("admin.edit")}</button>
                <button onClick={() => delArea(a.id)}>{t("admin.del")}</button>
              </span>
            </div>
          ))}
        </>
      )}

      {tab === "shops" && (
        <>
          {shops.map((s) => (
            <div key={s.id} className="order">
              <div>{s.name} · {s.status}</div>
              <div className="hint">{t("admin.owner")}：{s.owner_nickname}（{s.owner_phone}）</div>
              <span className="row">
                {s.status === "open" ? (
                  <button onClick={() => flipShop(s.id, "closed")}>{t("admin.suspend")}</button>
                ) : (
                  <button onClick={() => flipShop(s.id, "open")}>{t("admin.reopen")}</button>
                )}
              </span>
            </div>
          ))}
          {shops.length === 0 && <p className="empty">-</p>}
        </>
      )}

      {tab === "items" && (
        <>
          {items.map((it) => (
            <div key={it.id} className="order">
              <div>{it.title} · {it.status} · {nt(it.price_cents)}</div>
              <div className="hint">{t("admin.shop")}：{it.shop_name}</div>
              <span className="row">
                {it.status === "on" ? (
                  <button onClick={() => flipItem(it.id, "off")}>{t("admin.takeOff")}</button>
                ) : (
                  <button onClick={() => flipItem(it.id, "on")}>{t("admin.putOn")}</button>
                )}
              </span>
            </div>
          ))}
          {items.length === 0 && <p className="empty">-</p>}
        </>
      )}

      {tab === "users" && (
        <>
          {users.map((u) => (
            <div key={u.id} className="order">
              {u.nickname}（{u.phone}）· {u.role}
            </div>
          ))}
        </>
      )}

      {tab === "orders" && (
        <>
          {orders.map((o) => (
            <div key={o.id} className="order">
              <div>#{o.id} · {statusLabel(o.status)} · {nt(o.total_cents)}{o.date && ` · ${o.date}`}</div>
              <div className="hint">{t("admin.buyer")}{o.buyer_nickname} · {t("admin.shop")}：{o.shop_name}</div>
            </div>
          ))}
          {orders.length === 0 && <p className="empty">-</p>}
        </>
      )}
    </div>
  );
}
