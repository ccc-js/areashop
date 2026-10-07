// 極簡 toast：任何地方呼叫 toast("已儲存")，由 App 內的 ToastHost 顯示。
export function toast(msg: string) {
  window.dispatchEvent(new CustomEvent("app-toast", { detail: msg }));
}
