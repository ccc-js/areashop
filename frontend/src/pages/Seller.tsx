import { useEffect, useState } from "react";
import { api, type Product } from "../lib/api";
import { useAuth } from "../lib/auth";
import { useArea } from "../lib/area";

interface Shop {
  id: number;
  name: string;
  area_id: number;
}

export default function Seller() {
  const { user } = useAuth();
  const { current } = useArea();
  const [shops, setShops] = useState<Shop[]>([]);
  const [products, setProducts] = useState<Product[]>([]);
  const [shopName, setShopName] = useState("");
  const [kind, setKind] = useState("personal");
  const [address, setAddress] = useState("");
  const [hours, setHours] = useState("");
  const [title, setTitle] = useState("");
  const [price, setPrice] = useState("150");
  const [stock, setStock] = useState("10");
  const [msg, setMsg] = useState("");

  const load = () => {
    api<Shop[]>("/shops").then(setShops).catch(() => {});
  };
  useEffect(load, []);

  useEffect(() => {
    if (shops[0]) {
      api<{ items: Product[] }>(`/products?shop_id=${shops[0].id}`).then((d) => setProducts(d.items)).catch(() => {});
    }
  }, [shops]);

  if (!user) return <p>請先登入再開店。</p>;

  const openShop = async () => {
    try {
      await api("/shops", {
        method: "POST",
        body: JSON.stringify({
          name: shopName,
          area_id: current?.id,
          kind,
          address: address || undefined,
          opening_hours: hours || undefined,
          pickup_mode: kind === "store" ? "store" : "meetup",
        }),
      });
      setShopName("");
      setMsg("開店成功！");
      load();
    } catch (e) {
      setMsg(String(e));
    }
  };

  const addProduct = async () => {
    if (!shops[0]) {
      setMsg("請先開店");
      return;
    }
    try {
      await api("/products", {
        method: "POST",
        body: JSON.stringify({
          shop_id: shops[0].id,
          title,
          category: "other",
          price_cents: Math.round(Number(price) * 100),
          stock: Number(stock),
          unit: "份",
          pickup_places: [{ label: "面交", detail: "" }],
          images: [],
        }),
      });
      setTitle("");
      setMsg("上架成功！");
      const d = await api<{ items: Product[] }>(`/products?shop_id=${shops[0].id}`);
      setProducts(d.items);
    } catch (e) {
      setMsg(String(e));
    }
  };

  return (
    <div>
      <h2>賣家中心</h2>
      {shops.length === 0 ? (
        <div className="form">
          <h3>10 分鐘開店</h3>
          <input value={shopName} onChange={(e) => setShopName(e.target.value)} placeholder="店名，例如：美濃放山雞" />
          <select value={kind} onChange={(e) => setKind(e.target.value)}>
            <option value="personal">個人賣家（約面交）</option>
            <option value="store">有店面</option>
          </select>
          {kind === "store" && (
            <>
              <input value={address} onChange={(e) => setAddress(e.target.value)} placeholder="店址，例如：金城鎮模範街12號" />
              <input value={hours} onChange={(e) => setHours(e.target.value)} placeholder="營業時間，例如：每日 10:00-19:00" />
            </>
          )}
          <button onClick={openShop}>開店（{current?.county}{current?.township}）</button>
        </div>
      ) : (
        <>
          <p>我的店：{shops[0].name}</p>
          <div className="form">
            <h3>上架商品</h3>
            <input value={title} onChange={(e) => setTitle(e.target.value)} placeholder="商品名，例如：土雞蛋 10 顆" />
            <input value={price} onChange={(e) => setPrice(e.target.value)} placeholder="價格（元）" />
            <input value={stock} onChange={(e) => setStock(e.target.value)} placeholder="庫存" />
            <button onClick={addProduct}>上架</button>
          </div>
          <h3>我的商品（{products.length}）</h3>
          {products.map((p) => (
            <div key={p.id} className="order">{p.title} · NT${p.price_cents / 100} · 剩 {p.stock}</div>
          ))}
        </>
      )}
      {msg && <p className="error">{msg}</p>}
    </div>
  );
}
