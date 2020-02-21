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
extern crate diesel;
#[macro_use]
extern crate macros;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate globee;
extern crate jsonwebtoken as jwt;
extern crate serde;
extern crate uuid;
extern crate validator;

mod functional;
mod support;
mod unit;
