# plan0.2 — 服務預約（寄養 / 理髮 / 看診）

> 目標：在 v0.1 商品市集上疊加「服務 + 時段 + 預約」能力。
> 覆蓋場景：3 貓狗寄養、6 理髮、7 診所 ｜ 依賴：v0.1 的 users/shops/areas/Auth ｜ 工期參考：2–3 週

## 1. 本版目標與驗收

- [ ] 店家可定義服務（時長、價格、緩衝時間、須知）
- [ ] 店家可用週模板一鍵產生時段（例如理髮 09:00–18:00 每 30min）
- [ ] 居民可按日期選時段預約，同一時段不超賣
- [ ] 店家可確認/取消，居民可取消，履約後完成；爽約可記 no-show
- [ ] Demo：寄養 1 家、理髮 1 家、診所 1 家，各完成 1 筆預約

**不做**：線上付款訂金（v1.0）、視訊看診、病歷系統（診所僅做掛號登記）。

## 2. 前端交付

- 新路由：`/s/:id` 服務詳情（含可預約日曆）、`/book/:serviceId?slot=`、`/appointments`、`/appointments/:id`、` /provider/services|slots|appointments`（店家行事曆週視圖）。
- 元件：`SlotCalendar`（7 天橫滑，滿位灰掉）、`WeeklyTemplateEditor`（營業日+時段+每格人數）、`BookingConfirmSheet`（手機底部彈層，大按鈕）、`AppointmentBadge`。
- 店家行事曆：按天列出預約，可一鍵確認/取消/完成；診所顯示號次（slot 順序即號次）。
- 表單校驗：不可預約過去時間、不可預約店休日、寄養需填寵物體重/疫苗（Zod conditional）。

## 3. 後端交付

- migrations：
  - `services`（id, shop_id FK, kind[pet|hair|clinic|other], title, desc, duration_min, buffer_min default 0, price_cents, max_per_slot default 1, status[on|off], rules JSONB{cancel_before_hours, notice}）
  - `service_slots`（id, service_id FK, start_at timestamptz, end_at, capacity, booked_count default 0, status[open|full|closed]）+ unique(service_id, start_at) + index(service_id, start_at)
  - `appointments`（id, slot_id FK, service_id, shop_id, buyer_id, pet_info JSONB nullable, status[pending|confirmed|completed|cancelled|noshow], note, created_at）+ 部分唯一索引防重複預約（同一 user 同一 slot 唯一）
- 併發：預約寫入流程 `BEGIN → SELECT slot FOR UPDATE → 檢查 booked<capacity → INSERT appointment → booked_count+1 → COMMIT`，另加 Redis `SETNX booking:slot:{id}` 短鎖（5s）擋重複點擊。
- 模板產生：`POST /services/:id/generate-slots {from,to, weekday_hours:{1:[["09:00","12:00"]]...}, capacity}` 用事務批次 insert（ON CONFLICT DO NOTHING），回傳產生筆數。
- 狀態機：`pending→confirmed→completed | cancelled | noshow`；取消檢查 `now < start_at - cancel_before_hours` 否則 422；`noshow` 僅店主可記，計入 user.no_show_count（users 加欄）。
- 通知（簡易版，本版先站內+寫 DB，推播 v0.4）：`notifications` 表（user_id, kind, title, link），預約建立/確認/取消時寫入，前端 bell 輪詢。

## 4. API 契約（新增）

```text
POST /services {shop_id,kind,title,duration_min,price_cents,max_per_slot,rules}
GET  /services?area_id=&kind=&q=   GET /services/:id?from=&to= (附未來7天 slots)
POST /services/:id/generate-slots {from,to,weekday_hours,capacity}
POST /appointments {slot_id, note, pet_info?} → 201 {appointment}
PATCH /appointments/:id {action: confirm|cancel|complete|noshow}
GET  /appointments?role=buyer|provider&from=&to=
GET  /provider/calendar?shop_id=&from=&to= (按天聚合，店主 scope)
```

## 5. 領域差異（同一套表，三種配置）

- **寄養 pet**：duration 以天計（capacity 5 隻/天，slot 粒度=天），必填 pet_info{species,weight_kg,vaccine}，rules.cancel_before_hours=48。
- **理髮 hair**：duration 30/60min，buffer 10min，capacity 1（設計師=service），支援多 service（剪/染/燙各一）。
- **診所 clinic**：slot 粒度 15min 但 capacity 3（同號多掛），顯示號次 = 該天該 service 第 N 筆 confirmed，rules 需勾「僅掛號、不含診斷」同意。

## 6. 測試與 DoD

- 併發測試：同一 slot 100 併發只成功 capacity 筆（寫一個 `#[tokio::test]` 或 k6 script）。
- 非法取消（開診前 2h 內）回 422；過去時段不可預約。
- Playwright 1 條：居民預約理髮 → 店家確認 → 完成。
- 法務文案：診所服務頁固定頁尾「本平台僅提供掛號登記，不提供診斷與處方」。
