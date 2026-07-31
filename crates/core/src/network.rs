pub mod error;
pub mod handlers;
pub mod response;

pub use error::{ActionRequestError, CodedErrorObject};
pub use response::{
    ActionResponseDataSchema, BankActionResponseDataSchema, NpcActionResponseDataSchema,
};

pub use handlers::command_handler_router;

fn action_request_uri(
    character_name: &str,
    action_name: &str,
) -> Result<ureq::http::Uri, ActionRequestError> {
    ureq::http::Uri::builder()
        .scheme("https")
        .authority("api.artifactsmmo.com")
        .path_and_query(format!("/my/{character_name}/action/{action_name}",))
        .build()
        .map_err(Into::into)
}

fn action_post_request_builder(
    api_key: &str,
    character_name: &str,
    action_name: &str,
) -> Result<ureq::RequestBuilder<ureq::typestate::WithBody>, ActionRequestError> {
    action_request_uri(character_name, action_name)
        .inspect_err(|e| tracing::error!(?e))
        .map(ureq::post)
        .map(|req| {
            // apply the required headers, along with the api-key
            req.header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {api_key}"))
                .config()
                // make sure to config the request to not
                // eat the error payload!
                .http_status_as_error(false)
                .build()
        })
}

fn send_action_request_inner(
    api_key: &str,
    character_name: &str,
    action_name: &str,
    data: &[u8],
) -> Result<ureq::http::Response<ureq::Body>, ActionRequestError> {
    action_post_request_builder(api_key, character_name, action_name)
        .and_then(|req| req.send(data).map_err(Into::into))
}

#[tracing::instrument(level = "info", skip(api_key))]
fn send_action_request<Response>(
    api_key: &str,
    character_name: &str,
    action_name: &str,
    data: &[u8],
) -> Result<Response, ActionRequestError>
where
    Response: serde::de::DeserializeOwned,
{
    send_action_request_inner(api_key, character_name, action_name, data).and_then(|res| {
        let status = res.status();
        tracing::info!("status is {status}");
        let rdr = res.into_body().into_reader();
        if status.is_client_error() | status.is_server_error() {
            /// Simple wrapper type that the API returns in this case, no need
            /// to pollute local code to have this functionality
            #[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
            struct CodedErrorObjectResponse {
                error: CodedErrorObject,
            }

            let output: CodedErrorObjectResponse = serde_json::from_reader(rdr)?;
            Err(ActionRequestError::ApiError(output.error))
        } else {
            // tracing::info!("status is good!");
            let output: Response = serde_json::from_reader(rdr)?;
            Ok(output)
        }
    })
}
