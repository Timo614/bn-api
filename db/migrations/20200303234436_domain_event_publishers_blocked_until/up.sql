ALTER TABLE domain_event_publishers
ADD COLUMN blocked_until TIMESTAMP NOT NULL DEFAULT now();