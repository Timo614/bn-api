#![deny(unreachable_patterns)]
#![deny(unknown_lints)]
#![deny(unused_variables)]
#![deny(unused_imports)]
// Unused results is more often than not an error
#![deny(unused_must_use)]
#![deny(unused_extern_crates)]
extern crate actix_web;
extern crate bigneon_api;
extern crate bigneon_db;
extern crate chrono;
extern crate crypto;
extern crate diesel;
//extern crate dotenv;
extern crate lettre;
//extern crate lettre_email;
//extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate jwt;
extern crate uuid;
extern crate validator;

mod functional;
mod support;
mod unit;
