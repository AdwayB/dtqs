CREATE TABLE IF NOT EXISTS worker_nodes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id VARCHAR(64) NOT NULL UNIQUE,
    status VARCHAR(32) NOT NULL,
    last_health_check TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    current_task_id UUID NULL
);