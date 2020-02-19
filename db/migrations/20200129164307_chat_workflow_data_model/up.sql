CREATE TABLE chat_workflows
(
    id                UUID PRIMARY KEY     DEFAULT gen_random_uuid() NOT NULL,
    name              TEXT NOT NULL,
    status            TEXT NOT NULL,
    created_at        TIMESTAMP NOT NULL DEFAULT now(),
    updated_at        TIMESTAMP NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX index_chat_workflows_name ON chat_workflows (name);

CREATE TABLE chat_workflow_items
(
    id                UUID PRIMARY KEY     DEFAULT gen_random_uuid() NOT NULL,
    chat_workflow_id  UUID NOT NULL REFERENCES chat_workflows(id) ON DELETE CASCADE,
    item_type         TEXT NOT NULL,
    message           TEXT NULL,
    render_type       TEXT NULL,
    response_wait     INTEGER NOT NULL DEFAULT 0,
    created_at        TIMESTAMP NOT NULL DEFAULT now(),
    updated_at        TIMESTAMP NOT NULL DEFAULT now()
);
CREATE INDEX index_chat_workflow_items_chat_workflow_id ON chat_workflow_items (chat_workflow_id);
CREATE INDEX index_chat_workflow_items_item_type ON chat_workflow_items (item_type);

ALTER TABLE chat_workflows
    ADD COLUMN initial_chat_workflow_item_id UUID NULL REFERENCES chat_workflow_items (id);

CREATE TABLE chat_workflow_responses
(
    id                          UUID PRIMARY KEY     DEFAULT gen_random_uuid() NOT NULL,
    chat_workflow_item_id       UUID NOT NULL REFERENCES chat_workflow_items(id) ON DELETE CASCADE,
    response_type               TEXT NOT NULL,
    response                    TEXT NULL,
    answer_value                TEXT NULL,
    next_chat_workflow_item_id  UUID NULL REFERENCES chat_workflow_items(id) ON DELETE CASCADE,
    rank                        INTEGER NOT NULL,
    created_at                  TIMESTAMP NOT NULL DEFAULT now(),
    updated_at                  TIMESTAMP NOT NULL DEFAULT now()
);
CREATE INDEX index_chat_workflow_responses_response_type ON chat_workflow_responses (response_type);
CREATE INDEX index_chat_workflow_responses_chat_workflow_item_id ON chat_workflow_responses (chat_workflow_item_id);
CREATE INDEX index_chat_workflow_responses_next_chat_workflow_item_id ON chat_workflow_responses (next_chat_workflow_item_id);
CREATE UNIQUE INDEX index_chat_workflow_responses_item_id_answer_value ON chat_workflow_responses (chat_workflow_item_id, answer_value);

CREATE TABLE chat_sessions
(
    id                      UUID PRIMARY KEY     DEFAULT gen_random_uuid() NOT NULL,
    user_id                 UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    chat_workflow_id        UUID NOT NULL REFERENCES chat_workflows(id) ON DELETE CASCADE,
    chat_workflow_item_id   UUID NOT NULL REFERENCES chat_workflow_items(id) ON DELETE CASCADE,
    context                 JSONB NOT NULL,
    expires_at              TIMESTAMP NULL,
    created_at              TIMESTAMP NOT NULL DEFAULT now(),
    updated_at              TIMESTAMP NOT NULL DEFAULT now()
);
CREATE INDEX index_chat_sessions_user_id ON chat_sessions (user_id);
CREATE INDEX index_chat_sessions_chat_workflow_id_chat_workflow_item_id ON chat_sessions (chat_workflow_id, chat_workflow_item_id);

CREATE TABLE chat_workflow_interactions
(
    id                UUID PRIMARY KEY     DEFAULT gen_random_uuid() NOT NULL,
    chat_workflow_item_id UUID NOT NULL REFERENCES chat_workflow_items(id) ON DELETE CASCADE,
    chat_workflow_response_id UUID NOT NULL REFERENCES chat_workflow_responses(id) ON DELETE CASCADE,
    chat_session_id UUID NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
    input             TEXT NULL,
    created_at        TIMESTAMP NOT NULL DEFAULT now(),
    updated_at        TIMESTAMP NOT NULL DEFAULT now()
);
CREATE INDEX index_chat_workflow_interactions_chat_workflow_item_id_chat_workflow_response_id ON chat_workflow_interactions (chat_workflow_item_id, chat_workflow_response_id);
CREATE INDEX index_chat_workflow_interactions_chat_session_id ON chat_workflow_interactions (chat_session_id);
