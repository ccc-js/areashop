#![allow(unused_imports)]
pub mod area;
pub mod order;
pub mod order_item;
pub mod product;
pub mod shop;
pub mod user;

pub use area::Entity as Area;
pub use order::Entity as Order;
pub use order_item::Entity as OrderItem;
pub use product::Entity as Product;
pub use shop::Entity as Shop;
pub use user::Entity as User;
