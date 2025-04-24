use crate::mint::Mint;
use crate::routes::exchange::post_exchange;
use monexo_core::blind::{BlindedMessage, BlindedSignature};
use monexo_core::keyset::{Keyset, Keysets};
use monexo_core::primitives::{
    CurrencyUnit, MintInfoResponse, PostCurrencyExchangeRequest, PostCurrencyExchangeResponse,
    PostMeltOnchainRequest, PostMeltOnchainResponse, PostMeltQuoteOnchainRequest,
    PostMeltQuoteOnchainResponse, PostMintQuoteOnchainRequest, PostMintQuoteOnchainResponse,
    PostSwapRequest, PostSwapResponse,
};
use monexo_core::proof::{P2SHScript, Proof, Proofs};
use tracing::info;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;

use tower_http::cors::{Any, CorsLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::routes::default::{
    get_info, get_keys, get_keys_by_id, get_keysets, post_check_state, post_swap,
};
use crate::routes::onchain::{
    get_melt_quote_onchain, get_mint_quote_onchain, post_melt_onchain, post_melt_quote_onchain,
    post_mint_onchain, post_mint_quote_onchain,
};

pub async fn run_server(mint: Mint) -> anyhow::Result<()> {
    if let Some(ref buildtime) = mint.build_params.build_time {
        info!("build time: {}", buildtime);
    }

    if let Some(ref commithash) = mint.build_params.commit_hash {
        info!("git commit-hash: {}", commithash);
    }

    info!("listening on: {}", &mint.config.server.host_port);

    if let Some(ref onchain) = mint.config.onchain_backend {
        info!("onchain-min-confirmations: {}", onchain.min_confirmations);
        info!("onchain-min-amount: {}", onchain.min_amount);
        info!("onchain-max-amount: {}", onchain.max_amount);
    } else {
        info!("onchain-backend is not configured");
    }

    let listener = tokio::net::TcpListener::bind(&mint.config.server.host_port).await?;

    axum::serve(
        listener,
        app(mint)
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_headers(Any)
                    .allow_methods(Any)
                    .expose_headers(Any),
            )
            .into_make_service(),
    )
    .await?;

    Ok(())
}

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::onchain::post_mint_quote_onchain,
        crate::routes::onchain::get_mint_quote_onchain,
        crate::routes::onchain::post_mint_onchain,
        crate::routes::onchain::post_melt_quote_onchain,
        crate::routes::onchain::get_melt_quote_onchain,
        crate::routes::onchain::post_melt_onchain,
        crate::routes::default::post_swap,
        crate::routes::default::get_info,
        crate::routes::default::get_keysets,
        crate::routes::exchange::post_exchange,
    ),
    components(schemas(
        MintInfoResponse,
        CurrencyUnit,
        Keysets,
        Keyset,
        BlindedMessage,
        BlindedSignature,
        Proof,
        Proofs,
        P2SHScript,
        PostMintQuoteOnchainRequest,
        PostMintQuoteOnchainResponse,
        PostMeltQuoteOnchainRequest,
        PostMeltQuoteOnchainResponse,
        PostMeltOnchainRequest,
        PostMeltOnchainResponse,
        PostSwapRequest,
        PostSwapResponse,
        PostCurrencyExchangeRequest,
        PostCurrencyExchangeResponse,
    ))
)]
struct ApiDoc;

fn app(mint: Mint) -> Router {
    let default_routes = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/v1/keys", get(get_keys))
        .route("/v1/keys/:id", get(get_keys_by_id))
        .route("/v1/keysets", get(get_keysets))
        .route("/v1/swap", post(post_swap))
        .route("/v1/exchange", post(post_exchange))
        .route("/v1/checkstate", post(post_check_state))
        .route("/v1/info", get(get_info));

    let onchain_routes = {
        Router::new()
            .route("/v1/mint/quote/btconchain", post(post_mint_quote_onchain))
            .route(
                "/v1/mint/quote/btconchain/:quote",
                get(get_mint_quote_onchain),
            )
            .route("/v1/mint/btconchain", post(post_mint_onchain))
            .route("/v1/melt/quote/btconchain", post(post_melt_quote_onchain))
            .route(
                "/v1/melt/quote/btconchain/:quote",
                get(get_melt_quote_onchain),
            )
            .route("/v1/melt/btconchain", post(post_melt_onchain))
    };

    let general_routes = Router::new().route("/health", get(get_health));

    let server_config = mint.config.server.clone();
    let prefix = server_config.api_prefix.unwrap_or_else(|| "".to_owned());

    let router = Router::new()
        .nest(&prefix, default_routes)
        .nest(&prefix, onchain_routes)
        .nest("", general_routes)
        .with_state(mint);

    router
}

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "health check")
    ),
)]
async fn get_health() -> impl IntoResponse {
    StatusCode::OK
}

#[cfg(test)]
mod tests {

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use testcontainers::runners::AsyncRunner;
    use testcontainers::{ContainerAsync, ImageExt};
    use testcontainers_modules::postgres::Postgres;

    use crate::{
        config::{DatabaseConfig, MintConfig, MintInfoConfig},
        database::postgres::PostgresDB,
        mint::Mint,
        server::app,
    };
    use pretty_assertions::assert_eq;

    async fn create_postgres_image() -> anyhow::Result<ContainerAsync<Postgres>> {
        Ok(Postgres::default()
            .with_host_auth()
            .with_tag("16.6-alpine")
            .start()
            .await?)
    }

    async fn create_mock_db_empty(port: u16) -> anyhow::Result<PostgresDB> {
        let connection_string =
            &format!("postgres://postgres:postgres@127.0.0.1:{}/postgres", port);
        let db = PostgresDB::new(&DatabaseConfig {
            db_url: connection_string.to_owned(),
            ..Default::default()
        })
        .await?;
        db.migrate().await;
        Ok(db)
    }

    async fn create_mock_mint(info: MintInfoConfig, db_port: u16) -> anyhow::Result<Mint> {
        let db = create_mock_db_empty(db_port).await?;

        Ok(Mint::new(
            db,
            MintConfig {
                info,
                privatekey: "mytestsecret".to_string(),
                ..Default::default()
            },
            Default::default(),
        ))
    }

    #[tokio::test]
    async fn test_get_health() -> anyhow::Result<()> {
        let node = create_postgres_image().await?;

        let app =
            app(create_mock_mint(Default::default(), node.get_host_port_ipv4(5432).await?).await?);
        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty())?)
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        Ok(())
    }
}
