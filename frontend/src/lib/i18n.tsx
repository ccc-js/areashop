import { createContext, useContext, useState, type ReactNode } from "react";

export type Lang = "zh" | "cn" | "en";

// [繁體中文, 简体中文, English]；key 用途徑命名方便 grep
const dict = {
  "nav.logo": ["areashop 區域商店", "areashop 区域商店", "areashop Local Market"],
  "nav.seller": ["賣家中心", "卖家中心", "Seller Center"],
  "nav.orders": ["訂單", "订单", "Orders"],
  "nav.provider": ["接案管理", "接单管理", "Availability"],
  "nav.login": ["登入", "登录", "Log in"],
  "nav.register": ["註冊", "注册", "Sign up"],
  "nav.logout": ["登出", "登出", "Log out"],
  "nav.lang": ["語言", "语言", "Language"],
  "nav.zh": ["繁體中文", "繁體中文", "Traditional Chinese"],
  "nav.cn": ["简体中文", "简体中文", "Simplified Chinese"],
  "nav.en": ["English", "English", "English"],

  "common.loading": ["載入中…", "加载中…", "Loading…"],
  "common.loginFirst": ["請先登入。", "请先登录。", "Please log in first."],
  "common.loginFirstShop": ["請先登入再開店。", "请先登录再开店。", "Please log in to open a shop."],
  "common.saved": ["已儲存", "已保存", "Saved"],

  "auth.login": ["登入", "登录", "Log in"],
  "auth.register": ["註冊", "注册", "Sign up"],
  "auth.phone": ["手機號碼", "手机号码", "Phone number"],
  "auth.password": ["密碼", "密码", "Password"],
  "auth.passwordHint": ["密碼（至少6碼）", "密码（至少6位）", "Password (min 6 chars)"],
  "auth.nickname": ["暱稱", "昵称", "Nickname"],
  "auth.chooseArea": ["選擇我的地區", "选择我的地区", "Choose home area"],
  "auth.noAccount": ["還沒有帳號？", "还没有账号？", "No account yet?"],
  "auth.demo": ["開發種子帳號：0900000003 / password123", "开发种子账号：0900000003 / password123", "Demo account: 0900000003 / password123"],

  "home.county": ["縣市", "县市", "County"],
  "home.town": ["鄉鎮", "乡镇", "Township"],
  "home.all": ["不分區（全{county}）", "不分区（全{county}）", "All ({county})"],
  "home.nearby": ["只看附近，不看全站", "只看附近，不看全站", "Nearby only"],
  "home.searchPh": ["搜尋：水餃、放山雞、寄養…", "搜索：水饺、放山鸡、寄养…", "Search: dumplings, chicken, boarding…"],
  "home.search": ["搜尋", "搜索", "Search"],
  "home.empty": ["這個地區還沒有東西，去隔壁鄉鎮看看吧。", "这个地区还没有东西，去隔壁乡镇看看吧。", "Nothing here yet — check nearby areas."],
  "home.bookable": ["可預約", "可预约", "Bookable"],
  "home.category": ["分類", "分类", "Category"],
  "home.catAll": ["不分類", "不分类", "All"],
  "cat.fresh": ["生鮮", "生鲜", "Fresh"],
  "cat.food": ["食品", "食品", "Food"],
  "cat.daily": ["日用", "日用", "Daily"],
  "cat.service": ["服務", "服务", "Services"],
  "cat.other": ["其他", "其他", "Other"],
  "home.left": ["剩 {n}", "剩 {n}", "{n} left"],
  "home.soldout": ["售完", "售完", "Sold out"],

  "item.back": ["← 返回", "← 返回", "← Back"],
  "item.freeCancel": ["{n} 小時前可免費取消", "{n} 小时前可免费取消", "Free cancellation {n}h before"],
  "item.seller": ["賣家：", "卖家：", "Seller: "],
  "item.address": ["店址：", "店址：", "Address: "],
  "item.hours": ["營業：", "营业：", "Hours: "],
  "item.noStorefront": ["無店面，下單時與賣家約面交地點", "无店面，下单时与卖家约面交地点", "No storefront — arrange meetup with seller"],
  "item.pickDate": ["選日期（{m}）", "选日期（{m}）", "Pick a date ({m})"],
  "item.stock": ["庫存：", "库存：", "Stock: "],
  "item.slot": ["時段", "时段", "Time slot"],
  "item.anytime": ["不指定", "不指定", "Anytime"],
  "item.qty": ["數量", "数量", "Qty"],
  "item.note": ["說明", "说明", "Notes"],
  "item.notePh": ["例如：柴犬 8kg，疫苗齊全", "例如：柴犬 8kg，疫苗齐全", "e.g. Shiba 8kg, vaccinated"],
  "item.book": ["預約", "预约", "Book"],
  "item.order": ["下單", "下单", "Order"],
  "item.payMeetup": ["（面交付款）", "（面交付款）", "(pay on meetup)"],
  "item.needDate": ["請選日期", "请选择日期", "Please pick a date"],
  "item.closed": ["休", "休", "Off"],
  "item.full": ["滿", "满", "Full"],
  "item.available": ["可約", "可约", "Open"],
  "item.wd": ["週{wd}", "周{wd}", "{wd}"],

  "orders.title": ["我的訂單", "我的订单", "My Orders"],
  "orders.buyer": ["我是買家", "我是买家", "I'm a buyer"],
  "orders.seller": ["我是賣家", "我是卖家", "I'm a seller"],
  "orders.confirm": ["確認接單", "确认接单", "Confirm order"],
  "orders.ready": ["可面交", "可面交", "Ready for pickup"],
  "orders.complete": ["完成", "完成", "Complete"],
  "orders.noshow": ["記爽約", "记爽约", "No-show"],
  "orders.cancel": ["取消", "取消", "Cancel"],
  "orders.remark": ["備註：", "备注：", "Note: "],
  "orders.buyerIs": ["買家：", "买家：", "Buyer: "],
  "orders.empty": ["還沒有訂單。", "还没有订单。", "No orders yet."],

  "status.pending": ["待確認", "待确认", "Pending"],
  "status.confirmed": ["已確認", "已确认", "Confirmed"],
  "status.ready": ["可面交", "可面交", "Ready"],
  "status.completed": ["已完成", "已完成", "Completed"],
  "status.cancelled": ["已取消", "已取消", "Cancelled"],
  "status.noshow": ["爽約", "爽约", "No-show"],

  "seller.title": ["賣家中心", "卖家中心", "Seller Center"],
  "seller.open10": ["10 分鐘開店", "10 分钟开店", "Open a shop in 10 minutes"],
  "seller.shopNamePh": ["店名，例如：美濃放山雞", "店名，例如：美浓放山鸡", "Shop name, e.g. Meinong Chicken"],
  "seller.personal": ["個人賣家（約面交）", "个人卖家（约面交）", "Individual (meetup)"],
  "seller.store": ["有店面", "有店面", "With storefront"],
  "seller.addressPh": ["店址，例如：金城鎮模範街12號", "店址，例如：金城镇模范街12号", "Address, e.g. Jincheng Main St. 12"],
  "seller.hoursPhStore": ["營業時間，例如：每日 10:00-19:00", "营业时间，例如：每日 10:00-19:00", "Hours, e.g. daily 10:00-19:00"],
  "seller.hoursPhMeet": ["面交時間，例如：週六 09:00-11:00", "面交时间，例如：周六 09:00-11:00", "Meetup time, e.g. Sat 09:00-11:00"],
  "seller.openBtn": ["開店（{area}）", "开店（{area}）", "Open shop ({area})"],
  "seller.opened": ["開店成功！", "开店成功！", "Shop opened!"],
  "seller.myShop": ["我的店：", "我的店：", "My shop: "],
  "seller.settings": ["店鋪設定", "店铺设定", "Shop settings"],
  "seller.updateHours": ["更新營業時間", "更新营业时间", "Update hours"],
  "seller.hoursUpdated": ["營業時間已更新，買家在項目頁看得到。", "营业时间已更新，买家在项目页看得到。", "Hours updated — visible to buyers."],
  "seller.listTitle": ["上架（賣東西和賣服務同一種）", "上架（卖东西和卖服务同一种）", "List an item (goods & services, same flow)"],
  "seller.namePh": ["名稱，例如：土雞蛋 10 顆 / 男士快剪", "名称，例如：土鸡蛋 10 颗 / 男士快剪", "Name, e.g. eggs / haircut"],
  "seller.pricePh": ["價格（元）", "价格（元）", "Price (NTD)"],
  "seller.stockPh": ["庫存（空白 = 不限量）", "库存（空白 = 不限量）", "Stock (blank = unlimited)"],
  "seller.bookable": ["接受選日期預約（設好後去「接案管理」排時間）", "接受选日期预约（设好后去「接单管理」排时间）", "Accept date bookings (then set times in Availability)"],
  "seller.listBtn": ["上架", "上架", "List"],
  "seller.listedGo": ["上架成功！去「接案管理」設可接案時間。", "上架成功！去「接单管理」设可接案时间。", "Listed! Set available times in Availability."],
  "seller.listed": ["上架成功！", "上架成功！", "Listed!"],
  "seller.myItems": ["我的項目（{n}）", "我的项目（{n}）", "My items ({n})"],
  "seller.takeOff": ["下架", "下架", "Remove"],
  "seller.openFirst": ["請先開店", "请先开店", "Open a shop first"],
  "seller.needNamePrice": ["請填名稱、價格（元，須大於 0）", "请填名称、价格（元，须大于 0）", "Enter name and price (NTD, > 0)"],
  "seller.badStock": ["庫存請填 0 以上的數字，空白 = 不限量", "库存请填 0 以上的数字，空白 = 不限量", "Stock must be 0 or more; blank = unlimited"],
  "seller.needEither": ["不限量又不接預約就沒東西可賣了，請二選一", "不限量又不接预约就没东西可卖了，请二选一", "Unlimited with no booking means nothing to sell — pick one"],
  "seller.bookableTag": ["可預約", "可预约", "Bookable"],
  "seller.directTag": ["直接買", "直接买", "Buy now"],

  "provider.title": ["接案管理", "接单管理", "Availability"],
  "provider.shop": ["店家", "店家", "Shop"],
  "provider.pickShop": ["選店家", "选店家", "Select shop"],
  "provider.item": ["項目", "项目", "Item"],
  "provider.pickItem": ["選項目", "选项目", "Select item"],
  "provider.addTitle": ["新增可預約項目", "新增可预约项目", "Add bookable item"],
  "provider.namePh": ["例如：貓狗寄養 1 天", "例如：猫狗寄养 1 天", "e.g. Pet boarding × 1 day"],
  "provider.pricePh": ["價格（元）", "价格（元）", "Price (NTD)"],
  "provider.add": ["新增", "新增", "Add"],
  "provider.added": ["新增項目成功，接著設每週可接案時間。", "新增项目成功，接着设每周可接案时间。", "Item added. Now set weekly availability."],
  "provider.weekTitle": ["每週可接案時間（範本）", "每周可接案时间（范本）", "Weekly availability (template)"],
  "provider.wdOpen": ["週{wd}可接", "周{wd}可接", "{wd} available"],
  "provider.del": ["刪除", "删除", "Delete"],
  "provider.startPh": ["起點，如 09:00", "起点，如 09:00", "Start, e.g. 09:00"],
  "provider.endPh": ["終點，如 12:00", "终点，如 12:00", "End, e.g. 12:00"],
  "provider.addSlot": ["＋時段", "＋时段", "+ slot"],
  "provider.saveTpl": ["儲存範本", "保存范本", "Save template"],
  "provider.tplSaved": ["週範本已儲存。", "周范本已保存。", "Weekly template saved."],
  "provider.needEnds": ["起點終點都要填（自由填，如 09:00 或 早上九點）。", "起点终点都要填（自由填，如 09:00 或 早上九点）。", "Fill in both start and end (free text, e.g. 09:00)."],
  "provider.dayTitle": ["單日開關（{m}，點兩下快速開/關）", "单日开关（{m}，双击快速开/关）", "Day overrides ({m}, double-click to toggle)"],
  "provider.dblClick": ["點兩下快速開/關", "双击快速开/关", "Double-click for quick toggle"],
  "provider.closed": ["休", "休", "Off"],
  "provider.full": ["滿", "满", "Full"],
  "provider.available": ["可約", "可约", "Open"],
  "provider.notSpecified": ["不指定", "不指定", "Anytime"],
  "provider.confirm": ["確認", "确认", "Confirm"],
  "provider.complete": ["完成", "完成", "Complete"],
  "provider.noshow": ["爽約", "爽约", "No-show"],
  "provider.cancel": ["取消", "取消", "Cancel"],
  "provider.noOrders": ["這天還沒有訂單。", "这天还没有订单。", "No orders this day."],
  "provider.editDay": ["改這天（只影響這天，範本不動）", "改这天（只影响这天，范本不动）", "Edit this day only (template untouched)"],
  "provider.normal": ["正常接", "正常接", "Accepting"],
  "provider.fullDay": ["當日額滿", "当日额满", "Full day"],
  "provider.closedDay": ["當日不營業", "当日不营业", "Closed day"],
  "provider.notePh": ["備註，例如：下午器材維修", "备注，例如：下午器材维修", "Note, e.g. afternoon maintenance"],
  "provider.saveDay": ["儲存單日設定", "保存单日设定", "Save day setting"],
  "provider.daySaved": ["單日設定已儲存（範本不受影響）。", "单日设定已保存（范本不受影响）。", "Day setting saved (template untouched)."],
  "provider.preview": ["看居民視角", "看居民视角", "Preview as buyer"],
  "provider.dayOpen": ["可接", "可接", "Accepting"],
  "provider.dayClosed": ["不營業", "不营业", "Closed"],
  "provider.dayFull": ["額滿", "额满", "Full"],
} as const;

export type TKey = keyof typeof dict;

interface LangCtx {
  lang: Lang;
  setLang: (l: Lang) => void;
  t: (key: TKey, params?: Record<string, string | number>) => string;
  wdName: (wd: number) => string;
  statusLabel: (s: string) => string;
  catLabel: (s: string) => string;
}

export const CATS = ["fresh", "food", "daily", "service", "other"] as const;

const Ctx = createContext<LangCtx>(null as unknown as LangCtx);

const WD_ZH = ["日", "一", "二", "三", "四", "五", "六"];
const WD_EN = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

const IDX: Record<Lang, 0 | 1 | 2> = { zh: 0, cn: 1, en: 2 };

const STATUS_KEY: Record<string, TKey> = {
  pending: "status.pending",
  confirmed: "status.confirmed",
  ready: "status.ready",
  completed: "status.completed",
  cancelled: "status.cancelled",
  noshow: "status.noshow",
};

const CAT_KEY: Record<string, TKey> = {
  fresh: "cat.fresh",
  food: "cat.food",
  daily: "cat.daily",
  service: "cat.service",
  other: "cat.other",
};

function detect(): Lang {
  const saved = localStorage.getItem("areashop_lang");
  if (saved === "en" || saved === "cn" || saved === "zh") return saved;
  const nav = navigator.language.toLowerCase();
  if (nav.startsWith("en")) return "en";
  if (nav === "zh-cn" || nav === "zh-sg") return "cn";
  return "zh";
}

export function LangProvider({ children }: { children: ReactNode }) {
  const [lang, setLangState] = useState<Lang>(detect);
  const setLang = (l: Lang) => {
    setLangState(l);
    localStorage.setItem("areashop_lang", l);
  };
  const t = (key: TKey, params?: Record<string, string | number>) => {
    let s: string = dict[key][IDX[lang]];
    if (params) {
      for (const [k, v] of Object.entries(params)) s = s.replace(`{${k}}`, String(v));
    }
    return s;
  };
  const wdName = (wd: number) =>
    lang === "en" ? WD_EN[wd] : WD_ZH[wd];
  const statusLabel = (s: string) =>
    STATUS_KEY[s] ? t(STATUS_KEY[s]) : s;
  const catLabel = (s: string) =>
    CAT_KEY[s] ? t(CAT_KEY[s]) : s;
  return <Ctx.Provider value={{ lang, setLang, t, wdName, statusLabel, catLabel }}>{children}</Ctx.Provider>;
}

export const useLang = () => useContext(Ctx);
