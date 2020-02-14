
DROP INDEX IF EXISTS index_chat_workflow_interactions_chat_workflow_item_id_chat_workflow_response_id;
DROP INDEX IF EXISTS index_chat_workflow_interactions_chat_session_id;
DROP TABLE IF EXISTS chat_workflow_interactions;

DROP INDEX IF EXISTS index_chat_sessions_chat_workflow_id_chat_workflow_item_id;
DROP INDEX IF EXISTS index_chat_sessions_user_id;
DROP TABLE IF EXISTS chat_sessions;

DROP INDEX IF EXISTS index_chat_workflow_responses_item_id_answer_value;
DROP INDEX IF EXISTS index_chat_workflow_responses_response_type;
DROP INDEX IF EXISTS index_chat_workflow_responses_chat_workflow_item_id;
DROP INDEX IF EXISTS index_chat_workflow_responses_next_chat_workflow_item_id;
DROP TABLE IF EXISTS chat_workflow_responses;

ALTER TABLE chat_workflows DROP COLUMN initial_chat_workflow_item_id;

DROP INDEX IF EXISTS index_chat_workflow_items_item_type;
DROP INDEX IF EXISTS index_chat_workflow_items_chat_workflow_id;
DROP TABLE IF EXISTS chat_workflow_items;

DROP INDEX IF EXISTS index_chat_workflows_name;
DROP TABLE IF EXISTS chat_workflows;
