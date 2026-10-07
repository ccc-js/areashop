//! 彈性 DB schema：用 SeaORM `SchemaManager::create_table` 建表，
//! 同一份程式碼可在 SQLite（本地測試）與 PostgreSQL（擴大規模）上執行，
//! 不寫任何 `AUTOINCREMENT vs SERIAL`、`JSONB vs TEXT` 之類的方言 SQL。
//! 舊庫升級：新欄位用 has_column + ALTER TABLE ADD COLUMN 補上（兩邊通用）。

use sea_orm::{sea_query::*, ConnectionTrait, DbConn, DbErr};

use crate::entities::{
    area, availability_exception, availability_rule, item, notification, order, order_item, shop,
    user,
};

pub async fn setup_schema(db: &DbConn) -> Result<(), DbErr> {
    let schema = sea_orm::Schema::new(db.get_database_backend());

    // areas
    let stmt = schema
        .create_table_from_entity(area::Entity)
        .if_not_exists()
        .to_owned();
    create_if_not_exists(db, stmt).await?;

    // users
    let stmt = schema
        .create_table_from_entity(user::Entity)
        .if_not_exists()
        .to_owned();
    create_if_not_exists(db, stmt).await?;

    for entity_stmt in [
        schema
            .create_table_from_entity(shop::Entity)
            .if_not_exists()
            .to_owned(),
        // 統一項目（舊 products/services 表不再使用，改版需 --reseed）
        schema
            .create_table_from_entity(item::Entity)
            .if_not_exists()
            .to_owned(),
        schema
            .create_table_from_entity(order::Entity)
            .if_not_exists()
            .to_owned(),
        schema
            .create_table_from_entity(order_item::Entity)
            .if_not_exists()
            .to_owned(),
        schema
            .create_table_from_entity(availability_rule::Entity)
            .if_not_exists()
            .to_owned(),
        schema
            .create_table_from_entity(availability_exception::Entity)
            .if_not_exists()
            .to_owned(),
        schema
            .create_table_from_entity(notification::Entity)
            .if_not_exists()
            .to_owned(),
    ] {
        create_if_not_exists(db, entity_stmt).await?;
    }

    // 舊庫補欄位：直接 ALTER TABLE ADD COLUMN，已存在則忽略錯誤訊息。
    // pg: column "address" of relation "shops" already exists
    // sqlite: (code: 1) duplicate column name: address
    // 新庫建表時已含這些欄位，第一次 ALTER 即報重複，無害。
    let mut addr = ColumnDef::new(Alias::new("address"));
    addr.text();
    let mut hours = ColumnDef::new(Alias::new("opening_hours"));
    hours.text();
    for def in [addr, hours] {
        add_column_if_missing(db, "shops", def).await?;
    }
    // v0.2 簡化：例外改 full 旗標（舊庫的 capacity_override 留著不用）
    let mut full = ColumnDef::new(Alias::new("full"));
    full.boolean().not_null().default(false);
    add_column_if_missing(db, "availability_exceptions", full).await?;

    // v0.4 搜尋：items 加 category 欄（舊庫補上，預設 other）
    let mut cat = ColumnDef::new(Alias::new("category"));
    cat.string().not_null().default("other");
    add_column_if_missing(db, "items", cat).await?;

    // 補索引（items 列表最常用；建表語句不含 index，這裡跨 DB 通用）
    for (table, col) in [
        ("items", "shop_id"),
        ("items", "status"),
        ("shops", "area_id"),
        ("orders", "buyer_id"),
        ("orders", "shop_id"),
        ("availability_rules", "item_id"),
        ("availability_exceptions", "item_id"),
        ("order_items", "order_id"),
        ("notifications", "user_id"),
    ] {
        let idx = Index::create()
            .if_not_exists()
            .name(format!("idx_{table}_{col}"))
            .table(Alias::new(table))
            .col(Alias::new(col))
            .to_owned();
        let builder = db.get_database_backend();
        let _ = db.execute(builder.build(&idx)).await;
    }
    // 唯一索引：週範本一天一條、例外一天一條
    for (name, table, cols) in [
        (
            "uq_rules_item_weekday",
            "availability_rules",
            vec!["item_id", "weekday"],
        ),
        (
            "uq_exc_item_date",
            "availability_exceptions",
            vec!["item_id", "date"],
        ),
    ] {
        let mut create = Index::create();
        create
            .if_not_exists()
            .unique()
            .name(name)
            .table(Alias::new(table));
        for c in cols {
            create.col(Alias::new(c));
        }
        let builder = db.get_database_backend();
        let _ = db.execute(builder.build(&create)).await;
    }
    Ok(())
}

async fn create_if_not_exists(db: &DbConn, stmt: TableCreateStatement) -> Result<(), DbErr> {
    let builder = db.get_database_backend();
    db.execute(builder.build(&stmt)).await?;
    Ok(())
}

async fn add_column_if_missing(db: &DbConn, table: &str, def: ColumnDef) -> Result<(), DbErr> {
    let alter = Table::alter()
        .table(Alias::new(table))
        .add_column(def)
        .to_owned();
    let builder = db.get_database_backend();
    match db.execute(builder.build(&alter)).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("already exists") || msg.contains("duplicate column") {
                // 欄位已在，無需處理
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}
