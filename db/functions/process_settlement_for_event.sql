--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
-- PROCESS SETTLEMENT FOR EVENT
--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
DROP FUNCTION IF EXISTS process_settlement_for_event(settlement_id UUID, event_id UUID, start TIMESTAMP, "end" TIMESTAMP);
CREATE OR REPLACE FUNCTION process_settlement_for_event(settlement_id UUID, event_id UUID, start TIMESTAMP, "end" TIMESTAMP) RETURNS void AS $$
BEGIN

with order_item_ids as (
  SELECT oi.id FROM order_items oi
  INNER JOIN orders o on oi.order_id = o.id
  LEFT JOIN holds h ON oi.hold_id = h.id
  LEFT JOIN order_items oi_promo_code ON (oi_promo_code.item_type = 'Discount' AND oi.id = oi_promo_code.parent_id)
  WHERE ($3 IS NULL OR o.paid_at >= $3)
  AND ($4 IS NULL OR o.paid_at <= $4)
  AND oi.event_id = $2
  AND (oi.item_type <> 'EventFees' OR oi.client_fee_in_cents > 0)
  AND oi.item_type <> 'CreditCardFees'
  AND o.settlement_id IS NULL
  AND o.status = 'Paid'
  AND oi.parent_id IS NULL
  AND (oi.unit_price_in_cents + COALESCE(oi_promo_code.unit_price_in_cents, 0)) > 0
  AND oi.quantity <> oi.refunded_quantity
  AND o.box_office_pricing IS FALSE
)
INSERT INTO settlement_entries (settlement_id, event_id, ticket_type_id, face_value_in_cents, revenue_share_value_in_cents, online_sold_quantity, fee_sold_quantity, total_sales_in_cents, settlement_entry_type)
SELECT -- Group result set by face price to prevent multiple records for holds that match code discounts
  entries.settlement_id,
  entries.event_id,
  entries.ticket_type_id,
  entries.face_value_in_cents,
  entries.revenue_share_value_in_cents,
  SUM(online_sold_quantity),
  SUM(fee_sold_quantity),
  SUM(online_sold_quantity) * entries.face_value_in_cents + SUM(fee_sold_quantity) * entries.revenue_share_value_in_cents,
  entries.settlement_entry_type
FROM (
  SELECT
    $1 as settlement_id,
    oi.event_id,
    oi.ticket_type_id,
    CASE oi.item_type WHEN 'EventFees' THEN 0 ELSE CAST(oi.unit_price_in_cents + COALESCE(oi_promo_code.unit_price_in_cents, 0) AS BIGINT) END as face_value_in_cents,
    -- Event fees record list the fee as part of the revenue share for that item with 0 face value
    CASE oi.item_type WHEN 'EventFees' THEN CAST(oi.client_fee_in_cents AS BIGINT) ELSE CAST(COALESCE(oi_t_fees.client_fee_in_cents, 0) AS BIGINT) END as revenue_share_value_in_cents,
    -- Event fees list their quantity in the fee_sold_quantity field
    CASE oi.item_type WHEN 'EventFees' THEN 0 ELSE CAST(SUM(oi.quantity - oi.refunded_quantity) AS BIGINT) END as online_sold_quantity,
    CASE oi.item_type WHEN 'EventFees' THEN CAST(SUM(oi.quantity - oi.refunded_quantity) AS BIGINT) ELSE CAST(SUM(COALESCE(oi_t_fees.quantity - oi_t_fees.refunded_quantity, 0)) AS BIGINT) END as fee_sold_quantity,
    CASE oi.item_type WHEN 'EventFees' THEN 'EventFees' ELSE 'TicketType' END as settlement_entry_type
  FROM order_items oi
           INNER JOIN order_item_ids oi_ids ON oi.id = oi_ids.id
           INNER JOIN orders o ON oi.order_id = o.id
           LEFT JOIN order_items oi_promo_code ON (oi_promo_code.item_type = 'Discount' AND oi.id = oi_promo_code.parent_id)
           LEFT JOIN order_items oi_t_fees ON oi_t_fees.parent_id = oi.id AND oi_t_fees.item_type = 'PerUnitFees'
  GROUP BY
    oi.item_type,
    oi.event_id,
    oi.ticket_type_id,
    oi.unit_price_in_cents,
    oi.client_fee_in_cents,
    oi_t_fees.client_fee_in_cents,
    oi_promo_code.unit_price_in_cents
) entries
  GROUP BY
    entries.settlement_id,
    entries.event_id,
    entries.ticket_type_id,
    entries.face_value_in_cents,
    entries.revenue_share_value_in_cents,
    entries.settlement_entry_type
;

with order_item_ids as (
  SELECT oi.id FROM order_items oi
  INNER JOIN orders o on oi.order_id = o.id
  WHERE ($3 IS NULL OR o.paid_at >= $3)
  AND ($4 IS NULL OR o.paid_at <= $4)
  AND oi.event_id = $2
  AND o.settlement_id IS NULL
  AND o.status = 'Paid'
  AND oi.parent_id IS NULL
  AND oi.unit_price_in_cents > 0
  AND o.box_office_pricing IS FALSE
)
UPDATE orders SET settlement_id = $1
FROM order_item_ids oi_ids
JOIN order_items oi ON oi.id = oi_ids.id
JOIN orders o ON oi.order_id = o.id
WHERE orders.id = o.id;

END $$ LANGUAGE 'plpgsql';
