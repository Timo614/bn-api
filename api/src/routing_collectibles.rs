use crate::controllers::*;
use actix_web::web;

pub fn routes_collectibles(app: &mut web::ServiceConfig) {
    app.service(
        web::resource("/collections")
            .route(web::post().to(collections::create))
            .route(web::get().to(collections::index)),
    )
    .service(web::resource("/collections/{id}").route(web::delete().to(collections::delete)))
    .service(
        web::resource("/collections/{id}/items")
            .route(web::post().to(collection_items::create))
            .route(web::get().to(collection_items::index)),
    )
    .service(
        web::resource("/collections/items/{id}")
            .route(web::put().to(collection_items::update))
            .route(web::delete().to(collection_items::delete)),
    );
}
