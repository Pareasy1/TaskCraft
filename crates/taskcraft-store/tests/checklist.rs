use taskcraft_store::*;

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let s = Store::open(&dir.path().join("t.db")).unwrap();
    (dir, s)
}

fn input(title: &str) -> TaskInput {
    TaskInput {
        calendar_id: 1,
        title: title.into(),
        tz: "UTC".into(),
        ..Default::default()
    }
}

fn is_rule<T: std::fmt::Debug>(r: Result<T>, want: Rule) {
    match r {
        Err(StoreError::Rule(got)) => assert_eq!(got, want),
        other => panic!("expected Rule({want:?}), got {other:?}"),
    }
}

#[test]
fn add_list_order() {
    let (_d, s) = store();
    let t = s.insert_task(&input("t")).unwrap();
    let a = s.add_checklist_item(t, "a").unwrap();
    let b = s.add_checklist_item(t, "b").unwrap();
    let items = s.list_checklist(t).unwrap();
    assert_eq!(items.iter().map(|i| i.id).collect::<Vec<_>>(), vec![a, b]);
    assert_eq!(items[0].text, "a");
    assert!(!items[0].done);
    assert!(items[0].sort < items[1].sort);
    assert_eq!(items[0].task_id, t);
}

#[test]
fn text_trimmed_and_limited() {
    let (_d, s) = store();
    let t = s.insert_task(&input("t")).unwrap();
    let id = s.add_checklist_item(t, "  a  ").unwrap();
    assert_eq!(s.list_checklist(t).unwrap()[0].text, "a");
    is_rule(s.add_checklist_item(t, ""), Rule::ChecklistText);
    is_rule(s.add_checklist_item(t, "   "), Rule::ChecklistText);
    is_rule(
        s.add_checklist_item(t, &"x".repeat(201)),
        Rule::ChecklistText,
    );
    s.add_checklist_item(t, &"x".repeat(200)).unwrap();
    is_rule(
        s.update_checklist_item(id, Some(""), None),
        Rule::ChecklistText,
    );
}

#[test]
fn tick_and_rename() {
    let (_d, s) = store();
    let t = s.insert_task(&input("t")).unwrap();
    let id = s.add_checklist_item(t, "a").unwrap();
    s.update_checklist_item(id, None, Some(true)).unwrap();
    assert!(s.list_checklist(t).unwrap()[0].done);
    assert_eq!(s.get_task(t).unwrap().checklist_done, 1);
    s.update_checklist_item(id, Some(" b "), None).unwrap();
    let it = &s.list_checklist(t).unwrap()[0];
    assert_eq!(it.text, "b");
    assert!(it.done);
    assert!(matches!(
        s.update_checklist_item(999, None, Some(true)),
        Err(StoreError::NotFound)
    ));
}

#[test]
fn reorder_exact_set() {
    let (_d, s) = store();
    let t = s.insert_task(&input("t")).unwrap();
    let a = s.add_checklist_item(t, "a").unwrap();
    let b = s.add_checklist_item(t, "b").unwrap();
    let c = s.add_checklist_item(t, "c").unwrap();
    s.reorder_checklist(t, &[c, a, b]).unwrap();
    let items = s.list_checklist(t).unwrap();
    assert_eq!(
        items.iter().map(|i| i.id).collect::<Vec<_>>(),
        vec![c, a, b]
    );
    assert_eq!(items[0].sort, 1.0);
    assert!(matches!(
        s.reorder_checklist(t, &[a, b]),
        Err(StoreError::Invalid(_))
    ));
    assert!(matches!(
        s.reorder_checklist(t, &[a, b, c, c]),
        Err(StoreError::Invalid(_))
    ));
    assert!(matches!(
        s.reorder_checklist(t, &[a, b, 999]),
        Err(StoreError::Invalid(_))
    ));
    s.delete_checklist_item(a).unwrap();
    assert_eq!(s.list_checklist(t).unwrap().len(), 2);
    assert!(matches!(
        s.delete_checklist_item(a),
        Err(StoreError::NotFound)
    ));
}

#[test]
fn convert_to_subtask() {
    let (_d, s) = store();
    let t = s.insert_task(&input("t")).unwrap();
    let existing = s
        .insert_task(&TaskInput {
            parent_id: Some(t),
            ..input("old")
        })
        .unwrap();
    let item = s.add_checklist_item(t, "do it").unwrap();
    let new = s.checklist_item_to_subtask(item).unwrap();
    let task = s.get_task(new).unwrap();
    assert_eq!(task.title, "do it");
    assert_eq!(task.parent_id, Some(t));
    assert!(s.list_checklist(t).unwrap().is_empty());
    let subs: Vec<i64> = s.list_subtasks(t).unwrap().iter().map(|x| x.id).collect();
    assert_eq!(subs, vec![existing, new]);
}

#[test]
fn convert_on_subtask_rejected() {
    let (_d, s) = store();
    let t = s.insert_task(&input("t")).unwrap();
    let sub = s
        .insert_task(&TaskInput {
            parent_id: Some(t),
            ..input("s")
        })
        .unwrap();
    let item = s.add_checklist_item(sub, "x").unwrap();
    is_rule(s.checklist_item_to_subtask(item), Rule::ParentIsSubtask);
    assert_eq!(s.list_checklist(sub).unwrap().len(), 1);
}

#[test]
fn missing_task_not_found() {
    let (_d, s) = store();
    assert!(matches!(
        s.add_checklist_item(999, "a"),
        Err(StoreError::NotFound)
    ));
    assert!(matches!(s.list_checklist(999), Err(StoreError::NotFound)));
    assert!(matches!(
        s.reorder_checklist(999, &[]),
        Err(StoreError::NotFound)
    ));
    assert!(matches!(
        s.checklist_item_to_subtask(999),
        Err(StoreError::NotFound)
    ));
}
