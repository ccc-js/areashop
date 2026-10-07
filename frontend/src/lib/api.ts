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

export interface Item {
  id: number;
  shop_id: number;
  title: string;
  description: string | null;
  price_cents: number;
  unit: string;
  status: string;
  stock: number | null; // null = 不限量
  bookable: boolean;
  cancel_hours: number;
  notice: string | null;
}

export interface Shop {
  id: number;
  name: string;
  area_id: number;
  kind: string;
  address: string | null;
  opening_hours: string | null;
}

export interface OrderItem {
  id: number;
  order_id: number;
  item_id: number;
  qty: number;
  price_cents: number;
  title?: string; // 明細才有
}

export interface OrderBuyer {
  nickname: string;
  phone: string;
}

export interface Order {
  id: number;
  buyer_id: number;
  shop_id: number;
  status: string;
  pickup_at: string | null;
  date: string | null; // 有值 = 選日期的單
  window: string | null;
  total_cents: number;
  remark: string | null;
  items?: OrderItem[];
}

export interface OrderDetail extends Order {
  buyer: OrderBuyer;
  items: OrderItem[];
}

// 統一項目：賣東西＋賣服務同一種（stock null = 不限量，bookable = 可選日期）
export interface TimeWindow {
  start: string;
  end: string;
}

export const windowLabel = (w: TimeWindow) => `${w.start}-${w.end}`;

export interface ItemRule {
  id: number;
  item_id: number;
  weekday: number;
  open: boolean;
  windows: TimeWindow[];
}

export interface ItemDay {
  date: string;
  open: boolean;
  full: boolean;
  windows: TimeWindow[];
}

export interface ItemDetail extends Item {
  rules: ItemRule[];
  days: ItemDay[];
}

// YYYY-MM-DD（本地時區）
export const todayStr = (plus = 0) => {
  const d = new Date();
  d.setDate(d.getDate() + plus);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(
    d.getDate()
  ).padStart(2, "0")}`;
};

export const nt = (cents: number) => `NT$${(cents / 100).toLocaleString()}`;
