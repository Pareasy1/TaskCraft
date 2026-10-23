use chrono::{TimeZone, Utc};
use chrono_tz::Europe::Rome;
use taskcraft_store::*;

fn input(title: &str) -> TaskInput {
    TaskInput {
        calendar_id: 1,
        title: title.into(),
        tz: "UTC".into(),
        ..Default::default()
    }
}

/// Parent due Fri 30 Oct 2026 12:00 UTC with subtasks 27th 10:00, 30th date-only, undated.
fn source(s: &Store) -> i64 {
    let p = s
        .insert_task(&TaskInput {
            due: Some(Utc.with_ymd_and_hms(2026, 10, 30, 12, 0, 0).unwrap()),
            due_has_time: true,
            notes: Some("secret".into()),
            labels: vec!["Work".into()],
            priority: Priority::Urgent,
            ..input("Release")
        })
        .unwrap();
    s.add_checklist_item(p, "Tag").unwrap();
    let a = s
        .insert_task(&TaskInput {
            parent_id: Some(p),
            due: Some(Utc.with_ymd_and_hms(2026, 10, 27, 10, 0, 0).unwrap()),
            due_has_time: true,
            ..input("A")
        })
        .unwrap();
    s.add_checklist_item(a, "a1").unwrap();
    s.insert_task(&TaskInput {
        parent_id: Some(p),
        due: Some(Utc.with_ymd_and_hms(2026, 10, 30, 0, 0, 0).unwrap()),
        due_has_time: false,
        ..input("B")
    })
    .unwrap();
    s.insert_task(&TaskInput {
        parent_id: Some(p),
        ..input("C")
    })
    .unwrap();
    p
}

#[test]
fn save_computes_offsets() {
    let s = Store::open_in_memory().unwrap();
    let t = s.save_template(source(&s), "Rel", false).unwrap();
    let v = s.template_body(t).unwrap();
    assert_eq!(v.subtasks.len(), 3);
    assert_eq!(v.subtasks[0].due_offset_days, Some(-3));
    assert_eq!(v.subtasks[0].due_time.as_deref(), Some("10:00"));
    assert_eq!(v.subtasks[1].due_offset_days, Some(0));
    assert_eq!(v.subtasks[1].due_time, None);
    assert_eq!(v.subtasks[2].due_offset_days, None);
    assert_eq!(v.subtasks[0].checklist, vec!["a1"]);
    assert_eq!(v.checklist, vec!["Tag"]);
}

#[test]
fn save_drops_dates_and_attachments() {
    let s = Store::open_in_memory().unwrap();
    let t = s.save_template(source(&s), "Rel", false).unwrap();
    let json = serde_json::to_string(&s.template_body(t).unwrap()).unwrap();
    for bad in ["2026", "attachment", "\"id\"", "dueUtc", "\"due\""] {
        assert!(!json.contains(bad), "{bad} in {json}");
    }
    assert_eq!(s.list_templates().unwrap()[0].subtask_count, 3);
}

#[test]
fn subtask_source_rejected() {
    let s = Store::open_in_memory().unwrap();
    let p = source(&s);
    let sub = s.list_subtasks(p).unwrap()[0].id;
    assert!(matches!(
        s.save_template(sub, "X", false),
        Err(StoreError::Invalid(_))
    ));
}

#[test]
fn name_rules() {
    let s = Store::open_in_memory().unwrap();
    let p = s.insert_task(&input("T")).unwrap();
    for bad in ["", "  ", &"x".repeat(61)] {
        assert!(matches!(
            s.save_template(p, bad, false),
            Err(StoreError::Rule(Rule::TemplateName))
        ));
    }
    let id = s.save_template(p, "Évènement", false).unwrap();
    assert!(matches!(
        s.save_template(p, "éVÈNEMENT", false),
        Err(StoreError::Duplicate)
    ));
    s.insert_task(&TaskInput {
        parent_id: Some(p),
        ..input("kid")
    })
    .unwrap();
    assert_eq!(s.save_template(p, "éVÈNEMENT", true).unwrap(), id);
    assert_eq!(s.list_templates().unwrap()[0].subtask_count, 1);
    assert_eq!(s.template_by_name(" ÉVÈNEMENT ").unwrap().unwrap().id, id);
    assert!(s.template_by_name("nope").unwrap().is_none());
}

#[test]
fn rename_and_delete() {
    let s = Store::open_in_memory().unwrap();
    let p = s.insert_task(&input("T")).unwrap();
    let a = s.save_template(p, "A", false).unwrap();
    let b = s.save_template(p, "b", false).unwrap();
    s.rename_template(a, "Zed").unwrap();
    assert!(matches!(
        s.rename_template(a, "B"),
        Err(StoreError::Duplicate)
    ));
    s.rename_template(b, "B").unwrap();
    assert!(matches!(
        s.rename_template(a, ""),
        Err(StoreError::Rule(Rule::TemplateName))
    ));
    assert!(matches!(
        s.rename_template(999, "q"),
        Err(StoreError::NotFound)
    ));
    let names: Vec<String> = s
        .list_templates()
        .unwrap()
        .into_iter()
        .map(|t| t.name)
        .collect();
    assert_eq!(names, vec!["B", "Zed"]);
    s.delete_template(a).unwrap();
    assert!(matches!(s.delete_template(a), Err(StoreError::NotFound)));
    assert_eq!(s.list_templates().unwrap().len(), 1);
}

#[test]
fn create_input_wins() {
    let s = Store::open_in_memory().unwrap();
    let t = s.save_template(source(&s), "Rel", false).unwrap();
    let plan = s.create_plan("Other", "#fff").unwrap();
    let bucket = s.list_buckets(plan).unwrap()[0].id;
    let due = Utc.with_ymd_and_hms(2026, 11, 10, 12, 0, 0).unwrap();
    let id = s
        .create_from_template(
            t,
            &TaskInput {
                due: Some(due),
                due_has_time: true,
                plan_id: Some(plan),
                bucket_id: Some(bucket),
                labels: vec!["work".into(), "Extra".into()],
                ..input("New title")
            },
        )
        .unwrap();
    let task = s.get_task(id).unwrap();
    assert_eq!(task.title, "New title");
    assert_eq!(task.due, Some(due));
    assert_eq!((task.plan_id, task.bucket_id), (plan, bucket));
    assert_eq!(task.priority, Priority::Urgent);
    assert_eq!(task.notes.as_deref(), Some("secret"));
    let mut l: Vec<String> = task.labels.iter().map(|l| l.name.clone()).collect();
    l.sort();
    assert_eq!(l, vec!["Extra", "Work"]);
    // Input priority and notes win when set.
    let id2 = s
        .create_from_template(
            t,
            &TaskInput {
                priority: Priority::Low,
                notes: Some("mine".into()),
                ..input("Two")
            },
        )
        .unwrap();
    let t2 = s.get_task(id2).unwrap();
    assert_eq!(t2.priority, Priority::Low);
    assert_eq!(t2.notes.as_deref(), Some("mine"));
}

#[test]
fn create_children() {
    let s = Store::open_in_memory().unwrap();
    let t = s.save_template(source(&s), "Rel", false).unwrap();
    let due = Utc.with_ymd_and_hms(2026, 11, 10, 12, 0, 0).unwrap();
    let id = s
        .create_from_template(
            t,
            &TaskInput {
                due: Some(due),
                due_has_time: true,
                ..input("N")
            },
        )
        .unwrap();
    assert_eq!(s.list_checklist(id).unwrap()[0].text, "Tag");
    let kids = s.list_subtasks(id).unwrap();
    let titles: Vec<&str> = kids.iter().map(|k| k.title.as_str()).collect();
    assert_eq!(titles, vec!["A", "B", "C"]);
    assert!(kids.iter().all(|k| k.status == TaskStatus::Todo));
    assert_eq!(s.list_checklist(kids[0].id).unwrap()[0].text, "a1");
    assert_eq!(
        kids[0].due,
        Some(Utc.with_ymd_and_hms(2026, 11, 7, 10, 0, 0).unwrap())
    );
    assert!(kids[0].due_has_time);
    assert_eq!(
        kids[1].due,
        Some(Utc.with_ymd_and_hms(2026, 11, 10, 0, 0, 0).unwrap())
    );
    assert!(!kids[1].due_has_time);
    assert_eq!(kids[2].due, None);
}

#[test]
fn offsets_across_dst() {
    let s = Store::open_in_memory().unwrap();
    let rome = |y, m, d, h| {
        Rome.with_ymd_and_hms(y, m, d, h, 0, 0)
            .unwrap()
            .with_timezone(&Utc)
    };
    let mk = |parent_due, sub_due, tz: &str| {
        let p = s
            .insert_task(&TaskInput {
                due: Some(parent_due),
                due_has_time: true,
                tz: tz.into(),
                ..input("P")
            })
            .unwrap();
        s.insert_task(&TaskInput {
            parent_id: Some(p),
            due: Some(sub_due),
            due_has_time: true,
            tz: tz.into(),
            ..input("S")
        })
        .unwrap();
        p
    };
    // Rome DST ends Sun 25 Oct 2026: Mon 26 Oct minus 3 days crosses it.
    let p = mk(rome(2026, 10, 26, 9), rome(2026, 10, 23, 10), "Europe/Rome");
    let t = s.save_template(p, "dst", false).unwrap();
    let v = s.template_body(t).unwrap();
    assert_eq!(v.subtasks[0].due_offset_days, Some(-3));
    assert_eq!(v.subtasks[0].due_time.as_deref(), Some("10:00"));
    let id = s
        .create_from_template(
            t,
            &TaskInput {
                due: Some(rome(2026, 10, 26, 9)),
                due_has_time: true,
                tz: "Europe/Rome".into(),
                ..input("N")
            },
        )
        .unwrap();
    assert_eq!(
        s.list_subtasks(id).unwrap()[0].due,
        Some(rome(2026, 10, 23, 10))
    );
    // DST starts Sun 29 Mar 2026: Mon 30 Mar minus 2 days is Sat 28 Mar.
    let p2 = mk(rome(2026, 3, 30, 9), rome(2026, 3, 28, 10), "Europe/Rome");
    let t2 = s.save_template(p2, "dst2", false).unwrap();
    assert_eq!(
        s.template_body(t2).unwrap().subtasks[0].due_offset_days,
        Some(-2)
    );
    let id2 = s
        .create_from_template(
            t2,
            &TaskInput {
                due: Some(rome(2026, 3, 30, 9)),
                due_has_time: true,
                tz: "Europe/Rome".into(),
                ..input("N2")
            },
        )
        .unwrap();
    assert_eq!(
        s.list_subtasks(id2).unwrap()[0].due,
        Some(rome(2026, 3, 28, 10))
    );
}

#[test]
fn no_due_no_subtask_dues() {
    let s = Store::open_in_memory().unwrap();
    let t = s.save_template(source(&s), "Rel", false).unwrap();
    let id = s.create_from_template(t, &input("N")).unwrap();
    assert!(s.list_subtasks(id).unwrap().iter().all(|k| k.due.is_none()));
    assert!(matches!(
        s.create_from_template(999, &input("x")),
        Err(StoreError::NotFound)
    ));
}
