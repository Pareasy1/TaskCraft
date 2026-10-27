use std::path::Path;
use taskcraft_store::*;

fn mk(s: &Store, title: &str, parent: Option<i64>) -> i64 {
    s.insert_task(&TaskInput {
        title: title.into(),
        calendar_id: 1,
        tz: "UTC".into(),
        parent_id: parent,
        ..Default::default()
    })
    .unwrap()
}

fn is_rule<T: std::fmt::Debug>(r: Result<T>, rule: Rule) {
    assert!(
        matches!(&r, Err(StoreError::Rule(x)) if *x == rule),
        "{r:?}"
    );
}

fn dep(task_id: i64, depends_on: i64) -> Dependency {
    Dependency {
        task_id,
        depends_on,
    }
}

#[test]
fn add_and_list() {
    let s = Store::open_in_memory().unwrap();
    let (a, b) = (mk(&s, "a", None), mk(&s, "b", None));
    s.add_dependency(b, a).unwrap();
    assert_eq!(s.list_dependencies(None).unwrap(), vec![dep(b, a)]);
}

#[test]
fn add_is_idempotent() {
    let s = Store::open_in_memory().unwrap();
    let (a, b) = (mk(&s, "a", None), mk(&s, "b", None));
    s.add_dependency(b, a).unwrap();
    s.add_dependency(b, a).unwrap();
    assert_eq!(s.list_dependencies(None).unwrap().len(), 1);
}

#[test]
fn self_refused() {
    let s = Store::open_in_memory().unwrap();
    let a = mk(&s, "a", None);
    let r = s.add_dependency(a, a);
    is_rule(r, Rule::DependencySelf);
    assert_eq!(Rule::DependencySelf.code(), "dependency_self");
}

#[test]
fn direct_cycle_refused() {
    let s = Store::open_in_memory().unwrap();
    let (a, b) = (mk(&s, "a", None), mk(&s, "b", None));
    s.add_dependency(a, b).unwrap();
    is_rule(s.add_dependency(b, a), Rule::DependencyCycle);
    assert_eq!(s.list_dependencies(None).unwrap().len(), 1);
}

#[test]
fn three_long_cycle_refused() {
    let s = Store::open_in_memory().unwrap();
    let (a, b, c) = (mk(&s, "a", None), mk(&s, "b", None), mk(&s, "c", None));
    s.add_dependency(a, b).unwrap();
    s.add_dependency(b, c).unwrap();
    is_rule(s.add_dependency(c, a), Rule::DependencyCycle);
}

#[test]
fn unknown_task_not_found() {
    let s = Store::open_in_memory().unwrap();
    let a = mk(&s, "a", None);
    assert!(matches!(
        s.add_dependency(a, 999),
        Err(StoreError::NotFound)
    ));
    assert!(matches!(
        s.add_dependency(999, a),
        Err(StoreError::NotFound)
    ));
}

#[test]
fn remove_is_noop_when_missing() {
    let s = Store::open_in_memory().unwrap();
    let (a, b) = (mk(&s, "a", None), mk(&s, "b", None));
    s.remove_dependency(b, a).unwrap();
    s.add_dependency(b, a).unwrap();
    s.remove_dependency(b, a).unwrap();
    assert!(s.list_dependencies(None).unwrap().is_empty());
}

#[test]
fn blocked_by_counts_open_only() {
    let s = Store::open_in_memory().unwrap();
    let (a, b, c) = (mk(&s, "a", None), mk(&s, "b", None), mk(&s, "c", None));
    s.add_dependency(b, a).unwrap();
    s.add_dependency(b, c).unwrap();
    assert_eq!(s.get_task(b).unwrap().blocked_by, 2);
    assert_eq!(s.get_task(a).unwrap().blocked_by, 0);
    s.set_task_completed(a, true, chrono::Utc::now()).unwrap();
    assert_eq!(s.get_task(b).unwrap().blocked_by, 1);
}

#[test]
fn delete_removes_both_directions() {
    let s = Store::open_in_memory().unwrap();
    let (a, b, c) = (mk(&s, "a", None), mk(&s, "b", None), mk(&s, "c", None));
    s.add_dependency(b, a).unwrap();
    s.add_dependency(c, b).unwrap();
    s.delete_task(b).unwrap();
    assert!(s.list_dependencies(None).unwrap().is_empty());
}

#[test]
fn delete_parent_removes_subtask_rows() {
    let s = Store::open_in_memory().unwrap();
    let (p, other) = (mk(&s, "p", None), mk(&s, "o", None));
    let sub = mk(&s, "sub", Some(p));
    s.add_dependency(sub, other).unwrap();
    s.add_dependency(other, sub).unwrap_err(); // cycle, refused
    s.delete_task(p).unwrap();
    assert!(s.list_dependencies(None).unwrap().is_empty());
}

#[test]
fn duplicate_does_not_copy() {
    let s = Store::open_in_memory().unwrap();
    let (a, b) = (mk(&s, "a", None), mk(&s, "b", None));
    s.add_dependency(b, a).unwrap();
    let r = s
        .duplicate_task(b, " copy", Path::new("/nonexistent"))
        .unwrap();
    assert_eq!(s.get_task(r.id).unwrap().blocked_by, 0);
    assert_eq!(s.list_dependencies(None).unwrap(), vec![dep(b, a)]);
}

#[test]
fn list_filtered_by_ids() {
    let s = Store::open_in_memory().unwrap();
    let (a, b, c, d) = (
        mk(&s, "a", None),
        mk(&s, "b", None),
        mk(&s, "c", None),
        mk(&s, "d", None),
    );
    s.add_dependency(b, a).unwrap();
    s.add_dependency(c, b).unwrap();
    s.add_dependency(d, c).unwrap();
    assert_eq!(
        s.list_dependencies(Some(&[b])).unwrap(),
        vec![dep(b, a), dep(c, b)]
    );
    assert!(s.list_dependencies(Some(&[])).unwrap().is_empty());
}
