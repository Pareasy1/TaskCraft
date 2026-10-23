use chrono::{Duration, TimeZone, Utc};
use taskcraft_store::*;

fn input(title: &str) -> TaskInput {
    TaskInput {
        calendar_id: 1,
        title: title.into(),
        tz: "UTC".into(),
        ..Default::default()
    }
}

fn top_titles(s: &Store, bucket: i64) -> Vec<String> {
    let mut v: Vec<Task> = s
        .list_tasks(TaskFilter::All, Utc::now(), chrono_tz::UTC)
        .unwrap()
        .into_iter()
        .filter(|t| t.bucket_id == bucket && t.parent_id.is_none())
        .collect();
    v.sort_by(|a, b| a.sort_order.partial_cmp(&b.sort_order).unwrap());
    v.into_iter().map(|t| t.title).collect()
}

fn bucket_of(s: &Store, id: i64) -> i64 {
    s.get_task(id).unwrap().bucket_id
}

#[test]
fn duplicate_copies_everything() {
    let s = Store::open_in_memory().unwrap();
    let data = tempfile::tempdir().unwrap();
    let due = Utc.with_ymd_and_hms(2026, 11, 2, 9, 0, 0).unwrap();
    let id = s
        .insert_task(&TaskInput {
            due: Some(due),
            due_has_time: true,
            start: Some(due - Duration::days(1)),
            start_has_time: true,
            rrule: Some("FREQ=WEEKLY".into()),
            notes: Some("note".into()),
            labels: vec!["red".into(), "blue".into()],
            priority: Priority::Urgent,
            alerts: vec![AlertSpecUtc::Before(30)],
            ..input("X")
        })
        .unwrap();
    s.set_task_status(id, TaskStatus::Done, Utc::now()).unwrap();
    // A recurring task advances instead of finishing; force Done for the test.
    let id2 = s
        .insert_task(&TaskInput {
            status: TaskStatus::Done,
            notes: Some("n".into()),
            ..input("Y")
        })
        .unwrap();
    let c2 = s.duplicate_task(id2, " (copy)", data.path()).unwrap();
    let t2 = s.get_task(c2).unwrap();
    assert_eq!(
        (t2.title.as_str(), t2.status, t2.completed_at),
        ("Y (copy)", TaskStatus::Todo, None)
    );

    let c = s.duplicate_task(id, " (copy)", data.path()).unwrap();
    let (o, t) = (s.get_task(id).unwrap(), s.get_task(c).unwrap());
    assert_ne!(c, id);
    assert_eq!(t.title, "X (copy)");
    assert_eq!(t.status, TaskStatus::Todo);
    assert!(t.completed_at.is_none());
    assert_eq!(
        (
            t.due,
            t.due_has_time,
            t.start,
            t.start_has_time,
            &t.rrule,
            &t.notes,
            t.priority,
            &t.tz,
            t.calendar_id
        ),
        (
            o.due,
            o.due_has_time,
            o.start,
            o.start_has_time,
            &o.rrule,
            &o.notes,
            o.priority,
            &o.tz,
            o.calendar_id
        )
    );
    let names = |t: &Task| {
        let mut v: Vec<_> = t.labels.iter().map(|l| l.name.clone()).collect();
        v.sort();
        v
    };
    assert_eq!(names(&t), vec!["blue", "red"]);
    assert_eq!(t.alerts, o.alerts);
    assert_eq!((t.plan_id, t.bucket_id), (o.plan_id, o.bucket_id));
}

#[test]
fn duplicate_order() {
    let s = Store::open_in_memory().unwrap();
    let data = tempfile::tempdir().unwrap();
    let a = s.insert_task(&input("a")).unwrap();
    let _b = s.insert_task(&input("b")).unwrap();
    let c = s.insert_task(&input("c")).unwrap();
    let bk = bucket_of(&s, a);
    s.duplicate_task(a, "'", data.path()).unwrap();
    assert_eq!(top_titles(&s, bk), ["a", "a'", "b", "c"]);
    s.duplicate_task(c, "'", data.path()).unwrap();
    assert_eq!(top_titles(&s, bk), ["a", "a'", "b", "c", "c'"]);
}

#[test]
fn duplicate_order_survives_crowding() {
    let s = Store::open_in_memory().unwrap();
    let data = tempfile::tempdir().unwrap();
    let a = s.insert_task(&input("a")).unwrap();
    s.insert_task(&input("z")).unwrap();
    let bk = bucket_of(&s, a);
    // Repeatedly duplicating `a` halves the gap each time.
    let mut want = vec!["a".to_string(), "z".to_string()];
    for _ in 0..60 {
        s.duplicate_task(a, "+", data.path()).unwrap();
        want.insert(1, "a+".into());
    }
    assert_eq!(top_titles(&s, bk), want);
}

#[test]
fn duplicate_with_subtasks_and_checklists() {
    let s = Store::open_in_memory().unwrap();
    let data = tempfile::tempdir().unwrap();
    let p = s.insert_task(&input("P")).unwrap();
    let k1 = s
        .insert_task(&TaskInput {
            parent_id: Some(p),
            ..input("k1")
        })
        .unwrap();
    let k2 = s
        .insert_task(&TaskInput {
            parent_id: Some(p),
            ..input("k2")
        })
        .unwrap();
    s.set_task_completed(k1, true, Utc::now()).unwrap();
    let i1 = s.add_checklist_item(p, "one").unwrap();
    s.add_checklist_item(p, "two").unwrap();
    s.update_checklist_item(i1, None, Some(true)).unwrap();
    s.add_checklist_item(k2, "kid item").unwrap();

    let c = s.duplicate_task(p, " (copy)", data.path()).unwrap();
    let kids = s.list_subtasks(c).unwrap();
    assert_eq!(
        kids.iter().map(|k| k.title.as_str()).collect::<Vec<_>>(),
        ["k1", "k2"]
    );
    assert!(kids
        .iter()
        .all(|k| k.status == TaskStatus::Todo && k.completed_at.is_none()));
    assert!(kids.iter().all(|k| k.parent_id == Some(c)));
    let cl = s.list_checklist(c).unwrap();
    assert_eq!(
        cl.iter().map(|i| i.text.as_str()).collect::<Vec<_>>(),
        ["one", "two"]
    );
    assert!(cl.iter().all(|i| !i.done));
    assert_eq!(s.list_checklist(kids[1].id).unwrap().len(), 1);

    // Original untouched.
    let old = s.list_subtasks(p).unwrap();
    assert_eq!(old.iter().map(|k| k.id).collect::<Vec<_>>(), [k1, k2]);
    assert!(old[0].completed_at.is_some());
    assert!(s.list_checklist(p).unwrap()[0].done);
}

#[test]
fn duplicate_copies_attachment_files() {
    let s = Store::open_in_memory().unwrap();
    let data = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let f = src.path().join("part.stl");
    std::fs::write(&f, b"hello").unwrap();
    let t = s.insert_task(&input("t")).unwrap();
    let kid = s
        .insert_task(&TaskInput {
            parent_id: Some(t),
            ..input("kid")
        })
        .unwrap();
    let oa = s.add_file_attachment(t, &f, data.path()).unwrap();
    s.add_link_attachment(t, "https://example.com/x", Some("ex"))
        .unwrap();
    let ka = s.add_file_attachment(kid, &f, data.path()).unwrap();

    let c = s.duplicate_task(t, " (copy)", data.path()).unwrap();
    let atts = s.list_attachments(c).unwrap();
    assert_eq!(atts.len(), 2);
    let file = atts
        .iter()
        .find(|a| a.kind == AttachmentKind::File)
        .unwrap();
    assert_eq!((file.name.as_str(), file.size), ("part.stl", Some(5)));
    let stored = file.stored.clone().unwrap();
    assert_ne!(Some(&stored), oa.stored.as_ref());
    assert!(stored.starts_with(&format!("attachments/{c}/")), "{stored}");
    assert_eq!(std::fs::read(data.path().join(&stored)).unwrap(), b"hello");
    let link = atts
        .iter()
        .find(|a| a.kind == AttachmentKind::Link)
        .unwrap();
    assert_eq!(
        (link.name.as_str(), link.url.as_deref()),
        ("ex", Some("https://example.com/x"))
    );
    let ckid = s.list_subtasks(c).unwrap()[0].id;
    let kat = s.list_attachments(ckid).unwrap();
    assert_eq!(kat.len(), 1);
    assert_ne!(kat[0].stored, ka.stored);

    // Removing the copy's file leaves the original's intact.
    s.remove_attachment(file.id, Utc::now()).unwrap();
    s.purge_pending_files(Utc::now() + Duration::hours(1), data.path())
        .unwrap();
    assert!(!data.path().join(&stored).exists());
    assert_eq!(
        std::fs::read(data.path().join(oa.stored.unwrap())).unwrap(),
        b"hello"
    );
}

#[test]
fn duplicate_failure_leaves_no_files() {
    let s = Store::open_in_memory().unwrap();
    let data = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let f = src.path().join("a.txt");
    std::fs::write(&f, b"1").unwrap();
    let t = s.insert_task(&input("t")).unwrap();
    s.add_file_attachment(t, &f, data.path()).unwrap();
    let g = src.path().join("b.txt");
    std::fs::write(&g, b"2").unwrap();
    let b = s.add_file_attachment(t, &g, data.path()).unwrap();
    // The second stored file vanishes, so the copy fails midway.
    std::fs::remove_file(data.path().join(b.stored.unwrap())).unwrap();
    let before = s
        .list_tasks(TaskFilter::All, Utc::now(), chrono_tz::UTC)
        .unwrap()
        .len();
    assert!(s.duplicate_task(t, "'", data.path()).is_err());
    assert_eq!(
        s.list_tasks(TaskFilter::All, Utc::now(), chrono_tz::UTC)
            .unwrap()
            .len(),
        before
    );
    let n = std::fs::read_dir(data.path().join("attachments"))
        .unwrap()
        .count();
    assert_eq!(n, 1, "only the original task's folder remains");
    let new_dirs: Vec<_> = std::fs::read_dir(data.path().join("attachments"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(new_dirs, [std::ffi::OsString::from(t.to_string())]);
}

#[test]
fn duplicate_subtask_stays_under_parent() {
    let s = Store::open_in_memory().unwrap();
    let data = tempfile::tempdir().unwrap();
    let p = s.insert_task(&input("P")).unwrap();
    let k1 = s
        .insert_task(&TaskInput {
            parent_id: Some(p),
            ..input("k1")
        })
        .unwrap();
    s.insert_task(&TaskInput {
        parent_id: Some(p),
        ..input("k2")
    })
    .unwrap();
    let c = s.duplicate_task(k1, " (copy)", data.path()).unwrap();
    assert_eq!(s.get_task(c).unwrap().parent_id, Some(p));
    let titles: Vec<_> = s
        .list_subtasks(p)
        .unwrap()
        .into_iter()
        .map(|k| k.title)
        .collect();
    assert_eq!(titles, ["k1", "k1 (copy)", "k2"]);
}

#[test]
fn duplicate_missing_task_is_not_found() {
    let s = Store::open_in_memory().unwrap();
    let data = tempfile::tempdir().unwrap();
    assert!(matches!(
        s.duplicate_task(999, "x", data.path()),
        Err(StoreError::NotFound)
    ));
}
