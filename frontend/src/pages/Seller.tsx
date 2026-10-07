import { useEffect, useState } from "react";
import { api, nt, token, type Item } from "../lib/api";
import { useAuth } from "../lib/auth";
import { useLang, CATS } from "../lib/i18n";
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
  const { areas, current } = useArea();
  const { t, catLabel } = useLang();
  const [shops, setShops] = useState<Shop[]>([]);
  const [items, setItems] = useState<Item[]>([]);
  const [shopName, setShopName] = useState("");
  const [kind, setKind] = useState("personal");
  const [address, setAddress] = useState("");
  const [hours, setHours] = useState("");
  const [title, setTitle] = useState("");
  const [price, setPrice] = useState("");
  const [category, setCategory] = useState("other");
  // 開店地區：預設跟著目前地區，可改（避免店掛錯區）
  const [shopCounty, setShopCounty] = useState("");
  const [shopTownId, setShopTownId] = useState("");
  // 庫存空白 = 不限量（賣服務）；可預約打勾才設時間
  const [stock, setStock] = useState("");
  const [bookable, setBookable] = useState(false);
  const [photos, setPhotos] = useState<File[]>([]);
  const [uploading, setUploading] = useState(false);
  const [editHours, setEditHours] = useState("");
  const [shopId, setShopId] = useState(0);
  const [msg, setMsg] = useState("");

  const shop = shops.find((s) => s.id === shopId) ?? shops[0];

  const load = () => {
    api<Shop[]>("/shops").then((ss) => {
      setShops(ss);
      // 留在原本選的店；首次或店沒了才選第一家
      setShopId((prev) => (prev && ss.some((s) => s.id === prev) ? prev : ss[0]?.id ?? 0));
    }).catch(() => {});
  };
  useEffect(load, []); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (!shop) return;
    api<{ items: Item[] }>(`/items?shop_id=${shop.id}`).then((d) => setItems(d.items)).catch(() => {});
    setEditHours(shop.opening_hours ?? "");
  }, [shops, shopId]); // eslint-disable-line react-hooks/exhaustive-deps

  if (!user) return <p>{t("common.loginFirstShop")}</p>;

  const openShop = async () => {
    const areaId = Number(shopTownId) || current?.id;
    if (!shopName.trim()) {
      setMsg(t("seller.needName"));
      return;
    }
    if (!areaId) {
      setMsg(t("seller.needArea"));
      return;
    }
    try {
      await api("/shops", {
        method: "POST",
        body: JSON.stringify({
          name: shopName.trim(),
          area_id: areaId,
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
    if (!shop) return;
    try {
      await api(`/shops/${shop.id}`, {
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

  const uploadPhotos = async (): Promise<string[]> => {
    if (photos.length === 0) return [];
    const base = import.meta.env.VITE_API_URL ?? "/api/v1";
    const fd = new FormData();
    photos.slice(0, 5).forEach((f) => fd.append("files", f));
    const headers: Record<string, string> = {};
    const t = token();
    if (t) headers["Authorization"] = `Bearer ${t}`;
    const res = await fetch(`${base}/uploads`, { method: "POST", headers, body: fd });
    if (!res.ok) throw new Error(await res.text());
    const arr = (await res.json()) as { url: string }[];
    return arr.map((x) => x.url);
  };

  const addItem = async () => {
    if (!shop) {
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
      setUploading(true);
      const urls = await uploadPhotos();
      await api("/items", {
        method: "POST",
        body: JSON.stringify({
          shop_id: shop.id,
          title,
          price_cents: Math.round(Number(price) * 100),
          category,
          stock: stock.trim() === "" ? null : Number(stock),
          unit: "份",
          bookable,
          images: urls,
        }),
      });
      setTitle("");
      setPhotos([]);
      setMsg(bookable ? t("seller.listedGo") : t("seller.listed"));
      const d = await api<{ items: Item[] }>(`/items?shop_id=${shop.id}`);
      setItems(d.items);
    } catch (e) {
      setMsg(String(e));
    } finally {
      setUploading(false);
    }
  };

  const takeOff = async (id: number) => {
    if (!shop) return;
    try {
      await api(`/items/${id}`, { method: "PATCH", body: JSON.stringify({ status: "off" }) });
      const d = await api<{ items: Item[] }>(`/items?shop_id=${shop.id}`);
      setItems(d.items);
    } catch (e) {
      setMsg(String(e));
    }
  };

  return (
    <div>
      <h2>{t("seller.title")}</h2>
      {!shop ? (
        <div className="form">
          <h3>{t("seller.open10")}</h3>
          <input value={shopName} onChange={(e) => setShopName(e.target.value)} placeholder={t("seller.shopNamePh")} />
          <label>
            {t("home.county")}
            <select
              value={shopCounty || current?.county || ""}
              onChange={(e) => { setShopCounty(e.target.value); setShopTownId(""); }}
            >
              {[...new Set(areas.map((a) => a.county))].map((c) => (
                <option key={c} value={c}>{c}</option>
              ))}
            </select>
          </label>
          <label>
            {t("home.town")}
            <select value={shopTownId || String(current?.id ?? "")} onChange={(e) => setShopTownId(e.target.value)}>
              <option value="">{t("seller.pickTown")}</option>
              {areas.filter((a) => a.county === (shopCounty || current?.county)).map((a) => (
                <option key={a.id} value={a.id}>{a.township}</option>
              ))}
            </select>
          </label>
          <select value={kind} onChange={(e) => setKind(e.target.value)}>
            <option value="personal">{t("seller.personal")}</option>
            <option value="store">{t("seller.store")}</option>
          </select>
          {kind === "store" && (
            <input value={address} onChange={(e) => setAddress(e.target.value)} placeholder={t("seller.addressPh")} />
          )}
          <input value={hours} onChange={(e) => setHours(e.target.value)} placeholder={kind === "store" ? t("seller.hoursPhStore") : t("seller.hoursPhMeet")} />
          <button onClick={openShop}>{t("seller.openBtn")}</button>
        </div>
      ) : (
        <>
          <p>{t("seller.myShop")}{shop.name}</p>
          {shops.length > 1 && (
            <label>
              {t("seller.pickShop")}
              <select value={shop.id} onChange={(e) => setShopId(Number(e.target.value))}>
                {shops.map((s) => (
                  <option key={s.id} value={s.id}>{s.name}</option>
                ))}
              </select>
            </label>
          )}
          <div className="form">
            <h3>{t("seller.settings")}</h3>
            <input value={editHours} onChange={(e) => setEditHours(e.target.value)} placeholder={shop.kind === "store" ? t("seller.hoursPhStore") : t("seller.hoursPhMeet")} />
            <button onClick={saveHours}>{t("seller.updateHours")}</button>
          </div>
          <div className="form">
            <h3>{t("seller.listTitle")}</h3>
            <input value={title} onChange={(e) => setTitle(e.target.value)} placeholder={t("seller.namePh")} />
            <input value={price} onChange={(e) => setPrice(e.target.value)} placeholder={t("seller.pricePh")} />
            <label>
              {t("home.category")}
              <select value={category} onChange={(e) => setCategory(e.target.value)}>
                {CATS.map((c) => (
                  <option key={c} value={c}>{catLabel(c)}</option>
                ))}
              </select>
            </label>
            <input value={stock} onChange={(e) => setStock(e.target.value)} placeholder={t("seller.stockPh")} />
            <label>
              {t("seller.photos")}
              <input
                type="file"
                accept="image/*"
                multiple
                onChange={(e) => setPhotos(Array.from(e.target.files ?? []).slice(0, 5))}
              />
            </label>
            {photos.length > 0 && (
              <div className="hint">{t("seller.photosPicked", { n: photos.length })}</div>
            )}
            <label>
              <input type="checkbox" checked={bookable} onChange={(e) => setBookable(e.target.checked)} />
              {t("seller.bookable")}
            </label>
            <button onClick={addItem} disabled={uploading}>{uploading ? t("seller.uploading") : t("seller.listBtn")}</button>
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
