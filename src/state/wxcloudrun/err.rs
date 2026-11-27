#[inline]
pub fn invalid_upload_path() -> crate::Error {
    crate::Error::custom(43101, "Invalid CloudBase upload environment or path")
}

#[inline]
pub fn upload_link_request_failed() -> crate::Error {
    crate::Error::custom(53101, "CloudBase open-interface upload-link request failed")
}

#[inline]
pub fn upload_link_response_invalid() -> crate::Error {
    crate::Error::custom(53102, "CloudBase upload-link response is invalid")
}

#[inline]
pub fn cloud_call_rejected(code: i64) -> crate::Error {
    crate::Error::custom(
        53103,
        format!("WeChat rejected the upload-link request (errcode {code})"),
    )
}

#[inline]
pub fn download_call_rejected(code: i64) -> crate::Error {
    crate::Error::custom(
        53111,
        format!("WeChat rejected the temporary-download request (errcode {code})"),
    )
}

#[inline]
pub fn missing_environment() -> crate::Error {
    crate::Error::custom(53105, "Missing [wxcloudrun].env or WXCLOUDRUN_ENV")
}

#[inline]
pub fn invalid_prefix() -> crate::Error {
    crate::Error::custom(53106, "Invalid [wxcloudrun].prefix")
}

#[inline]
pub fn invalid_url_expire() -> crate::Error {
    crate::Error::custom(
        53107,
        "Invalid [wxcloudrun].url_expire; expected 1 through 604800 seconds",
    )
}

#[inline]
pub fn download_link_request_failed() -> crate::Error {
    crate::Error::custom(53108, "CloudBase temporary-download request failed")
}

#[inline]
pub fn download_link_response_invalid() -> crate::Error {
    crate::Error::custom(53109, "CloudBase temporary-download response is invalid")
}

#[inline]
pub fn file_download_link_rejected(code: i64) -> crate::Error {
    crate::Error::custom(
        53110,
        format!("WeChat rejected the file download-link request (errcode {code})"),
    )
}

#[inline]
pub fn invalid_file_id() -> crate::Error {
    crate::Error::custom(43102, "Invalid CloudBase file ID")
}

#[inline]
pub fn invalid_image_process() -> crate::Error {
    crate::Error::custom(43103, "Invalid COS image-processing command")
}
