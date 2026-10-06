import { Link, Route, Routes } from "react-router-dom";
import { AuthProvider, useAuth } from "./lib/auth";
import { AreaProvider } from "./lib/area";
import Home from "./pages/Home";
import ProductDetail from "./pages/ProductDetail";
import Orders from "./pages/Orders";
import Seller from "./pages/Seller";
import { Login, Register } from "./pages/Auth";

function Nav() {
  const { user, logout } = useAuth();
  return (
    <nav className="nav">
      <Link to="/" className="logo">areashop 區域商店</Link>
      <Link to="/seller">賣家中心</Link>
      <Link to="/orders">訂單</Link>
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

export default function App() {
  return (
    <AuthProvider>
      <AreaProvider>
        <Nav />
        <main className="main">
          <Routes>
            <Route path="/" element={<Home />} />
            <Route path="/p/:id" element={<ProductDetail />} />
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
