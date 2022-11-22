use activitystreams::{collection::{OrderedCollection, OrderedCollectionPage}, object};
use actix_web::{web, get, Responder};
use vertix_model::{Account, PageLimit};

use crate::{ApiState, error::Result};

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

    collection.collection_props.set_first_xsd_any_uri(
        format!("http://{domain}/users/{username}/outbox/page/1"))?;

    Ok(web::Json(collection))
}

#[get("/users/{username}/outbox/page/{page}")]
pub async fn get_account_outbox_page(
    params: web::Path<(String, u32)>,
    state: web::Data<ApiState>
) -> Result<impl Responder> {
    let (username, page) = params.into_inner();
    let page_limit = PageLimit { page, ..PageLimit::default() };

    let db = state.pool.get().await?;

    let account = Account::find_by_username(&*username, None, &*db).await?;

    let mut page = OrderedCollectionPage::new();

    let notes = Account::get_published_notes(&account, page_limit, &*db).await?;

    let items = notes.iter().map(|note| {
        let mut as_note = object::Note::full();
        as_note.base.object_props.set_content_xsd_string(note.content.clone())?;
        Ok(as_note)
    }).collect::<Result<Vec<_>>>()?;

    let domain = &state.domain;

    page.collection_page_props.set_part_of_xsd_any_uri(
        format!("http://{domain}/users/{username}/outbox"))?;

    page.collection_props.set_many_items_base_boxes(items)?;

    Ok(web::Json(page))
}
