use bigneon_db::models::enums::OrderItemTypes;
use bigneon_db::models::DisplayOrder;
use config::Config;
use diesel::PgConnection;
use errors::*;
use utils::communication::*;

pub fn refund_email(
    user_first_name: &String,
    user_email: String,
    display_order: DisplayOrder,
    amount_refunded: i64,
    config: &Config,
    conn: &PgConnection,
) -> Result<(), BigNeonError> {
    let source = CommAddress::from(config.communication_default_source_email.clone());
    let destinations = CommAddress::from(user_email);
    let title = "BigNeon Refund".to_string();
    let template_id = config.sendgrid_template_bn_refund.clone();
    let mut template_data = TemplateData::new();
    template_data.insert(String::from("name"), user_first_name.clone());
    //Construct an itemised breakdown using a HTML table
    let mut item_breakdown = r#"<table style="width:100%"><tbody>"#.to_string();
    item_breakdown
        .push_str("<tr><th>Units</th><th>Units Refunded</th><th>Description</th><th>Unit Price</th><th>Total</th></tr>");
    let mut total_fees = 0;
    for oi in &display_order.items {
        if oi.item_type == OrderItemTypes::Tickets {
            item_breakdown.push_str(r#"<tr><th align="center">"#);
            item_breakdown.push_str(&oi.quantity.to_string());
            item_breakdown.push_str("</th><th>");
            item_breakdown.push_str(&oi.refunded_quantity.to_string());
            item_breakdown.push_str("</th><th>");
            item_breakdown.push_str(&oi.description);
            item_breakdown.push_str(r#"</th><th align="right">$"#);
            item_breakdown.push_str(&format!("{:.*}", 2, oi.unit_price_in_cents as f64 / 100.0));
            item_breakdown.push_str(r#"</th><th align="right">$"#);
            item_breakdown.push_str(&format!(
                "{:.*}",
                2,
                ((oi.quantity - oi.refunded_quantity) * oi.unit_price_in_cents) as f64 / 100.0
            ));
            item_breakdown.push_str("</th></tr>");
        } else {
            //Accumulate fees
            total_fees += (oi.quantity - oi.refunded_quantity) * oi.unit_price_in_cents;
        }
    }
    item_breakdown.push_str("</tbody></table>");

    template_data.insert("amount_refunded".to_string(), amount_refunded.to_string());
    template_data.insert(
        "ticket_count".to_string(),
        display_order
            .items
            .iter()
            .filter(|i| i.item_type == OrderItemTypes::Tickets)
            .map(|i| i.quantity - i.refunded_quantity)
            .sum::<i64>()
            .to_string(),
    );
    template_data.insert(
        "total_fees".to_string(),
        format!("{:.*}", 2, total_fees as f64 / 100.0),
    );
    template_data.insert(
        "total_price".to_string(),
        format!("{:.*}", 2, display_order.total_in_cents as f64 / 100.0),
    );
    template_data.insert("item_breakdown".to_string(), item_breakdown);
    template_data.insert(
        "tickets_link".to_string(),
        format!("{}/hub", config.front_end_url),
    );

    // TODO: Perhaps move this to an event subscription
    Communication::new(
        CommunicationType::EmailTemplate,
        title,
        None,
        Some(source),
        destinations,
        Some(template_id),
        Some(vec![template_data]),
    )
    .queue(conn)
}
