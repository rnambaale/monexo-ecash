use crate::mint::Mint;
use monexo_core::blind::{BlindedMessage, BlindedSignature};
use monexo_core::keyset::{Keyset, Keysets};
use monexo_core::primitives::{CurrencyUnit, PostMintQuoteBtcOnchainRequest, PostMintQuoteBtcOnchainResponse};
use monexo_core::proof::{P2SHScript, Proof, Proofs};
use tracing::info;

use axum::Router;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};

use tower_http::cors::{Any, CorsLayer};
use utoipa_swagger_ui::SwaggerUi;
use utoipa::OpenApi;

use crate::routes::btconchain::{get_mint_quote_btconchain, post_mint_btconchain, post_mint_quote_btconchain};

pub async fn run_server(mint: Mint) -> anyhow::Result<()> {
    if let Some(ref buildtime) = mint.build_params.build_time {
        info!("build time: {}", buildtime);
    }

    if let Some(ref commithash) = mint.build_params.commit_hash {
        info!("git commit-hash: {}", commithash);
    }

    info!("listening on: {}", &mint.config.server.host_port);

    if let Some(ref onchain) = mint.config.btconchain_backend {
        info!(
            "btconchain-min-confirmations: {}",
            onchain.min_confirmations
        );
        info!("btconchain-min-amount: {}", onchain.min_amount);
        info!("btconchain-max-amount: {}", onchain.max_amount);
    } else {
        info!("btconchain-backend is not configured");
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
        crate::routes::btconchain::post_mint_quote_btconchain,
        crate::routes::btconchain::get_mint_quote_btconchain,
        crate::routes::btconchain::post_mint_btconchain,
        // crate::routes::btconchain::post_melt_quote_btconchain,
        // crate::routes::btconchain::get_melt_quote_btconchain,
        // crate::routes::btconchain::post_melt_btconchain,
    ),
    components(schemas(
        CurrencyUnit,
        Keysets,
        Keyset,
        BlindedMessage,
        BlindedSignature,
        Proof,
        Proofs,
        P2SHScript,
        PostMintQuoteBtcOnchainRequest,
        PostMintQuoteBtcOnchainResponse,
    ))
)]
struct ApiDoc;

fn app(mint: Mint) -> Router {
    let default_routes = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // .route("/v1/keys", get(get_keys))
        // .route("/v1/keys/:id", get(get_keys_by_id))
        // .route("/v1/keysets", get(get_keysets))
        // .route("/v1/mint/quote/bolt11", post(post_mint_quote_bolt11))
        // .route("/v1/mint/quote/bolt11/:quote", get(get_mint_quote_bolt11))
        // .route("/v1/mint/bolt11", post(post_mint_bolt11))
        // .route("/v1/melt/quote/bolt11", post(post_melt_quote_bolt11))
        // .route("/v1/melt/quote/bolt11/:quote", get(get_melt_quote_bolt11))
        // .route("/v1/melt/bolt11", post(post_melt_bolt11))
        // .route("/v1/swap", post(post_swap))
        // .route("/v1/info", get(get_info))
        ;

    let btconchain_routes = {
        Router::new()
            .route(
                "/v1/mint/quote/btconchain",
                post(post_mint_quote_btconchain),
            )
            .route(
                "/v1/mint/quote/btconchain/:quote",
                get(get_mint_quote_btconchain),
            )
            .route("/v1/mint/btconchain", post(post_mint_btconchain))
            // .route(
            //     "/v1/melt/quote/btconchain",
            //     post(post_melt_quote_btconchain),
            // )
            // .route(
            //     "/v1/melt/quote/btconchain/:quote",
            //     get(get_melt_quote_btconchain),
            // )
            // .route("/v1/melt/btconchain", post(post_melt_btconchain))
    };

    let general_routes = Router::new().route("/health", get(get_health));

    let server_config = mint.config.server.clone();
    let prefix = server_config.api_prefix.unwrap_or_else(|| "".to_owned());

    let router = Router::new()
        .nest(&prefix, default_routes)
        .nest(&prefix, btconchain_routes)
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
        config::{DatabaseConfig, MintConfig, MintInfoConfig}, database::postgres::PostgresDB, mint::Mint, server::app
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
