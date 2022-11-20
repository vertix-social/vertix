use vertix_model::*;
use actix_rt;
use anyhow::Result;
use aragog::Record;
use test_log::test;

#[test(actix_rt::test)]
async fn follow_publish_and_view_timeline() -> Result<()> {
    let conn = create_connection().await?;

    let account1 = Account::create(Account::new("account1".into()), &conn).await?;
    let account2 = Account::create(Account::new("account2".into()), &conn).await?;

    let mut follow = Follow::link(&account1, &account2, &conn).await?;

    follow.accepted = Some(true);
    follow.save(&conn).await?;

    let note = Note::publish(&account2, Note::new("Hello, world!".into()), &conn).await?;

    let timeline = Account::get_timeline(&account1, PageLimit::default(), &conn).await?;

    assert_eq!(timeline.len(), 1);
    assert!(timeline.iter().any(|t_note| t_note.key() == note.key()));

    let note2 = Note::publish(&account2, Note::new("This is my second post.".into()), &conn).await?;

    let timeline = Account::get_timeline(&account1, PageLimit::default(), &conn).await?;

    assert_eq!(timeline.len(), 2);
    assert_eq!(timeline[0].id(), note2.id());
    assert_eq!(timeline[1].id(), note.id());

    Ok(())
}

