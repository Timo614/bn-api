SELECT COUNT(*) OVER ()                                                                                                     AS total
        ,o.id                                                                                                               AS order_id
        ,u.first_name                                                                                                       AS customer_name_first
        ,u.last_name                                                                                                        AS customer_name_last
        ,u.email                                                                                                            AS customer_email_address
        ,e.name                                                                                                             AS event_name
        ,e.event_start                                                                                                      AS event_date
        ,tt.name                                                                                                            AS ticket_type_name
        ,o.paid_at                                                                                                          AS transaction_date
        ,o.platform                                                                                                         AS point_of_sale
        ,p.payment_method                                                                                                   AS payment_method
        ,oi_tickets.quantity                                                                                                AS qty_tickets_sold
        ,oi_tickets.refunded_quantity                                                                                       AS qty_tickets_refunded
        ,(oi_tickets.quantity - oi_tickets.refunded_quantity)                                                               AS qty_tickets_sold_net
        ,CAST(oi_tickets.unit_price_in_cents
                + COALESCE(oi_promo_code_discount.unit_price_in_cents, 0) AS BIGINT)                                        AS face_price_in_cents
        ,CAST(
                ((oi_tickets.unit_price_in_cents + COALESCE(oi_promo_code_discount.unit_price_in_cents, 0))
                * (COALESCE(oi_tickets.quantity, 0) - COALESCE(oi_tickets.refunded_quantity, 0)))
                AS BIGINT)                                                                                                  AS total_face_value_in_cents
        ,CAST(
                (COALESCE(oi_per_unit_fees.client_fee_in_cents, 0) + COALESCE(oi_event_fees.client_fee_in_cents, 0))
                AS bigint)                                                                                                  AS client_per_ticket_revenue_in_cents
        ,CAST(
                (COALESCE(oi_per_unit_fees.client_fee_in_cents, 0)
                        * (COALESCE(oi_per_unit_fees.quantity, 0) - COALESCE(oi_per_unit_fees.refunded_quantity, 0))) +
                (COALESCE(oi_event_fees.client_fee_in_cents, 0)
                        * (COALESCE(oi_event_fees.quantity, 0) - COALESCE(oi_event_fees.refunded_quantity, 0)))
                AS bigint)                                                                                                  AS client_per_order_revenue_in_cents
        ,CAST(
                COALESCE(oi_per_unit_fees.company_fee_in_cents, 0) + COALESCE(oi_event_fees.company_fee_in_cents, 0)
                AS bigint)                                                                                                  AS company_per_ticket_revenue_in_cents
        ,CAST(
                (COALESCE(oi_per_unit_fees.company_fee_in_cents, 0)
                        * (COALESCE(oi_per_unit_fees.quantity, 0) - COALESCE(oi_per_unit_fees.refunded_quantity, 0))) +
                (COALESCE(oi_event_fees.company_fee_in_cents, 0)
                        * (COALESCE(oi_event_fees.quantity, 0) - COALESCE(oi_event_fees.refunded_quantity, 0)))
                AS bigint)                                                                                                  AS company_per_order_revenue_in_cents
        ,CAST(
                (COALESCE(oi_credit_card_fees.unit_price_in_cents, 0)
                        * (COALESCE(oi_credit_card_fees.quantity, 0) - COALESCE(oi_credit_card_fees.refunded_quantity, 0)))
                AS bigint)                                                                                                  AS credit_card_processing_fees_in_cents
        ,CAST(
                -- face
                ((oi_tickets.unit_price_in_cents + COALESCE(oi_promo_code_discount.unit_price_in_cents, 0))
                        * (COALESCE(oi_tickets.quantity, 0) - COALESCE(oi_tickets.refunded_quantity, 0))) +
                -- client share
                (COALESCE(oi_per_unit_fees.client_fee_in_cents, 0)
                        * (COALESCE(oi_per_unit_fees.quantity, 0) - COALESCE(oi_per_unit_fees.refunded_quantity, 0))) +
                (COALESCE(oi_event_fees.client_fee_in_cents, 0)
                        * (COALESCE(oi_event_fees.quantity, 0) - COALESCE(oi_event_fees.refunded_quantity, 0))) +
                -- company share
                (COALESCE(oi_per_unit_fees.company_fee_in_cents, 0)
                        * (COALESCE(oi_per_unit_fees.quantity, 0) - COALESCE(oi_per_unit_fees.refunded_quantity, 0))) +
                (COALESCE(oi_event_fees.company_fee_in_cents, 0)
                        * (coalesce(oi_event_fees.quantity, 0) - coalesce(oi_event_fees.refunded_quantity, 0))) +
                -- credit card fees
                (COALESCE(oi_credit_card_fees.unit_price_in_cents, 0)
                        * (COALESCE(oi_credit_card_fees.quantity, 0) - COALESCE(oi_credit_card_fees.refunded_quantity, 0)))
         AS BIGINT)                                                                                                         AS gross
FROM orders o
INNER JOIN users u
        ON o.user_id = u.id
        OR o.on_behalf_of_user_id = u.id
INNER JOIN order_items oi_tickets
        ON o.id = oi_tickets.order_id
        AND oi_tickets.item_type = 'Tickets' -- sale of actual ticket
LEFT JOIN order_items oi_per_unit_fees
        ON (oi_per_unit_fees.item_type = 'PerUnitFees'
        AND oi_tickets.id = oi_per_unit_fees.parent_id)
LEFT JOIN order_items oi_event_fees
        ON (oi_event_fees.item_type = 'EventFees'
        AND o.id = oi_event_fees.order_id)
LEFT JOIN order_items oi_promo_code_discount
        ON (oi_promo_code_discount.item_type = 'Discount'
        AND oi_tickets.id = oi_promo_code_discount.parent_id)
LEFT JOIN order_items oi_credit_card_fees
        ON (oi_credit_card_fees.item_type = 'CreditCardFees'
        AND oi_tickets.order_id = oi_credit_card_fees.order_id)
INNER JOIN events e ON oi_tickets.event_id = e.id
INNER JOIN organizations org ON e.organization_id = org.id
INNER JOIN ticket_types tt ON oi_tickets.ticket_type_id = tt.id
INNER JOIN payments p ON o.id = p.order_id
        AND p.status = 'Completed'
WHERE o.status = 'Paid'
AND org.name not like 'Private -%' -- filters out internal test orgs
        AND($1 IS NULL OR o.paid_at >= $1)
        AND($2 IS NULL OR o.paid_at <= $2)
        AND($3 IS NULL OR e.event_start >= $3)
        AND($4 IS NULL OR e.event_start <= $4)
ORDER BY o.paid_at DESC
limit $6
    offset $5;