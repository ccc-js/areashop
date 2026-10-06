import { useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useAuth } from "../lib/auth";
import { useArea } from "../lib/area";

export function Login() {
  const nav = useNavigate();
  const { login } = useAuth();
  const [phone, setPhone] = useState("");
  const [password, setPassword] = useState("");
  const [err, setErr] = useState("");
  const go = async () => {
    try {
      await login(phone, password);
      nav("/");
    } catch (e) {
      setErr(String(e));
    }
  };
  return (
    <div className="form">
      <h2>登入</h2>
      <input value={phone} onChange={(e) => setPhone(e.target.value)} placeholder="手機號碼" />
      <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder="密碼" />
      {err && <p className="error">{err}</p>}
      <button onClick={go}>登入</button>
      <p>還沒有帳號？<Link to="/register">註冊</Link></p>
      <p className="hint">開發種子帳號：0900000003 / password123</p>
    </div>
  );
}

export function Register() {
  const nav = useNavigate();
  const { register } = useAuth();
  const { areas } = useArea();
  const [phone, setPhone] = useState("");
  const [password, setPassword] = useState("");
  const [nickname, setNickname] = useState("");
  const [areaId, setAreaId] = useState<number | null>(null);
  const [err, setErr] = useState("");
  const counties = [...new Set(areas.map((a) => a.county))];
  const go = async () => {
    try {
      await register(phone, password, nickname, areaId);
      nav("/");
    } catch (e) {
      setErr(String(e));
    }
  };
  return (
    <div className="form">
      <h2>註冊</h2>
      <input value={phone} onChange={(e) => setPhone(e.target.value)} placeholder="手機號碼" />
      <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder="密碼（至少6碼）" />
      <input value={nickname} onChange={(e) => setNickname(e.target.value)} placeholder="暱稱" />
      <select value={areaId ?? ""} onChange={(e) => setAreaId(Number(e.target.value))}>
        <option value="">選擇我的地區</option>
        {counties.map((c) => (
          <optgroup key={c} label={c}>
            {areas.filter((a) => a.county === c).map((a) => (
              <option key={a.id} value={a.id}>{a.township}</option>
            ))}
          </optgroup>
        ))}
      </select>
      {err && <p className="error">{err}</p>}
      <button onClick={go}>註冊</button>
    </div>
  );
}
