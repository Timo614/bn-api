UPDATE ticket_instances
SET
    order_item_id   = $1,
    reserved_until = $2,
    status = 'Reserved',
    updated_at = now()
WHERE id IN (SELECT t.id
             FROM ticket_instances AS t
                    INNER JOIN assets AS a ON t.asset_id = a.id
             WHERE (t.order_item_id IS NULL OR (t.reserved_until < now() AND t.status <> 'Purchased'))
               AND a.ticket_type_id = $3
               AND coalesce($4, 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11') =
                   coalesce(t.ticket_holding_id, 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11') -- dummy guid
             LIMIT $5 FOR UPDATE SKIP LOCKED)
    RETURNING
      id,
      asset_id,
      token_id,
      ticket_holding_id,
      order_item_id,
      wallet_id,
      reserved_until,
      status,
      created_at,
      updated_at;