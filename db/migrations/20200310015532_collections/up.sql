CREATE TABLE collections (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  name TEXT NOT NULL,
  user_id uuid NOT NULL REFERENCES users (id),
  featured_collectible_id uuid REFERENCES ticket_types (id),
  created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT now(),
  updated_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT now()
);

CREATE INDEX index_collections_user_id ON collections (user_id);
CREATE UNIQUE INDEX index_collections_name ON collections (name, user_id);



CREATE TABLE collection_items (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  collection_id uuid NOT NULL REFERENCES collections (id),
  collectible_id uuid NOT NULL REFERENCES ticket_types (id),
  next_collection_item_id uuid REFERENCES collection_items (id),
  created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT now(),
  updated_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT now()
);

CREATE INDEX index_collections_items_collection_id ON collection_items (collection_id);
CREATE INDEX index_collection_items_collectible_id ON collection_items (collectible_id);
CREATE INDEX index_collection_items_next_collection_item_id ON collection_items (next_collection_item_id);
CREATE UNIQUE INDEX index_collection_items_ticket_type_and_collection_id ON collection_items (collectible_id, collection_id);