extern crate chrono;
extern crate facebook;

use chrono::prelude::*;
use facebook::prelude::*;

fn main() {
    let access_token = "".to_string(); // Get from Graph Explorer tool - https://developers.facebook.com/tools/explorer
    let fb = FacebookClient::from_access_token(access_token);

    let _accounts = fb.me.accounts.list().unwrap();

    let event = Event {
        name: "Hello world".to_string(),
        category: Category::MusicEvent,
        description: "This is a test event".to_string(),
        start_time: Utc::now().naive_utc().to_string(),
        ticket_uri: None,
        timezone: "Africa/Harare".to_string(),
        cover: Some(CoverPhoto {
            source: "http://noimg.com".to_string(),
            offset_x: 0,
            offset_y: 0,
        }),
        place_id: Some(FBID("http://www.facebook.com/pages/<page_id>".to_string())),

        address: None,
    };
    fb.official_events.create(event).unwrap();
}
// Example json to use at https://developers.facebook.com/tools/explorer
//
/*
{
    "category": "WORKSHOP",
    "name": "Test",
    "description": "Test",
    "cover": {
        "source": "https://source.unsplash.com/random"
    },
    "place_id":

        "1078236045577061",
    "timezone": "UTC",
    "start_time": "1 Jan 2021"
}
*/
