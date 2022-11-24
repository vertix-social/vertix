use activitystreams::{collection::{OrderedCollection, OrderedCollectionPage}, object, activity, primitives::XsdAnyUri};
use actix_web::{web, get, Responder};
use chrono::{DateTime, FixedOffset};
use vertix_model::{Account, PageLimit, Recipient};

use crate::{ApiState, error::Result, formats::ActivityJson};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_account_outbox);
    cfg.service(get_account_outbox_page);
}

#[get("/users/{username}/outbox")]
pub async fn get_account_outbox(
    username: web::Path<String>,
    state: web::Data<ApiState>
) -> Result<impl Responder> {
    let db = state.pool.get().await?;

    let _ = Account::find_by_username(&*username, None, &*db).await?;

    let mut collection = OrderedCollection::new();
    
    let domain = &state.domain;

    collection.object_props.set_context_xsd_any_uri(activitystreams::context())?;
    collection.object_props.set_id(format!("http://{domain}/users/{username}/outbox"))?;
    collection.collection_props.set_first_xsd_any_uri(
        format!("http://{domain}/users/{username}/outbox/page/1"))?;

    Ok(ActivityJson(collection))
}

#[get("/users/{username}/outbox/page/{page}")]
pub async fn get_account_outbox_page(
    params: web::Path<(String, u32)>,
    state: web::Data<ApiState>
) -> Result<impl Responder> {
    let (username, page) = params.into_inner();
    let page_limit = PageLimit { page, ..PageLimit::default() };

    let domain = &state.domain;

    let db = state.pool.get().await?;

    let account = Account::find_by_username(&*username, None, &*db).await?;

    let mut col_page = OrderedCollectionPage::new();

    col_page.object_props.set_context_xsd_any_uri(activitystreams::context())?;

    let notes = Account::get_published_notes(&account, page_limit, &*db).await?;

    let items = notes.iter().map(|note| {
        let mut as_create = activity::Create::full();
        let mut as_note = object::Note::full();

        if let Some(created_at) = note.created_at.clone() {
            as_create.base.object_props.set_published(DateTime::<FixedOffset>::from(created_at))?;
        }

        fn recipient_to_uri(domain: &str, r: &Recipient) -> Result<XsdAnyUri> {
            match r {
                Recipient::Public => Ok(activitystreams::public()),
                Recipient::Account(id) =>
                    Ok(XsdAnyUri::try_from(format!("http://{domain}/users/{id}"))?)
            }
        }
    
        fn recipients_to_uri(domain: &str, rs: &[Recipient]) -> Result<Vec<XsdAnyUri>> {
            rs.iter().map(|r| recipient_to_uri(domain, r)).collect()
        }

        as_create.base.object_props.set_many_to_xsd_any_uris(
            recipients_to_uri(domain, &note.to)?)?;
        as_create.base.object_props.set_many_cc_xsd_any_uris(
            recipients_to_uri(domain, &note.cc)?)?;

        as_note.base.object_props.set_content_xsd_string(note.content.clone())?;

        as_create.base.create_props.set_actor_xsd_any_uri(
            format!("http://{domain}/users/{username}"))?;
        as_create.base.create_props.set_object_base_box(as_note)?;

        Ok(as_create)
    }).collect::<Result<Vec<_>>>()?;

    let domain = &state.domain;

    col_page.collection_page_props.set_part_of_xsd_any_uri(
        format!("http://{domain}/users/{username}/outbox"))?;

    if items.len() >= page_limit.limit as usize {
        col_page.collection_page_props.set_next_xsd_any_uri(
            format!("http://{domain}/users/{username}/outbox/page/{}", page + 1))?;
    }

    col_page.collection_props.set_many_items_base_boxes(items)?;

    Ok(ActivityJson(col_page))
}
