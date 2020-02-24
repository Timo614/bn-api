CREATE OR REPLACE FUNCTION redeem_key_unique_per_event(UUID, TEXT) RETURNS BOOLEAN AS $$
BEGIN
    RETURN (
        select not exists (
          select ti.redeem_key
          from ticket_instances ti
          join assets a on a.id = ti.asset_id
          join ticket_types tt on a.ticket_type_id = tt.id
          where (ti.id <> $1 and tt.event_id = (
            select tt.event_id
            from ticket_types tt
            join assets a on a.ticket_type_id = tt.id
            join ticket_instances ti on a.id = ti.asset_id
            where ti.id = $1
          ) and ti.redeem_key = $2)
        )
    );
END $$ LANGUAGE 'plpgsql';
