-- Define the organizations table
CREATE TABLE organizations (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  owner_user_id uuid NOT NULL REFERENCES users (id),
  name TEXT NOT NULL,
  address TEXT,
  city TEXT,
  state TEXT,
  country TEXT,
  postal_code TEXT,
  phone TEXT,
  event_fee_in_cents BIGINT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT now(),
  updated_at TIMESTAMP NOT NULL DEFAULT now()
);

-- Indices
CREATE INDEX index_organizations_owner_user_id ON organizations (owner_user_id);
CREATE UNIQUE INDEX index_organizations_name ON organizations (name);
