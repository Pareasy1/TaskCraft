use taskcraft_store::*;

thread_local! {
    static DB: std::cell::RefCell<std::path::PathBuf> = Default::default();
}

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    DB.with(|d| *d.borrow_mut() = path.clone());
    let s = Store::open(&path).unwrap();
    (dir, s)
}

/// (title, sort_order) of every task in a bucket, in display order.
fn rows(bucket: i64) -> Vec<(String, f64)> {
    let path = DB.with(|d| d.borrow().clone());
    let c = rusqlite::Connection::open(path).unwrap();
    let mut st = c
        .prepare("SELECT title, sort_order FROM tasks WHERE bucket_id=?1 ORDER BY sort_order, id")
        .unwrap();
    st.query_map([bucket], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect()
}

fn input(title: &str, plan: Option<i64>, bucket: Option<i64>) -> TaskInput {
    TaskInput {
        calendar_id: 1,
        title: title.into(),
        tz: "Europe/Rome".into(),
        plan_id: plan,
        bucket_id: bucket,
        ..Default::default()
    }
}

fn add(s: &Store, title: &str, bucket: i64) -> i64 {
    s.insert_task(&input(title, None, Some(bucket))).unwrap()
}

fn first_bucket(s: &Store, plan: i64) -> i64 {
    s.list_buckets(plan).unwrap()[0].id
}

fn order(_s: &Store, _plan: i64, bucket: i64) -> Vec<String> {
    rows(bucket).into_iter().map(|r| r.0).collect()
}

#[test]
fn insert_defaults_to_default_plan_end() {
    let (_d, s) = store();
    let p = s.default_plan_id().unwrap();
    let b = first_bucket(&s, p);
    let a = s.insert_task(&input("a", None, None)).unwrap();
    let c = s.insert_task(&input("c", None, None)).unwrap();
    let (a, c) = (s.get_task(a).unwrap(), s.get_task(c).unwrap());
    assert_eq!((a.plan_id, a.bucket_id, a.sort_order), (p, b, 1.0));
    assert_eq!((c.plan_id, c.bucket_id, c.sort_order), (p, b, 2.0));
}

#[test]
fn insert_with_bucket_sets_plan() {
    let (_d, s) = store();
    let pb = s.create_plan("B", "#fff").unwrap();
    let b = first_bucket(&s, pb);
    let t = s.insert_task(&input("x", None, Some(b))).unwrap();
    assert_eq!(s.get_task(t).unwrap().plan_id, pb);
}

#[test]
fn insert_mismatched_plan_bucket_rejected() {
    let (_d, s) = store();
    let pb = s.create_plan("B", "#fff").unwrap();
    let b = first_bucket(&s, pb);
    let default = s.default_plan_id().unwrap();
    let r = s.insert_task(&input("x", Some(default), Some(b)));
    assert!(matches!(r, Err(StoreError::Invalid(_))));
}

#[test]
fn update_plan_only_moves_to_first_bucket() {
    let (_d, s) = store();
    let a = s.default_plan_id().unwrap();
    let b2 = s.create_bucket(a, "Two").unwrap();
    let pb = s.create_plan("B", "#fff").unwrap();
    let bb = first_bucket(&s, pb);
    let existing = add(&s, "old", bb);
    let t = add(&s, "t", b2);
    assert_eq!(s.get_task(existing).unwrap().sort_order, 1.0);
    s.update_task(t, &input("t", Some(pb), None)).unwrap();
    let t = s.get_task(t).unwrap();
    assert_eq!((t.plan_id, t.bucket_id, t.sort_order), (pb, bb, 2.0));
}

#[test]
fn update_without_placement_keeps_it() {
    let (_d, s) = store();
    let a = s.default_plan_id().unwrap();
    let b2 = s.create_bucket(a, "Two").unwrap();
    let t = add(&s, "t", b2);
    let before = s.get_task(t).unwrap();
    s.update_task(t, &input("renamed", None, None)).unwrap();
    let after = s.get_task(t).unwrap();
    assert_eq!(
        (before.plan_id, before.bucket_id, before.sort_order),
        (after.plan_id, after.bucket_id, after.sort_order)
    );
}

fn abc(s: &Store) -> (i64, i64, [i64; 3]) {
    let p = s.default_plan_id().unwrap();
    let b = first_bucket(s, p);
    (p, b, [add(s, "a", b), add(s, "b", b), add(s, "c", b)])
}

#[test]
fn move_between_two_cards() {
    let (_d, s) = store();
    let (p, b, [a, bb, c]) = abc(&s);
    s.move_task(c, b, Some(a), Some(bb)).unwrap();
    assert_eq!(order(&s, p, b), ["a", "c", "b"]);
}

#[test]
fn move_to_top_and_bottom() {
    let (_d, s) = store();
    let (p, b, [a, _bb, c]) = abc(&s);
    s.move_task(c, b, None, Some(a)).unwrap();
    assert_eq!(order(&s, p, b), ["c", "a", "b"]);
    s.move_task(c, b, None, None).unwrap();
    assert_eq!(order(&s, p, b), ["a", "b", "c"]);
}

#[test]
fn move_across_plans_updates_plan() {
    let (_d, s) = store();
    let (_p, b, [a, _, _]) = abc(&s);
    let _ = b;
    let pb = s.create_plan("B", "#fff").unwrap();
    let bb = first_bucket(&s, pb);
    s.move_task(a, bb, None, None).unwrap();
    let t = s.get_task(a).unwrap();
    assert_eq!((t.plan_id, t.bucket_id), (pb, bb));
}

#[test]
fn move_neighbour_in_other_bucket_rejected() {
    let (_d, s) = store();
    let (p, b, [a, _, c]) = abc(&s);
    let other = s.create_bucket(p, "Other").unwrap();
    let r = s.move_task(c, other, Some(a), None);
    assert!(matches!(r, Err(StoreError::Invalid(_))));
    let r = s.move_task(c, b, None, Some(c + 100));
    assert!(matches!(r, Err(StoreError::Invalid(_))));
}

#[test]
fn many_midpoint_moves_renumber() {
    let (_d, s) = store();
    let p = s.default_plan_id().unwrap();
    let b = first_bucket(&s, p);
    let a = add(&s, "a", b);
    let bb = add(&s, "b", b);
    let mut ids = vec![a, bb];
    for i in 1..=60 {
        ids.push(add(&s, &format!("x{i}"), b));
    }
    for _ in 0..60 {
        let last = *ids.last().unwrap();
        let v: Vec<i64> = {
            let titles = order(&s, p, b);
            let _ = titles;
            let path = DB.with(|d| d.borrow().clone());
            let c = rusqlite::Connection::open(path).unwrap();
            let mut st = c
                .prepare(
                    "SELECT id FROM tasks WHERE bucket_id=?1 AND id<>?2 ORDER BY sort_order, id",
                )
                .unwrap();
            st.query_map([b, last], |r| r.get(0))
                .unwrap()
                .map(|r| r.unwrap())
                .collect()
        };
        let pos = v.iter().position(|t| *t == a).unwrap();
        let next = v[pos + 1];
        s.move_task(last, b, Some(a), Some(next)).unwrap();
        ids.pop();
        ids.insert(1, last);
    }
    let mut expected = vec!["a".to_string()];
    expected.extend((1..=60).map(|i| format!("x{i}")));
    expected.push("b".into());
    assert_eq!(order(&s, p, b), expected);
    let mut sorts: Vec<f64> = rows(b).into_iter().map(|r| r.1).collect();
    sorts.sort_by(|a, b| a.total_cmp(b));
    sorts.dedup();
    assert_eq!(sorts.len(), 62);
}

#[test]
fn update_same_plan_keeps_bucket_and_order() {
    let (_d, s) = store();
    let a = s.default_plan_id().unwrap();
    let b2 = s.create_bucket(a, "Two").unwrap();
    let first = add(&s, "first", b2);
    let t = add(&s, "t", b2);
    let _ = first;
    let before = s.get_task(t).unwrap();
    s.update_task(t, &input("t", Some(a), None)).unwrap();
    let after = s.get_task(t).unwrap();
    assert_eq!(
        (after.plan_id, after.bucket_id, after.sort_order),
        (a, b2, before.sort_order)
    );
}
