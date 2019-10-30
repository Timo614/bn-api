SELECT
  entries.operator_name,
  entries.operator_id,
  entries.event_name,
  entries.event_date,
  entries.external_payment_type,
  CAST(SUM(entries.number_of_tickets) AS BIGINT) as number_of_tickets,
  entries.face_value_in_cents,
  entries.revenue_share_value_in_cents,
  CAST(SUM(entries.total_sales_in_cents) AS BIGINT) as total_sales_in_cents
FROM (
  SELECT
    u.id as operator_id,
    concat(u.first_name, ' ', u.last_name) as operator_name,
    e.name as event_name,
    e.event_start as event_date,
    -- If set to false, the logic does not group on external payment type allowing the collection to reflect box office entries
    CASE WHEN $4 THEN o.external_payment_type ELSE null END as external_payment_type,
    CAST(SUM(oi.quantity - oi.refunded_quantity) FILTER (WHERE oi.item_type = 'Tickets') AS BIGINT) as number_of_tickets,
    CASE oi.item_type WHEN 'EventFees' THEN 0 ELSE CAST(oi.unit_price_in_cents + COALESCE(oi_promo_code.unit_price_in_cents, 0) AS BIGINT) END as face_value_in_cents,
    -- Event fees record list the fee as part of the revenue share for that item with 0 face value
    CASE oi.item_type WHEN 'EventFees' THEN CAST(oi.client_fee_in_cents AS BIGINT) ELSE CAST(COALESCE(oi_t_fees.client_fee_in_cents, 0) AS BIGINT) END as revenue_share_value_in_cents,
    CASE oi.item_type WHEN 'EventFees' THEN
      CAST(SUM(COALESCE(oi.client_fee_in_cents, 0) * (oi.quantity - oi.refunded_quantity)) AS BIGINT)
    ELSE
      CAST(SUM(
        (oi.unit_price_in_cents + COALESCE(oi_promo_code.unit_price_in_cents, 0)) * (oi.quantity - oi.refunded_quantity)
        + COALESCE(oi_t_fees.client_fee_in_cents, 0) * (COALESCE(oi_t_fees.quantity, 0) - COALESCE(oi_t_fees.refunded_quantity, 0))
      ) AS BIGINT)
    END as total_sales_in_cents
  FROM orders o
  JOIN order_items oi on o.id = oi.order_id
  LEFT JOIN order_items oi_promo_code ON (oi_promo_code.item_type = 'Discount' AND oi.id = oi_promo_code.parent_id)
  LEFT JOIN order_items oi_t_fees ON oi_t_fees.parent_id = oi.id AND oi_t_fees.item_type = 'PerUnitFees'
  JOIN events e on oi.event_id = e.id
  JOIN users u on o.user_id = u.id
  WHERE o.status = 'Paid'
    AND oi.parent_id IS NULL
    AND oi.quantity <> oi.refunded_quantity
    AND o.box_office_pricing IS TRUE
    AND e.organization_id = $1
    AND ($2 IS NULL OR o.paid_at >= $2)
    AND ($3 IS NULL OR o.paid_at <= $3)
  GROUP BY
    operator_name,
    operator_id,
    event_name,
    event_date,
    o.external_payment_type,
    oi.item_type,
    oi.unit_price_in_cents,
    oi.client_fee_in_cents,
    oi_t_fees.client_fee_in_cents,
    oi_promo_code.unit_price_in_cents
) entries
GROUP BY
  entries.operator_name,
  entries.operator_id,
  entries.event_name,
  entries.event_date,
  entries.external_payment_type,
  entries.face_value_in_cents,
  entries.revenue_share_value_in_cents
ORDER BY
  entries.operator_name,
  entries.operator_id,
  entries.event_date,
  entries.event_name,
  entries.face_value_in_cents,
  entries.revenue_share_value_in_cents;
