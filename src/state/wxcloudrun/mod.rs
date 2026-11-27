mod err;

use serde::{Deserialize, Serialize};

const UPLOAD_LINK_URL: &str = "http://api.weixin.qq.com/tcb/uploadfile";
const DOWNLOAD_LINK_URL: &str = "http://api.weixin.qq.com/tcb/batchdownloadfile";
const DEFAULT_URL_EXPIRE_SECONDS: u64 = 3600;
const MAX_URL_EXPIRE_SECONDS: u64 = 604800;

#[derive(Clone)]
pub struct WxCloudRun {
    client: reqwest::Client,
    env: String,
    prefix: String,
    url_expire_seconds: u64,
}

impl WxCloudRun {
    pub fn from_table(table: &toml::Table) -> crate::Result<Self> {
        let map = table
            .get("wxcloudrun")
            .and_then(toml::Value::as_table)
            .cloned()
            .unwrap_or_default();

        let env = match (map.get("env"), std::env::var("WXCLOUDRUN_ENV")) {
            (_, Ok(value)) => value,
            (Some(value), Err(std::env::VarError::NotPresent)) => value
                .as_str()
                .ok_or_else(|| crate::Error::custom(50001, "[wxcloudrun].env must be a string"))?
                .to_owned(),
            (None, Err(std::env::VarError::NotPresent)) => {
                return Err(crate::Error::custom(
                    50001,
                    "missing required configuration [wxcloudrun].env or WXCLOUDRUN_ENV",
                ));
            }
            (_, Err(std::env::VarError::NotUnicode(_))) => {
                return Err(crate::Error::custom(
                    50001,
                    "WXCLOUDRUN_ENV is not valid UTF-8",
                ));
            }
        };
        let env = env.trim();
        if env.is_empty() {
            return Err(err::missing_environment());
        }

        let prefix = match (map.get("prefix"), std::env::var("WXCLOUDRUN_PREFIX")) {
            (_, Ok(value)) => value,
            (Some(value), Err(std::env::VarError::NotPresent)) => value
                .as_str()
                .ok_or_else(|| crate::Error::custom(50001, "[wxcloudrun].prefix must be a string"))?
                .to_owned(),
            (None, Err(std::env::VarError::NotPresent)) => String::new(),
            (_, Err(std::env::VarError::NotUnicode(_))) => {
                return Err(crate::Error::custom(
                    50001,
                    "WXCLOUDRUN_PREFIX is not valid UTF-8",
                ));
            }
        }
        .trim_matches('/')
        .to_owned();
        if !prefix.is_empty() && !valid_cloud_path(&prefix) {
            return Err(err::invalid_prefix());
        }

        let url_expire_seconds = match (
            map.get("url_expire"),
            std::env::var("WXCLOUDRUN_URL_EXPIRE"),
        ) {
            (_, Ok(value)) => value
                .parse::<u64>()
                .ok()
                .filter(|seconds| (1..=MAX_URL_EXPIRE_SECONDS).contains(seconds))
                .ok_or_else(err::invalid_url_expire)?,
            (Some(value), Err(std::env::VarError::NotPresent)) => value
                .as_integer()
                .and_then(|seconds| u64::try_from(seconds).ok())
                .filter(|seconds| (1..=MAX_URL_EXPIRE_SECONDS).contains(seconds))
                .ok_or_else(err::invalid_url_expire)?,
            (None, Err(std::env::VarError::NotPresent)) => DEFAULT_URL_EXPIRE_SECONDS,
            (_, Err(std::env::VarError::NotUnicode(_))) => {
                return Err(crate::Error::custom(
                    50001,
                    "WXCLOUDRUN_URL_EXPIRE is not valid UTF-8",
                ));
            }
        };

        Ok(Self {
            client: super::default_http_client(),
            env: env.to_owned(),
            prefix,
            url_expire_seconds,
        })
    }

    pub fn env(&self) -> &str {
        &self.env
    }

    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    pub async fn get_upload_file_link(&self, relative_path: &str) -> crate::Result<UploadFileLink> {
        let path = self.object_path(relative_path)?;
        let response = self
            .client
            .post(UPLOAD_LINK_URL)
            .json(&UploadLinkRequest {
                env: &self.env,
                path: &path,
            })
            .send()
            .await
            .map_err(|_error| {
                #[cfg(feature = "log")]
                let _ = crate::state::log_request_error(
                    _error,
                    53101,
                    "CloudBase upload-link request failed",
                );
                err::upload_link_request_failed()
            })?;

        if !response.status().is_success() {
            #[cfg(feature = "log")]
            tracing::error!(
                code = 53101,
                http_status = %response.status(),
                "CloudBase upload-link request returned an error status"
            );
            return Err(err::upload_link_request_failed());
        }
        let openapi_seqid = openapi_seqid(&response);
        let response = response
            .json::<UploadLinkResponse>()
            .await
            .map_err(|_error| {
                #[cfg(feature = "log")]
                let _ = crate::state::log_request_error(
                    _error,
                    53102,
                    "CloudBase upload-link response parse failed",
                );
                err::upload_link_response_invalid()
            })?;

        if response.errcode != 0 {
            return Err(err::cloud_call_rejected(response.errcode));
        }

        Ok(UploadFileLink {
            url: required_field(response.url)?,
            token: required_field(response.token)?,
            authorization: required_field(response.authorization)?,
            file_id: required_field(response.file_id)?,
            cos_file_id: required_field(response.cos_file_id)?,
            path,
            openapi_seqid,
        })
    }

    pub async fn get_temporary_access_url(
        &self,
        file_id: &str,
    ) -> crate::Result<TemporaryAccessUrl> {
        if !self.owns_file_id(file_id) {
            return Err(err::invalid_file_id());
        }

        let response = self
            .client
            .post(DOWNLOAD_LINK_URL)
            .json(&DownloadLinkRequest {
                env: &self.env,
                file_list: [DownloadFileRequest {
                    fileid: file_id,
                    max_age: self.url_expire_seconds,
                }],
            })
            .send()
            .await
            .map_err(|_error| {
                #[cfg(feature = "log")]
                let _ = crate::state::log_request_error(
                    _error,
                    53108,
                    "CloudBase temporary-download request failed",
                );
                err::download_link_request_failed()
            })?;

        if !response.status().is_success() {
            #[cfg(feature = "log")]
            tracing::error!(
                code = 53108,
                http_status = %response.status(),
                "CloudBase temporary-download request returned an error status"
            );
            return Err(err::download_link_request_failed());
        }
        let openapi_seqid = openapi_seqid(&response);
        let response = response
            .json::<DownloadLinkResponse>()
            .await
            .map_err(|_error| {
                #[cfg(feature = "log")]
                let _ = crate::state::log_request_error(
                    _error,
                    53109,
                    "CloudBase temporary-download response parse failed",
                );
                err::download_link_response_invalid()
            })?;

        if response.errcode != 0 {
            return Err(err::download_call_rejected(response.errcode));
        }

        let file = response
            .file_list
            .into_iter()
            .next()
            .ok_or_else(err::download_link_response_invalid)?;
        if file.status != 0 {
            return Err(err::file_download_link_rejected(file.status));
        }
        let url =
            required_field(file.download_url).map_err(|_| err::download_link_response_invalid())?;
        let parsed_url =
            reqwest::Url::parse(&url).map_err(|_| err::download_link_response_invalid())?;
        if parsed_url.scheme() != "https" || parsed_url.host_str().is_none() {
            return Err(err::download_link_response_invalid());
        }

        Ok(TemporaryAccessUrl {
            file_id: file.fileid,
            url,
            expires_in_seconds: self.url_expire_seconds,
            openapi_seqid,
        })
    }

    pub async fn get_temporary_access_url_with_image_process(
        &self,
        file_id: &str,
        process: &str,
    ) -> crate::Result<TemporaryAccessUrl> {
        let mut access = self.get_temporary_access_url(file_id).await?;
        access.url = append_cos_image_process(&access.url, process)?;
        Ok(access)
    }

    fn object_path(&self, relative_path: &str) -> crate::Result<String> {
        let relative_path = relative_path.trim();
        if !valid_cloud_path(relative_path) {
            return Err(err::invalid_upload_path());
        }
        if self.prefix.is_empty() {
            Ok(relative_path.to_owned())
        } else {
            Ok(format!("{}/{relative_path}", self.prefix))
        }
    }

    fn owns_file_id(&self, file_id: &str) -> bool {
        let Some(file_id) = file_id.strip_prefix(&format!("cloud://{}.", self.env)) else {
            return false;
        };
        let Some((_, path)) = file_id.split_once('/') else {
            return false;
        };
        self.prefix.is_empty()
            || path == self.prefix
            || path
                .strip_prefix(&self.prefix)
                .is_some_and(|rest| rest.starts_with('/'))
    }
}

#[derive(Debug, Clone)]
pub struct UploadFileLink {
    pub url: String,
    pub token: String,
    pub authorization: String,
    pub file_id: String,
    pub cos_file_id: String,
    pub path: String,
    pub openapi_seqid: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TemporaryAccessUrl {
    pub file_id: String,
    pub url: String,
    pub expires_in_seconds: u64,
    pub openapi_seqid: Option<String>,
}

#[derive(Serialize)]
struct UploadLinkRequest<'a> {
    env: &'a str,
    path: &'a str,
}

#[derive(Deserialize)]
struct UploadLinkResponse {
    errcode: i64,
    #[serde(default)]
    url: String,
    #[serde(default)]
    token: String,
    #[serde(default)]
    authorization: String,
    #[serde(default)]
    file_id: String,
    #[serde(default)]
    cos_file_id: String,
}

#[derive(Serialize)]
struct DownloadLinkRequest<'a> {
    env: &'a str,
    file_list: [DownloadFileRequest<'a>; 1],
}

#[derive(Serialize)]
struct DownloadFileRequest<'a> {
    fileid: &'a str,
    max_age: u64,
}

#[derive(Deserialize)]
struct DownloadLinkResponse {
    errcode: i64,
    #[serde(default)]
    file_list: Vec<DownloadFileResponse>,
}

#[derive(Deserialize)]
struct DownloadFileResponse {
    fileid: String,
    download_url: String,
    status: i64,
}

fn openapi_seqid(response: &reqwest::Response) -> Option<String> {
    response
        .headers()
        .get("x-openapi-seqid")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn required_field(value: String) -> crate::Result<String> {
    if value.is_empty() {
        return Err(err::upload_link_response_invalid());
    }
    Ok(value)
}

fn append_cos_image_process(url: &str, process: &str) -> crate::Result<String> {
    if process.is_empty()
        || process.len() > 4096
        || !process.is_ascii()
        || process.bytes().any(|byte| {
            byte.is_ascii_control() || matches!(byte, b'?' | b'#' | b'&' | b'+' | b' ' | b'\\')
        })
    {
        return Err(err::invalid_image_process());
    }

    let bytes = process.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit()
            {
                return Err(err::invalid_image_process());
            }
            index += 3;
        } else {
            index += 1;
        }
    }

    let mut parsed_url =
        reqwest::Url::parse(url).map_err(|_| err::download_link_response_invalid())?;
    let query = parsed_url.query().unwrap_or_default();
    let query = if query.is_empty() {
        process.to_owned()
    } else {
        format!("{query}&{process}")
    };
    parsed_url.set_query(Some(&query));
    Ok(parsed_url.into())
}

fn valid_cloud_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains('\0')
        && path
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        _lock: MutexGuard<'static, ()>,
        previous: [(&'static str, Option<std::ffi::OsString>); 3],
    }

    impl EnvGuard {
        fn clean() -> Self {
            let lock = ENV_LOCK.lock().unwrap();
            let names = [
                "WXCLOUDRUN_ENV",
                "WXCLOUDRUN_PREFIX",
                "WXCLOUDRUN_URL_EXPIRE",
            ];
            let previous = names.map(|name| (name, std::env::var_os(name)));
            unsafe {
                for name in names {
                    std::env::remove_var(name);
                }
            }
            Self {
                _lock: lock,
                previous,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe {
                for (name, value) in &self.previous {
                    if let Some(value) = value {
                        std::env::set_var(name, value);
                    } else {
                        std::env::remove_var(name);
                    }
                }
            }
        }
    }

    #[test]
    fn reads_env_prefix_and_temporary_url_expiry_from_config() {
        let _env = EnvGuard::clean();
        let table = toml::from_str::<toml::Table>(
            r#"
                [wxcloudrun]
                env = "wx-env-id"
                prefix = "items/images"
                url_expire = 7200
            "#,
        )
        .unwrap();
        let cloudrun = WxCloudRun::from_table(&table).unwrap();

        assert_eq!(cloudrun.env(), "wx-env-id");
        assert_eq!(cloudrun.prefix(), "items/images");
        assert_eq!(cloudrun.url_expire_seconds, 7200);
        assert_eq!(
            cloudrun.object_path("usr_1/photo.png").unwrap(),
            "items/images/usr_1/photo.png"
        );
    }

    #[test]
    fn environment_variables_override_toml_and_can_supply_required_env() {
        let _env = EnvGuard::clean();
        let table = toml::from_str::<toml::Table>(
            r#"
                [wxcloudrun]
                env = "env-from-toml"
                prefix = "toml-prefix"
                url_expire = 7200
            "#,
        )
        .unwrap();
        unsafe {
            std::env::set_var("WXCLOUDRUN_ENV", " env-from-variable ");
            std::env::set_var("WXCLOUDRUN_PREFIX", "env-prefix/");
            std::env::set_var("WXCLOUDRUN_URL_EXPIRE", "1800");
        }
        let cloudrun = WxCloudRun::from_table(&table).unwrap();

        assert_eq!(cloudrun.env(), "env-from-variable");
        assert_eq!(cloudrun.prefix(), "env-prefix");
        assert_eq!(cloudrun.url_expire_seconds, 1800);

        let env_only = WxCloudRun::from_table(&toml::Table::new()).unwrap();
        assert_eq!(env_only.env(), "env-from-variable");
        assert_eq!(env_only.prefix(), "env-prefix");
        assert_eq!(env_only.url_expire_seconds, 1800);
    }

    #[test]
    fn rejects_invalid_prefix_and_upload_paths() {
        let _env = EnvGuard::clean();
        assert!(!valid_cloud_path("/absolute/file.png"));
        assert!(!valid_cloud_path("uploads/../private.png"));
        assert!(!valid_cloud_path("uploads\\file.png"));
        assert!(valid_cloud_path("items/user-1/photo.png"));
    }

    #[test]
    fn only_accepts_file_ids_for_configured_env_and_prefix() {
        let _env = EnvGuard::clean();
        let cloudrun = WxCloudRun::from_table(
            &toml::from_str::<toml::Table>(
                r#"
                    [wxcloudrun]
                    env = "wx-env-id"
                    prefix = "items/images"
                "#,
            )
            .unwrap(),
        )
        .unwrap();

        assert!(cloudrun.owns_file_id("cloud://wx-env-id.bucket/items/images/a.png"));
        assert!(!cloudrun.owns_file_id("cloud://other-env.bucket/items/images/a.png"));
        assert!(!cloudrun.owns_file_id("cloud://wx-env-id.bucket/private/a.png"));
    }

    #[test]
    fn appends_cos_image_processing_to_temporary_url_query() {
        let url = append_cos_image_process(
            "https://bucket.example/photo.jpg?sign=temporary&t=123",
            "imageMogr2/thumbnail/!50p/quality/85/format/webp",
        )
        .unwrap();

        assert_eq!(
            url,
            "https://bucket.example/photo.jpg?sign=temporary&t=123&imageMogr2/thumbnail/!50p/quality/85/format/webp"
        );
    }

    #[test]
    fn rejects_cos_image_processing_query_injection_and_invalid_escapes() {
        for process in [
            "",
            "?imageMogr2/thumbnail/!50p",
            "imageMogr2/thumbnail/!50p&admin=true",
            "imageMogr2/thumbnail/!50p+admin",
            "watermark/%GG",
            "imageMogr2/thumbnail/!50p#fragment",
        ] {
            assert!(append_cos_image_process("https://bucket.example/photo.jpg", process).is_err());
        }
    }
}
