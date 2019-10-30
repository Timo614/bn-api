use bigneon_db::models::enums::*;
use bigneon_db::models::*;
use config::Config;
use diesel::PgConnection;
use errors::*;
use futures::future::Either;
use futures::Future;
use log::Level::Trace;
use tokio::prelude::*;
use utils::expo;
use utils::sendgrid::mail as sendgrid;
use utils::twilio;
use utils::webhook;

pub fn send_async(
    domain_action: &DomainAction,
    config: &Config,
    conn: &PgConnection,
) -> impl Future<Item = (), Error = BigNeonError> {
    let communication: Communication = match serde_json::from_value(domain_action.payload.clone()) {
        Ok(v) => v,
        Err(e) => return Either::A(future::err(e.into())),
    };

    match config.environment {
        //TODO Maybe remove this environment check and just rely on the BLOCK_EXTERNAL_COMMS .env
        Environment::Test => Either::A(future::ok(())), //Disable communication system when testing
        _ => {
            let res = match config.block_external_comms {
                true => {
                    jlog!(Trace, "Blocked communication", {
                        "communication": communication
                    });

                    Either::A(future::ok(()))
                }
                _ => {
                    let destination_addresses = communication.destinations.get();

                    let future = match communication.comm_type {
                        CommunicationType::Email => sendgrid::send_email_async(
                            &config.sendgrid_api_key,
                            communication.source.as_ref().unwrap().get_first().unwrap(),
                            destination_addresses,
                            communication.title.clone(),
                            communication.body.clone(),
                            communication.categories.clone(),
                            communication.extra_data.clone(),
                        ),
                        CommunicationType::EmailTemplate => {
                            // Short circuit logic if communication template and template is blank
                            if communication.template_id == Some("".to_string()) {
                                jlog!(Trace, "Blocked communication, blank template ID", {
                                    "communication": communication
                                });
                                Box::new(future::ok(()))
                            } else {
                                sendgrid::send_email_template_async(
                                    &config.sendgrid_api_key,
                                    communication.source.as_ref().unwrap().get_first().unwrap(),
                                    &destination_addresses,
                                    communication.template_id.clone().unwrap(),
                                    communication.template_data.as_ref().unwrap(),
                                    communication.categories.clone(),
                                    communication.extra_data.clone(),
                                )
                            }
                        }
                        CommunicationType::Sms => twilio::send_sms_async(
                            &config.twilio_account_id,
                            &config.twilio_api_key,
                            communication.source.as_ref().unwrap().get_first().unwrap(),
                            destination_addresses,
                            &communication.body.unwrap_or(communication.title),
                        ),
                        CommunicationType::Push => expo::send_push_notification_async(
                            &destination_addresses,
                            &communication.body.unwrap_or(communication.title),
                        ),
                        CommunicationType::Webhook => webhook::send_webhook_async(
                            &destination_addresses,
                            &communication.body.unwrap_or(communication.title),
                            domain_action.main_table_id,
                            conn,
                            &config,
                        ),
                    };
                    Either::B(future)
                }
            };
            res
        }
    }
}
