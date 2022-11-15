use vertix_model::*;
use actix_rt;
use anyhow::Result;
use aragog::Record;

#[actix_rt::test]
async fn follow_publish_and_view_timeline() -> Result<()> {
    let conn = create_connection().await?;

    let account1 = Account::create(Account::new("account1".into()), &conn).await?;
    let account2 = Account::create(Account::new("account2".into()), &conn).await?;

    let mut follow = Follow::link(&account1, &account2, &conn).await?;

    follow.accepted = Some(true);
    follow.save(&conn).await?;

    let note = Note::publish(&account2, Note::new("Hello, world!".into()), &conn).await?;

    let timeline = Account::get_timeline(&account1, &conn).await?;

    assert_eq!(timeline.len(), 1);
    assert!(timeline.iter().any(|t_note| t_note.key() == note.key()));

    Ok(())
}

