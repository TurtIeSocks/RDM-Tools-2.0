//! SeaORM Entity. Generated by sea-orm-codegen 0.10.1

use std::collections::HashMap;

use crate::api::{ToMultiStruct, ToMultiVec, ToPointStruct, ToSingleStruct, ToSingleVec};

use super::{
    sea_orm_active_enums::Type, utils, Feature, FeatureCollection, NameType, NameTypeId, Order,
    QueryOrder, RdmInstanceArea,
};

use sea_orm::{entity::prelude::*, sea_query::Expr, QuerySelect, Set};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "instance")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub name: String,
    pub r#type: Type,
    #[sea_orm(column_type = "Custom(\"LONGTEXT\".to_owned())")]
    pub data: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::device::Entity")]
    Device,
}

impl Related<super::device::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Device.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

struct Instance {
    name: String,
    // r#type: Type,
    data: HashMap<String, Value>,
}

pub struct Query;

impl Query {
    pub async fn all(
        conn: &DatabaseConnection,
        instance_type: Option<String>,
    ) -> Result<Vec<NameTypeId>, DbErr> {
        let instance_type = utils::get_enum(instance_type);
        let items = if let Some(instance_type) = instance_type {
            Entity::find()
                .filter(Column::Type.eq(instance_type))
                .select_only()
                .column(Column::Name)
                .column_as(Column::Type, "instance_type")
                .order_by(Column::Name, Order::Asc)
                .into_model::<NameType>()
                .all(conn)
                .await?
        } else {
            Entity::find()
                .select_only()
                .column(Column::Name)
                .column_as(Column::Type, "instance_type")
                .order_by(Column::Name, Order::Asc)
                .into_model::<NameType>()
                .all(conn)
                .await?
        };
        Ok(items
            .into_iter()
            .enumerate()
            .map(|(i, item)| NameTypeId {
                id: i as u32,
                name: item.name,
                r#type: item.instance_type,
            })
            .collect())
    }

    pub async fn route(
        conn: &DatabaseConnection,
        instance_name: &String,
    ) -> Result<Feature, DbErr> {
        let items = Entity::find()
            .filter(Column::Name.eq(instance_name.trim().to_string()))
            .one(conn)
            .await?;
        if let Some(items) = items {
            Ok(utils::normalize::instance(items))
        } else {
            Err(DbErr::Custom("Instance not found".to_string()))
        }
    }

    pub async fn save(
        conn: &DatabaseConnection,
        area: FeatureCollection,
    ) -> Result<(usize, usize), DbErr> {
        let existing = Entity::find().all(conn).await?;
        let mut existing: Vec<Instance> = existing
            .into_iter()
            .map(|x| Instance {
                name: x.name,
                // r#type: x.r#type,
                data: serde_json::from_str(&x.data).unwrap(),
            })
            .collect();

        let mut inserts: Vec<ActiveModel> = vec![];
        let mut update_len = 0;

        for feat in area.into_iter() {
            if let Some(name) = feat.property("name") {
                if let Some(name) = name.as_str() {
                    let r#type = if let Some(instance_type) = feat.property("type") {
                        if let Some(instance_type) = instance_type.as_str() {
                            utils::get_enum(Some(instance_type.to_string()))
                        } else {
                            utils::get_enum_by_geometry(&feat.geometry.as_ref().unwrap().value)
                        }
                    } else {
                        utils::get_enum_by_geometry(&feat.geometry.as_ref().unwrap().value)
                    };
                    if let Some(r#type) = r#type {
                        let area = match r#type {
                            Type::CirclePokemon
                            | Type::CircleSmartPokemon
                            | Type::CircleRaid
                            | Type::CircleSmartRaid
                            | Type::ManualQuest => RdmInstanceArea::Single(
                                feat.clone().to_single_vec().to_single_struct(),
                            ),
                            Type::Leveling => {
                                RdmInstanceArea::Leveling(feat.clone().to_single_vec().to_struct())
                            }
                            Type::AutoQuest
                            | Type::PokemonIv
                            | Type::AutoPokemon
                            | Type::AutoTth => RdmInstanceArea::Multi(
                                feat.clone().to_multi_vec().to_multi_struct(),
                            ),
                        };
                        let new_area = json!(area);
                        let name = name.to_string();
                        let is_update = existing.iter_mut().find(|entry| entry.name == name);

                        if let Some(entry) = is_update {
                            entry.data.insert("area".to_string(), new_area);
                            Entity::update_many()
                                .col_expr(Column::Data, Expr::value(json!(entry.data).to_string()))
                                .col_expr(Column::Type, Expr::value(r#type))
                                .filter(Column::Name.eq(entry.name.to_string()))
                                .exec(conn)
                                .await?;
                            update_len += 1;
                        } else {
                            let mut active_model = ActiveModel {
                                name: Set(name.to_string()),
                                // r#type: Set(r#type),
                                // data: Set(json!({ "area": new_area }).to_string()),
                                ..Default::default()
                            };
                            active_model
                                .set_from_json(json!({
                                    "name": name,
                                    "type": r#type,
                                    "data": json!({ "area": new_area }).to_string(),
                                }))
                                .unwrap();

                            inserts.push(active_model)
                        }
                    }
                }
            }
        }
        let insert_len = inserts.len();
        if !inserts.is_empty() {
            Entity::insert_many(inserts).exec(conn).await?;
        }
        Ok((insert_len, update_len))
    }
}
