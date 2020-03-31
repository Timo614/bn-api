use chrono::prelude::*;
use diesel;
use diesel::dsl;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Nullable, Timestamp, Uuid as dUuid};
use models::*;
use schema::collection_items;
use utils::errors::*;
use uuid::Uuid;

#[derive(Clone, Debug, Identifiable, PartialEq, Deserialize, Serialize, Queryable, QueryableByName)]
#[table_name = "collection_items"]
pub struct CollectionItem {
    pub id: Uuid,
    pub collection_id: Uuid,
    pub collectible_id: Uuid,                  //ticket_type_id
    pub next_collection_item_id: Option<Uuid>, // for ordering in UI
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Deserialize, PartialEq, Serialize, QueryableByName)]
pub struct CollectionItemWithNumOwned {
    #[sql_type = "dUuid"]
    pub id: Uuid,
    #[sql_type = "dUuid"]
    pub collection_id: Uuid,
    #[sql_type = "dUuid"]
    pub collectible_id: Uuid,
    #[sql_type = "Nullable<dUuid>"]
    pub next_collection_item_id: Option<Uuid>, // for ordering in UI
    #[sql_type = "Timestamp"]
    pub created_at: NaiveDateTime,
    #[sql_type = "Timestamp"]
    pub updated_at: NaiveDateTime,
    #[sql_type = "BigInt"]
    pub number_owned: i64,
}

#[derive(Debug, Insertable, Deserialize)]
#[table_name = "collection_items"]
pub struct NewCollectionItem {
    pub collection_id: Uuid,
    pub collectible_id: Uuid,
}

#[derive(AsChangeset, Clone, Deserialize, Serialize)]
#[table_name = "collection_items"]
pub struct UpdateCollectionItemAttributes {
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub next_collection_item_id: Option<Option<Uuid>>,
}

impl CollectionItem {
    pub fn create(collection_id: Uuid, collectible_id: Uuid) -> NewCollectionItem {
        NewCollectionItem {
            collection_id,
            collectible_id,
        }
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<CollectionItem, DatabaseError> {
        collection_items::table
            .find(id)
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load collection item")
    }

    pub fn find_for_collection_with_num_owned(
        id: Uuid,
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<CollectionItemWithNumOwned>, DatabaseError> {
        let query = r#"select ci.*, number_owned
from collection_items ci
inner join collections c on ci.collection_id = c.id
inner join users u on c.user_id = u.id
INNER JOIN (
        select count(ti.id) as number_owned, tt.id
        from ticket_instances ti
        inner join order_items oi on ti.order_item_id = oi.id
        inner join orders o on oi.order_id = o.id and o.user_id = $2
        inner join ticket_types tt on oi.ticket_type_id = tt.id
        group by tt.id
) as tic on ci.collectible_id = tic.id
where c.id = $1
and u.id = $2"#;

        let results: Vec<CollectionItemWithNumOwned> = diesel::sql_query(query)
            .bind::<diesel::sql_types::Uuid, _>(id)
            .bind::<diesel::sql_types::Uuid, _>(user_id)
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load collection items")?;
        Ok(results)
    }

    pub fn update(
        item: Self,
        attrs: UpdateCollectionItemAttributes,
        conn: &PgConnection,
    ) -> Result<CollectionItem, DatabaseError> {
        diesel::update(&item)
            .set((attrs, collection_items::updated_at.eq(dsl::now)))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Error updating collection item")
    }

    pub fn destroy(item: Self, conn: &PgConnection) -> Result<(), DatabaseError> {
        diesel::delete(collection_items::table.filter(collection_items::id.eq(item.id)))
            .execute(conn)
            .to_db_error(ErrorCode::DeleteError, "Error removing collection item")?;

        Ok(())
    }
}

impl NewCollectionItem {
    pub fn commit(self, conn: &PgConnection) -> Result<CollectionItem, DatabaseError> {
        diesel::insert_into(collection_items::table)
            .values(self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create collection item")
    }
}
