use crate::model::*;
use crate::Store;

impl Store {
    pub fn list_plans(&self) -> Result<Vec<Plan>> {
        let mut st = self.conn.prepare(
            "SELECT p.id, p.name, p.color, p.sort,
               (SELECT count(*) FROM tasks t WHERE t.plan_id = p.id AND t.completed_at IS NULL)
             FROM plans p ORDER BY p.sort, p.id",
        )?;
        let rows = st.query_map([], |r| {
            Ok(Plan {
                id: r.get(0)?,
                name: r.get(1)?,
                color: r.get(2)?,
                sort: r.get(3)?,
                open_count: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn list_buckets(&self, plan_id: i64) -> Result<Vec<Bucket>> {
        let mut st = self.conn.prepare(
            "SELECT id, plan_id, name, sort FROM buckets WHERE plan_id = ?1 ORDER BY sort, id",
        )?;
        let rows = st.query_map([plan_id], |r| {
            Ok(Bucket {
                id: r.get(0)?,
                plan_id: r.get(1)?,
                name: r.get(2)?,
                sort: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn default_plan_id(&self) -> Result<i64> {
        let v: String = self.conn.query_row(
            "SELECT value FROM prefs WHERE key = 'default_plan_id'",
            [],
            |r| r.get(0),
        )?;
        v.parse()
            .map_err(|_| StoreError::Invalid("default_plan_id".into()))
    }
}
