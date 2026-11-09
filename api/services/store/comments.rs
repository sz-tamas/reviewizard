use super::*;
const COLS: &str = "id,review_id,file_path,start_line,end_line,snippet,type,body,created_at,updated_at,resolved,author,commit_sha,worktree,indexed";
fn scan(r: &rusqlite::Row<'_>) -> rusqlite::Result<Comment> {
    Ok(Comment {
        id: r.get(0)?,
        review_id: r.get(1)?,
        file_path: r.get(2)?,
        start_line: r.get(3)?,
        end_line: r.get(4)?,
        snippet: r.get(5)?,
        comment_type: r.get(6)?,
        body: r.get(7)?,
        created_at: r.get(8)?,
        updated_at: r.get(9)?,
        resolved: r.get(10)?,
        author: r.get(11)?,
        commit_sha: r.get(12)?,
        side: Side::from_flags(r.get(13)?, r.get(14)?),
        ..Default::default()
    })
}
pub(super) fn list(c: &Connection, id: i64) -> rusqlite::Result<Vec<Comment>> {
    c.prepare(&format!(
        "SELECT {COLS} FROM comments WHERE review_id=? ORDER BY file_path,start_line"
    ))?
    .query_map([id], scan)?
    .collect()
}
pub(super) fn get(c: &Connection, id: i64) -> rusqlite::Result<Comment> {
    let mut comment = c.query_row(
        &format!("SELECT {COLS} FROM comments WHERE id=?"),
        [id],
        scan,
    )?;
    comment.replies = super::replies::list_comment(c, id)?;
    Ok(comment)
}
pub(super) fn review_id(c: &Connection, id: i64) -> rusqlite::Result<i64> {
    c.query_row("SELECT review_id FROM comments WHERE id=?", [id], |r| {
        r.get(0)
    })
}
impl Store {
    pub fn add_comment(&self, comment: Comment) -> Result<Comment> {
        self.with(|c| { let ts=now(); let (worktree,indexed)=comment.side.flags();
            c.execute("INSERT INTO comments (review_id,file_path,start_line,end_line,snippet,type,body,author,commit_sha,worktree,indexed,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)",params![comment.review_id,comment.file_path,comment.start_line,comment.end_line,comment.snippet,comment.comment_type,comment.body,comment.author,comment.commit_sha,worktree,indexed,ts,ts])?;
            get(c,c.last_insert_rowid())
        })
    }
    pub fn get_comment(&self, id: i64) -> Result<Comment> {
        self.with(|c| get(c, id))
    }
    pub fn update_comment(&self, id: i64, edit: CommentEdit) -> Result<Comment> {
        self.with(|c| { c.execute("UPDATE comments SET body=?,type=?,start_line=?,end_line=?,snippet=?,commit_sha=?,updated_at=? WHERE id=?",params![edit.body,edit.comment_type,edit.start,edit.end,edit.snippet,edit.commit_sha,now(),id])?; get(c,id) })
    }
    pub fn set_comment_resolved(&self, id: i64, resolved: bool) -> Result<i64> {
        self.with(|c| {
            c.execute(
                "UPDATE comments SET resolved=? WHERE id=?",
                params![resolved, id],
            )?;
            review_id(c, id)
        })
    }
    pub fn delete_comment(&self, id: i64) -> Result<i64> {
        self.with(|c| {
            let review = review_id(c, id)?;
            c.execute("DELETE FROM comments WHERE id=?", [id])?;
            Ok(review)
        })
    }
}
