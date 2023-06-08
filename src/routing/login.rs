use actix_web::http::StatusCode;
use actix_web::web::Data;
use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;

pub async fn login_get(
    session: actix_session::Session,
    state: Data<crate::state::AppState>,
) -> impl Responder {
    if let Some(x) = state.redirect_to_profile(&session) {
        return x;
    }
    login_template(session, false, state).await
}

#[derive(Deserialize)]
pub struct FormData {
    username: String,
    password: String,
}

pub async fn login_post(
    session: actix_session::Session,
    form_data: web::Form<FormData>,
    pool: Data<sqlx::PgPool>,
    state: Data<crate::state::AppState>,
) -> impl Responder {
    if let Some(x) = state.redirect_to_profile(&session) {
        return x;
    }
    let mut transaction = match pool.begin().await {
        Ok(x) => x,
        Err(_) => return login_template(session, true, state).await,
    };
    if !sqlx::query!(
        "SELECT username FROM Users where username=$1 and password=crypt($2, password)",
        form_data.username,
        form_data.password
    )
    .fetch_all(&mut transaction)
    .await
    .unwrap_or_default()
    .is_empty()
    {
        if transaction.commit().await.is_ok() {
            crate::utils::set_cookie_param(&session, form_data.username.to_string()).unwrap();
            return HttpResponse::build(StatusCode::FOUND)
                .insert_header((
                    actix_web::http::header::LOCATION,
                    format!(
                        "{}/{}",
                        state.route_from_enum(super::RoutesEnum::Profile),
                        form_data.username
                    ),
                ))
                .finish();
        } else {
            return login_template(session, true, state).await;
        }
    }
    transaction.rollback().await.unwrap();
    login_template(session, true, state).await
}

async fn login_template(
    session: actix_session::Session,
    error: bool,
    state: Data<crate::state::AppState>,
) -> HttpResponse {
    let mut context = tera::Context::new();
    if error {
        context.insert("error", "invalid user");
    }
    state
        .render_template("login.j2", session, &mut context)
        .unwrap()
}
