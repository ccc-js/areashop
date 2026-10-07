import { useEffect, useState } from "react";
import { api, nt, type Item } from "../lib/api";
import { useAuth } from "../lib/auth";
import { useLang } from "../lib/i18n";
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
  const { t } = useLang();
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

  if (!user) return <p>{t("common.loginFirstShop")}</p>;

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
      setMsg(t("seller.opened"));
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
      setMsg(t("seller.hoursUpdated"));
      toast(t("common.saved"));
      load();
    } catch (e) {
      setMsg(String(e));
    }
  };

  const addItem = async () => {
    if (!shops[0]) {
      setMsg(t("seller.openFirst"));
      return;
    }
    if (!title.trim() || !(Number(price) > 0)) {
      setMsg(t("seller.needNamePrice"));
      return;
    }
    if (stock.trim() !== "" && !(Number(stock) >= 0)) {
      setMsg(t("seller.badStock"));
      return;
    }
    if (stock.trim() === "" && !bookable) {
      setMsg(t("seller.needEither"));
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
      setMsg(bookable ? t("seller.listedGo") : t("seller.listed"));
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
      <h2>{t("seller.title")}</h2>
      {shops.length === 0 ? (
        <div className="form">
          <h3>{t("seller.open10")}</h3>
          <input value={shopName} onChange={(e) => setShopName(e.target.value)} placeholder={t("seller.shopNamePh")} />
          <select value={kind} onChange={(e) => setKind(e.target.value)}>
            <option value="personal">{t("seller.personal")}</option>
            <option value="store">{t("seller.store")}</option>
          </select>
          {kind === "store" && (
            <input value={address} onChange={(e) => setAddress(e.target.value)} placeholder={t("seller.addressPh")} />
          )}
          <input value={hours} onChange={(e) => setHours(e.target.value)} placeholder={kind === "store" ? t("seller.hoursPhStore") : t("seller.hoursPhMeet")} />
          <button onClick={openShop}>{t("seller.openBtn", { area: `${current?.county ?? ""}${current?.township ?? ""}` })}</button>
        </div>
      ) : (
        <>
          <p>{t("seller.myShop")}{shops[0].name}</p>
          <div className="form">
            <h3>{t("seller.settings")}</h3>
            <input value={editHours} onChange={(e) => setEditHours(e.target.value)} placeholder={shops[0].kind === "store" ? t("seller.hoursPhStore") : t("seller.hoursPhMeet")} />
            <button onClick={saveHours}>{t("seller.updateHours")}</button>
          </div>
          <div className="form">
            <h3>{t("seller.listTitle")}</h3>
            <input value={title} onChange={(e) => setTitle(e.target.value)} placeholder={t("seller.namePh")} />
            <input value={price} onChange={(e) => setPrice(e.target.value)} placeholder={t("seller.pricePh")} />
            <input value={stock} onChange={(e) => setStock(e.target.value)} placeholder={t("seller.stockPh")} />
            <label>
              <input type="checkbox" checked={bookable} onChange={(e) => setBookable(e.target.checked)} />
              {t("seller.bookable")}
            </label>
            <button onClick={addItem}>{t("seller.listBtn")}</button>
          </div>
          <h3>{t("seller.myItems", { n: items.length })}</h3>
          {items.map((p) => (
            <div key={p.id} className="order">
              {p.title} · {nt(p.price_cents)}{p.stock != null && ` · ${t("home.left", { n: p.stock })}`} · {p.bookable ? t("seller.bookableTag") : t("seller.directTag")}
              <span className="row">
                <button onClick={() => takeOff(p.id)}>{t("seller.takeOff")}</button>
              </span>
            </div>
          ))}
        </>
      )}
      {msg && <p className="error">{msg}</p>}
    </div>
  );
}
