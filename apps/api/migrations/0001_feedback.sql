CREATE TABLE feedback (
  id TEXT PRIMARY KEY,
  created_at INTEGER NOT NULL,
  kind TEXT NOT NULL,
  app_version TEXT NOT NULL,
  os TEXT NOT NULL,
  client_id TEXT NOT NULL,
  ip_hash TEXT NOT NULL,
  has_email INTEGER NOT NULL,
  has_logs INTEGER NOT NULL,
  preview TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'new'
);

CREATE INDEX feedback_client_created ON feedback (client_id, created_at);
CREATE INDEX feedback_ip_created ON feedback (ip_hash, created_at);
CREATE INDEX feedback_status_created ON feedback (status, created_at);
