import { useEffect, useState } from "react";
import { api, type Item } from "../lib/api";
import { useAuth } from "../lib/auth";
import { toast } from "../lib/toast";
import { useArea } from "../lib/area";

interface Shop {
  id: number;
  name: string;
  area_id: number;
  kind: string;
  opening_hours: string | null;
}

export default function Seller() {
  const { user } = useAuth();
  const { current } = useArea();
  const [shops, setShops] = useState<Shop[]>([]);
  const [items, setItems] = useState<Item[]>([]);
  const [shopName, setShopName] = useState("");
  const [kind, setKind] = useState("personal");
  const [address, setAddress] = useState("");
  const [hours, setHours] = useState("");
  const [title, setTitle] = useState("");
  const [price, setPrice] = useState("");
  // 庫存空白 = 不限量（賣服務）；可預約打勾才設時間
  const [stock, setStock] = useState("");
  const [bookable, setBookable] = useState(false);
  const [editHours, setEditHours] = useState("");
  const [msg, setMsg] = useState("");

  const load = () => {
    api<Shop[]>("/shops").then((ss) => {
      setShops(ss);
      // 沒動過才帶入店家現值；正在輸入或剛存檔的不覆蓋
      setEditHours((prev) => (prev === "" && ss[0] ? ss[0].opening_hours ?? "" : prev));
    }).catch(() => {});
  };
  useEffect(load, []); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (shops[0]) {
      api<{ items: Item[] }>(`/items?shop_id=${shops[0].id}`).then((d) => setItems(d.items)).catch(() => {});
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
        }),
      });
      setShopName("");
      setMsg("開店成功！");
      load();
    } catch (e) {
      setMsg(String(e));
    }
  };

  const saveHours = async () => {
    if (!shops[0]) return;
    try {
      await api(`/shops/${shops[0].id}`, {
        method: "PATCH",
        body: JSON.stringify({ opening_hours: editHours }),
      });
      setMsg("營業時間已更新，買家在項目頁看得到。");
      toast("已儲存");
      load();
    } catch (e) {
      setMsg(String(e));
    }
  };

  const addItem = async () => {
    if (!shops[0]) {
      setMsg("請先開店");
      return;
    }
    if (!title.trim() || !(Number(price) > 0)) {
      setMsg("請填名稱、價格（元，須大於 0）");
      return;
    }
    if (stock.trim() !== "" && !(Number(stock) >= 0)) {
      setMsg("庫存請填 0 以上的數字，空白 = 不限量");
      return;
    }
    if (stock.trim() === "" && !bookable) {
      setMsg("不限量又不接預約就沒東西可賣了，請二選一");
      return;
    }
    try {
      await api("/items", {
        method: "POST",
        body: JSON.stringify({
          shop_id: shops[0].id,
          title,
          price_cents: Math.round(Number(price) * 100),
          stock: stock.trim() === "" ? null : Number(stock),
          unit: "份",
          bookable,
        }),
      });
      setTitle("");
      setMsg(bookable ? "上架成功！去「接案管理」設可接案時間。" : "上架成功！");
      const d = await api<{ items: Item[] }>(`/items?shop_id=${shops[0].id}`);
      setItems(d.items);
    } catch (e) {
      setMsg(String(e));
    }
  };

  const takeOff = async (id: number) => {
    try {
      await api(`/items/${id}`, { method: "PATCH", body: JSON.stringify({ status: "off" }) });
      const d = await api<{ items: Item[] }>(`/items?shop_id=${shops[0].id}`);
      setItems(d.items);
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
            <input value={address} onChange={(e) => setAddress(e.target.value)} placeholder="店址，例如：金城鎮模範街12號" />
          )}
          <input value={hours} onChange={(e) => setHours(e.target.value)} placeholder={kind === "store" ? "營業時間，例如：每日 10:00-19:00" : "面交時間，例如：週六 09:00-11:00"} />
          <button onClick={openShop}>開店（{current?.county}{current?.township}）</button>
        </div>
      ) : (
        <>
          <p>我的店：{shops[0].name}</p>
          <div className="form">
            <h3>店鋪設定</h3>
            <input value={editHours} onChange={(e) => setEditHours(e.target.value)} placeholder={shops[0].kind === "store" ? "營業時間，例如：每日 10:00-19:00" : "面交時間，例如：週六 09:00-11:00"} />
            <button onClick={saveHours}>更新營業時間</button>
          </div>
          <div className="form">
            <h3>上架（賣東西和賣服務同一種）</h3>
            <input value={title} onChange={(e) => setTitle(e.target.value)} placeholder="名稱，例如：土雞蛋 10 顆 / 男士快剪" />
            <input value={price} onChange={(e) => setPrice(e.target.value)} placeholder="價格（元）" />
            <input value={stock} onChange={(e) => setStock(e.target.value)} placeholder="庫存（空白 = 不限量）" />
            <label>
              <input type="checkbox" checked={bookable} onChange={(e) => setBookable(e.target.checked)} />
              接受選日期預約（設好後去「接案管理」排時間）
            </label>
            <button onClick={addItem}>上架</button>
          </div>
          <h3>我的項目（{items.length}）</h3>
          {items.map((p) => (
            <div key={p.id} className="order">
              {p.title} · NT${p.price_cents / 100}{p.stock != null && ` · 剩 ${p.stock}`} · {p.bookable ? "可預約" : "直接買"}
              <span className="row">
                <button onClick={() => takeOff(p.id)}>下架</button>
              </span>
            </div>
          ))}
        </>
      )}
      {msg && <p className="error">{msg}</p>}
    </div>
  );
}
