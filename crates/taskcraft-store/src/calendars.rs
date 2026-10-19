use crate::model::*;
use crate::Store;
use rusqlite::params;

impl Store {
    pub fn calendars(&self) -> Result<Vec<Calendar>> {
        let mut st = self
            .conn
            .prepare("SELECT id, name, color, visible, sort FROM calendars ORDER BY sort, id")?;
        let rows = st.query_map([], |r| {
            Ok(Calendar {
                id: r.get(0)?,
                name: r.get(1)?,
                color: r.get(2)?,
                visible: r.get(3)?,
                sort: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn upsert_calendar(&self, c: &CalendarInput) -> Result<i64> {
        if c.name.trim().is_empty() {
            return Err(StoreError::Invalid("calendar name is empty".into()));
        }
        match c.id {
            Some(id) => {
                let n = self.conn.execute(
                    "UPDATE calendars SET name = ?1, color = ?2, visible = ?3 WHERE id = ?4",
                    params![c.name.trim(), c.color, c.visible, id],
                )?;
                if n == 0 {
                    return Err(StoreError::NotFound);
                }
                Ok(id)
            }
            None => {
                self.conn.execute(
                    "INSERT INTO calendars(name, color, visible, sort)
                     VALUES (?1, ?2, ?3, (SELECT COALESCE(MAX(sort), -1) + 1 FROM calendars))",
                    params![c.name.trim(), c.color, c.visible],
                )?;
                Ok(self.conn.last_insert_rowid())
            }
        }
    }

    /// Deletes a calendar after moving its events and tasks to `move_to`.
    pub fn delete_calendar(&self, id: i64, move_to: i64) -> Result<()> {
        if id == move_to {
            return Err(StoreError::Invalid(
                "cannot move items into the calendar being deleted".into(),
            ));
        }
        self.in_tx(|tx| {
            let exists: i64 = tx.query_row(
                "SELECT COUNT(*) FROM calendars WHERE id = ?1",
                [move_to],
                |r| r.get(0),
            )?;
            if exists == 0 {
                return Err(StoreError::NotFound);
            }
            tx.execute(
                "UPDATE events SET calendar_id = ?1 WHERE calendar_id = ?2",
                [move_to, id],
            )?;
            tx.execute(
                "UPDATE tasks SET calendar_id = ?1 WHERE calendar_id = ?2",
                [move_to, id],
            )?;
            if tx.execute("DELETE FROM calendars WHERE id = ?1", [id])? == 0 {
                return Err(StoreError::NotFound);
            }
            Ok(())
        })
    }
}
