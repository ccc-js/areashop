//! 彈性 DB schema：用 SeaORM `SchemaManager::create_table` 建表，
//! 同一份程式碼可在 SQLite（本地測試）與 PostgreSQL（擴大規模）上執行，
//! 不寫任何 `AUTOINCREMENT vs SERIAL`、`JSONB vs TEXT` 之類的方言 SQL。
//! 舊庫升級：新欄位用 has_column + ALTER TABLE ADD COLUMN 補上（兩邊通用）。

use sea_orm::{sea_query::*, ConnectionTrait, DbConn, DbErr};

use crate::entities::{area, order, order_item, product, shop, user};

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
        schema
            .create_table_from_entity(product::Entity)
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
    let mut mode = ColumnDef::new(Alias::new("pickup_mode"));
    mode.string().not_null().default("meetup");
    for def in [addr, hours, mode] {
        let alter = Table::alter()
            .table(Alias::new("shops"))
            .add_column(def)
            .to_owned();
        let builder = db.get_database_backend();
        match db.execute(builder.build(&alter)).await {
            Ok(_) => {}
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("already exists") || msg.contains("duplicate column") {
                    // 欄位已在，無需處理
                } else {
                    return Err(e);
                }
            }
        }
    }

    // 補索引（products 列表最常用；建表語句不含 index，這裡跨 DB 通用）
    for (table, col) in [
        ("products", "shop_id"),
        ("products", "status"),
        ("shops", "area_id"),
        ("orders", "buyer_id"),
        ("orders", "shop_id"),
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
    Ok(())
}

async fn create_if_not_exists(db: &DbConn, stmt: TableCreateStatement) -> Result<(), DbErr> {
    let builder = db.get_database_backend();
    db.execute(builder.build(&stmt)).await?;
    Ok(())
}
