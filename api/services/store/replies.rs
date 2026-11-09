use super::*;
const COLS: &str = "id,comment_id,body,created_at,updated_at,author";
fn scan(r: &rusqlite::Row<'_>) -> rusqlite::Result<Reply> {
    Ok(Reply {
        id: r.get(0)?,
        comment_id: r.get(1)?,
        body: r.get(2)?,
        created_at: r.get(3)?,
        updated_at: r.get(4)?,
        author: r.get(5)?,
    })
}
pub(super) fn list_review(c: &Connection, id: i64) -> rusqlite::Result<Vec<Reply>> {
    c.prepare("SELECT r.id,r.comment_id,r.body,r.created_at,r.updated_at,r.author FROM replies r JOIN comments c ON c.id=r.comment_id WHERE c.review_id=? ORDER BY r.comment_id,r.created_at,r.id")?.query_map([id],scan)?.collect()
}
pub(super) fn list_comment(c: &Connection, id: i64) -> rusqlite::Result<Vec<Reply>> {
    c.prepare(&format!(
        "SELECT {COLS} FROM replies WHERE comment_id=? ORDER BY created_at,id"
    ))?
    .query_map([id], scan)?
    .collect()
}
fn get(c: &Connection, id: i64) -> rusqlite::Result<Reply> {
    c.query_row(
        &format!("SELECT {COLS} FROM replies WHERE id=?"),
        [id],
        scan,
    )
}
impl Store {
    pub fn add_reply(&self, comment_id: i64, body: &str, author: &str) -> Result<(Reply, i64)> {
        self.with(|c| { let review=super::comments::review_id(c,comment_id)?; let ts=now();
            c.execute("INSERT INTO replies (comment_id,body,author,created_at,updated_at) VALUES (?,?,?,?,?)",params![comment_id,body,author,ts,ts])?;
            Ok((get(c,c.last_insert_rowid())?,review))
        })
    }
    pub fn update_reply(&self, id: i64, body: &str) -> Result<(Reply, i64)> {
        self.with(|c| {
            c.execute(
                "UPDATE replies SET body=?,updated_at=? WHERE id=?",
                params![body, now(), id],
            )?;
            let reply = get(c, id)?;
            let review = super::comments::review_id(c, reply.comment_id)?;
            Ok((reply, review))
        })
    }
    pub fn delete_reply(&self, id: i64) -> Result<i64> {
        self.with(|c| {
            let reply = get(c, id)?;
            c.execute("DELETE FROM replies WHERE id=?", [id])?;
            super::comments::review_id(c, reply.comment_id)
        })
    }
}
