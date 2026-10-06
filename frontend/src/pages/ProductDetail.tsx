import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { api, nt, PICKUP_LABEL, type Product, type Shop } from "../lib/api";
import { useAuth } from "../lib/auth";

export default function ProductDetail() {
  const { id } = useParams();
  const nav = useNavigate();
  const { user } = useAuth();
  const [p, setP] = useState<Product | null>(null);
  const [shop, setShop] = useState<Shop | null>(null);
  const [qty, setQty] = useState(1);
  const [place, setPlace] = useState("");
  const [msg, setMsg] = useState("");

  useEffect(() => {
    api<Product>(`/products/${id}`)
      .then((prod) => {
        setP(prod);
        return api<Shop>(`/shops/${prod.shop_id}`);
      })
      .then(setShop)
      .catch((e) => setMsg(String(e)));
  }, [id]);

  const order = async () => {
    if (!user) {
      nav("/login");
      return;
    }
    if (!place) {
      setMsg("請填寫面交地點");
      return;
    }
    try {
      const o = await api<{ id: number }>(`/orders`, {
        method: "POST",
        body: JSON.stringify({ items: [{ product_id: Number(id), qty }], pickup_place: place }),
      });
      nav(`/orders?hl=${o.id}`);
    } catch (e) {
      setMsg(String(e));
    }
  };

  if (!p) return <p>{msg || "載入中…"}</p>;
  return (
    <div className="detail">
      <button className="back" onClick={() => nav(-1)}>← 返回</button>
      <h2>{p.title}</h2>
      <p className="price">{nt(p.price_cents)} / {p.unit}</p>
      <p>庫存：{p.stock}</p>
      {shop && (
        <div className="order">
          <div>賣家：{shop.name} · {PICKUP_LABEL[shop.pickup_mode] ?? shop.pickup_mode}</div>
          {shop.address && <div>店址：{shop.address}</div>}
          {shop.opening_hours && <div>營業：{shop.opening_hours}</div>}
          {!shop.address && <div className="hint">無店面，下單時與賣家約面交地點</div>}
        </div>
      )}
      <div className="buy-box">
        <label>數量 <input type="number" min={1} max={p.stock} value={qty} onChange={(e) => setQty(Number(e.target.value))} /></label>
        <label>面交地點 <input value={place} onChange={(e) => setPlace(e.target.value)} placeholder="例如：區公所前，週六 09:00" /></label>
        <button onClick={order} disabled={p.stock <= 0}>下單（面交付款）</button>
      </div>
      {msg && <p className="error">{msg}</p>}
    </div>
  );
}
