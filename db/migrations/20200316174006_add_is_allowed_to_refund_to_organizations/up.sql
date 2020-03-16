alter table organizations
  add is_allowed_to_refund boolean not null default 'f';
