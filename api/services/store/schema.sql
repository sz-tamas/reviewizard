CREATE TABLE IF NOT EXISTS reviews (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  repo_path  TEXT NOT NULL,
  base_ref   TEXT NOT NULL,
  head_ref   TEXT NOT NULL,
  head_sha   TEXT NOT NULL,
  status     TEXT NOT NULL DEFAULT 'draft',
  summary    TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS comments (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  review_id  INTEGER NOT NULL REFERENCES reviews(id) ON DELETE CASCADE,
  file_path  TEXT NOT NULL,
  start_line INTEGER NOT NULL,
  end_line   INTEGER NOT NULL,
  snippet    TEXT NOT NULL DEFAULT '',
  type       TEXT NOT NULL DEFAULT 'suggestion',
  body       TEXT NOT NULL DEFAULT '',
  author     TEXT NOT NULL DEFAULT 'reviewer',
  resolved   INTEGER NOT NULL DEFAULT 0,
  commit_sha TEXT NOT NULL DEFAULT '',
  worktree   INTEGER NOT NULL DEFAULT 0,
  indexed    INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_comments_review ON comments(review_id);
CREATE TABLE IF NOT EXISTS replies (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  comment_id INTEGER NOT NULL REFERENCES comments(id) ON DELETE CASCADE,
  body       TEXT NOT NULL DEFAULT '',
  author     TEXT NOT NULL DEFAULT 'reviewer',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_replies_comment ON replies(comment_id);
CREATE TABLE IF NOT EXISTS reviewed_files (
  review_id    INTEGER NOT NULL REFERENCES reviews(id) ON DELETE CASCADE,
  file_path    TEXT NOT NULL,
  reviewed_at  TEXT NOT NULL,
  content_hash TEXT NOT NULL DEFAULT '',
  worktree     INTEGER NOT NULL DEFAULT 0,
  indexed      INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (review_id, file_path)
);
CREATE TABLE IF NOT EXISTS projects (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  path TEXT NOT NULL UNIQUE,
  canonical_path TEXT NOT NULL,
  archived INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_reviews_repo_path ON reviews(repo_path);
