import { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import { api, type Area } from "./api";

interface AreaCtx {
  areas: Area[];
  current: Area | null;
  setCurrentId: (id: number) => void;
}

const Ctx = createContext<AreaCtx>(null as unknown as AreaCtx);

export function AreaProvider({ children }: { children: ReactNode }) {
  const [areas, setAreas] = useState<Area[]>([]);
  const [currentId, setCurrentId] = useState<number | null>(() => {
    const v = localStorage.getItem("areashop_area");
    return v ? Number(v) : null;
  });

  useEffect(() => {
    api<Area[]>("/areas").then((list) => {
      setAreas(list);
      if (!currentId && list.length > 0) {
        setCurrentId(list[0].id);
        localStorage.setItem("areashop_area", String(list[0].id));
      }
    }).catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const setCurrentIdAndSave = (id: number) => {
    setCurrentId(id);
    localStorage.setItem("areashop_area", String(id));
  };

  return (
    <Ctx.Provider
      value={{ areas, current: areas.find((a) => a.id === currentId) ?? null, setCurrentId: setCurrentIdAndSave }}
    >
      {children}
    </Ctx.Provider>
  );
}

export const useArea = () => useContext(Ctx);
