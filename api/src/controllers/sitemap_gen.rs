use actix_web::HttpResponse;
use bigneon_db::prelude::{Event, Slug};
use db::Connection;
use errors::BigNeonError;
use std::env;
use utils::gen_sitemap;

pub fn index(connection: Connection) -> Result<HttpResponse, BigNeonError> {
    //find all active events
    let events = Event::find_all_active_events(connection.get())?;
    let slug_ids = events
        .iter()
        .map(|e| e.slug_id)
        .filter(|s| s.is_some())
        .map(|s| s.unwrap())
        .collect::<Vec<_>>();

    let slugs = Slug::find_all(slug_ids, connection.get())?;

    let slug_s = slugs
        .iter()
        .map(|slug| create_url(&slug.slug))
        .collect::<Vec<_>>();

    let sitemap_xml = gen_sitemap::create_sitemap(&slug_s)?;

    Ok(HttpResponse::Ok()
        .content_type("text/xml")
        .body(sitemap_xml)
        .into())
}

fn create_url(slug: &str) -> String {
    format!("{}/tickets/{}", env::var("FRONT_END_URL").unwrap(), slug)
}
