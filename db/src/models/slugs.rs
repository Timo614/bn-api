use chrono::NaiveDateTime;
use diesel;
use diesel::prelude::*;
use models::*;
use regex::Regex;
use schema::slugs;
use unidecode::unidecode;
use utils::errors::*;
use utils::rand::random_alpha_string;
use utils::regexes;
use uuid::Uuid;

#[derive(Clone, Deserialize, Identifiable, Queryable, PartialEq, Debug, Serialize)]
pub struct Slug {
    pub id: Uuid,
    pub slug: String,
    pub main_table: Tables,
    pub main_table_id: Uuid,
    pub slug_type: SlugTypes,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable, Deserialize)]
#[table_name = "slugs"]
pub struct NewSlug {
    pub slug: String,
    pub main_table: Tables,
    pub main_table_id: Uuid,
    pub slug_type: SlugTypes,
}

#[derive(Debug, PartialEq)]
pub enum SlugContext {
    Event {
        id: Uuid,
        name: String,
        venue: Option<Venue>,
    },
    Organization {
        id: Uuid,
        name: String,
    },
    Venue {
        id: Uuid,
        name: String,
        city: String,
        state: String,
        country: String,
    },
}

impl Slug {
    pub fn create(
        slug: String,
        main_table: Tables,
        main_table_id: Uuid,
        slug_type: SlugTypes,
    ) -> NewSlug {
        NewSlug {
            slug,
            main_table,
            main_table_id,
            slug_type,
        }
    }

    pub fn primary_slug(
        main_table_id: Uuid,
        main_table: Tables,
        conn: &PgConnection,
    ) -> Result<Slug, DatabaseError> {
        let mut slugs = Slug::load_primary_slugs(&vec![main_table_id], main_table, conn)?;

        if slugs.is_empty() {
            return DatabaseError::business_process_error("Unable to load primary slug");
        }

        Ok(slugs.remove(0))
    }

    pub fn load_primary_slugs(
        main_table_ids: &[Uuid],
        main_table: Tables,
        conn: &PgConnection,
    ) -> Result<Vec<Slug>, DatabaseError> {
        use schema::*;

        match main_table {
            Tables::Events => events::table
                .inner_join(slugs::table.on(events::slug_id.eq(slugs::id.nullable())))
                .filter(events::id.eq_any(main_table_ids))
                .select(slugs::all_columns)
                .load(conn)
                .to_db_error(ErrorCode::QueryError, "Error loading slug"),
            Tables::Organizations => organizations::table
                .inner_join(slugs::table.on(organizations::slug_id.eq(slugs::id.nullable())))
                .filter(organizations::id.eq_any(main_table_ids))
                .select(slugs::all_columns)
                .load(conn)
                .to_db_error(ErrorCode::QueryError, "Error loading slug"),
            Tables::Venues => venues::table
                .inner_join(slugs::table.on(venues::slug_id.eq(slugs::id.nullable())))
                .filter(venues::id.eq_any(main_table_ids))
                .select(slugs::all_columns)
                .load(conn)
                .to_db_error(ErrorCode::QueryError, "Error loading slug"),
            _ => return DatabaseError::business_process_error("Unable to load primary slug"),
        }
    }

    pub fn destroy(
        main_table_id: Uuid,
        main_table: Tables,
        slug_type: SlugTypes,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        diesel::delete(
            slugs::table
                .filter(slugs::main_table_id.eq(main_table_id))
                .filter(slugs::main_table.eq(main_table))
                .filter(slugs::slug_type.eq(slug_type)),
        )
        .execute(conn)
        .to_db_error(ErrorCode::DeleteError, "Error removing slug")?;

        Ok(())
    }

    pub fn find_by_type(
        main_table_id: Uuid,
        main_table: Tables,
        slug_type: SlugTypes,
        conn: &PgConnection,
    ) -> Result<Slug, DatabaseError> {
        slugs::table
            .filter(slugs::main_table_id.eq(main_table_id))
            .filter(slugs::main_table.eq(main_table))
            .filter(slugs::slug_type.eq(slug_type))
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading slug")
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<Slug, DatabaseError> {
        slugs::table
            .find(id)
            .first::<Slug>(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading slug")
    }

    pub fn find_by_slug_type(
        slug_type: &str,
        conn: &PgConnection,
    ) -> Result<Vec<Slug>, DatabaseError> {
        slugs::table
            .filter(slugs::slug_type.eq(slug_type))
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading slug")
    }
    pub fn find_all(ids: Vec<Uuid>, conn: &PgConnection) -> Result<Vec<Slug>, DatabaseError> {
        slugs::table
            .filter(slugs::id.eq_any(ids))
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading slug")
    }

    pub fn find_by_slug(slug: &str, conn: &PgConnection) -> Result<Vec<Slug>, DatabaseError> {
        slugs::table
            .filter(slugs::slug.eq(slug))
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading slug")
    }

    pub fn find_first_for_city(
        city: &str,
        state: &str,
        country: &str,
        conn: &PgConnection,
    ) -> Result<Slug, DatabaseError> {
        use schema::*;
        // Find slug by city, state, and country
        slugs::table
            .inner_join(
                venues::table.on(venues::id
                    .eq(slugs::main_table_id)
                    .and(slugs::main_table.eq(Tables::Venues))),
            )
            .filter(slugs::slug_type.eq(SlugTypes::City))
            .filter(venues::city.eq(city))
            .filter(venues::state.eq(state))
            .filter(venues::country.eq(country))
            .select(slugs::all_columns)
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading slug")
    }

    pub fn generate_slug(
        slug_context: &SlugContext,
        slug_type: SlugTypes,
        conn: &PgConnection,
    ) -> Result<Slug, DatabaseError> {
        let main_table_id: Option<Uuid>;
        let main_table: Option<Tables>;
        let slug_name: Option<String>;

        match slug_context {
            SlugContext::Event {
                id,
                ref name,
                ref venue,
            } => {
                main_table_id = Some(*id);
                main_table = Some(Tables::Events);
                slug_name = Some(match venue {
                    Some(venue) => format!("{} at {} {}", &name, venue.name, venue.city),
                    None => name.clone(),
                });
            }
            SlugContext::Organization { id, ref name } => {
                main_table_id = Some(*id);
                main_table = Some(Tables::Organizations);
                slug_name = Some(name.clone());
            }
            SlugContext::Venue {
                id,
                ref name,
                ref city,
                ..
            } => {
                main_table_id = Some(*id);
                main_table = Some(Tables::Venues);

                if slug_type == SlugTypes::City {
                    slug_name = Some(city.clone());
                } else {
                    slug_name = Some(name.clone());
                }
            }
        }

        // Sanity check
        if main_table.is_none() || main_table_id.is_none() || slug_name.is_none() {
            return DatabaseError::business_process_error("Unable to generate slug");
        }

        let mut slug_record = None;
        match slug_type {
            SlugTypes::City => match slug_context {
                SlugContext::Venue {
                    city,
                    state,
                    country,
                    ..
                } => {
                    slug_record =
                        Slug::find_first_for_city(&city, &state, &country, conn).optional()?;
                }
                _ => (),
            },
            _ => (),
        }

        // If slug record is matched duplicate it for this type
        match slug_record {
            Some(slug_record) => Slug::create(
                slug_record.slug,
                main_table.unwrap(),
                main_table_id.unwrap(),
                slug_type,
            )
            .commit(conn),
            None => {
                let mut slug = Slug::create_slug(&slug_name.unwrap());
                loop {
                    let existing = Slug::find_by_slug(&slug, conn)?;
                    if existing.is_empty() {
                        break;
                    }
                    slug = format!("{}-{}", &slug, random_alpha_string(5));
                }

                Slug::create(slug, main_table.unwrap(), main_table_id.unwrap(), slug_type)
                    .commit(conn)
            }
        }
    }

    fn create_slug(name: &str) -> String {
        // Unwrap should be treated as a compile time error

        let only_characters = Regex::new(r#"[^a-zA-Z0-9]"#).unwrap();
        let duplicate_dashes = Regex::new(r#"-+"#).unwrap();

        let slug = unidecode(name);
        let slug = only_characters.replace_all(&slug, " ");
        let mut slug: String = duplicate_dashes
            .replace_all(&regexes::whitespace().replace_all(&slug.trim(), "-"), "-")
            .to_lowercase()
            .chars()
            .take(250)
            .collect();

        // If the slug is empty, generate a short random string
        if slug.len() == 0 {
            slug = random_alpha_string(5);
        }
        slug
    }
}

impl NewSlug {
    pub fn commit(self, conn: &PgConnection) -> Result<Slug, DatabaseError> {
        diesel::insert_into(slugs::table)
            .values(self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create slug")
    }
}
