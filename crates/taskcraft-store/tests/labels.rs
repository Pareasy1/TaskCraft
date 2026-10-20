use taskcraft_store::*;

fn task(s: &Store, title: &str, labels: &[&str]) -> i64 {
    s.insert_task(&TaskInput {
        calendar_id: 1,
        title: title.into(),
        tz: "Europe/Rome".into(),
        labels: labels.iter().map(|l| l.to_string()).collect(),
        ..Default::default()
    })
    .unwrap()
}

fn names(ls: &[TaskLabel]) -> Vec<&str> {
    ls.iter().map(|l| l.name.as_str()).collect()
}

fn id_of(s: &Store, name: &str) -> i64 {
    s.list_labels()
        .unwrap()
        .into_iter()
        .find(|l| l.name.eq_ignore_ascii_case(name))
        .unwrap()
        .id
}

#[test]
fn labels_created_and_reused_case_insensitively() {
    let s = Store::open_in_memory().unwrap();
    let a = task(&s, "A", &["home", "#Urgent"]);
    let b = task(&s, "B", &["HOME"]);
    let all = s.list_labels().unwrap();
    assert_eq!(names(&all), ["home", "Urgent"]);
    assert_eq!(all[0].color, "#3b82f6");
    assert_eq!(all[1].color, "#f97316");
    let home = id_of(&s, "home");
    assert!(s.get_task(a).unwrap().labels.iter().any(|l| l.id == home));
    assert_eq!(s.get_task(b).unwrap().labels[0].id, home);
}

#[test]
fn update_replaces_links() {
    let s = Store::open_in_memory().unwrap();
    let a = task(&s, "A", &["home"]);
    task(&s, "B", &["home"]);
    let mut input = TaskInput {
        calendar_id: 1,
        title: "A".into(),
        tz: "Europe/Rome".into(),
        ..Default::default()
    };
    input.labels = vec!["work".into()];
    s.update_task(a, &input).unwrap();
    assert_eq!(names(&s.get_task(a).unwrap().labels), ["work"]);
    assert_eq!(names(&s.list_labels().unwrap()), ["home", "work"]);
}

#[test]
fn rename_and_merge() {
    let s = Store::open_in_memory().unwrap();
    let a = task(&s, "A", &["home", "HOUSE"]);
    let house = id_of(&s, "house");
    s.update_label(house, "Home", "#123456").unwrap();
    assert_eq!(names(&s.list_labels().unwrap()), ["home"]);
    assert_eq!(names(&s.get_task(a).unwrap().labels), ["home"]);
}

#[test]
fn rename_and_recolor_in_place() {
    let s = Store::open_in_memory().unwrap();
    let a = task(&s, "A", &["home"]);
    let home = id_of(&s, "home");
    s.update_label(home, "House", "#123456").unwrap();
    let l = &s.get_task(a).unwrap().labels[0];
    assert_eq!(
        (l.id, l.name.as_str(), l.color.as_str()),
        (home, "House", "#123456")
    );
}

#[test]
fn rename_rejects_empty() {
    let s = Store::open_in_memory().unwrap();
    task(&s, "A", &["home"]);
    let home = id_of(&s, "home");
    assert!(matches!(
        s.update_label(home, "  ", "#123456"),
        Err(StoreError::Invalid(_))
    ));
}

#[test]
fn delete_label_unlinks() {
    let s = Store::open_in_memory().unwrap();
    let b = task(&s, "B", &["home"]);
    s.delete_label(id_of(&s, "home")).unwrap();
    assert!(s.get_task(b).unwrap().labels.is_empty());
    assert!(s.list_labels().unwrap().is_empty());
    assert!(matches!(s.delete_label(999), Err(StoreError::NotFound)));
}

#[test]
fn list_and_get_return_labels() {
    let s = Store::open_in_memory().unwrap();
    let a = task(&s, "A", &["zeta", "alpha"]);
    assert_eq!(names(&s.get_task(a).unwrap().labels), ["alpha", "zeta"]);
    let all = s
        .list_tasks(TaskFilter::All, chrono::Utc::now(), chrono_tz::Europe::Rome)
        .unwrap();
    assert_eq!(names(&all[0].labels), ["alpha", "zeta"]);
}

#[test]
fn search_matches_label() {
    let s = Store::open_in_memory().unwrap();
    let a = task(&s, "A", &["urgent"]);
    let hits = s.search("urgent").unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, a);
}

#[test]
fn deleting_task_removes_links() {
    let s = Store::open_in_memory().unwrap();
    let a = task(&s, "A", &["home"]);
    s.delete_task(a).unwrap();
    let b = task(&s, "B", &[]);
    assert!(s.get_task(b).unwrap().labels.is_empty());
    // The label survives and has no links left.
    s.delete_label(id_of(&s, "home")).unwrap();
}

#[test]
fn blank_and_overlong_names_dropped() {
    let s = Store::open_in_memory().unwrap();
    let long = "x".repeat(33);
    let a = task(&s, "A", &["  ", "#", &long, " ok "]);
    assert_eq!(names(&s.get_task(a).unwrap().labels), ["ok"]);
}
