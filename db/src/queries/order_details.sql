SELECT ticket_instance_id,
       order_item_id,
       description,
       ticket_price_in_cents,
       fees_price_in_cents,
       (ticket_price_in_cents + fees_price_in_cents) AS total_price_in_cents,
       status,
       status IN ('Purchased', 'Redeemed') AND (is_allowed_to_refund OR $4) AS refundable,
       CASE WHEN status <> 'Refunded' THEN attendee_email ELSE NULL END AS attendee_email,
       CASE WHEN status <> 'Refunded' THEN attendee_id ELSE NULL END AS attendee_id,
       CASE WHEN status <> 'Refunded' THEN attendee_first_name ELSE NULL END AS attendee_first_name,
       CASE WHEN status <> 'Refunded' THEN attendee_last_name ELSE NULL END AS attendee_last_name,
       attendee_id,
       attendee_first_name,
       attendee_last_name,
       ticket_type_id,
       ticket_type_name,
       code,
       code_type,
       pending_transfer_id,
       discount_price_in_cents
FROM (
         SELECT t.ticket_instance_id,
                t.order_item_id                    AS order_item_id,
                CASE
                    WHEN oi.item_type = 'EventFees' THEN 'Event Fees - ' || e.name
                  WHEN oi.item_type = 'CreditCardFees' THEN 'Credit Card Fees'
                    ELSE e.name || ' - ' || tt.name
                    END                            AS description,
                CASE
                    WHEN oi.item_type  = 'Tickets'
                    THEN oi.unit_price_in_cents ELSE 0
                    END                            AS ticket_price_in_cents,
                CASE
                    WHEN fi.unit_price_in_cents IS NULL THEN oi.company_fee_in_cents + oi.client_fee_in_cents
                    ELSE fi.unit_price_in_cents
                    END                            AS fees_price_in_cents,
                CASE
                    WHEN oi.quantity = oi.refunded_quantity OR rt.ticket_refunded_at IS NOT NULL THEN 'Refunded'
                    WHEN w.user_id <> COALESCE(o.on_behalf_of_user_id, o.user_id) THEN 'Transferred'
                    WHEN tfs.id IS NOT NULL THEN 'In Transfer'
                    WHEN ti.status IS NULL THEN 'Purchased'
                    ELSE ti.status
                    END                            AS status,
                e.settled_at,
                wallet_owner.email                 AS attendee_email,
                wallet_owner.id                    AS attendee_id,
                wallet_owner.first_name            AS attendee_first_name,
                wallet_owner.last_name             AS attendee_last_name,
                tt.id                              AS ticket_type_id,
                tt.name                            AS ticket_type_name,
                coalesce(h.redemption_code, c.redemption_code)           AS code,
                coalesce(h.hold_type, c.code_type) AS code_type,
                tfs.id                             AS pending_transfer_id,
                dis.unit_price_in_cents            AS discount_price_in_cents,
                orgs.is_allowed_to_refund          AS is_allowed_to_refund

         FROM (
                  SELECT DISTINCT ticket_instance_id, order_item_id
                  FROM (
                           SELECT NULL AS ticket_instance_id, oi.id AS order_item_id
                           FROM order_items oi
                           WHERE oi.order_id = $1
                             AND oi.item_type in ('EventFees', 'CreditCardFees')
                           UNION
                           SELECT rt.ticket_instance_id, rt.order_item_id
                           FROM refunded_tickets rt
                                    JOIN order_items oi ON rt.order_item_id = oi.id
                           WHERE oi.order_id = $1
                           UNION
                           SELECT ti.id AS ticket_instance_id, ti.order_item_id
                           FROM ticket_instances ti
                                    JOIN order_items oi ON ti.order_item_id = oi.id
                           WHERE oi.order_id = $1
                             AND oi.item_type = 'Tickets'
                       ) t
              ) t
                  INNER JOIN order_items oi ON t.order_item_id = oi.id
                  LEFT JOIN ticket_instances ti ON ti.id = t.ticket_instance_id
                  INNER JOIN events e ON oi.event_id = e.id
                  INNER JOIN organizations orgs ON e.organization_id = orgs.id
                  LEFT JOIN ticket_pricing tp ON oi.ticket_pricing_id = tp.id
                  INNER JOIN orders o ON oi.order_id = o.id
                  LEFT JOIN users u ON u.id = $3
                  LEFT JOIN organization_users ou ON orgs.id = ou.organization_id AND ou.user_id = u.id
                  LEFT JOIN event_users ep ON ep.event_id = e.id AND ep.user_id = u.id
                  LEFT JOIN ticket_types tt ON tp.ticket_type_id = tt.id
                  LEFT JOIN holds h ON oi.hold_id = h.id
                  LEFT JOIN codes c ON oi.code_id = c.id
                  LEFT JOIN wallets w ON ti.wallet_id = w.id
                  LEFT JOIN users wallet_owner ON w.user_id = wallet_owner.id
                  LEFT JOIN refunded_tickets rt ON rt.ticket_instance_id = ti.id AND rt.order_item_id = oi.id
                  LEFT JOIN order_items fi ON fi.parent_id = oi.id AND fi.item_type = 'PerUnitFees'
                  LEFT JOIN order_items dis ON dis.parent_id = oi.id AND dis.item_type = 'Discount'
                  LEFT JOIN (
                     SELECT tfs.id, tfst.ticket_instance_id
                     FROM transfer_tickets tfst
                              INNER JOIN transfers tfs ON tfst.transfer_id = tfs.id
                     WHERE tfs.status = 'Pending'
                 ) tfs ON tfs.ticket_instance_id = t.ticket_instance_id
         WHERE e.organization_id = ANY ($2)
           AND o.status IN ('Paid', 'PartiallyPaid', 'Cancelled')
           AND (
                 o.user_id = $3
                 OR o.on_behalf_of_user_id = $3
                 OR 'Admin' = ANY (u.role)
                 OR 'Super' = ANY (u.role)
                 OR (
                         NOT ('Promoter' = ANY (ou.role))
                         OR ep.id IS NOT NULL
                     )
             )
         ORDER BY oi.event_id, oi.item_type DESC, t.ticket_instance_id
     ) details
