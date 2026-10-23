use chrono::{Duration, Utc};
use std::path::Path;
use taskcraft_store::*;

fn setup() -> (Store, i64, tempfile::TempDir, tempfile::TempDir) {
    let s = Store::open_in_memory().unwrap();
    let id = s
        .insert_task(&TaskInput {
            title: "t".into(),
            calendar_id: 1,
            tz: "UTC".into(),
            ..Default::default()
        })
        .unwrap();
    (
        s,
        id,
        tempfile::tempdir().unwrap(),
        tempfile::tempdir().unwrap(),
    )
}

fn src(dir: &Path, name: &str, body: &[u8]) -> std::path::PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    p
}

fn is_rule<T: std::fmt::Debug>(r: Result<T>, rule: Rule) {
    assert!(
        matches!(&r, Err(StoreError::Rule(x)) if *x == rule),
        "{r:?}"
    );
}

#[test]
fn add_file_copies_into_data_dir() {
    let (s, t, data, sd) = setup();
    let p = src(sd.path(), "part.stl", b"hello");
    let a = s.add_file_attachment(t, &p, data.path()).unwrap();
    assert_eq!(
        (a.kind, a.size, a.name.as_str()),
        (AttachmentKind::File, Some(5), "part.stl")
    );
    let stored = a.stored.clone().unwrap();
    assert!(stored.starts_with(&format!("attachments/{t}/")), "{stored}");
    assert_eq!(std::fs::read(data.path().join(&stored)).unwrap(), b"hello");
    assert!(p.exists());
    assert_eq!(s.list_attachments(t).unwrap(), vec![a]);
}

#[test]
fn too_large_rejected() {
    let (s, t, data, sd) = setup();
    let p = src(sd.path(), "big.bin", &[0u8; 11]);
    is_rule(
        s.add_file_attachment_with_limit(t, &p, data.path(), 10),
        Rule::AttachmentTooLarge,
    );
    assert!(s.list_attachments(t).unwrap().is_empty());
    let d = data.path().join(format!("attachments/{t}"));
    assert!(!d.exists() || std::fs::read_dir(d).unwrap().count() == 0);
    let ok = src(sd.path(), "ok.bin", &[0u8; 10]);
    s.add_file_attachment_with_limit(t, &ok, data.path(), 10)
        .unwrap();
}

#[test]
fn sanitize_and_unique_names() {
    let n = sanitize_name("a/b\\..:c?.stl");
    assert!(!n.contains('/') && !n.contains('\\') && !n.contains(':') && !n.contains('?'));
    assert_eq!(sanitize_name("caffè 1.txt"), "caffè 1.txt");
    assert_eq!(sanitize_name(".."), "_");
    assert_eq!(sanitize_name(&"x".repeat(300)).chars().count(), 120);

    let (s, t, data, sd) = setup();
    let p = src(sd.path(), "same.txt", b"1");
    let a = s.add_file_attachment(t, &p, data.path()).unwrap();
    let b = s.add_file_attachment(t, &p, data.path()).unwrap();
    assert_ne!(a.stored, b.stored);
    let long = src(sd.path(), &"y".repeat(200), b"1");
    let c = s.add_file_attachment(t, &long, data.path()).unwrap();
    assert!(
        c.stored
            .unwrap()
            .rsplit('/')
            .next()
            .unwrap()
            .chars()
            .count()
            <= 124
    );
}

#[test]
fn missing_source_not_found() {
    let (s, t, data, sd) = setup();
    let r = s.add_file_attachment(t, &sd.path().join("nope"), data.path());
    assert!(matches!(r, Err(StoreError::NotFound)), "{r:?}");
}

#[test]
fn link_validation() {
    let (s, t, _d, _s) = setup();
    let a = s.add_link_attachment(t, "https://x.io/p", None).unwrap();
    assert_eq!(
        (a.kind, a.name.as_str(), a.url.as_deref()),
        (AttachmentKind::Link, "x.io", Some("https://x.io/p"))
    );
    let b = s
        .add_link_attachment(t, "http://y.org", Some(" Doc "))
        .unwrap();
    assert_eq!(b.name, "Doc");
    for bad in [
        "ftp://x.io/f",
        "javascript:alert(1)",
        "x.io",
        "https://",
        "https:// ",
    ] {
        is_rule(s.add_link_attachment(t, bad, None), Rule::AttachmentUrl);
    }
}

#[test]
fn list_rename_reorder() {
    let (s, t, _d, _s) = setup();
    let a = s.add_link_attachment(t, "https://a.io", None).unwrap();
    let b = s.add_link_attachment(t, "https://b.io", None).unwrap();
    let c = s.add_link_attachment(t, "https://c.io", None).unwrap();
    s.rename_attachment(b.id, "  Bee ").unwrap();
    assert!(matches!(
        s.rename_attachment(b.id, "  "),
        Err(StoreError::Invalid(_))
    ));
    assert!(matches!(
        s.rename_attachment(b.id, &"z".repeat(121)),
        Err(StoreError::Invalid(_))
    ));
    assert!(matches!(
        s.rename_attachment(999, "q"),
        Err(StoreError::NotFound)
    ));
    s.reorder_attachments(t, &[c.id, a.id, b.id]).unwrap();
    let l = s.list_attachments(t).unwrap();
    assert_eq!(
        l.iter().map(|x| x.id).collect::<Vec<_>>(),
        vec![c.id, a.id, b.id]
    );
    assert_eq!(l[2].name, "Bee");
    assert!(matches!(
        s.reorder_attachments(t, &[a.id]),
        Err(StoreError::Invalid(_))
    ));
}

#[test]
fn remove_queues_then_purge_after_delay() {
    let (s, t, data, sd) = setup();
    let a = s
        .add_file_attachment(t, &src(sd.path(), "f.txt", b"1"), data.path())
        .unwrap();
    let full = data.path().join(a.stored.as_ref().unwrap());
    let now = Utc::now();
    let r = s.remove_attachment(a.id, now).unwrap();
    assert_eq!(r.id, a.id);
    assert!(s.list_attachments(t).unwrap().is_empty());
    assert_eq!(
        s.purge_pending_files(now + Duration::seconds(10), data.path())
            .unwrap(),
        0
    );
    assert!(full.exists());
    assert_eq!(
        s.purge_pending_files(now + Duration::seconds(16), data.path())
            .unwrap(),
        1
    );
    assert!(!full.exists());
    assert_eq!(
        s.purge_pending_files(now + Duration::seconds(60), data.path())
            .unwrap(),
        0
    );
}

#[test]
fn restore_relinks_queued_file() {
    let (s, t, data, sd) = setup();
    let a = s
        .add_file_attachment(t, &src(sd.path(), "f.txt", b"abc"), data.path())
        .unwrap();
    let stored = a.stored.clone().unwrap();
    // not queued yet -> NotFound
    assert!(matches!(
        s.restore_file_attachment(t, "f.txt", &stored, 3),
        Err(StoreError::NotFound)
    ));
    let now = Utc::now();
    s.remove_attachment(a.id, now).unwrap();
    let r = s.restore_file_attachment(t, "f.txt", &stored, 3).unwrap();
    assert_eq!(
        (r.stored.as_deref(), r.size, r.kind),
        (Some(stored.as_str()), Some(3), AttachmentKind::File)
    );
    assert_eq!(s.list_attachments(t).unwrap().len(), 1);
    // un-queued: purge keeps the file
    assert_eq!(
        s.purge_pending_files(now + Duration::seconds(60), data.path())
            .unwrap(),
        0
    );
    assert!(data.path().join(&stored).exists());
    // queued, then purged (file gone) -> NotFound
    s.remove_attachment(r.id, now).unwrap();
    s.purge_pending_files(now + Duration::seconds(60), data.path())
        .unwrap();
    assert!(!data.path().join(&stored).exists());
    let r2 = s.restore_file_attachment(t, "f.txt", &stored, 3);
    assert!(matches!(r2, Err(StoreError::NotFound)), "{r2:?}");
    // hostile stored path
    assert!(matches!(
        s.restore_file_attachment(t, "f", "../x", 1),
        Err(StoreError::Invalid(_))
    ));
}

#[test]
fn cascade_queues_files() {
    let (s, t, data, sd) = setup();
    let sub = s
        .insert_task(&TaskInput {
            title: "sub".into(),
            calendar_id: 1,
            tz: "UTC".into(),
            parent_id: Some(t),
            ..Default::default()
        })
        .unwrap();
    let a = s
        .add_file_attachment(t, &src(sd.path(), "p.txt", b"1"), data.path())
        .unwrap();
    let b = s
        .add_file_attachment(sub, &src(sd.path(), "s.txt", b"2"), data.path())
        .unwrap();
    s.add_link_attachment(t, "https://x.io", None).unwrap();
    s.delete_task(t).unwrap();
    assert!(s.list_attachments(t).unwrap().is_empty());
    assert!(s.list_attachments(sub).unwrap().is_empty());
    let now = Utc::now() + Duration::seconds(16);
    assert_eq!(s.purge_pending_files(now, data.path()).unwrap(), 2);
    assert!(!data.path().join(a.stored.unwrap()).exists());
    assert!(!data.path().join(b.stored.unwrap()).exists());
}

#[test]
fn path_escape_refused() {
    let data = tempfile::tempdir().unwrap();
    let sd = tempfile::tempdir().unwrap();
    let db = data.path().join("t.db");
    let s = Store::open(&db).unwrap();
    let t = s
        .insert_task(&TaskInput {
            title: "t".into(),
            calendar_id: 1,
            tz: "UTC".into(),
            ..Default::default()
        })
        .unwrap();
    let a = s
        .add_file_attachment(t, &src(sd.path(), "f.txt", b"1"), data.path())
        .unwrap();
    let ok = s.attachment_path(a.id, data.path()).unwrap();
    assert!(ok.ends_with(a.stored.as_ref().unwrap().rsplit('/').next().unwrap()));
    // a file outside attachments/ that a hostile row points at
    std::fs::write(data.path().join("x"), b"secret").unwrap();
    let c = rusqlite::Connection::open(&db).unwrap();
    let ins = |stored: &str| -> i64 {
        c.execute(
            "INSERT INTO attachments(task_id, kind, name, stored, size, created_at, sort) VALUES (?1,'file','e',?2,1,'2026-10-01T00:00:00Z',99)",
            rusqlite::params![t, stored],
        )
        .unwrap();
        c.last_insert_rowid()
    };
    let evil = ins(&format!("attachments/{t}/../../x"));
    assert!(matches!(
        s.attachment_path(evil, data.path()),
        Err(StoreError::Invalid(_))
    ));
    let gone = ins(&format!("attachments/{t}/missing"));
    assert!(matches!(
        s.attachment_path(gone, data.path()),
        Err(StoreError::NotFound)
    ));
    let link = s.add_link_attachment(t, "https://x.io", None).unwrap();
    assert!(matches!(
        s.attachment_path(link.id, data.path()),
        Err(StoreError::Invalid(_))
    ));
}

#[test]
fn attachment_count_on_task() {
    let (s, t, data, sd) = setup();
    assert_eq!(s.get_task(t).unwrap().attachment_count, 0);
    s.add_link_attachment(t, "https://x.io", None).unwrap();
    s.add_file_attachment(t, &src(sd.path(), "f", b"1"), data.path())
        .unwrap();
    assert_eq!(s.get_task(t).unwrap().attachment_count, 2);
    let l = s
        .list_tasks(TaskFilter::All, Utc::now(), chrono_tz::Tz::UTC)
        .unwrap();
    assert_eq!(l.iter().find(|x| x.id == t).unwrap().attachment_count, 2);
}

#[cfg(unix)]
#[test]
fn unreadable_source_not_found() {
    use std::os::unix::fs::PermissionsExt;
    let (s, t, data, sd) = setup();
    let p = src(sd.path(), "locked", b"x");
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::File::open(&p).is_ok() {
        return; // running as root
    }
    let r = s.add_file_attachment(t, &p, data.path());
    assert!(matches!(r, Err(StoreError::NotFound)), "{r:?}");
    assert!(s.list_attachments(t).unwrap().is_empty());
    let d = data.path().join(format!("attachments/{t}"));
    assert_eq!(std::fs::read_dir(d).unwrap().count(), 0);
}

#[test]
fn long_non_ascii_name_fits() {
    let (s, t, data, sd) = setup();
    for name in [format!("{}.stl", "漢".repeat(150)), "😀".repeat(120)] {
        let a = s
            .add_file_attachment(t, &src(sd.path(), &name, b"1"), data.path())
            .unwrap();
        let stored = a.stored.unwrap();
        let file = stored.rsplit('/').next().unwrap();
        assert!(file.len() <= 255, "{}", file.len());
        assert!(data.path().join(&stored).exists());
        if name.ends_with(".stl") {
            assert!(file.ends_with(".stl"), "{file}");
        }
    }
}

#[cfg(unix)]
#[test]
fn purge_does_not_follow_symlinks_out() {
    let (s, t, data, sd) = setup();
    let outside = src(sd.path(), "keep.txt", b"keep");
    let a = s.add_file_attachment(t, &outside, data.path()).unwrap();
    let stored = a.stored.clone().unwrap();
    let now = Utc::now();
    s.remove_attachment(a.id, now).unwrap();
    // swap the task folder for a symlink to an outside directory
    let dir = data.path().join(format!("attachments/{t}"));
    let victim = sd.path().join("victim");
    std::fs::create_dir(&victim).unwrap();
    let name = stored.rsplit('/').next().unwrap();
    std::fs::write(victim.join(name), b"precious").unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    std::os::unix::fs::symlink(&victim, &dir).unwrap();
    s.purge_pending_files(now + Duration::seconds(60), data.path())
        .unwrap();
    assert!(victim.join(name).exists());
}
