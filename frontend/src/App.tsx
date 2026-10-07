import { useEffect, useState } from "react";
import { Link, Route, Routes } from "react-router-dom";
import { AuthProvider, useAuth } from "./lib/auth";
import { AreaProvider } from "./lib/area";
import Home from "./pages/Home";
import ItemDetail from "./pages/ItemDetail";
import Orders from "./pages/Orders";
import Seller from "./pages/Seller";
import Provider from "./pages/Provider";
import { Login, Register } from "./pages/Auth";

function Nav() {
  const { user, logout } = useAuth();
  return (
    <nav className="nav">
      <Link to="/" className="logo">areashop 區域商店</Link>
      <Link to="/seller">賣家中心</Link>
      <Link to="/orders">訂單</Link>
      <Link to="/provider">接案管理</Link>
      {user ? (
        <>
          <span>{user.nickname}</span>
          <button onClick={logout}>登出</button>
        </>
      ) : (
        <>
          <Link to="/login">登入</Link>
          <Link to="/register">註冊</Link>
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
            <Route path="/login" element={<Login />} />
            <Route path="/register" element={<Register />} />
          </Routes>
        </main>
      </AreaProvider>
    </AuthProvider>
  );
}
