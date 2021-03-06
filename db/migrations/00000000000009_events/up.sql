-- Define the events table
CREATE TABLE events (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  name TEXT NOT NULL,
  organization_id uuid NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
  venue_id uuid NULL REFERENCES venues (id) ON DELETE CASCADE,
  created_at TIMESTAMP NOT NULL DEFAULT now(),
  event_start TIMESTAMP NULL,
  door_time TIMESTAMP NULL,
  status TEXT NOT NULL,
  publish_date TIMESTAMP NULL,
  redeem_date TIMESTAMP NULL,
  fee_in_cents BIGINT NULL,
  promo_image_url TEXT NULL,
  additional_info TEXT NULL,
  age_limit INTEGER NULL,
  top_line_info VARCHAR(100) NULL,
  cancelled_at TIMESTAMP NULL,
  updated_at TIMESTAMP NOT NULL DEFAULT now()
);

-- Indices
CREATE INDEX index_events_organization_id ON events (organization_id);
CREATE INDEX index_events_venue_id ON events (venue_id);
