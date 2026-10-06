//! 開發用種子資料：兩個示範鄉鎮（高雄美濃 ＋ 金門金城）+ 店家 + 商品。
//! 冪等：areas 表中有資料就跳過。

use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

use crate::entities::{area, product, shop, user};

struct SeedShop<'a> {
    name: &'a str,
    kind: &'a str, // personal | store
    desc: &'a str,
    addr: &'a str, // 面交說明或店址描述
    address: Option<&'a str>, // 實體店址（有店面才填）
    opening_hours: Option<&'a str>,
    pickup_mode: &'a str, // store | meetup | both
    products: Vec<SeedProduct<'a>>,
}

struct SeedProduct<'a> {
    title: &'a str,
    category: &'a str,
    price_cents: i32,
    stock: i32,
    unit: &'a str,
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

async fn mk_shop_with_products(
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
        pickup_mode: Set(s.pickup_mode.to_string()),
        status: Set("open".to_string()),
        created_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    for p in &s.products {
        product::ActiveModel {
            shop_id: Set(sh.id),
            title: Set(p.title.to_string()),
            category: Set(p.category.to_string()),
            price_cents: Set(p.price_cents),
            stock: Set(p.stock),
            unit: Set(p.unit.to_string()),
            pickup_places: Set(serde_json::json!([{"label": s.addr, "detail": "週六 09:00-11:00"}])),
            pickup_note: Set(Some("請自備購物袋".to_string())),
            images: Set(serde_json::json!([])),
            status: Set("on".to_string()),
            created_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

pub async fn seed_if_empty(db: &sea_orm::DbConn) -> Result<(), sea_orm::DbErr> {
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
            opening_hours: None,
            pickup_mode: "meetup",
            desc: "自家養的放山雞，面交當日現宰",
            addr: "美濃區中山路一段面交",
            products: vec![
                SeedProduct { title: "放山土雞（全雞）", category: "poultry", price_cents: 45000, stock: 10, unit: "隻" },
                SeedProduct { title: "土雞蛋 10 顆", category: "poultry", price_cents: 12000, stock: 30, unit: "盒" },
            ],
        },
        SeedShop {
            name: "美濃小農菜攤",
            kind: "store",
            address: Some("高雄市美濃區泰安路38號"),
            opening_hours: Some("週二至週日 08:00-12:00"),
            pickup_mode: "store",
            desc: "當季野蓮、橙蜜番茄",
            addr: "美濃區泰安路菜市場口",
            products: vec![
                SeedProduct { title: "野蓮 1 把", category: "agri", price_cents: 8000, stock: 20, unit: "把" },
                SeedProduct { title: "橙蜜番茄 1 盒", category: "agri", price_cents: 15000, stock: 15, unit: "盒" },
            ],
        },
        SeedShop {
            name: "美濃水餃姨",
            kind: "personal",
            address: None,
            opening_hours: None,
            pickup_mode: "meetup",
            desc: "高麗菜水餃、韭菜水餃，冷凍面交",
            addr: "美濃區公所前面交",
            products: vec![
                SeedProduct { title: "高麗菜水餃 20 顆", category: "dumpling", price_cents: 16000, stock: 25, unit: "包" },
                SeedProduct { title: "韭菜水餃 20 顆", category: "dumpling", price_cents: 16000, stock: 25, unit: "包" },
            ],
        },
        SeedShop {
            name: "美濃快樂牛",
            kind: "personal",
            address: None,
            opening_hours: None,
            pickup_mode: "both",
            desc: "在地飼養牛，部位可預訂",
            addr: "美濃區福美路面交",
            products: vec![
                SeedProduct { title: "牛肋條 600g", category: "beef", price_cents: 52000, stock: 8, unit: "包" },
                SeedProduct { title: "牛腱心 600g", category: "beef", price_cents: 48000, stock: 8, unit: "包" },
            ],
        },
    ] {
        mk_shop_with_products(db, seller.id, meinong.id, &s).await?;
    }

    // ---- 金門店家（金城鎮） ----
    for s in [
        SeedShop {
            name: "金城放山雞",
            kind: "personal",
            address: None,
            opening_hours: None,
            pickup_mode: "meetup",
            desc: "金門土雞，肉質結實",
            addr: "金城鎮民生路口面交",
            products: vec![
                SeedProduct { title: "金門土雞（全雞）", category: "poultry", price_cents: 50000, stock: 8, unit: "隻" },
                SeedProduct { title: "金門土雞蛋 10 顆", category: "poultry", price_cents: 13000, stock: 20, unit: "盒" },
            ],
        },
        SeedShop {
            name: "金城小農菜圃",
            kind: "personal",
            address: None,
            opening_hours: None,
            pickup_mode: "meetup",
            desc: "金門高麗菜、蒜頭",
            addr: "金城鎮東門市場口",
            products: vec![
                SeedProduct { title: "金門高麗菜 1 顆", category: "agri", price_cents: 9000, stock: 15, unit: "顆" },
                SeedProduct { title: "金門蒜頭 1 斤", category: "agri", price_cents: 12000, stock: 12, unit: "斤" },
            ],
        },
        SeedShop {
            name: "金城水餃嫂",
            kind: "store",
            address: Some("金門縣金城鎮模範街12號"),
            opening_hours: Some("每日 10:00-19:00（週一公休）"),
            pickup_mode: "both",
            desc: "現包手工水餃，當日面交",
            addr: "金城鎮模範街口面交",
            products: vec![
                SeedProduct { title: "高麗菜豬肉水餃 20 顆", category: "dumpling", price_cents: 17000, stock: 20, unit: "包" },
            ],
        },
        SeedShop {
            name: "金城酒糟牛",
            kind: "store",
            address: Some("金門縣金城鎮西海路三段22號"),
            opening_hours: Some("週三至週日 09:00-17:00"),
            pickup_mode: "store",
            desc: "金門酒糟飼養牛，限量部位肉",
            addr: "金城鎮西門面交",
            products: vec![
                SeedProduct { title: "酒糟牛肋條 600g", category: "beef", price_cents: 56000, stock: 6, unit: "包" },
                SeedProduct { title: "酒糟牛火鍋片 400g", category: "beef", price_cents: 42000, stock: 10, unit: "盒" },
            ],
        },
    ] {
        mk_shop_with_products(db, km_seller.id, kinmen.id, &s).await?;
    }

    // ---- 金湖鎮 1 家（跨鄉鎮示範：切到金湖也看得到東西） ----
    mk_shop_with_products(
        db,
        km_seller.id,
        kinhu.id,
        &SeedShop {
            name: "金湖麵線伯",
            kind: "personal",
            desc: "金門手工麵線，料多實在",
            addr: "金湖鎮山外車站旁面交",
            address: None,
            opening_hours: None,
            pickup_mode: "meetup",
            products: vec![
                SeedProduct { title: "金門手工麵線 5 束", category: "other", price_cents: 20000, stock: 15, unit: "袋" },
                SeedProduct { title: "金門花生 1 包", category: "agri", price_cents: 15000, stock: 10, unit: "包" },
            ],
        },
    )
    .await?;

    Ok(())
}
