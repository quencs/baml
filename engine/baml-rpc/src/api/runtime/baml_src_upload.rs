use serde::{Deserialize, Serialize};

use crate::{
    ast::tops::{ASTId, AST},
    rpc::ApiEndpoint,
};

#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct CreateBamlSrcUploadRequest<'a> {
    pub baml_ast_id: ASTId<'a>,
    pub ast: &'a AST,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CreateBamlSrcUploadResponse<'a> {
    pub project_id: String,
    pub baml_ast_id: ASTId<'a>,
}

pub struct CreateBamlSrcUpload;

/// POST /v1/baml-src/upload
impl ApiEndpoint for CreateBamlSrcUpload {
    type Request<'a> = CreateBamlSrcUploadRequest<'a>;
    type Response<'a> = CreateBamlSrcUploadResponse<'a>;

    const PATH: &'static str = "/v1/baml-src/upload";
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckBamlSrcUploadStatus {
    DoesNotExist,
    Exists,
    // TODO: add partial information for better diffs
    // and to only send the diffs to the client
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CheckBamlSrcUploadRequest<'a> {
    pub baml_ast_id: ASTId<'a>,
}

pub struct CheckBamlSrcUpload;

/// POST /v1/baml-src/check-upload
impl ApiEndpoint for CheckBamlSrcUpload {
    type Request<'a> = CheckBamlSrcUploadRequest<'a>;
    type Response<'a> = CheckBamlSrcUploadStatus;

    const PATH: &'static str = "/v1/baml-src/check-upload";
}
