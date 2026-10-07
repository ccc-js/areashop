import { useEffect, useState } from "react";
import { Link, Route, Routes } from "react-router-dom";
import { AuthProvider, useAuth } from "./lib/auth";
import { AreaProvider } from "./lib/area";
import { LangProvider, useLang } from "./lib/i18n";
import Home from "./pages/Home";
import ItemDetail from "./pages/ItemDetail";
import Orders from "./pages/Orders";
import Seller from "./pages/Seller";
import Provider from "./pages/Provider";
import Admin from "./pages/Admin";
import { Login, Register } from "./pages/Auth";

function Nav() {
  const { user, logout } = useAuth();
  const { lang, setLang, t } = useLang();
  return (
    <nav className="nav">
      <Link to="/" className="logo">{t("nav.logo")}</Link>
      <Link to="/seller">{t("nav.seller")}</Link>
      <Link to="/orders">{t("nav.orders")}</Link>
      <Link to="/provider">{t("nav.provider")}</Link>
      {user?.role === "admin" && <Link to="/admin">{t("nav.admin")}</Link>}
      <select
        aria-label={t("nav.lang")}
        value={lang}
        onChange={(e) => setLang(e.target.value as "zh" | "cn" | "en")}
      >
        <option value="zh">{t("nav.zh")}</option>
        <option value="cn">{t("nav.cn")}</option>
        <option value="en">{t("nav.en")}</option>
      </select>
      {user ? (
        <>
          <span>{user.nickname}</span>
          <button onClick={logout}>{t("nav.logout")}</button>
        </>
      ) : (
        <>
          <Link to="/login">{t("nav.login")}</Link>
          <Link to="/register">{t("nav.register")}</Link>
        </>
      )}
    </nav>
  );
}

function ToastHost() {
  const [msg, setMsg] = useState("");
  useEffect(() => {
    let t: ReturnType<typeof setTimeout>;
    const h = (e: Event) => {
      setMsg((e as CustomEvent<string>).detail);
      clearTimeout(t);
      t = setTimeout(() => setMsg(""), 2000);
    };
    window.addEventListener("app-toast", h);
    return () => {
      window.removeEventListener("app-toast", h);
      clearTimeout(t);
    };
  }, []);
  if (!msg) return null;
  return <div className="toast">{msg}</div>;
}

export default function App() {
  return (
    <LangProvider>
    <AuthProvider>
      <AreaProvider>
        <Nav />
        <ToastHost />
        <main className="main">
          <Routes>
            <Route path="/" element={<Home />} />
            <Route path="/i/:id" element={<ItemDetail />} />
            <Route path="/provider" element={<Provider />} />
            <Route path="/orders" element={<Orders />} />
            <Route path="/seller" element={<Seller />} />
            <Route path="/admin" element={<Admin />} />
            <Route path="/login" element={<Login />} />
            <Route path="/register" element={<Register />} />
          </Routes>
        </main>
      </AreaProvider>
    </AuthProvider>
    </LangProvider>
  );
}
