use std::sync::Arc;

use actix_web::cookie::time::{Duration, OffsetDateTime};
use actix_web::cookie::{Cookie, SameSite};
use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse};
use fernet::Fernet;
use minijinja;
use serde::Serialize;

use crate::{constant, squire};

/// Struct for representing a JSON Response with a redirect URL.
#[derive(Serialize)]
struct RedirectResponse {
    redirect_url: String,
}

/// Struct for representing detailed errors in JSON format.
#[derive(Serialize)]
pub struct DetailError {
    pub detail: String,
}

/// Handles the login endpoint, verifying credentials and creating session tokens.
///
/// # Arguments
///
/// * `request` - A reference to the Actix web `HttpRequest` object.
/// * `config` - Configuration data for the application.
/// * `fernet` - Fernet object to encrypt the auth payload that will be set as `session_token` cookie.
/// * `session` - Session struct that holds the `session_mapping` to handle sessions.
///
/// # Returns
///
/// * `200` - HttpResponse with a `session_token` and redirect URL to the `/monitor` entrypoint.
/// * `401` - HttpResponse with an error message for failed authentication.
#[post("/login")]
pub async fn login(request: HttpRequest,
                   config: web::Data<Arc<squire::settings::Config>>,
                   fernet: web::Data<Arc<Fernet>>,
                   session: web::Data<Arc<constant::Session>>) -> HttpResponse {
    let verified = squire::authenticator::verify_login(&request, &config, &session);
    if let Err(err) = verified {
        let err_message = err.to_string();
        log::warn!("Error response::{}", err_message);
        return HttpResponse::Unauthorized().json(DetailError {
            detail: err_message
        });
    }

    let mapped = verified.unwrap();
    let payload = serde_json::to_string(&mapped).unwrap();
    let encrypted_payload = fernet.encrypt(payload.as_bytes());

    let cookie_duration = Duration::seconds(config.session_duration);
    let expiration = OffsetDateTime::now_utc() + cookie_duration;
    let cookie = Cookie::build("session_token", encrypted_payload)
        .http_only(true)
        .same_site(SameSite::Strict)
        .max_age(cookie_duration)
        .expires(expiration)
        .finish();
    log::info!("Session for '{}' will be valid until {}", mapped.get("username").unwrap(), expiration);

    let mut response = HttpResponse::Ok().json(RedirectResponse {
        redirect_url: "/monitor".to_string(),
    });
    response.add_cookie(&cookie).unwrap();
    response
}

/// Handles the logout endpoint, logging out the user and rendering the appropriate HTML page.
///
/// # Arguments
///
/// * `request` - A reference to the Actix web `HttpRequest` object.
/// * `fernet` - Fernet object to encrypt the auth payload that will be set as `session_token` cookie.
/// * `session` - Session struct that holds the `session_mapping` to handle sessions.
/// * `metadata` - Struct containing metadata of the application.
/// * `config` - Configuration data for the application.
/// * `template` - Configuration container for the loaded templates.
///
/// # Returns
///
/// Returns an `HTTPResponse` with the cookie for `session_token` reset if available.
#[get("/logout")]
pub async fn logout(request: HttpRequest,
                    fernet: web::Data<Arc<Fernet>>,
                    session: web::Data<Arc<constant::Session>>,
                    metadata: web::Data<Arc<constant::MetaData>>,
                    config: web::Data<Arc<squire::settings::Config>>,
                    template: web::Data<Arc<minijinja::Environment<'static>>>) -> HttpResponse {
    let host = request.connection_info().host().to_owned();
    let logout_template = template.get_template("logout").unwrap();
    let mut response = HttpResponse::build(StatusCode::OK);
    response.content_type("text/html; charset=utf-8");

    let rendered;
    let auth_response = squire::authenticator::verify_token(&request, &config, &fernet, &session);
    log::debug!("Session Validation Response: {}", auth_response.detail);

    if auth_response.username != "NA" {
        log::info!("{} from {} attempted to log out", auth_response.username, host)
    }

    if auth_response.ok {
        rendered = logout_template.render(minijinja::context!(
            version => metadata.pkg_version,
            detail => "You have been logged out successfully."
        )).unwrap();

        let mut cookie = Cookie::new("session_token", "");
        cookie.set_same_site(SameSite::Strict);
        cookie.make_removal();
        response.cookie(cookie);
    } else {
        log::debug!("{} - {}", auth_response.detail, host);
        rendered = logout_template.render(minijinja::context!(
                version => metadata.pkg_version,
                detail => "You are not logged in. Please click the button below to proceed.",
                show_login => true
            )).unwrap();
    }
    response.body(rendered)
}

/// Handles the error endpoint, rendering the appropriate HTML page based on session issues.
///
/// # Arguments
///
/// * `request` - A reference to the Actix web `HttpRequest` object.
/// * `metadata` - Struct containing metadata of the application.
/// * `template` - Configuration container for the loaded templates.
///
/// # Returns
///
/// HttpResponse with either a session expiry or unauthorized message.
#[get("/error")]
pub async fn error(request: HttpRequest,
                   metadata: web::Data<Arc<constant::MetaData>>,
                   template: web::Data<Arc<minijinja::Environment<'static>>>) -> HttpResponse {
    if let Some(detail) = request.cookie("detail") {
        log::info!("Error response for /error: {}", detail.value());
        let session = template.get_template("session").unwrap();
        return HttpResponse::build(StatusCode::UNAUTHORIZED)
            .content_type("text/html; charset=utf-8")
            .body(session.render(minijinja::context!(
                version => metadata.pkg_version,
                reason => detail.value()
            )).unwrap());
    }

    log::info!("Sending unauthorized response for /error");
    let error = template.get_template("error").unwrap();
    HttpResponse::build(StatusCode::UNAUTHORIZED)
        .content_type("text/html; charset=utf-8")
        .body(error.render(minijinja::context!(
            version => metadata.pkg_version,
            title => "LOGIN FAILED",
            description => "USER ERROR - REPLACE USER",
            help => r"Forgot Password?\n\nRelax and try to remember your password.",
            button_text => "LOGIN", button_link => "/",
            block_navigation => true
        )).unwrap())
}

/// Constructs an `HttpResponse` for failed `session_token` verification.
///
/// # Arguments
///
/// * `auth_response` - The authentication response containing details of the failure.
/// * `config` - Configuration data for the application.
///
/// # Returns
///
/// Returns an `HttpResponse` with a redirect, setting a cookie with the failure detail.
pub fn failed_auth(auth_response: squire::authenticator::AuthToken) -> HttpResponse {
    let mut response = HttpResponse::build(StatusCode::FOUND);
    let detail = auth_response.detail;
    let age = Duration::new(3, 0);
    let cookie = Cookie::build("detail", detail)
        .path("/error")
        .http_only(true)
        .same_site(SameSite::Strict)
        .max_age(age)
        .finish();
    response.cookie(cookie);
    response.append_header(("Location", "/error"));
    response.finish()
}
