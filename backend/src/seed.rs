//! 開發用種子資料：兩個示範鄉鎮（高雄美濃 ＋ 金門金城）+ 夏威夷英文區 + 店家 + 統一項目 + 訂單。
//! 冪等：基底（areas）只在空庫塞；訂單類、夏威夷區表空/缺就補，舊庫重開即有。

use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

use crate::entities::{area, item, order, order_item, shop, user};

struct SeedShop<'a> {
    name: &'a str,
    kind: &'a str, // personal | store
    desc: &'a str,
    addr: &'a str,            // 面交說明或店址描述
    address: Option<&'a str>, // 實體店址（有店面才填）
    opening_hours: Option<&'a str>,
    items: Vec<SeedItem<'a>>,
}

/// 統一項目：stock 有值 = 賣東西；bookable = 可選日期；兩者都開 = 混合
struct SeedItem<'a> {
    title: &'a str,
    desc: Option<&'a str>,
    price_cents: i32,
    unit: &'a str,
    stock: Option<i32>,
    bookable: bool,
    cancel_hours: i32,
    notice: Option<&'a str>,
}

async fn mk_area(
    db: &sea_orm::DbConn,
    county: &str,
    township: &str,
    lat: f64,
    lng: f64,
) -> Result<area::Model, sea_orm::DbErr> {
    area::ActiveModel {
        county: Set(county.to_string()),
        township: Set(township.to_string()),
        village: Set(None),
        center_lat: Set(Some(lat)),
        center_lng: Set(Some(lng)),
        radius_km: Set(Some(8.0)),
        ..Default::default()
    }
    .insert(db)
    .await
}

async fn mk_user(
    db: &sea_orm::DbConn,
    phone: &str,
    nickname: &str,
    home_area_id: i32,
    role: &str,
) -> Result<user::Model, sea_orm::DbErr> {
    user::ActiveModel {
        phone: Set(phone.to_string()),
        password_hash: Set(crate::auth::hash_password("password123").unwrap()),
        nickname: Set(nickname.to_string()),
        home_area_id: Set(Some(home_area_id)),
        role: Set(role.to_string()),
        created_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
}

/// 週二(2)至週六(6)開的預設範本（09:00-12:00、14:00-18:00）
async fn mk_week_template(db: &sea_orm::DbConn, item_id: i32) -> Result<(), sea_orm::DbErr> {
    for wd in 0..=6 {
        let open = (2..=6).contains(&wd);
        crate::entities::availability_rule::ActiveModel {
            item_id: Set(item_id),
            weekday: Set(wd),
            open: Set(open),
            windows: Set(if open {
                serde_json::json!([
                    {"start": "09:00", "end": "12:00"},
                    {"start": "14:00", "end": "18:00"},
                ])
            } else {
                serde_json::json!([])
            }),
            created_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

async fn mk_shop_with_items(
    db: &sea_orm::DbConn,
    owner_id: i32,
    area_id: i32,
    s: &SeedShop<'_>,
) -> Result<(), sea_orm::DbErr> {
    let sh = shop::ActiveModel {
        owner_id: Set(owner_id),
        area_id: Set(area_id),
        name: Set(s.name.to_string()),
        kind: Set(s.kind.to_string()),
        description: Set(Some(s.desc.to_string())),
        address_text: Set(Some(s.addr.to_string())),
        phone: Set(Some("0900000002".to_string())),
        address: Set(s.address.map(|a| a.to_string())),
        opening_hours: Set(s.opening_hours.map(|h| h.to_string())),
        status: Set("open".to_string()),
        created_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    for p in &s.items {
        let m = item::ActiveModel {
            shop_id: Set(sh.id),
            title: Set(p.title.to_string()),
            description: Set(p.desc.map(|d| d.to_string())),
            price_cents: Set(p.price_cents),
            unit: Set(p.unit.to_string()),
            images: Set(serde_json::json!([])),
            status: Set("on".to_string()),
            stock: Set(p.stock),
            bookable: Set(p.bookable),
            cancel_hours: Set(p.cancel_hours),
            notice: Set(p.notice.map(|n| n.to_string())),
            created_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(db)
        .await?;
        if p.bookable {
            mk_week_template(db, m.id).await?;
        }
    }
    Ok(())
}

pub async fn seed_if_empty(db: &sea_orm::DbConn) -> Result<(), sea_orm::DbErr> {
    use sea_orm::{ColumnTrait, PaginatorTrait, QueryFilter};
    let n = area::Entity::find().paginate(db, 1).num_items().await?;
    // 基底：只有空庫才塞（舊庫沿用，避免洗掉店家資料）
    if n == 0 {
        seed_base(db).await?;
    }

    // 夏威夷英文示範區：加法冪等——沒有就補（舊庫重開即有）
    let hi = area::Entity::find()
        .filter(area::Column::County.eq("Hawaii"))
        .paginate(db, 1)
        .num_items()
        .await?;
    if hi == 0 {
        seed_hawaii(db).await?;
    }

    // 訂單類：加法冪等——表空的就補（舊庫重開即有；店家訂單數 0..5 分布）
    let no = order::Entity::find().paginate(db, 1).num_items().await?;
    if no == 0 {
        seed_orders(db).await?;
    }
    Ok(())
}

async fn seed_base(db: &sea_orm::DbConn) -> Result<(), sea_orm::DbErr> {
    use sea_orm::PaginatorTrait;
    let n = area::Entity::find().paginate(db, 1).num_items().await?;
    if n > 0 {
        return Ok(());
    }

    // ---- 地區：美濃＋金門五鄉鎮 ----
    let meinong = mk_area(db, "高雄市", "美濃區", 22.8940, 120.5375).await?;
    let kinmen_city = mk_area(db, "金門縣", "金城鎮", 24.4333, 118.3167).await?;
    let kinhu = mk_area(db, "金門縣", "金湖鎮", 24.4417, 118.4258).await?;
    mk_area(db, "金門縣", "金沙鎮", 24.4986, 118.4319).await?;
    mk_area(db, "金門縣", "金寧鄉", 24.4358, 118.3331).await?;
    mk_area(db, "金門縣", "烈嶼鄉", 24.4231, 118.2317).await?;
    let kinmen = kinmen_city;

    // ---- 帳號（密碼皆 password123，僅開發用） ----
    mk_user(db, "0900000001", "管理員", meinong.id, "admin").await?;
    let seller = mk_user(db, "0900000002", "美濃小農", meinong.id, "user").await?;
    mk_user(db, "0900000003", "美濃居民", meinong.id, "user").await?;
    let km_seller = mk_user(db, "0900000004", "金門小農", kinmen.id, "user").await?;
    mk_user(db, "0900000005", "金城居民", kinmen.id, "user").await?;

    // ---- 美濃店家 ----
    for s in [
        SeedShop {
            name: "美濃放山雞",
            kind: "personal",
            address: None,
            opening_hours: Some("面交時間：週六 09:00-11:00"),
            desc: "自家養的放山雞，面交當日現宰",
            addr: "美濃區中山路一段面交",
            items: vec![
                SeedItem {
                    title: "放山土雞（全雞）",
                    desc: None,
                    price_cents: 45000,
                    unit: "隻",
                    stock: Some(10),
                    bookable: false,
                    cancel_hours: 24,
                    notice: None,
                },
                SeedItem {
                    title: "土雞蛋 10 顆",
                    desc: None,
                    price_cents: 12000,
                    unit: "盒",
                    stock: Some(30),
                    bookable: false,
                    cancel_hours: 24,
                    notice: None,
                },
            ],
        },
        SeedShop {
            name: "美濃小農菜攤",
            kind: "store",
            address: Some("高雄市美濃區泰安路38號"),
            opening_hours: Some("週二至週日 08:00-12:00"),
            desc: "當季野蓮、橙蜜番茄",
            addr: "美濃區泰安路菜市場口",
            items: vec![
                SeedItem {
                    title: "野蓮 1 把",
                    desc: None,
                    price_cents: 8000,
                    unit: "把",
                    stock: Some(20),
                    bookable: false,
                    cancel_hours: 24,
                    notice: None,
                },
                SeedItem {
                    title: "橙蜜番茄 1 盒",
                    desc: None,
                    price_cents: 15000,
                    unit: "盒",
                    stock: Some(15),
                    bookable: false,
                    cancel_hours: 24,
                    notice: None,
                },
            ],
        },
        SeedShop {
            name: "美濃水餃姨",
            kind: "personal",
            address: None,
            opening_hours: Some("面交時間：週三、週六 10:00-12:00"),
            desc: "高麗菜水餃、韭菜水餃，冷凍面交（也可預訂某天取）",
            addr: "美濃區公所前面交",
            items: vec![
                SeedItem {
                    title: "高麗菜水餃 20 顆",
                    desc: Some("現包冷凍，也可預訂面交日"),
                    price_cents: 16000,
                    unit: "包",
                    stock: Some(25),
                    bookable: true,
                    cancel_hours: 24,
                    notice: None,
                },
                SeedItem {
                    title: "韭菜水餃 20 顆",
                    desc: None,
                    price_cents: 16000,
                    unit: "包",
                    stock: Some(25),
                    bookable: false,
                    cancel_hours: 24,
                    notice: None,
                },
            ],
        },
        SeedShop {
            name: "美濃快樂牛",
            kind: "store",
            address: Some("高雄市美濃區福美路88號"),
            opening_hours: Some("週三至週日 09:00-17:00"),
            desc: "在地飼養牛，部位可預訂",
            addr: "美濃區福美路面交",
            items: vec![
                SeedItem {
                    title: "牛肋條 600g",
                    desc: None,
                    price_cents: 52000,
                    unit: "包",
                    stock: Some(8),
                    bookable: false,
                    cancel_hours: 24,
                    notice: None,
                },
                SeedItem {
                    title: "牛腱心 600g",
                    desc: None,
                    price_cents: 48000,
                    unit: "包",
                    stock: Some(8),
                    bookable: false,
                    cancel_hours: 24,
                    notice: None,
                },
            ],
        },
    ] {
        mk_shop_with_items(db, seller.id, meinong.id, &s).await?;
    }

    // ---- 金門店家（金城鎮） ----
    for s in [
        SeedShop {
            name: "金城放山雞",
            kind: "personal",
            address: None,
            opening_hours: Some("面交時間：週日 09:00-11:00"),
            desc: "金門土雞，肉質結實",
            addr: "金城鎮民生路口面交",
            items: vec![
                SeedItem {
                    title: "金門土雞（全雞）",
                    desc: None,
                    price_cents: 50000,
                    unit: "隻",
                    stock: Some(8),
                    bookable: false,
                    cancel_hours: 24,
                    notice: None,
                },
                SeedItem {
                    title: "金門土雞蛋 10 顆",
                    desc: None,
                    price_cents: 13000,
                    unit: "盒",
                    stock: Some(20),
                    bookable: false,
                    cancel_hours: 24,
                    notice: None,
                },
            ],
        },
        SeedShop {
            name: "金城小農菜圃",
            kind: "personal",
            address: None,
            opening_hours: Some("面交時間：每日 08:00-10:00"),
            desc: "金門高麗菜、蒜頭",
            addr: "金城鎮東門市場口",
            items: vec![
                SeedItem {
                    title: "金門高麗菜 1 顆",
                    desc: None,
                    price_cents: 9000,
                    unit: "顆",
                    stock: Some(15),
                    bookable: false,
                    cancel_hours: 24,
                    notice: None,
                },
                SeedItem {
                    title: "金門蒜頭 1 斤",
                    desc: None,
                    price_cents: 12000,
                    unit: "斤",
                    stock: Some(12),
                    bookable: false,
                    cancel_hours: 24,
                    notice: None,
                },
            ],
        },
        SeedShop {
            name: "金城水餃嫂",
            kind: "store",
            address: Some("金門縣金城鎮模範街12號"),
            opening_hours: Some("每日 10:00-19:00（週一公休）"),
            desc: "現包手工水餃，當日面交（也可預訂）",
            addr: "金城鎮模範街口面交",
            items: vec![SeedItem {
                title: "高麗菜豬肉水餃 20 顆",
                desc: Some("現包，也可預訂面交日"),
                price_cents: 17000,
                unit: "包",
                stock: Some(20),
                bookable: true,
                cancel_hours: 24,
                notice: None,
            }],
        },
        SeedShop {
            name: "金城酒糟牛",
            kind: "store",
            address: Some("金門縣金城鎮西海路三段22號"),
            opening_hours: Some("週三至週日 09:00-17:00"),
            desc: "金門酒糟飼養牛，限量部位肉",
            addr: "金城鎮西門面交",
            items: vec![
                SeedItem {
                    title: "酒糟牛肋條 600g",
                    desc: None,
                    price_cents: 56000,
                    unit: "包",
                    stock: Some(6),
                    bookable: false,
                    cancel_hours: 24,
                    notice: None,
                },
                SeedItem {
                    title: "酒糟牛火鍋片 400g",
                    desc: None,
                    price_cents: 42000,
                    unit: "盒",
                    stock: Some(10),
                    bookable: false,
                    cancel_hours: 24,
                    notice: None,
                },
            ],
        },
    ] {
        mk_shop_with_items(db, km_seller.id, kinmen.id, &s).await?;
    }

    // ---- 金湖鎮 1 家（跨鄉鎮示範：切到金湖也看得到東西） ----
    mk_shop_with_items(
        db,
        km_seller.id,
        kinhu.id,
        &SeedShop {
            name: "金湖麵線伯",
            kind: "personal",
            desc: "金門手工麵線，料多實在",
            addr: "金湖鎮山外車站旁面交",
            address: None,
            opening_hours: Some("面交時間：週六 14:00-16:00"),
            items: vec![
                SeedItem {
                    title: "金門手工麵線 5 束",
                    desc: None,
                    price_cents: 20000,
                    unit: "袋",
                    stock: Some(15),
                    bookable: false,
                    cancel_hours: 24,
                    notice: None,
                },
                SeedItem {
                    title: "金門花生 1 包",
                    desc: None,
                    price_cents: 15000,
                    unit: "包",
                    stock: Some(10),
                    bookable: false,
                    cancel_hours: 24,
                    notice: None,
                },
            ],
        },
    )
    .await?;

    // ---- 服務型項目（統一種子：直接掛在店下，沒有獨立店） ----
    // 寄養掛快樂牛？不——沿用舊 Demo：寄養/快剪/診所各一店
    for s in [
        SeedShop {
            name: "金城寵物寄養",
            kind: "personal",
            desc: "住家寄養，有大草地跑跑",
            addr: "金城鎮民族路口面交接送",
            address: None,
            opening_hours: Some("接送時間：每日 09:00-18:00（先預約）"),
            items: vec![SeedItem {
                title: "貓狗寄養 1 天",
                desc: Some("中小型犬貓皆可，請自備飼料；疫苗證明入住時出示"),
                price_cents: 60000,
                unit: "天",
                stock: None,
                bookable: true,
                cancel_hours: 48,
                notice: Some("請自備飼料與疫苗證明"),
            }],
        },
        SeedShop {
            name: "金城快剪",
            kind: "store",
            desc: "男士快剪，不用等",
            addr: "金城鎮中興路口",
            address: Some("金門縣金城鎮中興路50號"),
            opening_hours: Some("週二至週六 09:00-18:00"),
            items: vec![SeedItem {
                title: "男士快剪",
                desc: Some("含洗髮，約 30 分鐘"),
                price_cents: 30000,
                unit: "次",
                stock: None,
                bookable: true,
                cancel_hours: 24,
                notice: None,
            }],
        },
        SeedShop {
            name: "金城安心診所",
            kind: "store",
            desc: "家醫科掛號",
            addr: "金城鎮民權路口",
            address: Some("金門縣金城鎮民權路8號"),
            opening_hours: Some("週二至週六 09:00-12:00、14:00-18:00"),
            items: vec![SeedItem {
                title: "家醫科掛號",
                desc: Some("掛號登記用"),
                price_cents: 20000,
                unit: "號",
                stock: None,
                bookable: true,
                cancel_hours: 24,
                notice: Some("本平台僅提供掛號登記，不提供診斷與處方"),
            }],
        },
    ] {
        mk_shop_with_items(db, km_seller.id, kinmen.id, &s).await?;
    }

    Ok(())
}

/// 夏威夷英文示範區（給英文版 UI 用的測試資料）
async fn seed_hawaii(db: &sea_orm::DbConn) -> Result<(), sea_orm::DbErr> {
    use chrono::Datelike;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let honolulu = mk_area(db, "Hawaii", "Honolulu", 21.3099, -157.8581).await?;
    mk_area(db, "Hawaii", "Kailua", 21.4022, -157.7394).await?;

    // ---- 帳號（密碼皆 password123，僅開發用） ----
    let seller = mk_user(db, "0900000006", "Honolulu Farmer", honolulu.id, "user").await?;
    let buyer = mk_user(db, "0900000007", "Honolulu Local", honolulu.id, "user").await?;

    // ---- 店家 ----
    for s in [
        SeedShop {
            name: "Honolulu Poke Bowl",
            kind: "store",
            address: Some("123 Kalakaua Ave, Honolulu, HI"),
            opening_hours: Some("Tue-Sun 10:00-20:00"),
            desc: "Fresh ahi poke, made to order",
            addr: "Pickup at Kalakaua Ave store",
            items: vec![
                SeedItem {
                    title: "Ahi Poke Bowl",
                    desc: Some("Fresh tuna, rice, seaweed salad"),
                    price_cents: 1250,
                    unit: "bowl",
                    stock: Some(20),
                    bookable: false,
                    cancel_hours: 24,
                    notice: None,
                },
                SeedItem {
                    title: "Spam Musubi (3 pcs)",
                    desc: None,
                    price_cents: 450,
                    unit: "pack",
                    stock: Some(30),
                    bookable: false,
                    cancel_hours: 24,
                    notice: None,
                },
            ],
        },
        SeedShop {
            name: "Waikiki Surf Lessons",
            kind: "personal",
            address: None,
            opening_hours: Some("Lessons daily 08:00-17:00 (book first)"),
            desc: "Beginner surf lessons with local instructors",
            addr: "Meet at Waikiki Beach, in front of Duke statue",
            items: vec![SeedItem {
                title: "Beginner Surf Lesson",
                desc: Some("1-hour lesson, board included"),
                price_cents: 8000,
                unit: "session",
                stock: None,
                bookable: true,
                cancel_hours: 48,
                notice: Some("Please bring swimwear and sunscreen"),
            }],
        },
        SeedShop {
            name: "Kailua Shave Ice",
            kind: "store",
            address: Some("45 Kailua Rd, Kailua, HI"),
            opening_hours: Some("Daily 11:00-18:00"),
            desc: "Rainbow shave ice with island syrups",
            addr: "Pickup at Kailua Rd store",
            items: vec![SeedItem {
                title: "Rainbow Shave Ice",
                desc: None,
                price_cents: 650,
                unit: "cup",
                stock: Some(25),
                bookable: false,
                cancel_hours: 24,
                notice: None,
            }],
        },
    ] {
        mk_shop_with_items(db, seller.id, honolulu.id, &s).await?;
    }

    // ---- 訂單：2 筆直接買＋1 筆預約 ----
    mk_hawaii_order(db, buyer.id, "Honolulu Poke Bowl", 0, 2, "pending").await?;
    mk_hawaii_order(db, buyer.id, "Kailua Shave Ice", 0, 1, "completed").await?;

    // 預約一筆（推到週二～週六，配合範本）
    let sh = shop::Entity::find()
        .filter(shop::Column::Name.eq("Waikiki Surf Lessons"))
        .one(db)
        .await?
        .unwrap();
    let svc = item::Entity::find()
        .filter(item::Column::ShopId.eq(sh.id))
        .one(db)
        .await?
        .unwrap();
    let mut d = Utc::now().date_naive() + chrono::Duration::days(3);
    while matches!(d.weekday().num_days_from_sunday(), 0 | 1) {
        d = d.succ_opt().unwrap();
    }
    let om = order::ActiveModel {
        buyer_id: Set(buyer.id),
        shop_id: Set(sh.id),
        status: Set("confirmed".to_string()),
        pickup_at: Set(None),
        date: Set(Some(d.format("%Y-%m-%d").to_string())),
        window: Set(Some("09:00-12:00".to_string())),
        total_cents: Set(svc.price_cents),
        remark: Set(Some("First timer, need small board".to_string())),
        created_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    order_item::ActiveModel {
        order_id: Set(om.id),
        item_id: Set(svc.id),
        qty: Set(1),
        price_cents: Set(svc.price_cents),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(())
}

/// 夏威夷區用的直接買訂單小幫手
async fn mk_hawaii_order(
    db: &sea_orm::DbConn,
    buyer_id: i32,
    shop_name: &str,
    item_idx: usize,
    qty: i32,
    status: &str,
) -> Result<(), sea_orm::DbErr> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
    let sh = shop::Entity::find()
        .filter(shop::Column::Name.eq(shop_name))
        .one(db)
        .await?
        .unwrap();
    let items = item::Entity::find()
        .filter(item::Column::ShopId.eq(sh.id))
        .order_by_asc(item::Column::Id)
        .all(db)
        .await?;
    let p = &items[item_idx % items.len()];
    let om = order::ActiveModel {
        buyer_id: Set(buyer_id),
        shop_id: Set(sh.id),
        status: Set(status.to_string()),
        pickup_at: Set(None),
        date: Set(None),
        window: Set(None),
        total_cents: Set(p.price_cents * qty),
        remark: Set(None),
        created_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    order_item::ActiveModel {
        order_id: Set(om.id),
        item_id: Set(p.id),
        qty: Set(qty),
        price_cents: Set(p.price_cents),
        ..Default::default()
    }
    .insert(db)
    .await?;
    if status != "cancelled" {
        if let Some(st) = p.stock {
            let mut am: item::ActiveModel = item::Entity::find_by_id(p.id)
                .one(db)
                .await?
                .unwrap()
                .into();
            am.stock = Set(Some(st - qty));
            am.update(db).await?;
        }
    }
    Ok(())
}

/// 訂單種子：每家店 0～5 筆（有日期的＝預約，沒日期＝直接買）
async fn seed_orders(db: &sea_orm::DbConn) -> Result<(), sea_orm::DbErr> {
    use chrono::Datelike;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
    // (店名, 筆數, 是否預約) —— 0..5 都有人拿
    let demo = [
        ("美濃水餃姨", 5, false),
        ("美濃放山雞", 4, false),
        ("美濃小農菜攤", 4, false),
        ("金城放山雞", 3, false),
        ("金湖麵線伯", 3, false),
        ("金城水餃嫂", 2, false),
        ("金城小農菜圃", 2, false),
        ("美濃快樂牛", 1, false),
        ("金城酒糟牛", 1, false),
        ("金城寵物寄養", 2, true),
        ("金城快剪", 1, true),
        ("金城安心診所", 0, true),
    ];
    let statuses = ["pending", "confirmed", "ready", "completed", "cancelled"];
    let phones = ["0900000003", "0900000005"];
    let windows = ["09:00-12:00", "14:00-18:00"];
    let remarks = ["柴犬 8kg，疫苗齊全", "兩側推高、上面剪短", ""];
    for (shop_name, count, dated) in demo {
        let Some(sh) = shop::Entity::find()
            .filter(shop::Column::Name.eq(shop_name))
            .one(db)
            .await?
        else {
            continue;
        };
        let prods = item::Entity::find()
            .filter(item::Column::ShopId.eq(sh.id))
            .order_by_asc(item::Column::Id)
            .all(db)
            .await?;
        if prods.is_empty() {
            continue;
        }
        for i in 0..count {
            let p = &prods[(i as usize) % prods.len()];
            let Some(buyer) = user::Entity::find()
                .filter(user::Column::Phone.eq(phones[(i as usize) % phones.len()]))
                .one(db)
                .await?
            else {
                continue;
            };
            // 有日期的單：推到週二～週六；間隔拉開避免同人同日撞單
            let date = if dated {
                let mut d = Utc::now().date_naive() + chrono::Duration::days(3 + i as i64 * 2);
                while matches!(d.weekday().num_days_from_sunday(), 0 | 1) {
                    d = d.succ_opt().unwrap();
                }
                Some(d.format("%Y-%m-%d").to_string())
            } else {
                None
            };
            let window = if dated {
                Some(windows[(i as usize) % windows.len()].to_string())
            } else {
                None
            };
            // 重讀庫存，避免同一輪超扣（不限量直接過）
            let cur = item::Entity::find_by_id(p.id).one(db).await?.unwrap();
            if cur.stock.is_some_and(|v| v <= 0) {
                continue;
            }
            let qty = cur.stock.map(|v| v.min(1 + (i % 2))).unwrap_or(1);
            let status = statuses[(i as usize) % statuses.len()];
            let om = order::ActiveModel {
                buyer_id: Set(buyer.id),
                shop_id: Set(sh.id),
                status: Set(status.to_string()),
                pickup_at: Set(None),
                date: Set(date),
                window: Set(window),
                total_cents: Set(cur.price_cents * qty),
                remark: Set(if dated {
                    let r = remarks[(i as usize) % remarks.len()];
                    if r.is_empty() {
                        None
                    } else {
                        Some(r.to_string())
                    }
                } else {
                    None
                }),
                created_at: Set(Utc::now()),
                ..Default::default()
            }
            .insert(db)
            .await?;
            order_item::ActiveModel {
                order_id: Set(om.id),
                item_id: Set(cur.id),
                qty: Set(qty),
                price_cents: Set(cur.price_cents),
                ..Default::default()
            }
            .insert(db)
            .await?;
            // 非取消單扣庫存（與下單 API 一致；不限量不扣）
            if status != "cancelled" {
                if let Some(st) = cur.stock {
                    let mut am: item::ActiveModel = cur.into();
                    am.stock = Set(Some(st - qty));
                    am.update(db).await?;
                }
            }
        }
    }
    Ok(())
}
