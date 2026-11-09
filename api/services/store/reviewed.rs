use super::*;
pub(super) fn list(c: &Connection, id: i64) -> rusqlite::Result<Vec<ReviewedFile>> {
    c.prepare("SELECT file_path,content_hash,worktree,indexed FROM reviewed_files WHERE review_id=? ORDER BY file_path")?.query_map([id],|r| Ok(ReviewedFile { path:r.get(0)?,content_hash:r.get(1)?,side:Side::from_flags(r.get(2)?,r.get(3)?) }))?.collect()
}
impl Store {
    pub fn list_reviewed_files_full(&self, id: i64) -> Result<Vec<ReviewedFile>> {
        self.with(|c| list(c, id))
    }
    pub fn set_files_reviewed(
        &self,
        id: i64,
        marks: &[FileReviewMark],
        reviewed: bool,
        side: Side,
    ) -> Result<()> {
        self.with(|c| { let tx=c.transaction()?; let ts=now(); let (worktree,indexed)=side.flags();
            for mark in marks {
                if reviewed {
                    tx.execute("INSERT INTO reviewed_files (review_id,file_path,reviewed_at,content_hash,worktree,indexed) VALUES (?,?,?,?,?,?) ON CONFLICT(review_id,file_path) DO UPDATE SET reviewed_at=excluded.reviewed_at,content_hash=excluded.content_hash,worktree=excluded.worktree,indexed=excluded.indexed",params![id,mark.path,ts,mark.content_hash,worktree,indexed])?;
                } else { tx.execute("DELETE FROM reviewed_files WHERE review_id=? AND file_path=?",params![id,mark.path])?; }
            }
            tx.commit()
        })
    }
}
