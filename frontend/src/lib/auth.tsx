import { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import { api } from "./api";

export interface User {
  id: number;
  phone: string;
  nickname: string;
  home_area_id: number | null;
  role: string;
}

interface AuthCtx {
  user: User | null;
  login: (phone: string, password: string) => Promise<void>;
  register: (phone: string, password: string, nickname: string, home_area_id: number | null) => Promise<void>;
  logout: () => void;
}

const Ctx = createContext<AuthCtx>(null as unknown as AuthCtx);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);

  useEffect(() => {
    if (localStorage.getItem("areashop_token")) {
      api<User>("/auth/me").then(setUser).catch(() => localStorage.removeItem("areashop_token"));
    }
  }, []);

  const login = async (phone: string, password: string) => {
    const r = await api<{ access_token: string; user: User }>("/auth/login", {
      method: "POST",
      body: JSON.stringify({ phone, password }),
    });
    localStorage.setItem("areashop_token", r.access_token);
    setUser(r.user);
  };

  const register = async (phone: string, password: string, nickname: string, home_area_id: number | null) => {
    const r = await api<{ access_token: string; user: User }>("/auth/register", {
      method: "POST",
      body: JSON.stringify({ phone, password, nickname, home_area_id }),
    });
    localStorage.setItem("areashop_token", r.access_token);
    setUser(r.user);
  };

  const logout = () => {
    localStorage.removeItem("areashop_token");
    setUser(null);
  };

  return <Ctx.Provider value={{ user, login, register, logout }}>{children}</Ctx.Provider>;
}

export const useAuth = () => useContext(Ctx);
