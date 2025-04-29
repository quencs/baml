use serde::{Deserialize, Serialize};

// use crate::bhttp::{AuthzScope, ResolveAuthzScope};
use crate::rpc::ApiEndpoint;
use crate::s3::S3UploadMetadata;
use crate::ProjectId;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTraceEventUploadUrlRequest {}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTraceEventUploadUrlResponse {
    pub upload_url: String,
    pub upload_metadata: S3UploadMetadata,
}

pub struct CreateTraceEventUploadUrl;

// POST /v1/baml-trace/create-upload-url
impl ApiEndpoint for CreateTraceEventUploadUrl {
    type Request<'a> = CreateTraceEventUploadUrlRequest;
    type Response<'a> = CreateTraceEventUploadUrlResponse;

    const PATH: &'static str = "/v1/baml-trace/create-upload-url";
}

// Provide a stub trait and enum if not available in this crate
#[allow(dead_code)]
pub enum AuthzScope {
    OrgId(String),
    ProjectId(ProjectId),
}

#[allow(dead_code)]
pub trait ResolveAuthzScope {
    fn resolve_authz_scope(&self) -> AuthzScope;
}

impl ResolveAuthzScope for CreateTraceEventUploadUrlRequest {
    fn resolve_authz_scope(&self) -> AuthzScope {
        AuthzScope::ProjectId(ProjectId::new())
    }
}
