import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { api, nt, type Item, type Shop } from "../lib/api";
import { useLang } from "../lib/i18n";

// /shop/:id：店鋪頁（店資訊＋全部上架中項目；後端免動，全是現成 API）
export default function ShopPage() {
  const { id } = useParams();
  const { t } = useLang();
  const [shop, setShop] = useState<Shop | null>(null);
  const [items, setItems] = useState<Item[]>([]);
  const [err, setErr] = useState("");

  useEffect(() => {
    api<Shop>(`/shops/${id}`)
      .then((s) => {
        setShop(s);
        return api<{ items: Item[] }>(`/items?shop_id=${id}`);
      })
      .then((d) => {
        setItems(d.items);
        setErr("");
      })
      .catch((e) => setErr(String(e)));
  }, [id]);

  if (!shop) return <p>{err || t("common.loading")}</p>;
  return (
    <div>
      <h2>{shop.name}</h2>
      {shop.description && <p>{shop.description}</p>}
      {shop.address && <div>{t("item.address")}{shop.address}</div>}
      {shop.opening_hours && <div>{t("item.hours")}{shop.opening_hours}</div>}
      {!shop.address && <div className="hint">{t("item.noStorefront")}</div>}
      <h3>{t("shop.items", { n: items.length })}</h3>
      <div className="grid">
        {items.map((p) => (
          <Link key={p.id} className="card" to={`/i/${p.id}`}>
            <div className="card-title">{p.title}</div>
            <div className="card-meta">
              {nt(p.price_cents)}/{p.unit}
              {p.bookable && ` · ${t("home.bookable")}`}
            </div>
            <div className="card-stock">
              {p.stock != null && (p.stock > 0 ? t("home.left", { n: p.stock }) : t("home.soldout"))}
            </div>
          </Link>
        ))}
      </div>
      {items.length === 0 && <p className="empty">{t("shop.empty")}</p>}
    </div>
  );
}
