const BASE = import.meta.env.VITE_API_URL ?? "/api/v1";

export function token(): string | null {
  return localStorage.getItem("areashop_token");
}

export async function api<T>(path: string, init?: RequestInit): Promise<T> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(init?.headers as Record<string, string> | undefined),
  };
  const t = token();
  if (t) headers["Authorization"] = `Bearer ${t}`;
  const res = await fetch(`${BASE}${path}`, { ...init, headers });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || `HTTP ${res.status}`);
  }
  return res.json() as Promise<T>;
}

export interface Area {
  id: number;
  county: string;
  township: string;
  village: string | null;
}

export interface Product {
  id: number;
  shop_id: number;
  title: string;
  category: string;
  price_cents: number;
  stock: number;
  unit: string;
  status: string;
}

export interface Shop {
  id: number;
  name: string;
  area_id: number;
  kind: string;
  address: string | null;
  opening_hours: string | null;
  pickup_mode: string;
}

export const PICKUP_LABEL: Record<string, string> = {
  store: "到店自取",
  meetup: "約面交",
  both: "到店／面交皆可",
};

export interface OrderItem {
  id: number;
  order_id: number;
  product_id: number;
  qty: number;
  price_cents: number;
}

export interface Order {
  id: number;
  buyer_id: number;
  shop_id: number;
  status: string;
  pickup_place: string;
  pickup_at: string | null;
  total_cents: number;
  items?: OrderItem[];
}

export const nt = (cents: number) => `NT$${(cents / 100).toLocaleString()}`;

export const CATEGORY_LABEL: Record<string, string> = {
  agri: "農產品",
  poultry: "雞/蛋",
  beef: "牛肉",
  dumpling: "水餃",
  other: "其他",
};
