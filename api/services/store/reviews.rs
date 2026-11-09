use super::*;
const COLS: &str = "id, repo_path, base_ref, head_ref, head_sha, status, created_at, updated_at, summary, project_id, scope_from, scope_side, scope_configured, scope_mode";
fn scan(row: &rusqlite::Row<'_>) -> rusqlite::Result<Review> {
    Ok(Review {
        id: row.get(0)?,
        repo_path: row.get(1)?,
        base_ref: row.get(2)?,
        head_ref: row.get(3)?,
        head_sha: row.get(4)?,
        status: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        summary: row.get(8)?,
        project_id: row.get(9)?,
        scope_from: row.get(10)?,
        scope_side: Side::parse(&row.get::<_, String>(11)?).ok_or(rusqlite::Error::InvalidQuery)?,
        scope_configured: row.get(12)?,
        scope_mode: row.get(13)?,
        ..Default::default()
    })
}
pub(super) fn get(conn: &Connection, id: i64) -> rusqlite::Result<Review> {
    let mut review = conn.query_row(
        &format!("SELECT {COLS} FROM reviews WHERE id=?"),
        [id],
        scan,
    )?;
    review.comments = super::comments::list(conn, id)?;
    let replies = super::replies::list_review(conn, id)?;
    let mut grouped = std::collections::HashMap::<i64, Vec<Reply>>::new();
    for reply in replies {
        grouped.entry(reply.comment_id).or_default().push(reply);
    }
    for comment in &mut review.comments {
        comment.replies = grouped.remove(&comment.id).unwrap_or_default();
    }
    review.reviewed_files = super::reviewed::list(conn, id)?
        .into_iter()
        .map(|f| f.path)
        .collect();
    Ok(review)
}
impl Store {
    pub fn create_or_get_review(
        &self,
        repo: &str,
        base: &str,
        head: &str,
        sha: &str,
    ) -> Result<Review> {
        self.with(|conn| {
            let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let id = tx.query_row("SELECT id FROM reviews WHERE repo_path=? AND base_ref=? AND head_ref=? AND scope_configured=0 ORDER BY id DESC LIMIT 1", params![repo,base,head], |r| r.get::<_,i64>(0)).optional()?;
            let ts = now();
            let id = if let Some(id) = id {
                tx.execute("UPDATE reviews SET head_sha=?, updated_at=? WHERE id=?", params![sha,ts,id])?; id
            } else {
                tx.execute("INSERT INTO reviews (repo_path,base_ref,head_ref,head_sha,status,created_at,updated_at) VALUES (?,?,?,?,'draft',?,?)",params![repo,base,head,sha,ts,ts])?;
                tx.last_insert_rowid()
            };
            tx.commit()?;
            get(conn,id)
        })
    }
    pub fn get_review(&self, id: i64) -> Result<Review> {
        self.with(|c| get(c, id))
    }
    pub fn review_repo_head(&self, id: i64) -> Result<(String, String)> {
        let result: (String, String) = self.with(|c| {
            c.query_row(
                "SELECT repo_path,head_ref FROM reviews WHERE id=?",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
        })?;
        self.validate_registered_repo(&result.0)?;
        Ok(result)
    }
    pub fn count_reviews(&self) -> Result<i64> {
        self.with(|c| c.query_row("SELECT COUNT(*) FROM reviews", [], |r| r.get(0)))
    }
    pub fn reset_review(&self, id: i64) -> Result<()> {
        self.with(|c| {
            let tx = c.transaction()?;
            tx.execute("DELETE FROM comments WHERE review_id=?", [id])?;
            tx.execute("DELETE FROM reviewed_files WHERE review_id=?", [id])?;
            tx.execute(
                "UPDATE reviews SET summary='',updated_at=? WHERE id=?",
                params![now(), id],
            )?;
            tx.commit()
        })
    }
    pub fn set_review_summary(&self, id: i64, summary: &str) -> Result<()> {
        self.with(|c| {
            if c.execute(
                "UPDATE reviews SET summary=?,updated_at=? WHERE id=?",
                params![summary, now(), id],
            )? == 0
            {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
    }
    pub fn set_status(&self, id: i64, status: &str) -> Result<()> {
        self.with(|c| {
            c.execute(
                "UPDATE reviews SET status=CASE WHEN status='completed' AND ?='exported' THEN status ELSE ? END,updated_at=? WHERE id=?",
                params![status, status, now(), id],
            )?;
            Ok(())
        })
    }
    pub fn touch(&self, id: i64) -> Result<()> {
        self.with(|c| {
            c.execute(
                "UPDATE reviews SET updated_at=? WHERE id=?",
                params![now(), id],
            )?;
            Ok(())
        })
    }
    pub fn prune_drafts(&self, older_than: Duration) -> Result<usize> {
        let elapsed = chrono::Duration::from_std(older_than).map_err(ServiceError::internal)?;
        let cutoff = chrono::Utc::now()
            .checked_sub_signed(elapsed)
            .ok_or_else(|| ServiceError::internal("retention duration out of range"))?
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        self.with(|c| {
            c.execute(
                "DELETE FROM reviews WHERE status='draft' AND scope_configured=0 AND updated_at < ?",
                [cutoff],
            )
        })
    }
}
