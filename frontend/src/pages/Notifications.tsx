import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api } from "../lib/api";
import { useAuth } from "../lib/auth";
import { useLang } from "../lib/i18n";

interface Notif {
  id: number;
  kind: string;
  order_id: number;
  actor: string;
  read: boolean;
  created_at: string;
}

// /notifications：通知列表＋全部已讀
export default function Notifications() {
  const { user } = useAuth();
  const { t } = useLang();
  const [list, setList] = useState<Notif[]>([]);
  const [err, setErr] = useState("");

  const load = () =>
    api<Notif[]>("/notifications")
      .then((d) => {
        setList(d);
        setErr("");
      })
      .catch((e) => setErr(String(e)));

  useEffect(() => {
    load();
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  if (!user) return <p>{t("common.loginFirst")}</p>;

  const text = (n: Notif) => {
    const o = `#${n.order_id}`;
    switch (n.kind) {
      case "order_created":
        return t("notif.created", { actor: n.actor, order: o });
      case "confirmed":
        return t("notif.confirmed", { actor: n.actor, order: o });
      case "ready":
        return t("notif.ready", { order: o });
      case "completed":
        return t("notif.completed", { order: o });
      case "cancelled":
        return t("notif.cancelled", { actor: n.actor, order: o });
      case "noshow":
        return t("notif.noshow", { order: o });
      default:
        return `${n.kind} ${o}`;
    }
  };

  const readAll = async () => {
    try {
      await api("/notifications/read", { method: "POST", body: JSON.stringify({ ids: [] }) });
      load();
    } catch (e) {
      setErr(String(e));
    }
  };

  return (
    <div>
      <h2>{t("notif.title")}</h2>
      {list.some((x) => !x.read) && <button onClick={readAll}>{t("notif.readAll")}</button>}
      {err && <p className="error">{err}</p>}
      {list.map((n) => (
        <div key={n.id} className="order" style={n.read ? { opacity: 0.6 } : undefined}>
          <Link to="/orders">{text(n)}</Link>
        </div>
      ))}
      {list.length === 0 && <p className="empty">{t("notif.empty")}</p>}
    </div>
  );
}
