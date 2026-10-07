import { useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useAuth } from "../lib/auth";
import { useArea } from "../lib/area";
import { useLang } from "../lib/i18n";

export function Login() {
  const nav = useNavigate();
  const { login } = useAuth();
  const { t } = useLang();
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
      <h2>{t("auth.login")}</h2>
      <input value={phone} onChange={(e) => setPhone(e.target.value)} placeholder={t("auth.phone")} />
      <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder={t("auth.password")} />
      {err && <p className="error">{err}</p>}
      <button onClick={go}>{t("auth.login")}</button>
      <p>{t("auth.noAccount")}<Link to="/register">{t("auth.register")}</Link></p>
      <p className="hint">{t("auth.demo")}</p>
    </div>
  );
}

export function Register() {
  const nav = useNavigate();
  const { register } = useAuth();
  const { areas } = useArea();
  const { t } = useLang();
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
      <h2>{t("auth.register")}</h2>
      <input value={phone} onChange={(e) => setPhone(e.target.value)} placeholder={t("auth.phone")} />
      <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder={t("auth.passwordHint")} />
      <input value={nickname} onChange={(e) => setNickname(e.target.value)} placeholder={t("auth.nickname")} />
      <select value={areaId ?? ""} onChange={(e) => setAreaId(Number(e.target.value))}>
        <option value="">{t("auth.chooseArea")}</option>
        {counties.map((c) => (
          <optgroup key={c} label={c}>
            {areas.filter((a) => a.county === c).map((a) => (
              <option key={a.id} value={a.id}>{a.township}</option>
            ))}
          </optgroup>
        ))}
      </select>
      {err && <p className="error">{err}</p>}
      <button onClick={go}>{t("auth.register")}</button>
    </div>
  );
}
