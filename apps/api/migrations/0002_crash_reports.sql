CREATE TABLE crash_reports (
  id TEXT PRIMARY KEY,
  created_at INTEGER NOT NULL,
  app_version TEXT NOT NULL,
  ip_hash TEXT NOT NULL,
  minidump_bytes INTEGER NOT NULL,
  object_prefix TEXT NOT NULL
);

CREATE INDEX crash_reports_ip_created ON crash_reports (ip_hash, created_at);
