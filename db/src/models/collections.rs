use chrono::prelude::*;
use diesel;
use diesel::dsl;
use diesel::prelude::*;
use models::*;
use schema::collections;
use utils::errors::*;
use uuid::Uuid;

#[derive(Clone, Debug, Identifiable, PartialEq, Deserialize, Serialize, Queryable, QueryableByName)]
#[table_name = "collections"]
pub struct Collection {
    pub id: Uuid,
    pub name: String,
    pub user_id: Uuid,
    pub featured_collectible_id: Option<Uuid>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable, Deserialize)]
#[table_name = "collections"]
pub struct NewCollection {
    pub name: String,
    pub user_id: Uuid,
}

#[derive(AsChangeset, Clone, Deserialize, Serialize)]
#[table_name = "collections"]
pub struct UpdateCollectionAttributes {
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub featured_collectible_id: Option<Option<Uuid>>,
}

impl Collection {
    pub fn create(name: &str, user_id: Uuid) -> NewCollection {
        NewCollection {
            name: name.to_string().to_owned(),
            user_id,
        }
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<Collection, DatabaseError> {
        collections::table
            .find(id)
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load collection")
    }

    pub fn find_for_user(user_id: Uuid, conn: &PgConnection) -> Result<Vec<Collection>, DatabaseError> {
        collections::table
            .filter(collections::user_id.eq(user_id))
            .select(collections::all_columns)
            .order_by(collections::name.asc())
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load collections")
    }

    pub fn update(
        item: Self,
        attrs: UpdateCollectionAttributes,
        conn: &PgConnection,
    ) -> Result<Collection, DatabaseError> {
        diesel::update(&item)
            .set((attrs, collections::updated_at.eq(dsl::now)))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Error updating collection")
    }

    pub fn destroy(item: Self, conn: &PgConnection) -> Result<(), DatabaseError> {
        diesel::delete(collections::table.filter(collections::id.eq(item.id)))
            .execute(conn)
            .to_db_error(ErrorCode::DeleteError, "Error removing collection")?;

        Ok(())
    }
}

impl NewCollection {
    pub fn commit(self, conn: &PgConnection) -> Result<Collection, DatabaseError> {
        diesel::insert_into(collections::table)
            .values(self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create collection")
    }
}
